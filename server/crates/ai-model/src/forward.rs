//! 纯 Rust 前向：主干 + per-candidate 头 + 价值头。
//!
//! 与训练侧（candle）保持同一套张量命名与计算顺序，推理不依赖任何 ML 运行时。

use crate::ModelError;
use crate::config::ModelConfig;
use crate::weights::Weights;
use ai_core::{ActionFeatures, Observation};

/// 前向一次：返回每个候选动作的 logit 与状态价值。
///
/// 调用方只把合法候选传进来，因此不需要额外掩码。
pub fn forward(
    weights: &Weights,
    cfg: &ModelConfig,
    obs: &Observation,
    features: &[ActionFeatures],
) -> Result<(Vec<f32>, f32), ModelError> {
    let obs_vec = obs.to_vec();
    if obs_vec.len() != cfg.obs_dim {
        return Err(ModelError::Shape(format!(
            "观测长度 {} 与配置 {} 不符",
            obs_vec.len(),
            cfg.obs_dim
        )));
    }
    let trunk = trunk(weights, cfg, &obs_vec)?;
    let value = value(weights, cfg, &trunk)?;

    // 选英雄整批走独立头（设计 D2），与训练侧的分派规则一致。
    if ai_core::features::is_hero_choice_batch(features) {
        let heroes: Vec<i32> = features
            .iter()
            .map(|feature| hero_index(obs, feature))
            .collect();
        return Ok((hero_head_scores(weights, cfg, &trunk, &heroes)?, value));
    }

    let card_table = tensor(weights, "card.emb")?;
    let hero_table = tensor(weights, "hero.emb")?;
    let skill_table = tensor(weights, "skill.emb")?;
    let emb = cfg.card_emb;
    let candidate_weight = tensor(weights, "cand.0.weight")?;
    let candidate_bias = tensor(weights, "cand.0.bias")?;
    let output_weight = tensor(weights, "cand.1.weight")?;
    let output_bias = tensor(weights, "cand.1.bias")?;
    let inner = cfg.candidate_input();
    expect_len(candidate_weight, cfg.hidden * inner, "cand.0.weight")?;
    expect_len(candidate_bias, cfg.hidden, "cand.0.bias")?;
    expect_len(output_weight, cfg.hidden, "cand.1.weight")?;
    expect_len(output_bias, 1, "cand.1.bias")?;

    let mut input = Vec::with_capacity(inner);
    let mut logits = Vec::with_capacity(features.len());
    for feature in features {
        input.clear();
        input.extend(embedding(card_table, emb, feature.card, "card.emb")?);
        input.extend(embedding(
            hero_table,
            emb,
            hero_index(obs, feature),
            "hero.emb",
        )?);
        input.extend(embedding(skill_table, emb, feature.skill, "skill.emb")?);
        input.extend(embedding(card_table, emb, feature.target_card, "card.emb")?);
        input.extend_from_slice(&feature.values);
        input.extend_from_slice(&trunk);
        debug_assert_eq!(input.len(), inner);

        let hidden = relu_layer(candidate_weight, candidate_bias, &input, cfg.hidden, inner);
        let logit = dot(&hidden, output_weight) + output_bias[0];
        logits.push(logit);
    }
    Ok((logits, value))
}

/// 只要状态价值：C0 的单棋盘强度 `s(board, hero)` 与 critic 初始化都走这里。
///
/// 只用主干预价值头，不读取任何嵌入表。
pub fn board_value(
    weights: &Weights,
    cfg: &ModelConfig,
    obs: &Observation,
) -> Result<f32, ModelError> {
    let obs_vec = obs.to_vec();
    if obs_vec.len() != cfg.obs_dim {
        return Err(ModelError::Shape(format!(
            "观测长度 {} 与配置 {} 不符",
            obs_vec.len(),
            cfg.obs_dim
        )));
    }
    let trunk = trunk(weights, cfg, &obs_vec)?;
    value(weights, cfg, &trunk)
}

/// 选英雄专用头：对每个候选英雄打分。
///
/// 与通用候选头分开参数（设计 D2=A），输入为英雄嵌入与主干的拼接。
pub fn forward_hero(
    weights: &Weights,
    cfg: &ModelConfig,
    obs: &Observation,
    heroes: &[i32],
) -> Result<Vec<f32>, ModelError> {
    let trunk = trunk(weights, cfg, &obs.to_vec())?;
    hero_head_scores(weights, cfg, &trunk, heroes)
}

