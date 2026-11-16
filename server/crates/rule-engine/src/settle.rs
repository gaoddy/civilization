//! 回合结算：配对、把战斗接入回合、伤害、淘汰与排名。
//!
//! 规则口径见 `docs/versions/v0.1/product/rules/core-rules.md`
//! §5.3–§5.4、§7、§8.3。配对随机性只来自 [`Stream::Matchmaking`]，
//! 战斗随机性全部发生在 [`crate::combat::simulate`] 内部；本模块不含浮点，
//! 也不读取时间、环境或全局随机源。

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use crate::combat;
use crate::data::{Catalog, Era, GameConfig};
use crate::effects::Duration;
use crate::engine::Engine;
use crate::error::EngineError;
use crate::events::GameEvent;
use crate::rng::Stream;
use crate::state::{
    AuraEffect, GameState, HandCard, InstanceId, MinionInstance, PendingChoice, Phase, PlayerState,
    Seat,
};

/// 配对在放弃"排除上一场对手"前最多重洗的次数。
const PAIRING_ATTEMPTS: u8 = 100;

/// 一对配对。
///
/// `ghost.is_some()` 时 `b == a`，表示 `a` 对战该出局座位的幽灵阵容；
/// 真实战 `ghost == None`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pairing {
    pub a: Seat,
    pub b: Seat,
    /// 幽灵战的幽灵归属座位（已出局者）；真实战为 `None`。
    pub ghost: Option<Seat>,
}

impl Pairing {
    /// 是否幽灵战：落单配对（`a == b`）或已标注幽灵归属座位。
    ///
    /// `pair_round` 的重试阶段幽灵尚未抽取，此时仅由 `a == b` 成立；
    /// 结算与视图统一用本判据，避免两处各写一套。
    pub fn is_ghost(&self) -> bool {
        self.a == self.b || self.ghost.is_some()
    }
}

/// 单座位视角的一场战斗报告。
///
/// 报告只由 [`Engine::resolve_combat`] 的本次调用返回，不写入 [`GameState`]，
/// 因此不参与 `state_hash`。阵容为战前快照，且已折叠光环——客户端看到的是
/// 实际参战的数值。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeatCombatReport {
    pub seat: Seat,
    /// 真实战为对手座位；幽灵战为幽灵归属座位。
    pub opponent_seat: Option<Seat>,
    pub mirror: bool,
    /// 本方战前阵容（已折叠光环）。
    pub own_board: Vec<MinionInstance>,
    /// 真实战为对手战前阵容；幽灵战为幽灵阵容。
    pub opponent_board: Vec<MinionInstance>,
    /// 本场战斗的行动序列。
    pub actions: Vec<combat::CombatAction>,
    /// 与 `actions` 同序同长：每条动作执行后受影响随从的状态。
    pub states: Vec<Vec<combat::MinionSnapshot>>,
    /// 本场战斗造成的伤害；平局为 0。
    pub damage: i32,
    /// 本场胜者；平局为 `None`。
    pub winner: Option<Seat>,
}

/// 一次战斗结算的完整产物：事件流与逐座位报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatOutcome {
    pub events: Vec<GameEvent>,
    pub reports: BTreeMap<Seat, SeatCombatReport>,
}

impl Engine {
    /// 结束招募：生成本回合配对并进入 `Combat`。只做配对，不模拟战斗。
    pub fn finish_recruit(&mut self, state: &mut GameState) -> Result<Vec<GameEvent>, EngineError> {
        if state.phase == Phase::Recruit {
            self.recompute_auras(state);
        }
        finish_recruit(state)
    }

    /// 按配对顺序模拟全部战斗，写入伤害、对手与阵容快照，进入 `Settle`。
    ///
    /// 返回的事件流与逐座位战斗报告；报告不写入 `GameState`，不影响 `state_hash`。
    pub fn resolve_combat(&mut self, state: &mut GameState) -> Result<CombatOutcome, EngineError> {
        resolve_combat(self, state)
    }

    /// 结算伤害、淘汰与排名；正常结束时停留在 `Settle`，游戏结束时进入 `GameOver`。
    pub fn resolve_settle(&mut self, state: &mut GameState) -> Result<Vec<GameEvent>, EngineError> {
        let result = resolve_settle(&self.catalog.config, state);
        self.recompute_auras(state);
        result
    }
}

