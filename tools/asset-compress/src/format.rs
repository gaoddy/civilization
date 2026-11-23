//! 压缩块格式。GST2 的 format 字段取值：BC7=22、ASTC 4x4=35。

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Bc7,
    Astc4x4,
}

impl Format {
    pub fn format_id(self) -> u32 {
        match self {
            Format::Bc7 => 22,
            Format::Astc4x4 => 35,
        }
    }

    /// 每 4×4 块占用的字节数（两种格式都是 16）。
    pub fn block_bytes(self) -> usize {
        16
    }
}
