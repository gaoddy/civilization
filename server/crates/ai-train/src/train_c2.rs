//! C2 训练：8 人完整对局 + 联盟自博弈，并按 `c1_mix` 混入 C1 单座位局。
//!
//! 演员从 C1 权重初始化，critic 的价值头从 C0 权重初始化；学习者座位每局轮换，
//! 其余座位由联盟抽对手（当前策略 / 快照 / 锚点 / 随机）。

use std::ops::Range;

use ai_core::{Action, ActionFeatures, CardIndex, Observation};
use ai_model::{ModelConfig, Weights, forward};
use rule_engine::candidates::allocate_candidates;
use rule_engine::data::Catalog;
use rule_engine::error::EngineError;
use rule_engine::playstyles;
use rule_engine::state::Seat;

use crate::data_c0::BoardSample;
use crate::env::PLAYER_COUNT;
use crate::env_full::FullEnv;
use crate::env_recruit::{RecruitEnv, RewardMode};
use crate::eval::DECISIONS_PER_TURN;
use crate::league::{League, PolicyId};
use crate::ppo::{
    Batch, Metrics, PpoConfig, TrainModel, log_prob, masked_probs, sample_action, train_step,
};
use crate::random_policy::RandomPolicy;

/// 单局最多推进的决策数；正常对局远小于这个上限，超过视为异常并截断。
const EPISODE_GUARD: usize = 200_000;

/// 一局从哪个环境采样。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvMix {
    /// C1 单座位构筑局。
    C1,
    /// C2 八人完整对局。
    C2,
}

/// 按 `c1_mix` 交替选择环境的确定性配比器。
///
/// 用累加器而不是每次独立掷骰：任意前缀内的 C1 局数都贴着配置值，
/// 训练中不会出现某个批次全是 C2 的偏斜。
pub struct MixSampler {
    ratio: f32,
    acc: f32,
}

impl MixSampler {
    pub fn new(ratio: f32) -> Self {
        Self {
            ratio: ratio.clamp(0.0, 1.0),
            acc: 0.0,
        }
    }

    pub fn pick(&mut self) -> EnvMix {
        self.acc += self.ratio;
        if self.acc >= 1.0 {
            self.acc -= 1.0;
            EnvMix::C1
        } else {
            EnvMix::C2
        }
    }
}

/// C2 训练配置。
#[derive(Debug, Clone)]
pub struct C2Config {
    /// C1 单座位局在总对局中的占比。
    pub c1_mix: f32,
    /// 联盟保留的快照数。
    pub league_size: usize,
    /// 每多少次 PPO 更新加一份快照。
    pub snapshot_every: usize,
    /// 对局种子池；按对局序号循环取用。
    pub seeds: Range<u64>,
    pub episodes: usize,
    pub batch_episodes: usize,
    pub model: ModelConfig,
    pub ppo: PpoConfig,
    pub seed: u64,
}

impl Default for C2Config {
    fn default() -> Self {
        Self {
            c1_mix: 0.3,
            league_size: 8,
            snapshot_every: 20,
            seeds: 0..10_000,
            episodes: 1000,
            batch_episodes: 8,
            model: ModelConfig {
                obs_dim: ai_core::OBS_DIM,
                card_emb: 16,
                hidden: 64,
                layers: 2,
            },
            ppo: PpoConfig::default(),
            seed: 0,
        }
    }
}

/// 训练产物。
#[derive(Debug)]
pub struct C2Output {
    pub weights: Weights,
    pub episodes: usize,
    pub mean_placement: f32,
    pub mean_return: f32,
    pub snapshots: usize,
    pub last_metrics: Metrics,
}

/// C2 训练错误。
#[derive(Debug)]
pub enum C2Error {
    Candle(candle_core::Error),
    Engine(EngineError),
    Model(ai_model::ModelError),
    /// 初始化时缺少必需的张量。
    MissingTensor(String),
    Shape(String),
    NoData(String),
}

