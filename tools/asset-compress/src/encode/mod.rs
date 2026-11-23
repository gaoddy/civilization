//! 编码器抽象。
//!
//! 输入统一为 RGBA8 原始像素；输出是紧凑排列的 4×4 块字节（BC7 16B/块，
//! ASTC 4x4 16B/块），由 `godot::ctex` 包装成 GST2 容器。

pub mod cpu;
pub mod gpu;
pub mod hybrid;

pub use cpu::CpuEncoder;
pub use gpu::{GpuEncoder, GpuError};
pub use hybrid::StandardEncoder;

use crate::Format;

#[derive(Debug)]
pub enum EncodeError {
    /// 尺寸为 0、像素长度不匹配，或无法按 4×4 分块。
    BadSize { w: u32, h: u32 },
    Astc(String),
    Bc7(String),
}

impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncodeError::BadSize { w, h } => write!(f, "尺寸 {w}x{h} 无法按 4×4 分块"),
            EncodeError::Astc(e) => write!(f, "ASTC 编码失败：{e}"),
            EncodeError::Bc7(e) => write!(f, "BC7 编码失败：{e}"),
        }
    }
}

impl std::error::Error for EncodeError {}

pub trait Encoder: Sync {
    fn encode(&self, rgba: &[u8], w: u32, h: u32, fmt: Format) -> Result<Vec<u8>, EncodeError>;

    /// 批量编码；顺序与输入一致。默认逐张编码，GPU/CPU 执行层各自覆盖。
    fn encode_batch(
        &self,
        items: &[(&[u8], u32, u32)],
        fmt: Format,
    ) -> Vec<Result<Vec<u8>, EncodeError>> {
        items.iter().map(|&(rgba, w, h)| self.encode(rgba, w, h, fmt)).collect()
    }
}
