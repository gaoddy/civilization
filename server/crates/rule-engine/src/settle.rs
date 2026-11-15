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
    AuraEffect, GameState, HandCard, InstanceId, MinionInstance, Modifier, Phase, PlayerState, Seat,
};

/// 配对在放弃"排除上一场对手"前最多重洗的次数。
const PAIRING_ATTEMPTS: u8 = 100;

/// 一对配对。
///
/// `mirror == true` 时 `b == a`，表示 `a` 对战自己上一场阵容的镜像幽灵。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pairing {
    pub a: Seat,
    pub b: Seat,
    pub mirror: bool,
}

/// 单座位视角的一场战斗报告。
///
/// 报告只由 [`Engine::resolve_combat`] 的本次调用返回，不写入 [`GameState`]，
/// 因此不参与 `state_hash`。阵容为战前快照，且已折叠光环——客户端看到的是
/// 实际参战的数值。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeatCombatReport {
    pub seat: Seat,
    /// 非镜像战为对手座位；镜像战为 `None`。
    pub opponent_seat: Option<Seat>,
    pub mirror: bool,
    /// 本方战前阵容（已折叠光环）。
    pub own_board: Vec<MinionInstance>,
    /// 非镜像战为对手战前阵容；镜像战为幽灵阵容。
    pub opponent_board: Vec<MinionInstance>,
    /// 本场战斗的行动序列。
    pub actions: Vec<combat::CombatAction>,
    /// 本场战斗造成的伤害；镜像战恒为 0。
    pub damage: i32,
    /// 本场胜者；镜像未获胜时为 `None`。
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
/// 奇数人数时洗牌结果最后一名获得镜像配对。整轮最多重洗
/// [`PAIRING_ATTEMPTS`] 次以避开上一场对手，全部失败时采用最后一次洗牌。
/// 本函数只读取 `state.rng` 与 `state.previous_opponent`。
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

    let mut last = Vec::new();
    for _ in 0..PAIRING_ATTEMPTS {
        fisher_yates(&mut alive, state);
        last = pair_adjacent(&alive);
        if !repeats_previous_opponent(&last, &state.previous_opponent) {
            return last;
        }
    }
    last
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
            mirror: false,
        });
        index += 2;
    }
    if let Some(&seat) = alive.get(index) {
        pairings.push(Pairing {
            a: seat,
            b: seat,
            mirror: true,
        });
    }
    pairings
}

