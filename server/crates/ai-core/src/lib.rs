//! 观测与动作编码：训练与推理共用的唯一一份实现。
//!
//! 本 crate 只做纯数据转换，不读时钟、环境变量或文件，也不依赖任何 ML 运行时。

pub mod action;
pub mod catalog;
pub mod features;
pub mod history;
pub mod observe;

pub use action::{Action, MAX_REARRANGE_PER_TURN, rearrange_within_budget, rearrange_within_limit};
pub use catalog::{CardIndex, catalog_hash};
pub use features::{ACTION_FEATURE_DIM, ActionFeatures};
pub use history::{BoardSnapshot, History};
pub use observe::{
    ABILITY_FEATURES, Ability, BOARD_KEYWORD_FEATURES, BOARD_SLOTS, DISCOVER_FEATURES,
    DISCOVER_SLOTS, Encoder, HAND_SLOTS, HERO_POWER_FEATURES, HERO_POWER_SLOTS, HeroPower,
    KEYWORD_BITS, KEYWORD_LAYERS, MAX_TARGET_CANDIDATES, MAX_USES_PER_TURN, OBS_DIM,
    OPPONENT_SEAT_FEATURES, Observation, SEATS, SHOP_AFFORD_FEATURES, SHOP_SLOTS, Slot,
    TARGET_FEATURES, encode_keywords, playstyle_mask,
};

/// 观测/动作编码的 schema 版本；字段顺序或语义变更时递增。
///
/// 2：补齐商店可购买性、关键词与层数、英雄/随从技能次数、待选候选与本回合对手座位。
/// 3：修正 `History` 的语义——按观察者分份，槽位是对手座位；维度不变，旧检查点需重训。
pub const SCHEMA_VERSION: u32 = 3;

#[cfg(test)]
mod tests {
    #[test]
    fn schema_version_matches_observation_layout() {
        assert_eq!(super::SCHEMA_VERSION, 3);
    }
}
