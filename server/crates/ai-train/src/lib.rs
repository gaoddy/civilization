//! 训练侧：三个环境、PPO、联盟与 CLI。
//!
//! `candle` 依赖放在可选 feature `train` 后，`cargo test --workspace` 默认不编译它；
//! 二进制始终可编译，只有训练子命令需要 `--features train`。

pub mod data_c0;
pub mod env;
pub mod random_policy;

/// 训练功能所在的 feature 名。
pub const TRAIN_FEATURE: &str = "train";