impl std::fmt::Display for C2Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Candle(error) => write!(f, "训练失败：{error}"),
            Self::Engine(error) => write!(f, "环境失败：{}", error.0),
            Self::Model(error) => write!(f, "模型失败：{error}"),
            Self::MissingTensor(name) => write!(f, "缺少张量：{name}"),
            Self::Shape(message) => write!(f, "张量形状不符：{message}"),
            Self::NoData(message) => write!(f, "没有可用数据：{message}"),
        }
    }
}

impl std::error::Error for C2Error {}

impl From<candle_core::Error> for C2Error {
    fn from(error: candle_core::Error) -> Self {
        Self::Candle(error)
    }
}

impl From<EngineError> for C2Error {
    fn from(error: EngineError) -> Self {
        Self::Engine(error)
    }
}

impl From<ai_model::ModelError> for C2Error {
    fn from(error: ai_model::ModelError) -> Self {
        Self::Model(error)
    }
}

/// 把 C1 权重装进模型：主干与候选头必须齐全，价值头留给 C0。
///
/// 缺张量或形状不符时报错并指出张量名，避免静默半初始化。
pub fn load_actor(model: &mut TrainModel, weights: &Weights) -> Result<(), C2Error> {
    let expected = model.weights()?;
    for (name, tensor) in &expected.tensors {
        if name.starts_with("value.") {
            continue;
        }
        match weights.tensors.get(name) {
            Some(found) if found.len() == tensor.len() => {}
            Some(found) => {
                return Err(C2Error::Shape(format!(
                    "{name} 长度 {} 与模型 {} 不符",
                    found.len(),
                    tensor.len()
                )));
            }
            None => return Err(C2Error::MissingTensor(name.clone())),
        }
    }
    model.load_matching(weights)?;
    Ok(())
}

/// 把 C0 权重里的价值头装进模型；主干沿用演员的。
pub fn load_critic(model: &mut TrainModel, weights: &Weights) -> Result<(), C2Error> {
    let expected = model.weights()?;
    let names: Vec<&String> = expected
        .tensors
        .keys()
        .filter(|name| name.starts_with("value."))
        .collect();
    if names.is_empty() {
        return Err(C2Error::NoData("模型里没有价值头".to_string()));
    }
    for name in names {
        if !weights.tensors.contains_key(name) {
            return Err(C2Error::MissingTensor(name.clone()));
        }
    }
    model.load_matching(weights)?;
    Ok(())
}

/// 联盟给某个座位抽到的对手。
enum Opponent {
    Current,
    Weights(Box<(Weights, ModelConfig)>),
    Random,
}

fn resolve_opponent(env: &FullEnv, policy: PolicyId) -> Result<Opponent, C2Error> {
    Ok(match policy {
        PolicyId::Current => Opponent::Current,
        PolicyId::Random => Opponent::Random,
        PolicyId::Snapshot(_) | PolicyId::Anchor => {
            let weights = env
                .league()
                .weights(policy)
                .cloned()
                .ok_or_else(|| C2Error::NoData(format!("联盟缺少对手 {policy:?}")))?;
            let model = ModelConfig::from_weights(&weights)?;
            Opponent::Weights(Box::new((weights, model)))
        }
    })
}

#[allow(clippy::too_many_arguments)]
fn record(
    batch: &mut Batch,
    obs: Observation,
    choice: usize,
    mask: Vec<bool>,
    logp: f32,
    value: f32,
    reward: f32,
    done: bool,
    features: Vec<ActionFeatures>,
) {
    batch.obs.push(obs);
    batch.actions.push(choice);
    batch.masks.push(mask);
    batch.logps.push(logp);
    batch.values.push(value);
    batch.rewards.push(reward);
    batch.dones.push(done);
    batch.candidates.push(features);
}

fn empty_batch() -> Batch {
    Batch {
        obs: Vec::new(),
        actions: Vec::new(),
        masks: Vec::new(),
        logps: Vec::new(),
        values: Vec::new(),
        rewards: Vec::new(),
        dones: Vec::new(),
        candidates: Vec::new(),
    }
}

