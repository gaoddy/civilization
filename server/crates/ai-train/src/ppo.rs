//! PPO：GAE、优势归一化、clip、熵奖励与动作掩码。
//!
//! 动作打分与推理侧（`ai_model`）同构：主干 + 价值头 + per-candidate 头，
//! 张量命名一致，训练得到的权重可直接被推理侧加载。
//! 训练与采样解耦：采样侧用 [`Batch`] 记录决策，[`train_step`] 只消费批次。

use ai_core::{ACTION_FEATURE_DIM, ActionFeatures, OBS_DIM, Observation};
use ai_model::{ModelConfig, Weights};
use candle_core::{DType, Device, Tensor};
use candle_nn::{AdamW, Linear, Module, Optimizer, ParamsAdamW, VarBuilder, VarMap};

use crate::model::{extract_weights, seed_weights};
use crate::random_policy::RandomPolicy;

/// 价值损失权重；计划未把它放进配置，这里固定。
pub const VALUE_COEF: f32 = 0.5;

/// 观测头部中英雄所在的列。
const HERO_COLUMN: usize = 3;

/// PPO 超参数。
#[derive(Debug, Clone)]
pub struct PpoConfig {
    pub gamma: f32,
    pub lambda: f32,
    pub clip: f32,
    pub lr: f32,
    pub entropy: f32,
    pub epochs: usize,
}

impl Default for PpoConfig {
    fn default() -> Self {
        Self {
            gamma: 0.99,
            lambda: 0.95,
            clip: 0.2,
            lr: 3e-4,
            entropy: 0.01,
            epochs: 4,
        }
    }
}

/// 一次 `train_step` 的指标。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    pub policy_loss: f32,
    pub value_loss: f32,
    pub entropy: f32,
    pub clip_fraction: f32,
}

/// 一批决策：每条记录当次观测、候选动作与采样时的对数概率。
#[derive(Debug, Clone, PartialEq)]
pub struct Batch {
    pub obs: Vec<Observation>,
    /// 动作在 `candidates[i]` 中的下标。
    pub actions: Vec<usize>,
    /// 与 `candidates[i]` 等长；`false` 表示该候选不可选。
    pub masks: Vec<Vec<bool>>,
    /// 采样时的对数概率，用于 importance ratio。
    pub logps: Vec<f32>,
    pub values: Vec<f32>,
    pub rewards: Vec<f32>,
    pub dones: Vec<bool>,
    /// 每次决策的候选特征；训练侧按这份列表逐个打分。
    pub candidates: Vec<Vec<ActionFeatures>>,
}

impl Batch {
    pub fn len(&self) -> usize {
        self.obs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.obs.is_empty()
    }
}

/// 训练侧策略网络：主干 + 价值头 + 候选头 + 三张嵌入表。
pub struct TrainModel {
    var_map: VarMap,
    optimizer: Option<AdamW>,
    trunk: Vec<Linear>,
    value_hidden: Linear,
    value_out: Linear,
    candidate_hidden: Linear,
    candidate_out: Linear,
    card: Tensor,
    hero: Tensor,
    skill: Tensor,
    cfg: ModelConfig,
    card_count: usize,
    hero_count: usize,
    skill_count: usize,
}

impl TrainModel {
    pub fn new(
        cfg: &ModelConfig,
        card_count: usize,
        hero_count: usize,
        skill_count: usize,
        seed: u64,
    ) -> candle_core::Result<Self> {
        let device = Device::Cpu;
        let var_map = VarMap::new();
        let vb = VarBuilder::from_varmap(&var_map, DType::F32, &device);
        build_layers(vb, cfg, card_count, hero_count, skill_count)?;
        seed_weights(&var_map, seed)?;
        let vb = VarBuilder::from_varmap(&var_map, DType::F32, &device);
        let layers = build_layers(vb, cfg, card_count, hero_count, skill_count)?;
        Ok(Self {
            var_map,
            optimizer: None,
            trunk: layers.trunk,
            value_hidden: layers.value_hidden,
            value_out: layers.value_out,
            candidate_hidden: layers.candidate_hidden,
            candidate_out: layers.candidate_out,
            card: layers.card,
            hero: layers.hero,
            skill: layers.skill,
            cfg: *cfg,
            card_count,
            hero_count,
            skill_count,
        })
    }

