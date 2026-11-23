//! GST2（Godot 4.7 的 `.ctex` 容器）读写。
//!
//! 布局（设计文档 10.1，已用现网产物逐字节验证）：
//!
//! ```text
//! 0   4   "GST2"
//! 4   4   version = 1
//! 8   4   width (u32)
//! 12  4   height (u32)
//! 16  4   df = 0x0D000000
//! 20  4   mipmap_limit = 0xFFFFFFFF
//! 24  12  reserved = 0
//! 36  4   data_format = 0
//! 40  2   width (u16)
//! 42  2   height (u16)
//! 44  4   mipmaps = 0
//! 48  4   format (22 = BC7, 35 = ASTC 4x4)
//! 52  ... payload = ceil(w/4) * ceil(h/4) * 16
//! ```

use crate::Format;

pub const HEADER_LEN: usize = 52;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CtexHeader {
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub payload_len: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CtexError {
    /// 宽或高超出 u16 字段能表达的范围（Godot 的 `.ctex` 头里宽高是 u16）。
    Oversized { width: u32, height: u32 },
    /// 负载长度与 `ceil(w/4) * ceil(h/4) * 16` 不一致。
    PayloadLen { got: usize, want: usize },
}

impl std::fmt::Display for CtexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CtexError::Oversized { width, height } => {
                write!(f, "纹理尺寸 {width}x{height} 超出 .ctex u16 上限")
            }
            CtexError::PayloadLen { got, want } => {
                write!(f, ".ctex 负载长度 {got}，应为 {want}")
            }
        }
    }
}

impl std::error::Error for CtexError {}

pub fn expected_payload_len(w: u32, h: u32) -> usize {
    (w as usize).div_ceil(4) * (h as usize).div_ceil(4) * 16
}

pub fn encode_ctex(fmt: Format, w: u32, h: u32, blocks: &[u8]) -> Result<Vec<u8>, CtexError> {
    if w == 0 || h == 0 || w > u16::MAX as u32 || h > u16::MAX as u32 {
        return Err(CtexError::Oversized { width: w, height: h });
    }
    let want = expected_payload_len(w, h);
    if blocks.len() != want {
        return Err(CtexError::PayloadLen { got: blocks.len(), want });
    }

    let mut out = Vec::with_capacity(HEADER_LEN + want);
    out.extend_from_slice(b"GST2");
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&w.to_le_bytes());
    out.extend_from_slice(&h.to_le_bytes());
    out.extend_from_slice(&0x0D00_0000u32.to_le_bytes());
    out.extend_from_slice(&u32::MAX.to_le_bytes());
    out.extend_from_slice(&[0u8; 12]);
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(w as u16).to_le_bytes());
    out.extend_from_slice(&(h as u16).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&fmt.format_id().to_le_bytes());
    out.extend_from_slice(blocks);
    Ok(out)
}

pub fn parse_header(bytes: &[u8]) -> Option<CtexHeader> {
    if bytes.len() < HEADER_LEN || &bytes[0..4] != b"GST2" {
        return None;
    }
    let rd32 = |o: usize| u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
    Some(CtexHeader {
        width: rd32(8),
        height: rd32(12),
        format: rd32(48),
        payload_len: bytes.len() - HEADER_LEN,
    })
}
