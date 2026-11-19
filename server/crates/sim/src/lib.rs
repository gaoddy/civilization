//! 模拟器库：确定性 AI、整局驱动、批量运行与基线报告。
//!
//! 所有随机性只来自 [`Engine::new`] 注入的种子；本库不读时钟、不读环境变量，
//! 不使用浮点参与任何判定。

pub mod ai;
pub mod bot;
pub mod report;

use std::collections::{BTreeMap, BTreeSet};

use rule_engine::data::{Catalog, Playstyle, card_in_pool};
use rule_engine::engine::Engine;
use rule_engine::error::EngineError;
use rule_engine::hash::{EventHasher, state_hash};
use rule_engine::intent::Intent;
use rule_engine::playstyles;
use rule_engine::state::{GameState, PendingChoice, Phase, Seat};

/// 引擎 32 字节种子与日志 64 位种子之间的对应，实现已迁至
/// [`rule_engine::rng::seed_from_u64`]，此处保持 `sim` 对外 API 不变。
pub use rule_engine::rng::seed_from_u64;

/// 固定 8 人对局，与引擎一致。
pub const PLAYER_COUNT: Seat = 8;

/// 单局最大驱动步数上限：防止 AI 缺陷导致死循环。正常对局远低于此值。
pub const MAX_STEPS_PER_GAME: usize = 200_000;

/// 单座位在单个回合内最多重规划次数：防止引擎给出与 AI 计划不一致的拒绝后原地打转。
const MAX_REPLANS_PER_SEAT_TURN: usize = 64;

/// 一局中按顺序执行的一条意图。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentRecord {
    pub seat: Seat,
    pub intent: Intent,
}

/// 驱动记录的类型。
///
/// 各座位在 `EndRecruit` 之后串行进入战斗与结算，因此每条意图之后都必须标出
/// 该座位是否结束招募，重放时才能精确复现引擎调用顺序。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// 该座位执行了一条招募意图。
    Intent,
    /// 该座位提交 `EndRecruit` 成功。
    EndSeat,
    /// 全部玩家结束招募后的 `finish_recruit`。
    FinishRecruit,
    /// `resolve_combat`。
    ResolveCombat,
    /// `resolve_settle`（可能进入 `GameOver`）。
    ResolveSettle,
    /// 进入下一回合的 `start_turn`。
    StartTurn,
}

/// 一条驱动记录：`seat` 只对 `Intent` / `EndSeat` 有意义。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepRecord {
    pub seat: Seat,
    pub intent: Intent,
    pub step: Step,
}

/// 对局日志，`replay` 的唯一输入，也是批量模式的逐局记录。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameLog {
    pub seed: u64,
    pub steps: Vec<StepRecord>,
    pub final_hash: u64,
    /// 事件序列累计哈希；重放时逐条比对，确保事件流一致。
    pub event_hash: u64,
    pub ranking: Vec<Seat>,
    pub turns: u8,
    /// 局内统计，供报告使用，不参与重放。
    pub stats: GameStats,
}

/// 单局统计。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameStats {
    /// 每名玩家首次升到下一时代的回合；对局结束时仍未升级为 `None`。
    pub era_upgrade_turns: [Option<u8>; 8],
    /// 本局三连次数。
    pub triples: u32,
    /// 第 20 回合结算后的存活人数；对局未到第 20 回合为 `None`。
    /// 采样时点为第 20 回合 `resolve_settle` 完成之后、开始下一回合之前（见 open-questions）。
    pub survivors_at_turn_20: Option<u8>,
    /// 对局结束时的存活人数。
    pub survivors_at_end: u8,
    /// 每次战斗失败的伤害（一方被判负记一次）。
    pub damage_per_loss: Vec<i32>,
    /// 冠军座位（排名第 1）。
    pub winner: Seat,
    /// 冠军最终阵容的玩法（按随从 `playstyle` 统计）。
    pub winner_playstyle: Playstyle,
}

