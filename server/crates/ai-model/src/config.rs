//! 模型结构配置。

use serde::{Deserialize, Serialize};

use crate::ModelError;
use crate::weights::Weights;

/// 前向网络的结构参数。
///
/// 主干预候选头都是「线性 → ReLU」堆叠；`layers` 是主干的隐藏层数。
/// 价值头固定一层隐藏层，候选头固定一层隐藏层。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelConfig {
    pub obs_dim: usize,
    pub card_emb: usize,
    pub hidden: usize,
    pub layers: usize,
}

impl ModelConfig {
    /// 候选头输入宽度：实体、目标、技能、英雄四路嵌入 + 数值特征 + 主干。
    pub fn candidate_input(&self) -> usize {
        4 * self.card_emb + ai_core::ACTION_FEATURE_DIM + self.hidden
    }

    /// 从权重张量的形状反推结构参数。
    ///
    /// 权重文件只记录嵌入表条目数，不记录网络宽度；评测与线上推理都要按
    /// 权重本身推断结构，避免调用方另配一份可能与权重不符的配置。
    /// 没有嵌入表时（C0 只训练主干与价值头）`card_emb` 回退为 1。
    pub fn from_weights(weights: &Weights) -> Result<Self, ModelError> {
        let obs_dim = ai_core::OBS_DIM;
        let layers = weights
            .tensors
            .keys()
            .filter(|name| name.starts_with("trunk.") && name.ends_with(".weight"))
            .count();
        if layers == 0 {
            return Err(ModelError::MissingTensor("trunk.*.weight".to_string()));
        }
        let first = weights
            .get("trunk.0.weight")
            .ok_or_else(|| ModelError::MissingTensor("trunk.0.weight".to_string()))?;
        if first.is_empty() || first.len() % obs_dim != 0 {
            return Err(ModelError::Shape(format!(
                "trunk.0.weight 长度 {} 不是观测维度 {obs_dim} 的整数倍",
                first.len()
            )));
        }
        let hidden = first.len() / obs_dim;
        let mut card_emb = 1;
        for (name, count) in [
            ("card.emb", weights.card_count),
            ("hero.emb", weights.hero_count),
        ] {
            if let Some(table) = weights.get(name) {
                if count == 0 || table.len() % count != 0 {
                    return Err(ModelError::Shape(format!("{name} 长度与条目数不符")));
                }
                card_emb = table.len() / count;
                break;
            }
        }
        Ok(Self {
            obs_dim,
            card_emb,
            hidden,
            layers,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weights_determine_the_same_config() {
        let mut weights = Weights::default();
        weights.card_count = 4;
        weights.set("trunk.0.weight", vec![0.0; 8 * ai_core::OBS_DIM]);
        weights.set("trunk.0.bias", vec![0.0; 8]);
        weights.set("card.emb", vec![0.0; 4 * 3]);
        let config = ModelConfig::from_weights(&weights).expect("反推");
        assert_eq!(
            config,
            ModelConfig {
                obs_dim: ai_core::OBS_DIM,
                card_emb: 3,
                hidden: 8,
                layers: 1,
            }
        );
    }

    #[test]
    fn weights_without_trunk_are_rejected() {
        let weights = Weights::default();
        assert!(ModelConfig::from_weights(&weights).is_err());
    }
}