fn repeats_previous_opponent(
    pairings: &[Pairing],
    previous: &BTreeMap<Seat, Option<Seat>>,
) -> bool {
    pairings.iter().any(|pairing| {
        if pairing.mirror {
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
        if pairing.mirror {
            resolve_mirror(engine, state, pairing.a, &mut events, &mut reports);
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

fn resolve_real(
    engine: &Engine,
    state: &mut GameState,
    pairing: Pairing,
    events: &mut Vec<GameEvent>,
    reports: &mut BTreeMap<Seat, SeatCombatReport>,
) {
    let catalog = &engine.catalog;
    let (a, b) = (pairing.a, pairing.b);
    // 快照必须在战斗改变棋盘之前截取；光环只折叠进战斗克隆，持久快照保持原样。
    let raw_a = board_of(state, a).to_vec();
    let raw_b = board_of(state, b).to_vec();
    let board_a = fold_auras(catalog, &state.auras, &raw_a);
    let board_b = fold_auras(catalog, &state.auras, &raw_b);

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
    let winner = match log.winner {
        Some(winner) => winner,
        None => tie_break(state, a, b),
    };
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
    state.previous_opponent.insert(a, Some(b));
    state.previous_opponent.insert(b, Some(a));
    state.previous_board.insert(a, raw_a);
    state.previous_board.insert(b, raw_b);

    // 报告只从本次调用返回：双方各自视角的阵容互换，行动序列与伤害相同。
    let actions = log.actions;
    reports.insert(
        a,
        SeatCombatReport {
            seat: a,
            opponent_seat: Some(b),
            mirror: false,
            own_board: board_a.clone(),
            opponent_board: board_b.clone(),
            actions: actions.clone(),
            damage,
            winner: Some(winner),
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
            damage,
            winner: Some(winner),
        },
    );

    events.push(GameEvent::CombatResolved {
        winner,
        loser,
        damage,
    });
}

/// 镜像幽灵：不造成、也不承受真实伤害；胜者记入 `last_won`，败者什么都不发生。
fn resolve_mirror(
    engine: &Engine,
    state: &mut GameState,
    seat: Seat,
    events: &mut Vec<GameEvent>,
    reports: &mut BTreeMap<Seat, SeatCombatReport>,
) {
    let catalog = &engine.catalog;
    let current = board_of(state, seat).to_vec();
    let board = fold_auras(catalog, &state.auras, &current);
    let ghost = mirror_ghost_with_auras(catalog, state, seat);

    // 镜像战斗同样结算效果，但不写回牌池、手牌或持久棋盘。
    let hero = hero_of(state, seat);
    let setup = combat::CombatSetup {
        attacker_hero: hero.as_deref(),
        // 幽灵快照没有英雄卡，防御方被动不结算。
        defender_hero: None,
        next_instance_id: Some(state.next_instance_id),
        pool: None,
        attacker_hand: hand_len(state, seat),
        defender_hand: 0,
        hand_slots: catalog.config.hand_slots,
    };
    let log = combat::simulate_with(
        engine,
        &board,
        &ghost,
        seat,
        seat,
        seat,
        &mut state.rng,
        setup,
    );
    state.next_instance_id = state.next_instance_id.max(log.next_instance_id);
    let won = log.defender_survivors.is_empty() && !log.attacker_survivors.is_empty();
    if won {
        state.last_won.insert(seat);
        state.last_winner = Some(seat);
    }
    state.previous_board.insert(seat, current);

    // 镜像战没有对手座位，胜利才记录胜者；幽灵不产生真实伤害。
    reports.insert(
        seat,
        SeatCombatReport {
            seat,
            opponent_seat: None,
            mirror: true,
            own_board: board,
            opponent_board: ghost,
            actions: log.actions,
            damage: 0,
            winner: won.then_some(seat),
        },
    );

    events.push(GameEvent::CombatResolved {
        winner: seat,
        loser: seat,
        damage: 0,
    });
}

/// 把光环派生值折叠进一份棋盘克隆，供战斗模拟使用。
///
/// 原始随从不受影响：数值以永久 `Modifier` 追加，关键词以永久授予追加（同名只
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
        if attack != 0 || health != 0 {
            minion.mods.push(Modifier {
                attack,
                health,
                duration: Duration::Permanent,
            });
        }
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

/// 某个座位的镜像幽灵阵容：优先取上一场快照，缺失或为空时退回当前阵容。
///
/// 返回值已经过 [`strip_ghost`] 处理。
pub fn mirror_ghost(state: &GameState, seat: Seat) -> Vec<MinionInstance> {
    match state.previous_board.get(&seat) {
        Some(previous) if !previous.is_empty() => strip_ghost(previous),
        _ => strip_ghost(board_of(state, seat)),
    }
}

/// 折叠当前光环后的镜像幽灵阵容。
///
/// `previous_board` 快照保留原始实例编号，`state.auras` 以同一 `InstanceId`
/// 为键，因此上一场快照与当前阵容两条分支都能直接查到对应光环。
pub fn mirror_ghost_with_auras(
    catalog: &Catalog,
    state: &GameState,
    seat: Seat,
) -> Vec<MinionInstance> {
    fold_auras(catalog, &state.auras, &mirror_ghost(state, seat))
}

/// 幽灵快照只保留永久属性：卡面基础值、成长、繁衍与传染层。
///
/// 临时修正 `mods` 与授予关键词 `granted` 一律丢弃；战斗内的临时标记
/// （护盾已用、复生已用、剩余攻击次数等）也重置。
pub fn strip_ghost(board: &[MinionInstance]) -> Vec<MinionInstance> {
    board
        .iter()
        .map(|minion| MinionInstance {
            instance_id: minion.instance_id,
            card_id: minion.card_id.clone(),
            golden: minion.golden,
            growth: minion.growth,
            growth_health_only: minion.growth_health_only,
            breed: minion.breed,
            breed_applied: 0,
            contagion: minion.contagion,
            granted: Vec::new(),
            mods: Vec::new(),
            shield_used: false,
            reborn_used: false,
            attacks_left: 0,
            cannot_attack: false,
        })
        .collect()
}

/// 清除所有棋盘随从的 `combat` 时长效果：临时修正、临时关键词与禁攻标记。
///
/// 「禁攻」用 `granted` 中的占位条目记录来源时长；只有不再剩任何「禁攻」授予时
/// 才清除布尔标记，因此 `permanent` 的禁攻会保留到下一回合。
fn clear_combat_duration(state: &mut GameState) {
    for player in &mut state.players {
        for minion in &mut player.board {
            minion
                .mods
                .retain(|modifier| modifier.duration != Duration::Combat);
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
/// 只写传染层数与成长记录；临时修正、授予关键词、护盾/复生等战斗标记保持原样。
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
            stored.growth = updated.growth;
            stored.growth_health_only = updated.growth_health_only;
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