impl Default for GameStats {
    fn default() -> Self {
        Self {
            era_upgrade_turns: [None; 8],
            triples: 0,
            survivors_at_turn_20: None,
            survivors_at_end: 0,
            damage_per_loss: Vec::new(),
            winner: 0,
            winner_playstyle: Playstyle::Neutral,
        }
    }
}

/// 批量结果：每局一条日志。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchJson {
    pub seed: u64,
    pub games: Vec<GameLog>,
}

/// 第 i 局的种子：`seed + game_index`（64 位回绕）。
pub fn derive_seed(seed: u64, game_index: usize) -> u64 {
    seed.wrapping_add(game_index as u64)
}

/// 本局可用英雄卡按 id 升序取 8 张，依次分配给座位 0..8。
///
/// 只保留通过 [`card_in_pool`] 的英雄卡（按本局 `enabled_playstyles` 过滤；
/// 目录 `cards` 为 `BTreeMap`，遍历顺序即 id 升序）。不足 8 张时从头循环取，
/// 保证每个座位都有英雄；没有任何可用英雄时返回错误。
pub fn assign_heroes(
    catalog: &Catalog,
    enabled: &BTreeSet<Playstyle>,
) -> Result<BTreeMap<Seat, String>, EngineError> {
    let heroes: Vec<&str> = catalog
        .cards
        .values()
        .filter(|card| card.is_hero && card_in_pool(card, enabled))
        .map(|card| card.id.as_str())
        .collect();
    if heroes.is_empty() {
        return Err(EngineError("目录中没有本局玩法可用的英雄卡".to_string()));
    }
    let heroes = (0..usize::from(PLAYER_COUNT))
        .map(|seat| (seat as Seat, heroes[seat % heroes.len()].to_string()))
        .collect();
    Ok(heroes)
}

/// 加载仓库中已提交的 v0.1 数据（编译期嵌入，不依赖当前目录）。
pub fn embedded_catalog() -> Result<Catalog, EngineError> {
    Catalog::from_json(
        include_str!("../../../data/v0.1/cards.json"),
        include_str!("../../../data/v0.1/skills.json"),
        include_str!("../../../data/v0.1/tokens.json"),
        include_str!("../../../data/v0.1/config.json"),
        include_str!("../../../data/v0.1/taxonomy.json"),
    )
    .map_err(|err| EngineError(format!("加载卡牌目录失败: {err}")))
}

fn check_players(players: usize) -> Result<(), EngineError> {
    if players != usize::from(PLAYER_COUNT) {
        return Err(EngineError(format!(
            "只支持 {PLAYER_COUNT} 人局，实际 {players}"
        )));
    }
    Ok(())
}

/// 以库内目录跑一局；`seed` 为日志与引擎种子，并用同一个 `seed` 抽取本局 5 个玩法。
pub fn run_game(catalog: Catalog, seed: u64) -> Result<GameLog, EngineError> {
    let playstyles = playstyles::draw(seed);
    let (mut engine, mut state) = Engine::new(catalog, seed_from_u64(seed), playstyles);
    play_game(&mut engine, &mut state, seed, usize::from(PLAYER_COUNT))
}

/// 在已构造好的引擎与状态下跑完整一局：开局、逐回合驱动到 `GameOver`。
pub fn play_game(
    engine: &mut Engine,
    state: &mut GameState,
    seed: u64,
    players: usize,
) -> Result<GameLog, EngineError> {
    check_players(players)?;
    let heroes = assign_heroes(&engine.catalog, &state.enabled_playstyles)?;
    let mut event_hasher = EventHasher::new();
    event_hasher.extend(&engine.start_game(state, &heroes)?);
    drive(engine, state, seed, &mut event_hasher)
}

