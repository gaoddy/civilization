//! 纯 Rust 模型：结构配置、权重格式与前向计算。
//!
//! 训练侧用 candle 复现同一套张量；推理侧只用本 crate，不引入 ML 运行时。

pub mod config;
pub mod forward;
pub mod weights;

use std::fmt;

pub use config::ModelConfig;
pub use forward::{board_value, forward, forward_hero};
pub use weights::{WEIGHTS_VERSION, Weights};

/// 加载或前向过程中的错误。
#[derive(Debug)]
pub enum ModelError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Version(u32),
    MissingTensor(String),
    Shape(String),
    Index { table: String, index: i32 },
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "读写失败：{error}"),
            Self::Json(error) => write!(f, "清单解析失败：{error}"),
            Self::Version(version) => write!(f, "权重版本 {version} 不受支持"),
            Self::MissingTensor(name) => write!(f, "缺少张量 {name}"),
            Self::Shape(message) => write!(f, "形状不匹配：{message}"),
            Self::Index { table, index } => write!(f, "{table} 索引越界：{index}"),
        }
    }
}

impl std::error::Error for ModelError {}

impl From<std::io::Error> for ModelError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for ModelError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn weights_version_is_one() {
        assert_eq!(super::WEIGHTS_VERSION, 1);
    }
}
