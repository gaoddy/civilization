//! C0 网络：与 `ai_model` 同构的主干 + 价值头，只在训练侧用 candle。

use ai_core::OBS_DIM;
use ai_model::{ModelConfig, Weights};
use candle_core::{DType, Device, Tensor};
use candle_nn::{AdamW, Linear, Module, Optimizer, ParamsAdamW, VarBuilder, VarMap};

use crate::random_policy::RandomPolicy;

/// C0 训练配置。
#[derive(Debug, Clone)]
pub struct C0Config {
    pub steps: usize,
    pub batch: usize,
    pub lr: f64,
    pub hidden: usize,
    pub layers: usize,
    pub card_emb: usize,
    pub seed: u64,
    /// 留出比例：按分层顺序每 `1 / held_out` 对取一对。
    pub held_out: f64,
}

impl Default for C0Config {
    fn default() -> Self {
        Self {
            steps: 20_000,
            batch: 64,
            lr: 0.001,
            hidden: 64,
            layers: 2,
            card_emb: 16,
            seed: 0,
            held_out: 0.2,
        }
    }
}

impl C0Config {
    pub fn model_config(&self) -> ModelConfig {
        ModelConfig {
            obs_dim: OBS_DIM,
            card_emb: self.card_emb,
            hidden: self.hidden,
            layers: self.layers,
        }
    }
}

pub use crate::data_c0::board_observation;

/// 与 `ai_model` 同构的 candle 网络。
pub struct C0Net {
    trunk: Vec<Linear>,
    value_hidden: Linear,
    value_out: Linear,
}

impl C0Net {
    pub fn build(vb: VarBuilder<'_>, cfg: &C0Config) -> candle_core::Result<Self> {
        let mut trunk = Vec::with_capacity(cfg.layers);
        let mut width = OBS_DIM;
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
        Ok(Self {
            trunk,
            value_hidden,
            value_out,
        })
    }

    /// 批量价值：输入 `[batch, OBS_DIM]`，输出 `[batch]`。
    pub fn forward(&self, x: &Tensor) -> candle_core::Result<Tensor> {
        let mut current = x.clone();
        for layer in &self.trunk {
            current = layer.forward(&current)?.relu()?;
        }
        let hidden = self.value_hidden.forward(&current)?.relu()?;
        self.value_out.forward(&hidden)?.squeeze(1)
    }
}

/// Bradley–Terry 训练：`P(a 胜 b) = sigmoid(s(a) − s(b))`。
pub fn fit(
    obs_a: &[Vec<f32>],
    obs_b: &[Vec<f32>],
    a_wins: &[bool],
    cfg: &C0Config,
) -> Result<Weights, candle_core::Error> {
    let device = Device::Cpu;
    let var_map = VarMap::new();
    let vb = VarBuilder::from_varmap(&var_map, DType::F32, &device);
    let net = C0Net::build(vb, cfg)?;
    seed_weights(&var_map, cfg.seed)?;
    let mut optimizer = AdamW::new(
        var_map.all_vars(),
        ParamsAdamW {
            lr: cfg.lr,
            ..ParamsAdamW::default()
        },
    )?;

    let mut rng = RandomPolicy::new(cfg.seed);
    let batch = cfg.batch.min(a_wins.len()).max(1);
    for _ in 0..cfg.steps {
        let mut rows_a = Vec::with_capacity(batch * OBS_DIM);
        let mut rows_b = Vec::with_capacity(batch * OBS_DIM);
        let mut targets: Vec<f32> = Vec::with_capacity(batch);
        for _ in 0..batch {
            let index = rng.below(a_wins.len() as u64) as usize;
            rows_a.extend_from_slice(&obs_a[index]);
            rows_b.extend_from_slice(&obs_b[index]);
            targets.push(if a_wins[index] { 1.0 } else { 0.0 });
        }
        let x_a = Tensor::from_vec(rows_a, (batch, OBS_DIM), &device)?;
        let x_b = Tensor::from_vec(rows_b, (batch, OBS_DIM), &device)?;
        let target = Tensor::from_vec(targets, batch, &device)?;
        let diff = (net.forward(&x_a)? - net.forward(&x_b)?)?;
        let loss = bce_with_logits(&diff, &target)?;
        optimizer.backward_step(&loss)?;
    }
    extract_weights(&var_map)
}

/// 用种子初始化全部权重。
///
/// candle 的默认初始化取全局随机源，同一份配置两次训练会得到不同权重；
/// 这里按名字排序后逐个用 `RandomPolicy` 生成 `±1/√fan_in` 的均匀分布。
fn seed_weights(var_map: &VarMap, seed: u64) -> candle_core::Result<()> {
    let device = Device::Cpu;
    let mut rng = RandomPolicy::new(seed ^ 0x5DEE_D1CE_2026);
    let mut data = var_map.data().lock().expect("VarMap 锁");
    let mut names: Vec<String> = data.keys().cloned().collect();
    names.sort();
    for name in names {
        let var = data.get_mut(&name).expect("变量存在");
        let dims = var.as_tensor().dims().to_vec();
        let fan_in = dims.last().copied().unwrap_or(1).max(1);
        let bound = 1.0 / (fan_in as f32).sqrt();
        let size = var.as_tensor().elem_count();
        let values: Vec<f32> = (0..size)
            .map(|_| {
                let unit = rng.below(1 << 24) as f32 / (1u64 << 24) as f32;
                (unit * 2.0 - 1.0) * bound
            })
            .collect();
        var.set(&Tensor::from_vec(values, dims, &device)?)?;
    }
    Ok(())
}

/// 数值稳定的带 logit 二元交叉熵。
///
/// `-（y log σ(z) + (1-y) log(1-σ(z))）` 直接按 `sigmoid` 再取对数，在 |z| 很大时
/// 会出现 `log(0)` 并产生 NaN；这里用 `max(z,0) - z·y + log(1+exp(-|z|))` 等价形式。
pub fn bce_with_logits(logits: &Tensor, target: &Tensor) -> candle_core::Result<Tensor> {
    let zero = logits.zeros_like()?;
    let positive = logits.maximum(&zero)?;
    let weighted = (logits * target)?;
    let softplus_negative = logits.abs()?.neg()?.exp()?.affine(1.0, 1.0)?.log()?;
    ((positive - weighted)? + softplus_negative)?.mean_all()
}

/// 把 candle 变量转成推理侧权重。
pub fn extract_weights(var_map: &VarMap) -> Result<Weights, candle_core::Error> {
    let mut weights = Weights::default();
    let data = var_map.data().lock().expect("VarMap 锁");
    for (name, var) in data.iter() {
        let values: Vec<f32> = var.as_tensor().flatten_all()?.to_vec1()?;
        weights.set(name, values);
    }
    Ok(weights)
}