/// 批量跑 `games` 局：第 i 局种子为 `seed + i`。`players` 只允许 8。
pub fn run_games(games: usize, seed: u64, players: usize) -> Result<BatchJson, EngineError> {
    check_players(players)?;
    let catalog = embedded_catalog()?;
    let mut logs = Vec::with_capacity(games);
    for index in 0..games {
        let game_seed = derive_seed(seed, index);
        logs.push(run_game(catalog.clone(), game_seed)?);
    }
    Ok(BatchJson { seed, games: logs })
}

/// 驱动主循环：`Recruit` 阶段逐座位执行 AI 意图直到 `EndRecruit`，
/// 然后 `finish_recruit` → `resolve_combat` → `resolve_settle` → `start_turn`，
/// 直到 `GameOver`。
fn drive(
    engine: &mut Engine,
    state: &mut GameState,
    seed: u64,
    event_hasher: &mut EventHasher,
) -> Result<GameLog, EngineError> {
    let mut steps = Vec::new();
    let mut stats = GameStats::default();
    if state.turn >= 20 {
        stats.survivors_at_turn_20 = Some(alive_count(state));
    }
    while state.phase != Phase::GameOver {
        run_turn(engine, state, &mut steps, &mut stats, event_hasher)?;
        // 第 20 回合招募结束、`finish_recruit` 与战斗结算之前采样：与
        // `docs/current/technical/rule-engine-and-simulator.md` 的口径一致。
        if state.turn == 20 && stats.survivors_at_turn_20.is_none() {
            stats.survivors_at_turn_20 = Some(alive_count(state));
        }

        event_hasher.extend(&engine.finish_recruit(state)?);
        steps.push(boundary(Step::FinishRecruit));
        event_hasher.extend(&engine.resolve_combat(state)?.events);
        steps.push(boundary(Step::ResolveCombat));
        let events = engine.resolve_settle(state)?;
        event_hasher.extend(&events);
        steps.push(boundary(Step::ResolveSettle));
        for event in &events {
            if let rule_engine::events::GameEvent::DamageTaken { amount, .. } = event {
                stats.damage_per_loss.push(*amount);
            }
        }
        if state.phase != Phase::GameOver {
            event_hasher.extend(&engine.start_turn(state)?);
            steps.push(boundary(Step::StartTurn));
        }
    }
    // 循环出口必定是 `GameOver`：在此采样一次结束时的存活人数，不用 0 当「未采样」哨兵。
    stats.survivors_at_end = alive_count(state);
    finish_stats(engine, state, &mut stats);
    Ok(GameLog {
        seed,
        steps,
        final_hash: state_hash(state),
        event_hash: event_hasher.finish(),
        ranking: state.ranking.clone(),
        turns: state.turn,
        stats,
    })
}

fn boundary(step: Step) -> StepRecord {
    StepRecord {
        seat: 0,
        intent: Intent::EndRecruit,
        step,
    }
}

/// 当前存活（未淘汰）的玩家数；驱动在 `GameOver` 时用它记录 `stats.survivors_at_end`。
pub fn alive_players(state: &GameState) -> u8 {
    alive_count(state)
}

fn alive_count(state: &GameState) -> u8 {
    u8::try_from(
        state
            .players
            .iter()
            .filter(|player| !player.eliminated)
            .count(),
    )
    .unwrap_or(u8::MAX)
}

/// 该座位是否还有未消费的发现 / 进化 / 目标待选。
fn seat_has_pending(state: &GameState, seat: Seat) -> bool {
    state.pending.iter().any(|pending| match pending {
        PendingChoice::Discover {
            seat: pending_seat, ..
        }
        | PendingChoice::Evolve {
            seat: pending_seat, ..
        }
        | PendingChoice::Target {
            seat: pending_seat, ..
        } => *pending_seat == seat,
    })
}

fn finish_stats(engine: &Engine, state: &GameState, stats: &mut GameStats) {
    let winner = state.ranking.first().copied().unwrap_or(0);
    stats.winner = winner;
    stats.winner_playstyle = state
        .players
        .get(usize::from(winner))
        .map(|player| {
            ai::dominant_playstyle(
                player.board.iter().map(|minion| minion.card_id.as_str()),
                &engine.catalog,
            )
        })
        .unwrap_or(Playstyle::Neutral);
}