/// 为存活玩家生成配对。
///
/// 存活玩家按座位升序排列后用 Fisher–Yates 洗牌，相邻两项成对；
/// 奇数人数时洗牌结果最后一名获得落单配对，并在整轮洗牌结束后从全部出局者
/// 中随机抽一名作为其幽灵。整轮最多重洗 [`PAIRING_ATTEMPTS`] 次以避开上一场
/// 对手，全部失败时采用最后一次洗牌。
/// 本函数只读取 `state.rng`、`state.previous_opponent` 与 `state.ranking`。
pub fn pair_round(state: &mut GameState) -> Vec<Pairing> {
    let mut alive: Vec<Seat> = state
        .players
        .iter()
        .filter(|player| !player.eliminated && player.hp > 0)
        .map(|player| player.seat)
        .collect();
    alive.sort_unstable();

    if alive.is_empty() {
        return Vec::new();
    }

    let mut pairings = Vec::new();
    for _ in 0..PAIRING_ATTEMPTS {
        fisher_yates(&mut alive, state);
        pairings = pair_adjacent(&alive);
        if !repeats_previous_opponent(&pairings, &state.previous_opponent) {
            break;
        }
    }

    // 只有存在落单配对时才抽一次幽灵；偶数人数局面的随机流保持不变。
    let ghost = if pairings.iter().any(Pairing::is_ghost) {
        ghost_seat(state)
    } else {
        None
    };
    for pairing in &mut pairings {
        if pairing.is_ghost() {
            pairing.ghost = ghost;
        }
    }
    pairings
}

/// 从全部出局者里随机抽一名作为幽灵归属座位。
///
/// `ranking` 是出局顺序（最近出局在前），作为候选全集；空集合返回 `None`，
/// 落单座位对战空阵容。抽取走 [`Stream::Matchmaking`]。
fn ghost_seat(state: &mut GameState) -> Option<Seat> {
    if state.ranking.is_empty() {
        return None;
    }
    let bound = state.ranking.len() as u64;
    let index = state.rng.below(Stream::Matchmaking, bound) as usize;
    state.ranking.get(index).copied()
}

/// 受到伤害 = 败者时代 + ceil(胜方存活随从星级和 / 2)，并夹在配置的上下限内。
pub fn damage_formula(config: &GameConfig, loser_era: Era, winner_stars_sum: u32) -> i32 {
    let era = loser_era as i32;
    let stars = winner_stars_sum.saturating_add(1) / 2;
    era.saturating_add(stars as i32)
        .max(config.damage_floor)
        .min(config.damage_cap)
}

fn fisher_yates(alive: &mut [Seat], state: &mut GameState) {
    for index in (1..alive.len()).rev() {
        let bound = (index + 1) as u64;
        let other = state.rng.below(Stream::Matchmaking, bound) as usize;
        alive.swap(index, other);
    }
}

fn pair_adjacent(alive: &[Seat]) -> Vec<Pairing> {
    let mut pairings = Vec::with_capacity(alive.len().div_ceil(2));
    let mut index = 0;
    while index + 1 < alive.len() {
        pairings.push(Pairing {
            a: alive[index],
            b: alive[index + 1],
            ghost: None,
        });
        index += 2;
    }
    if let Some(&seat) = alive.get(index) {
        pairings.push(Pairing {
            a: seat,
            b: seat,
            ghost: None,
        });
    }
    pairings
}

fn repeats_previous_opponent(
    pairings: &[Pairing],
    previous: &BTreeMap<Seat, Option<Seat>>,
) -> bool {
    pairings.iter().any(|pairing| {
        // 重试循环内 ghost 尚未填充，`is_ghost()` 此时等价于 `a == b`；
        // 落单配对不参与「上一场对手」比较。
        if pairing.is_ghost() {
            return false;
        }
        previous_opponent(previous, pairing.a) == Some(pairing.b)
            || previous_opponent(previous, pairing.b) == Some(pairing.a)
    })
}

fn previous_opponent(previous: &BTreeMap<Seat, Option<Seat>>, seat: Seat) -> Option<Seat> {
    previous.get(&seat).copied().flatten()
}

