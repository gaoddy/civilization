//! C1 训练：单座位构筑环境 + PPO。
//!
//! 每批采集若干整局，按候选动作采样并记录；训练与采样解耦，奖励由
//! `RecruitEnv` 给出（C0 棋盘强度差或战斗结果）。

use ai_core::{ActionFeatures, CardIndex};
use ai_model::{ModelConfig, Weights};
use rule_engine::data::Catalog;
use rule_engine::error::EngineError;

use crate::data_c0::BoardSample;
use crate::env_recruit::{RecruitEnv, RewardMode};
use crate::ppo::{
    Batch, Metrics, PpoConfig, TrainModel, log_prob, masked_probs, sample_action, train_step,
};
use crate::random_policy::RandomPolicy;

/// 单局最多推进的决策数；正常对局远小于这个上限，超过视为异常并截断。
const EPISODE_GUARD: usize = 20_000;

/// C1 训练配置。
///
/// 默认结构与 C0 产物一致（64×2、嵌入 16），才能把 C0 的主干与价值头
/// 直接带进来；放大结构属于后续单独的实验。
#[derive(Debug, Clone)]
pub struct C1Config {
    pub model: ModelConfig,
    pub ppo: PpoConfig,
    /// 总训练局数。
    pub episodes: usize,
    /// 每次 PPO 更新采集的局数。
    pub batch_episodes: usize,
    pub seed: u64,
    /// 奖励用 C0 棋盘强度差（需要 C0 权重）；否则直接用战斗结果。
    pub value_reward: bool,
    /// 用 C0 权重初始化主干与价值头。
    pub init_from_c0: bool,
}

impl Default for C1Config {
    fn default() -> Self {
        Self {
            model: ModelConfig {
                obs_dim: ai_core::OBS_DIM,
                card_emb: 16,
                hidden: 64,
                layers: 2,
            },
            ppo: PpoConfig::default(),
            episodes: 200,
            batch_episodes: 4,
            seed: 0,
            value_reward: true,
            init_from_c0: true,
        }
    }
}

/// 训练产物。
#[derive(Debug)]
pub struct C1Output {
    pub weights: Weights,
    pub episodes: usize,
    pub mean_return: f32,
    pub last_metrics: Metrics,
}

/// C1 训练错误。
#[derive(Debug)]
pub enum C1Error {
    Candle(candle_core::Error),
    Engine(EngineError),
    NoData(String),
}

impl std::fmt::Display for C1Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Candle(error) => write!(f, "训练失败：{error}"),
            Self::Engine(error) => write!(f, "环境失败：{}", error.0),
            Self::NoData(message) => write!(f, "没有可用数据：{message}"),
        }
    }
}

impl std::error::Error for C1Error {}

impl From<candle_core::Error> for C1Error {
    fn from(error: candle_core::Error) -> Self {
        Self::Candle(error)
    }
}

impl From<EngineError> for C1Error {
    fn from(error: EngineError) -> Self {
        Self::Engine(error)
    }
}

pub fn train_c1(
    catalog: &Catalog,
    distribution: &[BoardSample],
    c0: Option<&Weights>,
    cfg: &C1Config,
) -> Result<C1Output, C1Error> {
    if distribution.is_empty() {
        return Err(C1Error::NoData("棋盘分布为空".to_string()));
    }
    let heroes: Vec<String> = catalog.heroes.keys().cloned().collect();
    if heroes.is_empty() {
        return Err(C1Error::NoData("目录里没有英雄".to_string()));
    }
    let index = CardIndex::build(catalog);
    let mut model = TrainModel::new(
        &cfg.model,
        index.card_count(),
        index.hero_count(),
        index.skill_count(),
        cfg.seed,
    )?;
    if cfg.init_from_c0
        && let Some(c0) = c0
    {
        model.load_matching(c0)?;
    }
    let value_weights = if cfg.value_reward { c0.cloned() } else { None };
    if cfg.value_reward && value_weights.is_none() {
        return Err(C1Error::NoData("价值奖励需要 C0 权重".to_string()));
    }
    let mut env = RecruitEnv::new(catalog, value_weights, cfg.model);
    env.set_reward_mode(if cfg.value_reward {
        RewardMode::Value
    } else {
        RewardMode::Combat
    });

    let mut rng = RandomPolicy::new(cfg.seed ^ 0x51EE_D1CE);
    let mut returns = Vec::with_capacity(cfg.episodes);
    let mut episodes = 0usize;
    let mut last_metrics = Metrics {
        policy_loss: 0.0,
        value_loss: 0.0,
        entropy: 0.0,
        clip_fraction: 0.0,
    };
    while episodes < cfg.episodes {
        let mut batch = Batch {
            obs: Vec::new(),
            actions: Vec::new(),
            masks: Vec::new(),
            logps: Vec::new(),
            values: Vec::new(),
            rewards: Vec::new(),
            dones: Vec::new(),
            candidates: Vec::new(),
        };
        let wanted = cfg.batch_episodes.min(cfg.episodes - episodes);
        for _ in 0..wanted {
            let opponent = &distribution[episodes % distribution.len()];
            let hero = heroes[rng.below(heroes.len() as u64) as usize].clone();
            env.reset(cfg.seed.wrapping_add(episodes as u64), &hero, opponent)?;
            episodes += 1;
            let mut episode_return = 0.0f32;
            let mut steps = 0usize;
            let mut turn = env.turn();
            let mut per_turn = 0usize;
            while !env.is_done() && steps < EPISODE_GUARD {
                if env.turn() != turn {
                    turn = env.turn();
                    per_turn = 0;
                }
                let obs = env.observe();
                let candidates = env.action_candidates();
                let features: Vec<ActionFeatures> =
                    candidates.iter().map(|(_, feature)| *feature).collect();
                let (logits, value) = model.logits(&obs, &features)?;
                let mask = vec![true; features.len()];
                let forced = if per_turn >= crate::eval::DECISIONS_PER_TURN {
                    candidates
                        .iter()
                        .position(|(candidate, _)| matches!(candidate, ai_core::Action::End))
                } else {
                    None
                };
                let choice = forced
                    .unwrap_or_else(|| sample_action(&masked_probs(&logits, &mask), &mut rng));
                let logp = log_prob(&logits, &mask, choice);
                let action = candidates[choice].0.clone();
                let stepped = env.step(action)?;
                episode_return += stepped.reward;
                batch.obs.push(obs);
                batch.actions.push(choice);
                batch.masks.push(mask);
                batch.logps.push(logp);
                batch.values.push(value);
                batch.rewards.push(stepped.reward);
                batch.dones.push(stepped.done);
                batch.candidates.push(features);
                steps += 1;
                per_turn += 1;
            }
            returns.push(episode_return);
        }
        if !batch.is_empty() {
            last_metrics = train_step(&mut model, &batch, &cfg.ppo)?;
        }
    }

    let mean_return = if returns.is_empty() {
        0.0
    } else {
        returns.iter().sum::<f32>() / returns.len() as f32
    };
    Ok(C1Output {
        weights: model.weights()?,
        episodes,
        mean_return,
        last_metrics,
    })
}