/// 一个招募回合：未出局座位按座位升序，各自反复执行 AI 意图直到 `EndRecruit`。
///
/// 引擎的 `EndRecruit` 只标记该座位（`recruit_done`），全员未出局玩家结束后阶段才变为
/// `Combat`；本函数退出时阶段必为 `Combat`，随后由 `finish_recruit` 生成配对。
fn run_turn(
    engine: &mut Engine,
    state: &mut GameState,
    steps: &mut Vec<StepRecord>,
    stats: &mut GameStats,
    event_hasher: &mut EventHasher,
) -> Result<(), EngineError> {
    let turn = state.turn;
    for seat in 0..PLAYER_COUNT {
        if state.players[usize::from(seat)].eliminated {
            continue;
        }
        if state.players[usize::from(seat)].recruit_done {
            continue;
        }
        let mut replans = 0usize;
        let mut blocked: Vec<Intent> = Vec::new();
        loop {
            let Some(intent) = ai::plan(state, seat, &engine.catalog, &blocked) else {
                return Err(EngineError(format!(
                    "座位 {seat} 在回合 {turn} 无可执行意图（阶段 {:?}）",
                    state.phase
                )));
            };
            let ends = matches!(intent, Intent::EndRecruit);
            if ends && seat_has_pending(state, seat) {
                // AI 已没有可消费的待选：按超时视为放弃。不记录 `EndRecruit`，
                // 由本回合的 `finish_recruit` 统一清空待选并进入战斗。
                break;
            }
            match engine.apply(state, seat, intent.clone()) {
                Ok(outcome) => {
                    blocked.clear();
                    event_hasher.extend(&outcome.events);
                    // 成功执行说明本回合仍有进展；预算只用于识别「连续被拒、状态不变」
                    // 的死循环，跨成功动作累计会把正常回合（例如手牌里长期躺着一张
                    // 无合法目标的效果牌、每次成功动作后又重试一次）误判为死循环。
                    replans = 0;
                    record_outcome(&outcome.events, seat, turn, stats);
                    steps.push(StepRecord {
                        seat,
                        intent,
                        step: if ends { Step::EndSeat } else { Step::Intent },
                    });
                    if steps.len() > MAX_STEPS_PER_GAME {
                        return Err(EngineError("驱动步数超过上限，疑似死循环".into()));
                    }
                    if ends {
                        break;
                    }
                }
                Err(reject) => {
                    if !ai::planning_reject(reject.code) {
                        return Err(EngineError(format!(
                            "座位 {seat} 在回合 {turn} 的意图 {intent:?} 被拒绝: {} {}",
                            reject.code.as_str(),
                            reject.message
                        )));
                    }
                    replans += 1;
                    if replans > MAX_REPLANS_PER_SEAT_TURN {
                        return Err(EngineError(format!(
                            "座位 {seat} 在回合 {turn} 重规划次数超限，最后拒绝: {} {}",
                            reject.code.as_str(),
                            reject.message
                        )));
                    }
                    // 状态未变：本轮跳过被拒意图，换下一条候选（见 `ai::plan`）。
                    blocked.push(intent);
                }
            }
        }
    }
    Ok(())
}

fn record_outcome(
    events: &[rule_engine::events::GameEvent],
    seat: Seat,
    turn: u8,
    stats: &mut GameStats,
) {
    use rule_engine::events::GameEvent;
    let slot = usize::from(seat);
    for event in events {
        match event {
            GameEvent::TripleFormed {
                seat: found_seat, ..
            } if *found_seat == seat => {
                stats.triples += 1;
            }
            GameEvent::EraUpgraded { seat: up_seat, .. } if *up_seat == seat => {
                if let Some(entry) = stats.era_upgrade_turns.get_mut(slot)
                    && entry.is_none()
                {
                    *entry = Some(turn);
                }
            }
            _ => {}
        }
    }
}

