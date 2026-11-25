//! 纯 Rust 神经策略：编码观测 → 对合法动作逐个打分 → 取 argmax。
//!
//! 不依赖 candle：训练产物是扁平 `f32` 权重，推理走 [`ai_model::forward`]。
//! 任何推理失败或候选为空都返回 `None`，由调用方回退启发式策略。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use ai_core::{Action, ActionFeatures, CardIndex, Encoder, History};
use ai_model::{ModelConfig, ModelError, Weights, forward};
use rule_engine::data::Catalog;
use rule_engine::engine::Engine;
use rule_engine::intent::Intent;
use rule_engine::legal::legal_actions;
use rule_engine::state::{GameState, PendingChoice, Phase, Seat};
use rule_engine::view::view;

use crate::Policy;

/// 神经策略的加载与结构错误。
#[derive(Debug)]
pub enum PolicyError {
    Model(ModelError),
    /// 权重里的嵌入表条目数或清单指纹与当前卡表不符。
    Catalog(String),
    /// 训练产物清单读不出来。
    Manifest(String),
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Model(error) => write!(f, "权重失败：{error}"),
            Self::Catalog(message) => write!(f, "卡表不符：{message}"),
            Self::Manifest(message) => write!(f, "清单失败：{message}"),
        }
    }
}

impl std::error::Error for PolicyError {}

impl From<ModelError> for PolicyError {
    fn from(error: ModelError) -> Self {
        Self::Model(error)
    }
}

/// 进程级共享缓存：键为（权重目录，卡表指纹）。
type SharedCache = Mutex<HashMap<(PathBuf, String), Arc<NeuralPolicy>>>;

/// 由训练产物驱动的策略。
pub struct NeuralPolicy {
    weights: Weights,
    model: ModelConfig,
    encoder: Encoder,
    history: History,
}

impl NeuralPolicy {
    /// 用内存中的权重构造；嵌入表条目数必须与卡表一致。
    pub fn from_weights(weights: Weights, catalog: &Catalog) -> Result<Self, PolicyError> {
        let model = ModelConfig::from_weights(&weights)?;
        let index = CardIndex::build(catalog);
        if weights.card_count != index.card_count()
            || weights.hero_count != index.hero_count()
            || weights.skill_count != index.skill_count()
        {
            return Err(PolicyError::Catalog(format!(
                "权重条目 {} / {} / {}，卡表 {} / {} / {}",
                weights.card_count,
                weights.hero_count,
                weights.skill_count,
                index.card_count(),
                index.hero_count(),
                index.skill_count()
            )));
        }
        Ok(Self {
            weights,
            model,
            encoder: Encoder::new(index),
            history: History::new(),
        })
    }

    /// 从目录下的 `weights.json` 加载。
    ///
    /// 同时校验同目录 `manifest.json` 的 `catalog_hash`：权重是旧卡表训出来的
    /// 时候直接拒绝，避免把嵌入查错。清单缺失或没有该字段时跳过校验（旧产物）。
    pub fn load(dir: &Path, catalog: &Catalog) -> Result<Self, PolicyError> {
        verify_catalog_hash(dir, catalog)?;
        let weights = Weights::load(&dir.join("weights.json"))?;
        Self::from_weights(weights, catalog)
    }

    /// 进程级共享加载：同一（目录，卡表指纹）只读一次权重，之后复用同一实例。
    ///
    /// 缓存不失效：换了权重文件需要重启进程才会重读。进程内的所有对局共用
    /// 一份只读策略，避免每个 `GameTask` 各加载一次。
    pub fn load_shared(dir: &Path, catalog: &Catalog) -> Result<Arc<Self>, PolicyError> {
        static CACHE: OnceLock<SharedCache> = OnceLock::new();
        let key = (dir.to_path_buf(), ai_core::catalog_hash(catalog));
        let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        let mut guard = cache.lock().expect("神经策略缓存锁");
        if let Some(policy) = guard.get(&key) {
            return Ok(policy.clone());
        }
        let policy = Arc::new(Self::load(dir, catalog)?);
        guard.insert(key, policy.clone());
        Ok(policy)
    }
}

/// 共享实例也能直接当策略用（`GameTask` 持有 `Box<dyn Policy>`）。
impl Policy for Arc<NeuralPolicy> {
    fn plan(
        &self,
        state: &GameState,
        seat: Seat,
        engine: &Engine,
        blocked: &[Intent],
    ) -> Option<Intent> {
        Policy::plan(self.as_ref(), state, seat, engine, blocked)
    }
}

/// 清单里记录训练时的卡表指纹；与当前卡表不一致即拒绝加载。
fn verify_catalog_hash(dir: &Path, catalog: &Catalog) -> Result<(), PolicyError> {
    let Ok(bytes) = std::fs::read(dir.join("manifest.json")) else {
        return Ok(());
    };
    let manifest: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| PolicyError::Manifest(format!("manifest.json 解析失败：{error}")))?;
    let Some(expected) = manifest
        .get("catalog_hash")
        .and_then(|value| value.as_str())
    else {
        return Ok(());
    };
    let actual = ai_core::catalog_hash(catalog);
    if expected != actual {
        return Err(PolicyError::Catalog(format!(
            "清单 catalog_hash {expected} 与当前卡表 {actual} 不符"
        )));
    }
    Ok(())
}

impl Policy for NeuralPolicy {
    fn plan(
        &self,
        state: &GameState,
        seat: Seat,
        engine: &Engine,
        blocked: &[Intent],
    ) -> Option<Intent> {
        if state.phase != Phase::Recruit {
            return None;
        }
        let seat_view = view(engine, state, seat)?;
        let obs = self.encoder.encode(&seat_view, &self.history);

        let mut intents = Vec::new();
        let mut features = Vec::new();
        for action in legal_actions(engine, state, seat)
            .iter()
            .filter_map(Action::from_intent)
        {
            let Some(intent) = action.to_intent() else {
                continue;
            };
            if blocked.contains(&intent) {
                continue;
            }
            intents.push(intent);
            features.push(ActionFeatures::from_action(
                &action,
                state,
                seat,
                self.encoder.index(),
            ));
        }
        if features.is_empty() {
            // 有待选时引擎拒绝 `EndRecruit`（也拒绝与待选类型不匹配的选择）：这里
            // 猜任何一个意图都会被拒，交给上层按超时放弃。
            let pending_open = state.pending.iter().any(|choice| seat_of(choice) == seat);
            if pending_open {
                return None;
            }
            return (!blocked.contains(&Intent::EndRecruit)).then_some(Intent::EndRecruit);
        }
        let (logits, _) = forward(&self.weights, &self.model, &obs, &features).ok()?;
        intents.get(argmax(&logits)).cloned()
    }
}

/// 待选所属座位。
fn seat_of(choice: &PendingChoice) -> Seat {
    match choice {
        PendingChoice::Discover { seat, .. }
        | PendingChoice::Evolve { seat, .. }
        | PendingChoice::Target { seat, .. } => *seat,
    }
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