/// 配对并进入 `Combat`。全员 `EndRecruit` 时阶段已由 `end_recruit` 推进到 `Combat`
/// （`PhaseChanged` 已发出），这里只补齐配对；`Recruit` 阶段进入本函数（超时推进）时
/// 生成配对并发出 `PhaseChanged`。
///
/// `Combat` 分支幂等：已有配对时直接返回空事件，不重新配对（不消耗 matchmaking 流、
/// 不覆盖对战），重复调用不改变状态。
fn finish_recruit(state: &mut GameState) -> Result<Vec<GameEvent>, EngineError> {
    if state.phase == Phase::Combat && !state.pairings.is_empty() {
        return Ok(Vec::new());
    }
    if state.phase != Phase::Recruit && state.phase != Phase::Combat {
        return Err(EngineError(format!(
            "当前阶段 {:?} 不能结束招募",
            state.phase
        )));
    }
    // 目标待选引用本回合刚入场的随从，跨回合后棋盘已变：进入战斗前一律作废（战吼落空）。
    // 发现与进化待选允许玩家下回合再选，保持现状。
    state
        .pending
        .retain(|pending| !matches!(pending, PendingChoice::Target { .. }));
    state.pairings = pair_round(state);
    if state.phase == Phase::Combat {
        return Ok(Vec::new());
    }
    state.phase = Phase::Combat;
    Ok(vec![GameEvent::PhaseChanged {
        phase: Phase::Combat,
        turn: state.turn,
    }])
}

fn resolve_combat(engine: &Engine, state: &mut GameState) -> Result<CombatOutcome, EngineError> {
    if state.phase != Phase::Combat {
        return Err(EngineError(format!(
            "当前阶段 {:?} 不能结算战斗",
            state.phase
        )));
    }

    // `last_won` 描述"本回合赢下自己战斗"的座位，每回合重新统计。
    state.last_won.clear();

    let pairings = state.pairings.clone();
    let mut events = Vec::new();
    let mut reports = BTreeMap::new();
    for pairing in pairings {
        if pairing.is_ghost() {
            resolve_ghost(engine, state, pairing, &mut events, &mut reports);
        } else {
            resolve_real(engine, state, pairing, &mut events, &mut reports);
        }
    }

    state.phase = Phase::Settle;
    events.push(GameEvent::PhaseChanged {
        phase: Phase::Settle,
        turn: state.turn,
    });
    Ok(CombatOutcome { events, reports })
}

/// 本场真实战的胜者：`None` 表示平局。
///
/// 双方同回合清场（两侧 survivors 都为空）一律平局；行动数到上限而双方仍有
/// 存活者时仍按 §5.3 的生命 → 累计伤害 → 座位判胜。
fn seat_winner(state: &GameState, log: &combat::CombatLog, a: Seat, b: Seat) -> Option<Seat> {
    match log.winner {
        Some(winner) => Some(winner),
        None if log.attacker_survivors.is_empty() && log.defender_survivors.is_empty() => None,
        None => Some(tie_break(state, a, b)),
    }
}