/// 跑一局 C1 单座位局，把学习者的决策写进批次。
#[allow(clippy::too_many_arguments)]
fn run_c1_episode(
    env: &mut RecruitEnv,
    model: &mut TrainModel,
    distribution: &[BoardSample],
    heroes: &[String],
    episode_index: usize,
    seed: u64,
    rng: &mut RandomPolicy,
    batch: &mut Batch,
) -> Result<f32, C2Error> {
    let opponent = &distribution[episode_index % distribution.len()];
    let hero = heroes[rng.below(heroes.len() as u64) as usize].clone();
    env.reset(seed.wrapping_add(episode_index as u64), &hero, opponent)?;

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
        let forced = if per_turn >= DECISIONS_PER_TURN {
            candidates
                .iter()
                .position(|(candidate, _)| matches!(candidate, Action::End))
        } else {
            None
        };
        let choice = forced.unwrap_or_else(|| sample_action(&masked_probs(&logits, &mask), rng));
        let logp = log_prob(&logits, &mask, choice);
        let stepped = env.step(candidates[choice].0.clone())?;
        episode_return += stepped.reward;
        record(
            batch,
            obs,
            choice,
            mask,
            logp,
            value,
            stepped.reward,
            stepped.done,
            features,
        );
        steps += 1;
        per_turn += 1;
    }
    Ok(episode_return)
}

/// 跑一局 C2 完整对局，学习者座位轮换，其余座位按联盟对手出招。
fn run_c2_episode(
    catalog: &Catalog,
    env: &mut FullEnv,
    model: &mut TrainModel,
    episode_index: usize,
    seed: u64,
    rng: &mut RandomPolicy,
    batch: &mut Batch,
) -> Result<(f32, Option<u8>), C2Error> {
    let candidates = allocate_candidates(seed, &playstyles::draw(seed), catalog);
    env.reset(seed, &candidates)?;
    let learner = (episode_index % PLAYER_COUNT) as Seat;

    let mut opponents: Vec<Option<Opponent>> = (0..PLAYER_COUNT).map(|_| None).collect();
    for seat in 0..PLAYER_COUNT as Seat {
        if seat == learner {
            continue;
        }
        let policy = env.sample_policy(seat);
        opponents[usize::from(seat)] = Some(resolve_opponent(env, policy)?);
    }

    let mut episode_return = 0.0f32;
    let mut steps = 0usize;
    let mut turn = env.turn();
    let mut per_turn = vec![0usize; PLAYER_COUNT];
    // 学习者最后一次采样；若对局在别人手里结束，要用它补一条终局 transition。
    let mut learner_sample: Option<(Observation, usize, Vec<bool>, f32, f32, Vec<ActionFeatures>)> =
        None;
    let mut learner_recorded_done = false;
    while !env.is_done() && steps < EPISODE_GUARD {
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
        let forced = if per_turn[index] >= DECISIONS_PER_TURN {
            candidates
                .iter()
                .position(|(candidate, _)| matches!(candidate, Action::End))
        } else {
            None
        };

        let (choice, recorded) = if seat == learner {
            let mask = vec![true; features.len()];
            let (logits, value) = model.logits(&obs, &features)?;
            let choice =
                forced.unwrap_or_else(|| sample_action(&masked_probs(&logits, &mask), rng));
            let logp = log_prob(&logits, &mask, choice);
            (choice, Some((mask, logp, value)))
        } else {
            let opponent = opponents[index]
                .as_ref()
                .ok_or_else(|| C2Error::NoData("对手未初始化".to_string()))?;
            let choice = opponent_choice(opponent, &obs, &features, model, forced, rng)?;
            (choice, None)
        };
        let stepped = env.step(candidates[choice].0.clone())?;
        let reward = if stepped.done {
            stepped.terminal_reward[index]
        } else {
            0.0
        };
        if let Some((mask, logp, value)) = recorded {
            // 对局回报只统计学习者自己的奖励；最后行动者的名次奖励与学习者无关。
            episode_return += reward;
            learner_recorded_done = stepped.done;
            learner_sample = Some((
                obs.clone(),
                choice,
                mask.clone(),
                logp,
                value,
                features.clone(),
            ));
            record(
                batch,
                obs,
                choice,
                mask,
                logp,
                value,
                reward,
                stepped.done,
                features,
            );
        }
        steps += 1;
        per_turn[index] += 1;
    }
    // 学习者多半不是最后一个行动者：对局结束后必须为它补一条带名次奖励的
    // 终局 transition，否则它的轨迹永远 `done = false`，GAE 会跨局 bootstrap，
    // 名次奖励也从不进入损失。
    if env.is_done()
        && !learner_recorded_done
        && let Some((obs, choice, mask, logp, value, features)) = learner_sample
    {
        let reward = env
            .placement(learner)
            .map(|place| {
                (PLAYER_COUNT as f32 + 1.0 - f32::from(place)) / PLAYER_COUNT as f32
            })
            .unwrap_or(0.0);
        episode_return += reward;
        record(batch, obs, choice, mask, logp, value, reward, true, features);
    }
    Ok((episode_return, env.placement(learner)))
}

