//! 训练侧：三个环境、PPO、联盟与 CLI。
//!
//! `candle` 依赖放在可选 feature `train` 后，`cargo test --workspace` 默认不编译它；
//! 训练二进制声明 `required-features = ["train"]`。

pub mod env;
pub mod random_policy;

/// 训练功能所在的 feature 名。
pub const TRAIN_FEATURE: &str = "train";