fn resolve_real(
    engine: &Engine,
    state: &mut GameState,
    pairing: Pairing,
    events: &mut Vec<GameEvent>,
    reports: &mut BTreeMap<Seat, SeatCombatReport>,
) {
    let catalog = &engine.catalog;
    let (a, b) = (pairing.a, pairing.b);
    // 快照必须在战斗改变棋盘之前截取；光环只折叠进战斗克隆。
    let board_a = fold_auras(catalog, &state.auras, board_of(state, a));
    let board_b = fold_auras(catalog, &state.auras, board_of(state, b));

    let first = match state.last_winner {
        Some(winner) if winner == a || winner == b => winner,
        _ => state.first_seat,
    };
    let attacker_hero = hero_of(state, a);
    let defender_hero = hero_of(state, b);
    let setup = combat::CombatSetup {
        attacker_hero: attacker_hero.as_deref(),
        defender_hero: defender_hero.as_deref(),
        next_instance_id: Some(state.next_instance_id),
        pool: Some(&state.pool),
        attacker_hand: hand_len(state, a),
        defender_hand: hand_len(state, b),
        hand_slots: catalog.config.hand_slots,
    };
    let log = combat::simulate_with(
        engine,
        &board_a,
        &board_b,
        a,
        b,
        first,
        &mut state.rng,
        setup,
    );
    write_back_combat_log(catalog, state, &log);
    // 战斗内写入的传染层数（以及成长记录）需要回到持久棋盘；其余字段保持不变。
    write_back_persistent(state, a, &log.attacker_final);
    write_back_persistent(state, b, &log.defender_final);
    let winner = seat_winner(state, &log, a, b);
    let damage = match winner {
        Some(winner) => {
            let loser = if winner == a { b } else { a };
            let survivors: BTreeSet<InstanceId> = if winner == a {
                log.attacker_survivors.iter().copied().collect()
            } else {
                log.defender_survivors.iter().copied().collect()
            };
            let winner_board = if winner == a { &board_a } else { &board_b };
            let stars = winner_board
                .iter()
                .filter(|minion| survivors.contains(&minion.instance_id))
                .map(|minion| card_level(catalog, &minion.card_id))
                .fold(0u32, u32::saturating_add);

            let damage = damage_formula(&catalog.config, player_era(state, loser), stars);
            state.pending_settle.insert(loser, damage);
            if let Some(player) = player_mut(state, winner) {
                player.damage_dealt = player.damage_dealt.saturating_add(damage);
            }
            state.last_winner = Some(winner);
            state.last_won.insert(winner);
            damage
        }
        // 互清平局：双方都不扣血、不计胜者，先手保持不变。
        None => 0,
    };

    state.previous_opponent.insert(a, Some(b));
    state.previous_opponent.insert(b, Some(a));

    // 报告只从本次调用返回：双方各自视角的阵容互换，行动序列与伤害相同。
    let actions = log.actions;
    let states = log.states;
    reports.insert(
        a,
        SeatCombatReport {
            seat: a,
            opponent_seat: Some(b),
            mirror: false,
            own_board: board_a.clone(),
            opponent_board: board_b.clone(),
            actions: actions.clone(),
            states: states.clone(),
            damage,
            winner,
        },
    );
    reports.insert(
        b,
        SeatCombatReport {
            seat: b,
            opponent_seat: Some(a),
            mirror: false,
            own_board: board_b,
            opponent_board: board_a,
            actions,
            states,
            damage,
            winner,
        },
    );

    match winner {
        Some(winner) => {
            let loser = if winner == a { b } else { a };
            events.push(GameEvent::CombatResolved {
                winner,
                loser,
                damage,
            });
        }
        None => events.push(GameEvent::CombatDrawn { a, b }),
    }
}