    /// 用权重文件里同名同形状的张量覆盖当前变量；其余张量保持不动。
    ///
    /// 用于把 C0 的主干与价值头、或上一个检查点的策略参数带进新网络。
    /// 返回实际覆盖的张量数；形状不同的张量报错，避免静默半初始化。
    pub fn load_matching(&mut self, weights: &Weights) -> candle_core::Result<usize> {
        let device = Device::Cpu;
        let mut loaded = 0usize;
        {
            let mut data = self.var_map.data().lock().expect("VarMap 锁");
            for (name, values) in &weights.tensors {
                let Some(var) = data.get_mut(name) else {
                    continue;
                };
                let dims = var.as_tensor().dims().to_vec();
                if var.as_tensor().elem_count() != values.len() {
                    return Err(candle_core::Error::Msg(format!(
                        "张量 {name} 形状 {dims:?} 与权重 {} 个元素不符",
                        values.len()
                    )));
                }
                var.set(&Tensor::from_vec(values.clone(), dims, &device)?)?;
                loaded += 1;
            }
        }
        self.rebuild()?;
        Ok(loaded)
    }

    /// 变量被原地修改后重建模块引用（`Linear` 持有的是旧存储的克隆）。
    fn rebuild(&mut self) -> candle_core::Result<()> {
        let device = Device::Cpu;
        let vb = VarBuilder::from_varmap(&self.var_map, DType::F32, &device);
        let layers = build_layers(
            vb,
            &self.cfg,
            self.card_count,
            self.hero_count,
            self.skill_count,
        )?;
        self.trunk = layers.trunk;
        self.value_hidden = layers.value_hidden;
        self.value_out = layers.value_out;
        self.candidate_hidden = layers.candidate_hidden;
        self.candidate_out = layers.candidate_out;
        self.card = layers.card;
        self.hero = layers.hero;
        self.skill = layers.skill;
        Ok(())
    }

    pub fn config(&self) -> &ModelConfig {
        &self.cfg
    }

    /// 导出为推理侧权重。
    pub fn weights(&self) -> Result<Weights, candle_core::Error> {
        let mut weights = extract_weights(&self.var_map)?;
        weights.card_count = self.card_count;
        weights.hero_count = self.hero_count;
        weights.skill_count = self.skill_count;
        Ok(weights)
    }

