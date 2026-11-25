//! 固定种子评测：每局 1 个学习者座位 + 7 个锚点座位，贪心策略，座位轮换。
//!
//! 同一份权重、同一组种子必须逐字段复现；所有随机都从种子派生，不读时钟。

use ai_core::{Action, ActionFeatures};
use ai_model::{ModelConfig, Weights, forward};
use rule_engine::candidates::{SEAT_COUNT, allocate_candidates};
use rule_engine::data::Catalog;
use rule_engine::playstyles;
use rule_engine::state::Seat;
use serde::Serialize;

use crate::env::PLAYER_COUNT;
use crate::env_full::FullEnv;
use crate::eval::DECISIONS_PER_TURN;
use crate::league::{League, PolicyId};
use crate::random_policy::RandomPolicy;

/// 单局最多推进的决策数；超过视为异常并报错，避免评测挂死。
const GUARD: usize = 200_000;

/// 评测配置。
#[derive(Debug, Clone)]
pub struct EvalConfig {
    pub seeds: Vec<u64>,
    /// 学习者座位在 `0..learner_seats` 内轮换；默认 8（全部座位）。
    pub learner_seats: usize,
    /// 对手锚点；为空时用学习者自己的权重做同桌自博弈。
    pub anchors: Vec<PolicyId>,
    /// 每座位每回合的决策上限；超过后强制结束回合（贪心策略会原地打转）。
    pub max_decisions_per_turn: usize,
}

impl Default for EvalConfig {
    fn default() -> Self {
        Self {
            seeds: (0..2000).collect(),
            learner_seats: SEAT_COUNT,
            anchors: Vec::new(),
            max_decisions_per_turn: DECISIONS_PER_TURN,
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
    /// 名次优于同桌锚点均值的局数。
    pub anchor_wins: usize,
    /// 名次劣于同桌锚点均值的局数。
    pub anchor_losses: usize,
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
    Random,
}

impl Opponent {
    fn weights(weights: &Weights) -> Result<Self, EvalError> {
        let model = ModelConfig::from_weights(weights)?;
        Ok(Self::Weights(Box::new((weights.clone(), model))))
    }
}

fn resolve_opponent(league: &League, policy: PolicyId) -> Result<Opponent, EvalError> {
    Ok(match policy {
        // 当前策略的权重不在联盟里，评测侧没有训练模型，只能落到随机兜底。
        PolicyId::Current | PolicyId::Random => Opponent::Random,
        PolicyId::Snapshot(_) | PolicyId::Anchor => {
            let weights = league.weights(policy).ok_or_else(|| {
                EvalError::NoData(format!("联盟缺少对手 {policy:?} 的权重"))
            })?;
            Opponent::weights(weights)?
        }
    })
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
        anchors.push(Opponent::weights(weights)?);
    }

    let mut env = FullEnv::new(catalog, League::new(0, 0));
    let mut placements = Vec::with_capacity(cfg.seeds.len());
    let mut anchor_wins = 0usize;
    let mut anchor_losses = 0usize;

    for (seed_index, seed) in cfg.seeds.iter().copied().enumerate() {
        let learner = (seed_index % cfg.learner_seats) as Seat;
        let candidates = allocate_candidates(seed, &playstyles::draw(seed), catalog);
        env.reset(seed, &candidates)
            .map_err(|error| EvalError::NoData(error.0))?;
        let mut rng = RandomPolicy::new(seed ^ 0xE7A1_0BEE ^ u64::from(learner));

        let mut turn = env.turn();
        let mut per_turn = vec![0usize; PLAYER_COUNT];
        let mut steps = 0usize;
        while steps < GUARD {
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
                        Opponent::Random => rng.below(features.len() as u64) as usize,
                    }
                }
            };
            env.step(candidates[choice].0.clone())
                .map_err(|error| EvalError::NoData(error.0))?;
            steps += 1;
            per_turn[index] += 1;
        }

        let placement = env
            .placement(learner)
            .ok_or_else(|| EvalError::NoData(format!("种子 {seed} 的对局没有结束")))?;
        placements.push(f32::from(placement));
        let anchor_placements: Vec<f32> = (0..PLAYER_COUNT as Seat)
            .filter(|seat| *seat != learner)
            .filter_map(|seat| env.placement(seat))
            .map(f32::from)
            .collect();
        if !anchor_placements.is_empty() {
            let anchor_mean =
                anchor_placements.iter().sum::<f32>() / anchor_placements.len() as f32;
            let own = f32::from(placement);
            if own < anchor_mean {
                anchor_wins += 1;
            } else if own > anchor_mean {
                anchor_losses += 1;
            }
        }
    }

    Ok(report(placements, anchor_wins, anchor_losses))
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

fn report(placements: Vec<f32>, anchor_wins: usize, anchor_losses: usize) -> EvalReport {
    let games = placements.len();
    let mean = placements.iter().sum::<f32>() / games as f32;
    let variance = if games > 1 {
        placements
            .iter()
            .map(|place| (place - mean).powi(2))
            .sum::<f32>()
            / (games - 1) as f32
    } else {
        0.0
    };
    let ci95 = 1.96 * variance.sqrt() / (games as f32).sqrt();
    let win_rate = placements.iter().filter(|place| **place == 1.0).count() as f32 / games as f32;
    let top4_rate = placements.iter().filter(|place| **place <= 4.0).count() as f32 / games as f32;
    EvalReport {
        games,
        mean_placement: mean,
        win_rate,
        top4_rate,
        ci95,
        anchor_wins,
        anchor_losses,
    }
}