/// 幽灵战：落单座位对战一名出局者的最终阵容，按真实伤害结算。
///
/// 双方同回合清场一律平局；幽灵获胜时落单座位按 [`damage_formula`] 扣血，
/// 落单座位获胜时计入 `last_won`/`last_winner`。幽灵侧不结算英雄被动，也不
/// 写回牌池、手牌、持久棋盘或 `previous_opponent`。
fn resolve_ghost(
    engine: &Engine,
    state: &mut GameState,
    pairing: Pairing,
    events: &mut Vec<GameEvent>,
    reports: &mut BTreeMap<Seat, SeatCombatReport>,
) {
    let catalog = &engine.catalog;
    let seat = pairing.a;
    let ghost_seat = pairing.ghost;
    let own_board = fold_auras(catalog, &state.auras, board_of(state, seat));
    let ghost = ghost_board(engine, state, ghost_seat);
    let (ghost, next_instance_id) = renumber_ghost(state, &own_board, ghost_seat, ghost);

    let hero = hero_of(state, seat);
    let setup = combat::CombatSetup {
        attacker_hero: hero.as_deref(),
        // 幽灵没有英雄卡，防御方被动不结算。
        defender_hero: None,
        next_instance_id: Some(next_instance_id),
        pool: None,
        attacker_hand: hand_len(state, seat),
        defender_hand: 0,
        hand_slots: catalog.config.hand_slots,
    };
    let log = combat::simulate_with(
        engine,
        &own_board,
        &ghost,
        seat,
        seat,
        seat,
        &mut state.rng,
        setup,
    );
    state.next_instance_id = state.next_instance_id.max(log.next_instance_id);

    // 双方同回合清场一律平局；幽灵侧清场而己方仍有存活才算幽灵获胜。
    let cleared = log.attacker_survivors.is_empty() && log.defender_survivors.is_empty();
    let ghost_won = ghost_seat.is_some() && !cleared && log.attacker_survivors.is_empty();
    // 星级和取「幽灵战前棋盘 × 幽灵存活」，口径同真实战的「胜方战前棋盘 × 存活」。
    let stars = ghost
        .iter()
        .filter(|minion| log.defender_survivors.contains(&minion.instance_id))
        .map(|minion| card_level(catalog, &minion.card_id))
        .fold(0u32, u32::saturating_add);
    let damage = if ghost_won {
        let damage = damage_formula(&catalog.config, player_era(state, seat), stars);
        state.pending_settle.insert(seat, damage);
        damage
    } else {
        0
    };
    if !ghost_won && !cleared {
        state.last_won.insert(seat);
        state.last_winner = Some(seat);
    }

    let winner = if ghost_won {
        ghost_seat
    } else if cleared {
        None
    } else {
        Some(seat)
    };

    reports.insert(
        seat,
        SeatCombatReport {
            seat,
            opponent_seat: ghost_seat,
            mirror: true,
            own_board,
            opponent_board: ghost,
            actions: log.actions,
            states: log.states,
            damage,
            winner,
        },
    );

    match winner {
        None => events.push(GameEvent::CombatDrawn { a: seat, b: seat }),
        Some(winner) if winner == seat => events.push(GameEvent::CombatResolved {
            winner: seat,
            loser: ghost_seat.unwrap_or(seat),
            damage: 0,
        }),
        Some(winner) => events.push(GameEvent::CombatResolved {
            winner,
            loser: seat,
            damage,
        }),
    }
}

/// 把光环派生值折叠进一份棋盘克隆，供战斗模拟使用。
///
/// 原始随从不受影响：数值写入克隆的 `aura` 层，关键词以永久授予追加（同名只
/// 保留一次）。若目标在折叠前生命 > 0、折叠后 ≤ 0，则收窄负向生命修正，使总
/// 生命恰好为 1（§3.2「光环失效不致死」）。
pub fn fold_auras(
    catalog: &Catalog,
    auras: &BTreeMap<InstanceId, AuraEffect>,
    board: &[MinionInstance],
) -> Vec<MinionInstance> {
    let mut folded = board.to_vec();
    for minion in &mut folded {
        let Some(aura) = auras.get(&minion.instance_id) else {
            continue;
        };
        let attack = aura.attack;
        let mut health = aura.health;
        let before = combat::effective_health(catalog, minion);
        if before > 0 && before.saturating_add(health) <= 0 {
            health = 1 - before;
        }
        minion.stats.attack.set_aura(attack);
        minion.stats.health.set_aura(health);
        for keyword in &aura.keywords {
            if !minion
                .granted
                .iter()
                .any(|(existing, _)| existing == keyword)
            {
                minion.granted.push((keyword.clone(), Duration::Permanent));
            }
        }
    }
    folded
}

/// 幽灵阵容：幽灵归属座位（已出局）的最终棋盘，按它自己的光环来源折叠一次。
///
/// 只在本次模拟的克隆上生效，不写回 `state.auras`。`None` 与未知座位都返回
/// 空阵容。不做属性剥离：`turn` 层在出局那次结算已被清除，`permanent`/`growth`
/// 是死时的真实数值。
pub fn ghost_board(engine: &Engine, state: &GameState, ghost: Option<Seat>) -> Vec<MinionInstance> {
    let Some(corpse) = ghost.and_then(|seat| player(state, seat)) else {
        return Vec::new();
    };
    let auras = engine.player_auras(state, corpse);
    fold_auras(&engine.catalog, &auras, &corpse.board)
}

