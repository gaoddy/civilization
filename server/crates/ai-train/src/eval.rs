//! C1 评测：在冻结棋盘分布上按固定种子跑贪心打法。
//!
//! 评测只做推理（`ai_model::forward`），不依赖 candle；同一份权重、同一组种子
//! 与同一份分布必须给出同一份报告，用于比较检查点。

use ai_core::Action;
use ai_model::{ModelConfig, Weights, forward};
use rule_engine::data::Catalog;
use serde::Serialize;

use crate::data_c0::BoardSample;
use crate::env_recruit::{RecruitEnv, RewardMode};

/// 评测报告：`win_rate` 是最后一回合战斗的胜率，`mean_margin` 是同一回合
/// 奖励（胜 +1、平 0、负 −1）的均值。
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct EvalReport {
    pub win_rate: f32,
    pub mean_margin: f32,
    pub games: usize,
}

/// 单回合最多决策数；超过后强制结束招募，避免零费动作之间原地打转。
pub const DECISIONS_PER_TURN: usize = 256;
/// 单局最多推进的决策数；正常对局远小于这个上限，超过视为异常并截断。
const STEP_GUARD: usize = 20_000;

pub fn eval_c1(
    catalog: &Catalog,
    weights: &Weights,
    distribution: &[BoardSample],
    seeds: &[u64],
) -> EvalReport {
    if distribution.is_empty() || seeds.is_empty() {
        return EvalReport {
            win_rate: 0.0,
            mean_margin: 0.0,
            games: 0,
        };
    }
    let model = ModelConfig::from_weights(weights).expect("权重应能反推结构");
    let mut env = RecruitEnv::new(catalog, Some(weights.clone()), model);
    env.set_reward_mode(RewardMode::Combat);
    let mut wins = 0usize;
    let mut margin = 0.0f64;
    for (position, seed) in seeds.iter().enumerate() {
        let opponent = &distribution[position % distribution.len()];
        env.reset(*seed, &opponent.hero, opponent)
            .expect("评测重置失败");
        let mut last_reward = 0.0f32;
        let mut steps = 0usize;
        let mut turn = env.turn();
        let mut per_turn = 0usize;
        while !env.is_done() && steps < STEP_GUARD {
            if env.turn() != turn {
                turn = env.turn();
                per_turn = 0;
            }
            let candidates = env.action_candidates();
            let features: Vec<_> = candidates.iter().map(|(_, feature)| *feature).collect();
            let (logits, _) =
                forward(weights, &model, &env.observe(), &features).expect("评测前向");
            let forced = if per_turn >= DECISIONS_PER_TURN {
                candidates
                    .iter()
                    .position(|(candidate, _)| matches!(candidate, Action::End))
            } else {
                None
            };
            let best = forced.unwrap_or_else(|| argmax(&logits));
            let action = candidates[best].0.clone();
            let stepped = env.step(action).expect("贪心动作应是合法动作");
            if stepped.combat.is_some() {
                last_reward = stepped.reward;
            }
            steps += 1;
            per_turn += 1;
        }
        if last_reward > 0.0 {
            wins += 1;
        }
        margin += f64::from(last_reward);
    }
    let games = seeds.len();
    EvalReport {
        win_rate: wins as f32 / games as f32,
        mean_margin: (margin / games as f64) as f32,
        games,
    }
}

fn argmax(logits: &[f32]) -> usize {
    let mut best = 0usize;
    for (index, logit) in logits.iter().enumerate() {
        if *logit > logits[best] {
            best = index;
        }
    }
    best
}
