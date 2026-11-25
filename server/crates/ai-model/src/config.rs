//! 模型结构配置。

use serde::{Deserialize, Serialize};

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
}