/// 为幽灵战准备阵容：把所有与己方棋盘重号的实例编号换成新号。
///
/// 幽灵通常来自其它座位，实例编号全局唯一，本函数不会触发改号；保留它是为了
/// 防止将来复制同座位阵容时，战斗内的定位（`BattleField::locate` 按编号先查
/// 攻方）把幽灵的伤害打到己方克隆上。`ghost` 是幽灵归属座位，同时约束
/// 「没有归属座位就没有幽灵阵容」。
///
/// 返回重新编号后的幽灵与「幽灵最大编号 + 1」的分配器起点；调用方仍需在战斗
/// 结束后把分配器推进到战斗实际使用的位置。
fn renumber_ghost(
    state: &GameState,
    own_board: &[MinionInstance],
    ghost: Option<Seat>,
    mut ghost_board: Vec<MinionInstance>,
) -> (Vec<MinionInstance>, InstanceId) {
    debug_assert!(
        ghost.is_some() || ghost_board.is_empty(),
        "没有幽灵归属座位时幽灵阵容必须为空"
    );
    let mut used: BTreeSet<InstanceId> =
        own_board.iter().map(|minion| minion.instance_id).collect();
    let mut next = own_board
        .iter()
        .chain(ghost_board.iter())
        .map(|minion| minion.instance_id)
        .max()
        .unwrap_or(0)
        .saturating_add(1)
        .max(state.next_instance_id);
    for minion in &mut ghost_board {
        if used.contains(&minion.instance_id) {
            minion.instance_id = next;
            next = next.saturating_add(1);
        }
        used.insert(minion.instance_id);
    }
    (ghost_board, next)
}

/// 清除所有棋盘随从的 `combat` 时长效果：本回合数值层、临时关键词与禁攻标记。
///
/// 「禁攻」用 `granted` 中的占位条目记录来源时长；只有不再剩任何「禁攻」授予时
/// 才清除布尔标记，因此 `permanent` 的禁攻会保留到下一回合。
fn clear_combat_duration(state: &mut GameState) {
    for player in &mut state.players {
        for minion in &mut player.board {
            minion.stats.attack.clear_turn();
            minion.stats.health.clear_turn();
            minion
                .granted
                .retain(|(_, duration)| *duration != Duration::Combat);
            if !minion.granted.iter().any(|(keyword, _)| keyword == "禁攻") {
                minion.cannot_attack = false;
            }
        }
    }
}

fn resolve_settle(
    config: &GameConfig,
    state: &mut GameState,
) -> Result<Vec<GameEvent>, EngineError> {
    if state.phase != Phase::Settle {
        return Err(EngineError(format!(
            "当前阶段 {:?} 不能结算伤害",
            state.phase
        )));
    }

    let mut events = Vec::new();
    // §2.5：进入 Settle 前清除本回合临时增益与临时关键词。
    clear_combat_duration(state);
    let pending = std::mem::take(&mut state.pending_settle);
    for (seat, damage) in pending {
        if let Some(player) = player_mut(state, seat) {
            player.hp = player.hp.saturating_sub(damage);
            events.push(GameEvent::DamageTaken {
                seat,
                amount: damage,
            });
        }
    }
    let mut newly_eliminated = Vec::new();
    let mut seats: Vec<Seat> = state.players.iter().map(|player| player.seat).collect();
    seats.sort_unstable();
    for seat in seats {
        let eliminated_already = match player(state, seat) {
            Some(player) => player.eliminated,
            None => true,
        };
        let hp = player(state, seat).map(|player| player.hp).unwrap_or(1);
        if eliminated_already || hp > 0 {
            continue;
        }
        let alive_before = state
            .players
            .iter()
            .filter(|player| !player.eliminated)
            .count() as u8;
        if let Some(player) = player_mut(state, seat) {
            player.eliminated = true;
        }
        newly_eliminated.push(seat);
        events.push(GameEvent::PlayerEliminated {
            seat,
            place: alive_before,
        });
    }

    if !newly_eliminated.is_empty() {
        let mut ranking: Vec<Seat> = newly_eliminated.iter().rev().copied().collect();
        ranking.extend(state.ranking.iter().copied());
        state.ranking = ranking;
    }
    state.pairings.clear();

    let alive: Vec<Seat> = state
        .players
        .iter()
        .filter(|player| !player.eliminated)
        .map(|player| player.seat)
        .collect();

    if let Some(&survivor) = alive.first().filter(|_| alive.len() == 1) {
        let mut ranking = vec![survivor];
        ranking.extend(state.ranking.iter().copied());
        state.ranking = ranking;
        state.phase = Phase::GameOver;
        events.push(GameEvent::GameOver {
            ranking: state.ranking.clone(),
        });
    } else if state.turn >= config.turn_limit {
        let mut ranked = alive;
        ranked.sort_by(|&left, &right| compare_alive(state, left, right));
        ranked.extend(state.ranking.iter().copied());
        state.ranking = ranked;
        state.phase = Phase::GameOver;
        events.push(GameEvent::GameOver {
            ranking: state.ranking.clone(),
        });
    }

    Ok(events)
}

