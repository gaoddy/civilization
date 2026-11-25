//! 固定种子评测：每局 1 个学习者座位 + 7 个锚点座位，贪心策略，座位轮换。
//!
//! 同一份权重、同一组种子必须逐字段复现；所有随机都从种子派生，不读时钟。
//! 默认锚点是启发式机器人，报告学习者相对锚点的名次优势（配对差的 95% 置信区间）。

use ai_core::{Action, ActionFeatures};
use ai_model::{ModelConfig, Weights, forward};
use rule_engine::candidates::{SEAT_COUNT, allocate_candidates};
use rule_engine::data::Catalog;
use rule_engine::engine::Engine;
use rule_engine::playstyles;
use rule_engine::state::{GameState, Seat};
use serde::Serialize;

use crate::env::PLAYER_COUNT;
use crate::env_full::FullEnv;
use crate::eval::DECISIONS_PER_TURN;
use crate::league::{League, PolicyId};
use crate::random_policy::RandomPolicy;

/// 单局最多推进的决策数；超过视为异常并报错，避免评测挂死。
const GUARD: usize = 200_000;

/// 评测种子起点：与训练种子池（`C2Config::seeds`，默认 `0..10_000`）不相交，
/// 避免「在训练过的种子上评测」。
pub const EVAL_SEED_BASE: u64 = 1_000_000;

/// 评测对手锚点。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// 启发式机器人（`bot_ai::plan`），即验收门槛里的「启发式锚点」。
    Heuristic,
    /// 联盟里的权重（快照或外部权重）。
    Weights(PolicyId),
}

/// 评测配置。
#[derive(Debug, Clone)]
pub struct EvalConfig {
    pub seeds: Vec<u64>,
    /// 学习者座位在 `0..learner_seats` 内轮换；默认 8（全部座位）。
    pub learner_seats: usize,
    /// 对手锚点；为空时等价于只有启发式锚点。
    pub anchors: Vec<Anchor>,
    /// 每座位每回合的决策上限；超过后强制结束回合（贪心策略会原地打转）。
    pub max_decisions_per_turn: usize,
    /// 单局最多推进的决策数；超过视为截断，该局跳过并计入 `truncated`。
    pub max_steps_per_game: usize,
}

impl Default for EvalConfig {
    fn default() -> Self {
        Self {
            seeds: (EVAL_SEED_BASE..EVAL_SEED_BASE + 2000).collect(),
            learner_seats: SEAT_COUNT,
            anchors: Vec::new(),
            max_decisions_per_turn: DECISIONS_PER_TURN,
            max_steps_per_game: GUARD,
        }
    }
}

/// 评测报告。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EvalReport {
    pub games: usize,
    pub mean_placement: f32,
    pub win_rate: f32,
    pub top4_rate: f32,
    /// 平均名次的 95% 置信区间半宽（1.96 · sd / √n）。
    pub ci95: f32,
    /// 同桌锚点座位在全部评测局上的平均名次。
    pub anchor_mean: f32,
    /// 名次优势：`anchor_mean - mean_placement`，为正表示比锚点打得好。
    pub advantage: f32,
    /// 名次优势的 95% 置信区间半宽（按局配对差的 1.96 · sd / √n）。
    pub advantage_ci95: f32,
    /// 名次优于同桌锚点均值的局数。
    pub anchor_wins: usize,
    /// 名次劣于同桌锚点均值的局数。
    pub anchor_losses: usize,
    /// 被步数上限截断、未计入统计的局数。
    pub truncated: usize,
}

/// 评测错误。
#[derive(Debug)]
pub enum EvalError {
    NoData(String),
    Model(ai_model::ModelError),
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoData(message) => write!(f, "没有可用数据：{message}"),
            Self::Model(error) => write!(f, "模型失败：{error}"),
        }
    }
}

impl std::error::Error for EvalError {}

impl From<ai_model::ModelError> for EvalError {
    fn from(error: ai_model::ModelError) -> Self {
        Self::Model(error)
    }
}

enum Opponent {
    Weights(Box<(Weights, ModelConfig)>),
    Heuristic,
    Random,
}

