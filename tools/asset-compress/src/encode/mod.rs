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
    /// 尺寸为 0，或像素长度与 `w * h * 4` 不符。
    BadSize { w: u32, h: u32 },
    Astc(String),
    Bc7(String),
}

impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncodeError::BadSize { w, h } => write!(f, "尺寸 {w}x{h} 的像素数据不合法"),
            EncodeError::Astc(e) => write!(f, "ASTC 编码失败：{e}"),
            EncodeError::Bc7(e) => write!(f, "BC7 编码失败：{e}"),
        }
    }
}

impl std::error::Error for EncodeError {}

/// 把非 4 倍数的一边按边缘复制补齐到块边界；已对齐时返回 `None`。
///
/// 补齐后的像素不进入纹理（`.ctex` 头保留原尺寸），只是让 BC7 能按 4×4 分块。
/// Godot 对这类素材是双线性放大到块边界再编码，会把边缘拉糊；这里保留锐利边缘，
/// 与 astcenc 对非 4 倍数尺寸的内部补齐方式一致。
pub fn pad_to_blocks(rgba: &[u8], w: u32, h: u32) -> Option<(Vec<u8>, u32, u32)> {
    if w % 4 == 0 && h % 4 == 0 {
        return None;
    }
    let (pw, ph) = (w.div_ceil(4) * 4, h.div_ceil(4) * 4);
    let mut out = vec![0u8; (pw * ph * 4) as usize];
    for y in 0..ph {
        let sy = y.min(h - 1);
        for x in 0..pw {
            let sx = x.min(w - 1);
            let s = ((sy * w + sx) * 4) as usize;
            let d = ((y * pw + x) * 4) as usize;
            out[d..d + 4].copy_from_slice(&rgba[s..s + 4]);
        }
    }
    Some((out, pw, ph))
}

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