/// 第 20 回合收束时的排名优先级：生命 → 上战胜者 → 累计伤害 → 座位。
fn compare_alive(state: &GameState, left: Seat, right: Seat) -> Ordering {
    let (left_player, right_player) = match (player(state, left), player(state, right)) {
        (Some(left_player), Some(right_player)) => (left_player, right_player),
        _ => return left.cmp(&right),
    };
    let left_won = state.last_won.contains(&left);
    let right_won = state.last_won.contains(&right);
    right_player
        .hp
        .cmp(&left_player.hp)
        .then_with(|| right_won.cmp(&left_won))
        .then_with(|| right_player.damage_dealt.cmp(&left_player.damage_dealt))
        .then_with(|| left.cmp(&right))
}

/// 局内实时名次：存活者按 [`compare_alive`] 从强到弱排 1..=存活数，
/// 已出局者接在其后。
///
/// `ranking` 的约定是「最近出局在前」（`resolve_settle` 把本回合新出局者
/// `rev()` 后前插），因此下标 0 的出局者名次紧挨存活者。
/// `GameOver` 阶段直接与 [`crate::view::placement_of`] 一致；若 `placement`
/// 缺失（榜单异常），回落到存活者算法，保证结果确定。
pub fn rank_of(state: &GameState, seat: Seat) -> Option<u8> {
    if let Some(place) = crate::view::placement_of(state, seat) {
        return Some(place);
    }
    let player = player(state, seat)?;
    if player.eliminated {
        // 已出局者占据尾部名次：存活数 + 榜内下标 + 1。
        let alive = state.players.iter().filter(|p| !p.eliminated).count();
        let index = state.ranking.iter().position(|s| *s == seat)?;
        u8::try_from(alive + index + 1).ok()
    } else {
        // 存活者：1 + 比它强的存活者数，比较器与第 20 回合收束完全一致。
        let better = state
            .players
            .iter()
            .filter(|p| !p.eliminated && p.seat != seat)
            .filter(|p| compare_alive(state, p.seat, seat) == Ordering::Less)
            .count();
        u8::try_from(better + 1).ok()
    }
}

/// 双方同时清场时按 §5.3 判胜：生命高 → 累计伤害高 → 座位小。
fn tie_break(state: &GameState, a: Seat, b: Seat) -> Seat {
    match (player(state, a), player(state, b)) {
        (Some(left), Some(right)) => {
            if left.hp != right.hp {
                return if left.hp > right.hp { a } else { b };
            }
            if left.damage_dealt != right.damage_dealt {
                return if left.damage_dealt > right.damage_dealt {
                    a
                } else {
                    b
                };
            }
            a.min(b)
        }
        _ => a.min(b),
    }
}

fn player(state: &GameState, seat: Seat) -> Option<&PlayerState> {
    state.players.iter().find(|player| player.seat == seat)
}

fn player_mut(state: &mut GameState, seat: Seat) -> Option<&mut PlayerState> {
    state.players.iter_mut().find(|player| player.seat == seat)
}

/// 把战斗结束时的克隆中需要持久化的字段写回存储随从（按 `instance_id` 匹配）。
///
/// 只写传染层数与成长层；回写后按分层重算 `current`（携带持久实例自身的
/// 永久与回合层，但不携带战斗伤害与光环）。授予关键词、护盾/复生等战斗标记
/// 保持原样。
fn write_back_persistent(state: &mut GameState, seat: Seat, final_board: &[MinionInstance]) {
    let Some(player) = player_mut(state, seat) else {
        return;
    };
    for stored in &mut player.board {
        if let Some(updated) = final_board
            .iter()
            .find(|minion| minion.instance_id == stored.instance_id)
        {
            stored.contagion = updated.contagion;
            for (stored_stat, updated_stat) in [
                (&mut stored.stats.attack, &updated.stats.attack),
                (&mut stored.stats.health, &updated.stats.health),
            ] {
                stored_stat.add_growth(updated_stat.growth.saturating_sub(stored_stat.growth));
            }
            for stat in [&mut stored.stats.attack, &mut stored.stats.health] {
                debug_assert_eq!(stat.aura, 0, "持久实例不应携带光环层");
                stat.revive_to(stat.layers());
            }
        }
    }
}