/// 英雄头本体：输入为「英雄嵌入 + 主干」，逐候选打分。
fn hero_head_scores(
    weights: &Weights,
    cfg: &ModelConfig,
    trunk: &[f32],
    heroes: &[i32],
) -> Result<Vec<f32>, ModelError> {
    let table = tensor(weights, "hero.emb")?;
    let inner = cfg.card_emb + cfg.hidden;
    let weight = tensor(weights, "hero_head.0.weight")?;
    let bias = tensor(weights, "hero_head.0.bias")?;
    let out_weight = tensor(weights, "hero_head.1.weight")?;
    let out_bias = tensor(weights, "hero_head.1.bias")?;
    expect_len(weight, cfg.hidden * inner, "hero_head.0.weight")?;
    expect_len(bias, cfg.hidden, "hero_head.0.bias")?;
    expect_len(out_weight, cfg.hidden, "hero_head.1.weight")?;
    expect_len(out_bias, 1, "hero_head.1.bias")?;

    let mut input = Vec::with_capacity(inner);
    let mut logits = Vec::with_capacity(heroes.len());
    for hero in heroes {
        input.clear();
        input.extend(embedding(table, cfg.card_emb, *hero, "hero.emb")?);
        input.extend_from_slice(trunk);
        let hidden = relu_layer(weight, bias, &input, cfg.hidden, inner);
        logits.push(dot(&hidden, out_weight) + out_bias[0]);
    }
    Ok(logits)
}

/// 主干：`layers` 个「线性 + ReLU」隐藏层。
fn trunk(weights: &Weights, cfg: &ModelConfig, obs: &[f32]) -> Result<Vec<f32>, ModelError> {
    let mut current = obs.to_vec();
    let mut width = cfg.obs_dim;
    for layer in 0..cfg.layers {
        let weight = tensor(weights, &format!("trunk.{layer}.weight"))?;
        let bias = tensor(weights, &format!("trunk.{layer}.bias"))?;
        expect_len(weight, cfg.hidden * width, "trunk.weight")?;
        expect_len(bias, cfg.hidden, "trunk.bias")?;
        current = relu_layer(weight, bias, &current, cfg.hidden, width);
        width = cfg.hidden;
    }
    if cfg.layers == 0 {
        return Err(ModelError::Shape("主干至少一层".to_string()));
    }
    Ok(current)
}

/// 价值头：一层隐藏层输出标量。
fn value(weights: &Weights, cfg: &ModelConfig, trunk: &[f32]) -> Result<f32, ModelError> {
    let weight = tensor(weights, "value.0.weight")?;
    let bias = tensor(weights, "value.0.bias")?;
    expect_len(weight, cfg.hidden * cfg.hidden, "value.0.weight")?;
    expect_len(bias, cfg.hidden, "value.0.bias")?;
    let hidden = relu_layer(weight, bias, trunk, cfg.hidden, cfg.hidden);
    let out_weight = tensor(weights, "value.1.weight")?;
    let out_bias = tensor(weights, "value.1.bias")?;
    expect_len(out_weight, cfg.hidden, "value.1.weight")?;
    expect_len(out_bias, 1, "value.1.bias")?;
    Ok(dot(&hidden, out_weight) + out_bias[0])
}

/// `relu(W x + b)`；`weight` 为行主序 `rows × cols`。
fn relu_layer(weight: &[f32], bias: &[f32], x: &[f32], rows: usize, cols: usize) -> Vec<f32> {
    let mut out = Vec::with_capacity(rows);
    for (row, bias) in bias.iter().enumerate().take(rows) {
        let start = row * cols;
        let sum: f32 = weight[start..start + cols]
            .iter()
            .zip(x)
            .map(|(weight, value)| weight * value)
            .sum();
        out.push((sum + bias).max(0.0));
    }
    out
}

fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}

fn tensor<'a>(weights: &'a Weights, name: &str) -> Result<&'a [f32], ModelError> {
    weights
        .get(name)
        .ok_or_else(|| ModelError::MissingTensor(name.to_string()))
}

fn expect_len(data: &[f32], expected: usize, name: &str) -> Result<(), ModelError> {
    if data.len() != expected {
        return Err(ModelError::Shape(format!(
            "{name} 长度 {} 与期望 {expected} 不符",
            data.len()
        )));
    }
    Ok(())
}

fn embedding(table: &[f32], dim: usize, index: i32, name: &str) -> Result<Vec<f32>, ModelError> {
    if index < 0 {
        return Ok(vec![0.0; dim]);
    }
    let start = index as usize * dim;
    if start + dim > table.len() {
        return Err(ModelError::Index {
            table: name.to_string(),
            index,
        });
    }
    Ok(table[start..start + dim].to_vec())
}

fn hero_index(obs: &Observation, feature: &ActionFeatures) -> i32 {
    if feature.hero >= 0 {
        feature.hero
    } else {
        obs.hero
    }
}