/// 重放一条对局日志：逐条复现引擎调用，并要求最终哈希与排名与日志一致。
///
/// 本局玩法由日志种子重新推导（`playstyles::draw(log.seed)`），与批量运行时
/// 按同一局种子抽取的结果一致，因此重放与原始对局使用同一组玩法。
/// 返回重放得到的最终状态哈希。
pub fn replay(log: &GameLog) -> Result<u64, EngineError> {
    let playstyles = playstyles::draw(log.seed);
    let (mut engine, mut state) =
        Engine::new(embedded_catalog()?, seed_from_u64(log.seed), playstyles);
    let heroes = assign_heroes(&engine.catalog, &state.enabled_playstyles)?;
    let mut event_hasher = EventHasher::new();
    event_hasher.extend(&engine.start_game(&mut state, &heroes)?);
    for (index, record) in log.steps.iter().enumerate() {
        step_engine(&mut engine, &mut state, record, &mut event_hasher)
            .map_err(|err| EngineError(format!("第 {index} 条记录复现失败: {err}")))?;
    }
    if state.phase != Phase::GameOver {
        return Err(EngineError(format!(
            "日志复现结束后阶段为 {:?}，未到 GameOver",
            state.phase
        )));
    }
    let event_hash = event_hasher.finish();
    if event_hash != log.event_hash {
        return Err(EngineError(format!(
            "重放事件序列不一致：期望 {:016x}，实际 {event_hash:016x}",
            log.event_hash
        )));
    }
    let hash = state_hash(&state);
    if hash != log.final_hash {
        return Err(EngineError(format!(
            "重放哈希不一致：期望 {:016x}，实际 {hash:016x}",
            log.final_hash
        )));
    }
    if state.ranking != log.ranking {
        return Err(EngineError("重放排名不一致".into()));
    }
    Ok(hash)
}

fn step_engine(
    engine: &mut Engine,
    state: &mut GameState,
    record: &StepRecord,
    event_hasher: &mut EventHasher,
) -> Result<(), EngineError> {
    match record.step {
        Step::Intent | Step::EndSeat => engine
            .apply(state, record.seat, record.intent.clone())
            .map(|outcome| event_hasher.extend(&outcome.events))
            .map_err(|reject| {
                EngineError(format!(
                    "座位 {} 的意图 {:?} 被拒绝: {} {}",
                    record.seat,
                    record.intent,
                    reject.code.as_str(),
                    reject.message
                ))
            }),
        Step::FinishRecruit => engine
            .finish_recruit(state)
            .map(|events| event_hasher.extend(&events)),
        Step::ResolveCombat => engine
            .resolve_combat(state)
            .map(|outcome| event_hasher.extend(&outcome.events)),
        Step::ResolveSettle => engine
            .resolve_settle(state)
            .map(|events| event_hasher.extend(&events)),
        Step::StartTurn => engine
            .start_turn(state)
            .map(|events| event_hasher.extend(&events)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_from_u64_is_deterministic() {
        for seed in [0u64, 1, 2, u64::MAX, 0x0123_4567_89ab_cdef] {
            let bytes = seed_from_u64(seed);
            assert_eq!(seed_from_u64(seed), bytes);
            assert_eq!(&bytes[..8], &seed.to_le_bytes());
        }
        assert_ne!(seed_from_u64(0), seed_from_u64(1));
    }

    #[test]
    fn embedded_catalog_loads() {
        assert!(embedded_catalog().is_ok());
    }

    #[test]
    fn unplayable_targeted_effect_does_not_abort_game() {
        let catalog = embedded_catalog().expect("内置卡表应能加载");
        run_game(catalog, 659).expect("手牌里的定向效果牌找不到合法目标时不应中断整局");
    }
}