/// 把战斗日志里的持久副作用写回对局状态：
/// 召唤 card 扣池、`to=hand` 复制进手牌、推进实例编号。
///
/// 召唤物与复制品不进战场；死亡不改变持久棋盘。
fn write_back_combat_log(catalog: &Catalog, state: &mut GameState, log: &combat::CombatLog) {
    for (card_id, spent) in &log.pool_spent {
        if let Some(count) = state.pool.get_mut(card_id) {
            *count = count.saturating_sub(*spent);
        }
    }
    for copy in &log.hand_copies {
        let Some(player) = player_mut(state, copy.seat) else {
            continue;
        };
        if player.hand.len() < catalog.config.hand_slots {
            player.hand.push(HandCard {
                card_id: copy.card_id.clone(),
                golden: copy.golden,
            });
        }
    }
    state.next_instance_id = state.next_instance_id.max(log.next_instance_id);
}

fn hero_of(state: &GameState, seat: Seat) -> Option<String> {
    player(state, seat).and_then(|player| player.hero.clone())
}

fn hand_len(state: &GameState, seat: Seat) -> usize {
    player(state, seat)
        .map(|player| player.hand.len())
        .unwrap_or(0)
}

fn board_of(state: &GameState, seat: Seat) -> &[MinionInstance] {
    player(state, seat)
        .map(|player| player.board.as_slice())
        .unwrap_or(&[])
}

fn player_era(state: &GameState, seat: Seat) -> Era {
    player(state, seat)
        .map(|player| player.era)
        .unwrap_or(Era::Prehistoric)
}

fn card_level(catalog: &Catalog, card_id: &str) -> u32 {
    catalog
        .cards
        .get(card_id)
        .map(|card| u32::from(card.level))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CARDS_JSON: &str = include_str!("../../../data/v0.1/cards.json");
    const SKILLS_JSON: &str = include_str!("../../../data/v0.1/skills.json");
    const TOKENS_JSON: &str = include_str!("../../../data/v0.1/tokens.json");
    const CONFIG_JSON: &str = include_str!("../../../data/v0.1/config.json");

    fn test_catalog() -> Catalog {
        Catalog::from_json(CARDS_JSON, SKILLS_JSON, TOKENS_JSON, CONFIG_JSON)
            .expect("已提交的 v0.1 数据应能完整加载")
    }

    fn empty_log() -> combat::CombatLog {
        combat::CombatLog {
            actions: Vec::new(),
            states: Vec::new(),
            attacker_survivors: Vec::new(),
            defender_survivors: Vec::new(),
            winner: None,
            action_count: 0,
            trigger_count: 0,
            summon_count: 0,
            pool_spent: BTreeMap::new(),
            hand_copies: Vec::new(),
            next_instance_id: 1,
            attacker_final: Vec::new(),
            defender_final: Vec::new(),
        }
    }

    #[test]
    fn mutual_wipe_is_a_draw() {
        let (_, state) = Engine::new(test_catalog(), [7; 32]);
        let mut log = empty_log();
        log.winner = None; // 双方 survivors 都为空
        assert_eq!(seat_winner(&state, &log, 0, 1), None);
    }

    #[test]
    fn action_limit_still_breaks_by_hp() {
        let (_, mut state) = Engine::new(test_catalog(), [7; 32]);
        state.players[0].hp = 30;
        state.players[1].hp = 10;
        let mut log = empty_log();
        log.winner = None;
        log.attacker_survivors = vec![1]; // 双方都还有存活者：行动上限，不是清场
        log.defender_survivors = vec![2];
        assert_eq!(seat_winner(&state, &log, 0, 1), Some(0));
    }
}