    /// 批量前向：`obs` 为 `[batch, OBS_DIM]`，`offsets` 长 `batch + 1`。
    ///
    /// 返回每个候选的 logit（扁平、按决策顺序）与每个决策的状态价值。
    pub fn forward(
        &self,
        obs: &Tensor,
        candidates: &[ActionFeatures],
        offsets: &[usize],
    ) -> candle_core::Result<(Tensor, Tensor)> {
        let batch = obs.dims()[0];
        if offsets.len() != batch + 1 || offsets[batch] != candidates.len() {
            return Err(candle_core::Error::Msg(format!(
                "候选分段与批次不符：batch {batch}、offsets {offsets:?}、候选 {}",
                candidates.len()
            )));
        }
        let device = obs.device();

        let mut hidden = obs.clone();
        for layer in &self.trunk {
            hidden = layer.forward(&hidden)?.relu()?;
        }
        let value = self
            .value_out
            .forward(&self.value_hidden.forward(&hidden)?.relu()?)?
            .squeeze(1)?;

        let total = candidates.len();
        if total == 0 {
            let logits = Tensor::zeros(0, DType::F32, device)?;
            return Ok((logits, value));
        }

        let mut row_ids = Vec::with_capacity(total);
        for decision in 0..batch {
            for _ in offsets[decision]..offsets[decision + 1] {
                row_ids.push(decision as i64);
            }
        }
        let row_ids = Tensor::from_vec(row_ids, total, device)?;
        let trunk_rows = hidden.index_select(&row_ids, 0)?;

        let obs_hero: Vec<f32> = obs
            .narrow(1, HERO_COLUMN, 1)?
            .flatten_all()?
            .to_vec1::<f32>()?;
        let hero_indices: Vec<i32> = candidates
            .iter()
            .enumerate()
            .map(|(position, feature)| {
                if feature.hero >= 0 {
                    feature.hero
                } else {
                    let decision = decision_of(offsets, position);
                    obs_hero[decision] as i32
                }
            })
            .collect();
        let card_indices: Vec<i32> = candidates.iter().map(|feature| feature.card).collect();
        let target_indices: Vec<i32> = candidates
            .iter()
            .map(|feature| feature.target_card)
            .collect();
        let skill_indices: Vec<i32> = candidates.iter().map(|feature| feature.skill).collect();

        let card_table = self.card.clone();
        let hero_table = self.hero.clone();
        let skill_table = self.skill.clone();
        let card_rows = embed(&card_table, &card_indices, "card.emb")?;
        let hero_rows = embed(&hero_table, &hero_indices, "hero.emb")?;
        let skill_rows = embed(&skill_table, &skill_indices, "skill.emb")?;
        let target_rows = embed(&card_table, &target_indices, "card.emb")?;

        let mut values = Vec::with_capacity(total * ACTION_FEATURE_DIM);
        for feature in candidates {
            values.extend_from_slice(&feature.values);
        }
        let values = Tensor::from_vec(values, (total, ACTION_FEATURE_DIM), device)?;
        let input = Tensor::cat(
            &[
                card_rows,
                hero_rows,
                skill_rows,
                target_rows,
                values,
                trunk_rows,
            ],
            1,
        )?;
        let scored = self.candidate_hidden.forward(&input)?.relu()?;
        let logits = self.candidate_out.forward(&scored)?.squeeze(1)?;
        Ok((logits, value))
    }

    /// 单条决策的 logit 与价值，供采样侧使用。
    pub fn logits(
        &self,
        obs: &Observation,
        candidates: &[ActionFeatures],
    ) -> candle_core::Result<(Vec<f32>, f32)> {
        let device = Device::Cpu;
        let input = Tensor::from_vec(obs.to_vec(), (1, OBS_DIM), &device)?;
        let mut offsets = vec![0usize];
        offsets.push(candidates.len());
        let (logits, value) = self.forward(&input, candidates, &offsets)?;
        Ok((logits.to_vec1::<f32>()?, value.to_vec1::<f32>()?[0]))
    }
}

struct Layers {
    trunk: Vec<Linear>,
    value_hidden: Linear,
    value_out: Linear,
    candidate_hidden: Linear,
    candidate_out: Linear,
    card: Tensor,
    hero: Tensor,
    skill: Tensor,
}

fn build_layers(
    vb: VarBuilder<'_>,
    cfg: &ModelConfig,
    card_count: usize,
    hero_count: usize,
    skill_count: usize,
) -> candle_core::Result<Layers> {
    let mut trunk = Vec::with_capacity(cfg.layers);
    let mut width = cfg.obs_dim;
    for layer in 0..cfg.layers {
        trunk.push(candle_nn::linear(
            width,
            cfg.hidden,
            vb.pp(format!("trunk.{layer}")),
        )?);
        width = cfg.hidden;
    }
    let value_hidden = candle_nn::linear(cfg.hidden, cfg.hidden, vb.pp("value.0"))?;
    let value_out = candle_nn::linear(cfg.hidden, 1, vb.pp("value.1"))?;
    let candidate_hidden = candle_nn::linear(cfg.candidate_input(), cfg.hidden, vb.pp("cand.0"))?;
    let candidate_out = candle_nn::linear(cfg.hidden, 1, vb.pp("cand.1"))?;
    let card = vb.get((card_count, cfg.card_emb), "card.emb")?;
    let hero = vb.get((hero_count, cfg.card_emb), "hero.emb")?;
    let skill = vb.get((skill_count, cfg.card_emb), "skill.emb")?;
    Ok(Layers {
        trunk,
        value_hidden,
        value_out,
        candidate_hidden,
        candidate_out,
        card,
        hero,
        skill,
    })
}