fn opponent_choice(
    opponent: &Opponent,
    obs: &Observation,
    features: &[ActionFeatures],
    model: &mut TrainModel,
    forced: Option<usize>,
    rng: &mut RandomPolicy,
) -> Result<usize, C2Error> {
    if let Some(forced) = forced {
        return Ok(forced);
    }
    Ok(match opponent {
        Opponent::Current => {
            let (logits, _) = model.logits(obs, features)?;
            let mask = vec![true; features.len()];
            sample_action(&masked_probs(&logits, &mask), rng)
        }
        Opponent::Weights(boxed) => {
            let (weights, cfg) = &**boxed;
            let (logits, _) = forward(weights, cfg, obs, features)?;
            let mask = vec![true; features.len()];
            sample_action(&masked_probs(&logits, &mask), rng)
        }
        Opponent::Random => rng.below(features.len() as u64) as usize,
    })
}

/// 训练 C2 策略。
pub fn train_c2(
    catalog: &Catalog,
    distribution: &[BoardSample],
    c0: Option<&Weights>,
    c1: Option<&Weights>,
    cfg: &C2Config,
) -> Result<C2Output, C2Error> {
    let span = cfg.seeds.end.saturating_sub(cfg.seeds.start);
    if span == 0 {
        return Err(C2Error::NoData("种子池为空".to_string()));
    }
    if cfg.c1_mix > 0.0 && distribution.is_empty() {
        return Err(C2Error::NoData("C1 混训需要棋盘分布".to_string()));
    }
    let heroes: Vec<String> = catalog.heroes.keys().cloned().collect();
    if heroes.is_empty() {
        return Err(C2Error::NoData("目录里没有英雄".to_string()));
    }
    let index = CardIndex::build(catalog);
    let mut model = TrainModel::new(
        &cfg.model,
        index.card_count(),
        index.hero_count(),
        index.skill_count(),
        cfg.seed,
    )?;
    if let Some(c1) = c1 {
        load_actor(&mut model, c1)?;
    }
    if let Some(c0) = c0 {
        load_critic(&mut model, c0)?;
    }
    let mut c1_env = RecruitEnv::new(catalog, c0.cloned(), cfg.model);
    if c0.is_some() {
        c1_env.set_reward_mode(RewardMode::Value);
    }

    let mut league = League::new(cfg.seed ^ 0x1EA6_0E01, cfg.league_size);
    if let Some(c1) = c1 {
        league.set_anchor(c1.clone());
    }
    let mut env = FullEnv::new(catalog, league);

    let mut rng = RandomPolicy::new(cfg.seed ^ 0x51EE_D1CE);
    let mut mix = MixSampler::new(cfg.c1_mix);
    let mut returns = Vec::with_capacity(cfg.episodes);
    let mut placements = Vec::new();
    let mut episodes = 0usize;
    let mut updates = 0usize;
    let mut last_metrics = Metrics {
        policy_loss: 0.0,
        value_loss: 0.0,
        entropy: 0.0,
        clip_fraction: 0.0,
    };
    while episodes < cfg.episodes {
        let mut batch = empty_batch();
        let wanted = cfg.batch_episodes.min(cfg.episodes - episodes);
        for _ in 0..wanted {
            let seed = cfg.seeds.start + (episodes as u64 % span);
            let episode_return = match mix.pick() {
                EnvMix::C1 => run_c1_episode(
                    &mut c1_env,
                    &mut model,
                    distribution,
                    &heroes,
                    episodes,
                    seed,
                    &mut rng,
                    &mut batch,
                )?,
                EnvMix::C2 => {
                    let (episode_return, placement) = run_c2_episode(
                        catalog, &mut env, &mut model, episodes, seed, &mut rng, &mut batch,
                    )?;
                    if let Some(place) = placement {
                        placements.push(f32::from(place));
                    }
                    episode_return
                }
            };
            returns.push(episode_return);
            episodes += 1;
        }
        if !batch.is_empty() {
            last_metrics = train_step(&mut model, &batch, &cfg.ppo)?;
            updates += 1;
            if cfg.snapshot_every > 0 && updates.is_multiple_of(cfg.snapshot_every) {
                let snapshot = model.weights()?;
                env.league_mut().add_snapshot(snapshot);
            }
        }
    }

    let mean = |values: &[f32]| {
        if values.is_empty() {
            0.0
        } else {
            values.iter().sum::<f32>() / values.len() as f32
        }
    };
    Ok(C2Output {
        weights: model.weights()?,
        episodes,
        mean_placement: mean(&placements),
        mean_return: mean(&returns),
        snapshots: env.league().snapshots().len(),
        last_metrics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn catalog() -> Catalog {
        Catalog::from_json(
            include_str!("../../../data/v0.1/cards.json"),
            include_str!("../../../data/v0.1/heroes.json"),
            include_str!("../../../data/v0.1/skills.json"),
            include_str!("../../../data/v0.1/config.json"),
            include_str!("../../../data/v0.1/taxonomy.json"),
        )
        .expect("卡表")
    }

    /// 学习者不是最后行动者时，也必须恰有一条带名次奖励的终局 transition，
    /// 否则 GAE 会跨局 bootstrap，名次目标根本不进入损失。
    #[test]
    fn every_learner_episode_ends_with_one_terminal_transition() {
        let catalog = catalog();
        let cfg = C2Config {
            seeds: 0..8,
            ..C2Config::default()
        };
        let mut env = FullEnv::new(&catalog, League::new(cfg.seed, cfg.league_size));
        let index = ai_core::CardIndex::build(&catalog);
        let mut model = TrainModel::new(
            &cfg.model,
            index.card_count(),
            index.hero_count(),
            index.skill_count(),
            cfg.seed,
        )
        .expect("模型");
        let mut rng = RandomPolicy::new(cfg.seed);

        for episode in 0..8usize {
            let mut batch = empty_batch();
            let (episode_return, placement) = run_c2_episode(
                &catalog,
                &mut env,
                &mut model,
                episode,
                episode as u64,
                &mut rng,
                &mut batch,
            )
            .expect("对局应能跑完");
            let dones: Vec<usize> = batch
                .dones
                .iter()
                .enumerate()
                .filter(|(_, done)| **done)
                .map(|(row, _)| row)
                .collect();
            assert_eq!(dones.len(), 1, "第 {episode} 局应恰有一条终局 transition");
            assert_eq!(dones[0], batch.len() - 1, "终局 transition 应在最末");
            let place = placement.expect("终局应有名次");
            let expected = (PLAYER_COUNT as f32 + 1.0 - f32::from(place)) / PLAYER_COUNT as f32;
            assert!(
                (batch.rewards[dones[0]] - expected).abs() < 1e-6,
                "终局奖励应等于名次奖励：{} vs {expected}",
                batch.rewards[dones[0]]
            );
            assert!(
                (episode_return - expected).abs() < 1e-6,
                "对局回报应只含终局名次奖励：{episode_return} vs {expected}，place {place}"
            );
        }
    }
}