impl Opponent {
    fn weights(weights: &Weights) -> Result<Self, EvalError> {
        let model = ModelConfig::from_weights(weights)?;
        Ok(Self::Weights(Box::new((weights.clone(), model))))
    }
}

fn resolve_opponent(league: &League, anchor: Anchor) -> Result<Opponent, EvalError> {
    let policy = match anchor {
        Anchor::Heuristic => return Ok(Opponent::Heuristic),
        Anchor::Weights(policy) => policy,
    };
    Ok(match policy {
        // 当前策略的权重不在联盟里，评测侧没有训练模型，只能落到随机兜底。
        PolicyId::Current | PolicyId::Random => Opponent::Random,
        PolicyId::Snapshot(_) | PolicyId::Anchor => Opponent::weights(
            league
                .weights(policy)
                .ok_or_else(|| EvalError::NoData(format!("联盟缺少对手 {policy:?} 的权重")))?,
        )?,
    })
}

/// 启发式锚点把 `bot_ai::plan` 的意图映射成候选下标。
///
/// 计划不可用、或意图不在候选里时退回 `End`（没有就退回第一条候选）：评测
/// 不因某个座位的一次决策失败而中断。
fn heuristic_choice(
    state: &GameState,
    engine: &Engine,
    seat: Seat,
    candidates: &[(Action, ActionFeatures)],
) -> usize {
    if let Some(intent) = bot_ai::plan(state, seat, engine, &[])
        && let Some(action) = Action::from_intent(&intent)
        && let Some(index) = candidates
            .iter()
            .position(|(candidate, _)| *candidate == action)
    {
        return index;
    }
    candidates
        .iter()
        .position(|(candidate, _)| matches!(candidate, Action::End))
        .unwrap_or(0)
}

/// 用固定种子集评测一份权重。
pub fn eval_full(
    catalog: &Catalog,
    weights: &Weights,
    league: &League,
    cfg: &EvalConfig,
) -> Result<EvalReport, EvalError> {
    if cfg.seeds.is_empty() {
        return Err(EvalError::NoData("评测种子为空".to_string()));
    }
    if cfg.learner_seats == 0 || cfg.learner_seats > SEAT_COUNT {
        return Err(EvalError::NoData(format!(
            "learner_seats 应在 1..={SEAT_COUNT}，实际 {}",
            cfg.learner_seats
        )));
    }
    let learner_model = ModelConfig::from_weights(weights)?;
    let mut anchors: Vec<Opponent> = cfg
        .anchors
        .iter()
        .map(|policy| resolve_opponent(league, *policy))
        .collect::<Result<_, _>>()?;
    if anchors.is_empty() {
        anchors.push(Opponent::Heuristic);
    }

    let mut env = FullEnv::new(catalog, League::new(0, 0));
    let mut placements = Vec::with_capacity(cfg.seeds.len());
    let mut anchor_means = Vec::with_capacity(cfg.seeds.len());
    let mut anchor_wins = 0usize;
    let mut anchor_losses = 0usize;
    let mut truncated = 0usize;

    for (seed_index, seed) in cfg.seeds.iter().copied().enumerate() {
        let learner = (seed_index % cfg.learner_seats) as Seat;
        let candidates = allocate_candidates(seed, &playstyles::draw(seed), catalog);
        env.reset(seed, &candidates)
            .map_err(|error| EvalError::NoData(error.0))?;
        let mut rng = RandomPolicy::new(seed ^ 0xE7A1_0BEE ^ u64::from(learner));

        let mut turn = env.turn();
        let mut per_turn = vec![0usize; PLAYER_COUNT];
        let mut steps = 0usize;
        while steps < cfg.max_steps_per_game {
            let Some(seat) = env.current_seat() else {
                break;
            };
            if env.turn() != turn {
                turn = env.turn();
                per_turn = vec![0usize; PLAYER_COUNT];
            }
            let index = usize::from(seat);
            let obs = env.observe(seat);
            let candidates = env.action_candidates(seat);
            if candidates.is_empty() {
                break;
            }
            let features: Vec<ActionFeatures> =
                candidates.iter().map(|(_, feature)| *feature).collect();
            let forced = if per_turn[index] >= cfg.max_decisions_per_turn {
                candidates
                    .iter()
                    .position(|(candidate, _)| matches!(candidate, Action::End))
            } else {
                None
            };
            let choice = match forced {
                Some(choice) => choice,
                None if seat == learner => {
                    let (logits, _) = forward(weights, &learner_model, &obs, &features)?;
                    argmax(&logits)
                }
                None => {
                    let opponent = &anchors[(seed_index + index) % anchors.len()];
                    match opponent {
                        Opponent::Weights(boxed) => {
                            let (opp_weights, opp_model) = &**boxed;
                            let (logits, _) = forward(opp_weights, opp_model, &obs, &features)?;
                            argmax(&logits)
                        }
                        Opponent::Heuristic => {
                            heuristic_choice(env.state(), env.engine(), seat, &candidates)
                        }
                        Opponent::Random => rng.below(features.len() as u64) as usize,
                    }
                }
            };
            env.step(candidates[choice].0.clone())
                .map_err(|error| EvalError::NoData(error.0))?;
            steps += 1;
            per_turn[index] += 1;
        }

        // 截断局没有名次：跳过该局并计数，不让一局异常把整批评测打掉。
        let Some(placement) = env.placement(learner) else {
            truncated += 1;
            continue;
        };
        placements.push(f32::from(placement));
        let anchor_placements: Vec<f32> = (0..PLAYER_COUNT as Seat)
            .filter(|seat| *seat != learner)
            .filter_map(|seat| env.placement(seat))
            .map(f32::from)
            .collect();
        if !anchor_placements.is_empty() {
            let anchor_mean =
                anchor_placements.iter().sum::<f32>() / anchor_placements.len() as f32;
            anchor_means.push(anchor_mean);
            let own = f32::from(placement);
            if own < anchor_mean {
                anchor_wins += 1;
            } else if own > anchor_mean {
                anchor_losses += 1;
            }
        }
    }

    if placements.is_empty() {
        return Err(EvalError::NoData(format!(
            "评测对局全部被截断（{truncated} 局），请提高 max_steps_per_game"
        )));
    }
    Ok(report(
        placements,
        anchor_means,
        anchor_wins,
        anchor_losses,
        truncated,
    ))
}