/// 某候选所属的决策下标。
fn decision_of(offsets: &[usize], position: usize) -> usize {
    offsets
        .partition_point(|start| *start <= position)
        .saturating_sub(1)
}

/// 嵌入查表：负索引取零向量，越界报错。
fn embed(table: &Tensor, indices: &[i32], name: &str) -> candle_core::Result<Tensor> {
    let device = table.device();
    let count = table.dims()[0];
    let mut clamped = Vec::with_capacity(indices.len());
    let mut present = Vec::with_capacity(indices.len());
    for &index in indices {
        if index < 0 {
            clamped.push(0i64);
            present.push(0f32);
        } else {
            let index = index as usize;
            if index >= count {
                return Err(candle_core::Error::Msg(format!(
                    "{name} 索引越界：{index}（表长 {count}）"
                )));
            }
            clamped.push(index as i64);
            present.push(1f32);
        }
    }
    let rows = table.index_select(&Tensor::from_vec(clamped, indices.len(), device)?, 0)?;
    let mask = Tensor::from_vec(present, (indices.len(), 1), device)?;
    rows.broadcast_mul(&mask)
}

/// 掩码 softmax：被屏蔽项严格为 0；全部屏蔽时全 0。
pub fn masked_probs(logits: &[f32], mask: &[bool]) -> Vec<f32> {
    assert_eq!(logits.len(), mask.len(), "logit 与掩码长度不符");
    let mut probs = vec![0.0f32; logits.len()];
    let mut max = f32::NEG_INFINITY;
    for (logit, valid) in logits.iter().zip(mask) {
        if *valid && *logit > max {
            max = *logit;
        }
    }
    if !max.is_finite() {
        return probs;
    }
    let mut total = 0.0f32;
    for (index, (logit, valid)) in logits.iter().zip(mask).enumerate() {
        if *valid {
            let value = (logit - max).exp();
            probs[index] = value;
            total += value;
        }
    }
    if total > 0.0 {
        for probability in &mut probs {
            *probability /= total;
        }
    }
    probs
}

/// 按概率采样一个下标；概率全 0 时退回 0。
pub fn sample_action(probs: &[f32], rng: &mut RandomPolicy) -> usize {
    let total: f32 = probs.iter().sum();
    if total <= 0.0 {
        return 0;
    }
    let point = (rng.below(1 << 24) as f32 / (1u64 << 24) as f32) * total;
    let mut accumulated = 0.0f32;
    for (index, probability) in probs.iter().enumerate() {
        accumulated += probability;
        if point < accumulated {
            return index;
        }
    }
    probs
        .iter()
        .rposition(|probability| *probability > 0.0)
        .unwrap_or(0)
}

/// 某个动作在掩码 softmax 下的对数概率。
pub fn log_prob(logits: &[f32], mask: &[bool], action: usize) -> f32 {
    let probs = masked_probs(logits, mask);
    probs[action].max(1e-12).ln()
}

/// 广义优势估计；`dones` 为真表示该步之后没有后续状态。
pub fn gae(values: &[f32], rewards: &[f32], dones: &[bool], cfg: &PpoConfig) -> Vec<f32> {
    let count = rewards.len();
    let mut advantages = vec![0.0f32; count];
    let mut carried = 0.0f32;
    for step in (0..count).rev() {
        let nonterminal = if dones.get(step).copied().unwrap_or(true) {
            0.0
        } else {
            1.0
        };
        let next_value = if step + 1 < count {
            values.get(step + 1).copied().unwrap_or(0.0) * nonterminal
        } else {
            0.0
        };
        let delta = rewards[step] + cfg.gamma * next_value - values[step];
        carried = delta + cfg.gamma * cfg.lambda * nonterminal * carried;
        advantages[step] = carried;
    }
    advantages
}

/// 用一批决策更新策略；自适应优化器的状态在模型上跨调用保留。
pub fn train_step(
    model: &mut TrainModel,
    batch: &Batch,
    cfg: &PpoConfig,
) -> candle_core::Result<Metrics> {
    validate(batch)?;
    if model.optimizer.is_none() {
        let optimizer = AdamW::new(
            model.var_map.all_vars(),
            ParamsAdamW {
                lr: f64::from(cfg.lr),
                ..ParamsAdamW::default()
            },
        )?;
        model.optimizer = Some(optimizer);
    }
    let device = Device::Cpu;
    let count = batch.len();

    let advantages = gae(&batch.values, &batch.rewards, &batch.dones, cfg);
    let mut returns = Vec::with_capacity(count);
    for (advantage, value) in advantages.iter().zip(&batch.values) {
        returns.push(advantage + value);
    }
    let normalised = normalise(&advantages);
    let advantage_tensor = Tensor::from_vec(normalised.clone(), count, &device)?;
    let return_tensor = Tensor::from_vec(returns, count, &device)?;
    let old_logps = Tensor::from_vec(batch.logps.clone(), count, &device)?;

    let obs: Vec<f32> = batch.obs.iter().flat_map(Observation::to_vec).collect();
    let obs = Tensor::from_vec(obs, (count, OBS_DIM), &device)?;
    let mut candidates = Vec::new();
    let mut offsets = vec![0usize];
    for decisions in &batch.candidates {
        candidates.extend_from_slice(decisions);
        offsets.push(candidates.len());
    }

    let mut metrics = Metrics {
        policy_loss: 0.0,
        value_loss: 0.0,
        entropy: 0.0,
        clip_fraction: 0.0,
    };
    for _ in 0..cfg.epochs.max(1) {
        let (logits, values) = model.forward(&obs, &candidates, &offsets)?;
        let logps = gather_logps(&logits, batch, &offsets, &device)?;
        let entropy = gather_entropy(&logits, batch, &offsets, &device)?;
        let ratio = logps.broadcast_sub(&old_logps)?.exp()?;
        let clipped = ratio.clamp(f64::from(1.0 - cfg.clip), f64::from(1.0 + cfg.clip))?;
        let unclipped_objective = ratio.broadcast_mul(&advantage_tensor)?;
        let clipped_objective = clipped.broadcast_mul(&advantage_tensor)?;
        let objective = unclipped_objective.minimum(&clipped_objective)?;
        let policy_loss = objective.mean_all()?.neg()?;
        let value_loss = values.broadcast_sub(&return_tensor)?.sqr()?.mean_all()?;
        let entropy_mean = entropy.mean_all()?;
        let weighted_value = value_loss.affine(f64::from(VALUE_COEF), 0.0)?;
        let weighted_entropy = entropy_mean.affine(f64::from(cfg.entropy), 0.0)?;
        let loss = policy_loss
            .broadcast_add(&weighted_value)?
            .broadcast_sub(&weighted_entropy)?;
        let optimizer = model.optimizer.as_mut().expect("优化器已创建");
        optimizer.backward_step(&loss)?;

        let ratio_values = ratio.to_vec1::<f32>()?;
        metrics = Metrics {
            policy_loss: policy_loss.to_scalar::<f32>()?,
            value_loss: value_loss.to_scalar::<f32>()?,
            entropy: entropy_mean.to_scalar::<f32>()?,
            clip_fraction: ratio_values
                .iter()
                .filter(|value| (**value - 1.0).abs() > cfg.clip)
                .count() as f32
                / count as f32,
        };
    }
    Ok(metrics)
}