fn argmax(logits: &[f32]) -> usize {
    let mut best = 0usize;
    for (index, value) in logits.iter().enumerate() {
        if *value > logits[best] {
            best = index;
        }
    }
    best
}

/// 半宽 95% 置信区间：`1.96 · sd / √n`（样本标准差）。
fn half_ci95(samples: &[f32]) -> f32 {
    let n = samples.len();
    if n < 2 {
        return 0.0;
    }
    let mean = samples.iter().sum::<f32>() / n as f32;
    let variance = samples
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f32>()
        / (n - 1) as f32;
    1.96 * variance.sqrt() / (n as f32).sqrt()
}

fn report(
    placements: Vec<f32>,
    anchor_means: Vec<f32>,
    anchor_wins: usize,
    anchor_losses: usize,
    truncated: usize,
) -> EvalReport {
    let games = placements.len();
    let mean = placements.iter().sum::<f32>() / games as f32;
    let ci95 = half_ci95(&placements);
    // 名次优势按局配对：同一局里学习者与锚点共享商店、对局强度等共同方差，
    // 配对差的方差远小于两组独立均值之差。
    let diffs: Vec<f32> = anchor_means
        .iter()
        .zip(&placements)
        .map(|(anchor, own)| anchor - own)
        .collect();
    let advantage = if diffs.is_empty() {
        0.0
    } else {
        diffs.iter().sum::<f32>() / diffs.len() as f32
    };
    let anchor_mean = if anchor_means.is_empty() {
        0.0
    } else {
        anchor_means.iter().sum::<f32>() / anchor_means.len() as f32
    };
    let win_rate = placements.iter().filter(|place| **place == 1.0).count() as f32 / games as f32;
    let top4_rate = placements.iter().filter(|place| **place <= 4.0).count() as f32 / games as f32;
    EvalReport {
        games,
        mean_placement: mean,
        win_rate,
        top4_rate,
        ci95,
        anchor_mean,
        advantage,
        advantage_ci95: half_ci95(&diffs),
        anchor_wins,
        anchor_losses,
        truncated,
    }
}