fn gather_logps(
    logits: &Tensor,
    batch: &Batch,
    offsets: &[usize],
    device: &Device,
) -> candle_core::Result<Tensor> {
    let mut logps = Vec::with_capacity(batch.len());
    for decision in 0..batch.len() {
        let start = offsets[decision];
        let length = offsets[decision + 1] - start;
        let log_soft = masked_log_softmax(logits, batch, decision, start, length, device)?;
        logps.push(log_soft.narrow(0, batch.actions[decision], 1)?.squeeze(0)?);
    }
    Tensor::stack(&logps, 0)
}

fn gather_entropy(
    logits: &Tensor,
    batch: &Batch,
    offsets: &[usize],
    device: &Device,
) -> candle_core::Result<Tensor> {
    let mut entropies = Vec::with_capacity(batch.len());
    for decision in 0..batch.len() {
        let start = offsets[decision];
        let length = offsets[decision + 1] - start;
        let log_soft = masked_log_softmax(logits, batch, decision, start, length, device)?;
        let probabilities = log_soft.exp()?;
        let entropy = probabilities.broadcast_mul(&log_soft)?.sum_all()?.neg()?;
        entropies.push(entropy);
    }
    Tensor::stack(&entropies, 0)
}

/// 一段候选的掩码 log-softmax；被屏蔽项为 −1e9，保证概率严格为 0。
fn masked_log_softmax(
    logits: &Tensor,
    batch: &Batch,
    decision: usize,
    start: usize,
    length: usize,
    device: &Device,
) -> candle_core::Result<Tensor> {
    let segment = logits.narrow(0, start, length)?;
    let penalty: Vec<f32> = batch.masks[decision]
        .iter()
        .map(|valid| if *valid { 0.0 } else { -1e9 })
        .collect();
    let penalty = Tensor::from_vec(penalty, length, device)?;
    let masked = segment.broadcast_add(&penalty)?;
    let log_normaliser = masked.exp()?.sum_all()?.log()?;
    masked.broadcast_sub(&log_normaliser)
}

/// 批次内标准化优势；单条批次不标准化（标准化会把唯一优势压成 0）。
fn normalise(values: &[f32]) -> Vec<f32> {
    let count = values.len() as f32;
    if values.len() < 2 {
        return values.to_vec();
    }
    let mean = values.iter().sum::<f32>() / count;
    let variance = values
        .iter()
        .map(|value| (value - mean) * (value - mean))
        .sum::<f32>()
        / count;
    let deviation = variance.sqrt();
    if deviation < 1e-6 {
        values.iter().map(|value| value - mean).collect()
    } else {
        values
            .iter()
            .map(|value| (value - mean) / deviation)
            .collect()
    }
}

fn validate(batch: &Batch) -> candle_core::Result<()> {
    let count = batch.len();
    for (name, length) in [
        ("actions", batch.actions.len()),
        ("masks", batch.masks.len()),
        ("logps", batch.logps.len()),
        ("values", batch.values.len()),
        ("rewards", batch.rewards.len()),
        ("dones", batch.dones.len()),
        ("candidates", batch.candidates.len()),
    ] {
        if length != count {
            return Err(candle_core::Error::Msg(format!(
                "批次字段 {name} 长度 {length} 与 obs {count} 不符"
            )));
        }
    }
    for decision in 0..count {
        let candidates = batch.candidates[decision].len();
        if batch.masks[decision].len() != candidates {
            return Err(candle_core::Error::Msg(format!(
                "决策 {decision} 掩码长度与候选数不符"
            )));
        }
        if !batch.masks[decision].iter().any(|valid| *valid) {
            return Err(candle_core::Error::Msg(format!(
                "决策 {decision} 没有可选择项"
            )));
        }
        if batch.actions[decision] >= candidates || !batch.masks[decision][batch.actions[decision]]
        {
            return Err(candle_core::Error::Msg(format!(
                "决策 {decision} 的动作 {} 不在可选范围内",
                batch.actions[decision]
            )));
        }
    }
    Ok(())
}
