//! standard 档编排：BC7 走 GPU，ASTC 仍走 CPU astcenc（medium，与 Godot 同档）。

use super::{CpuEncoder, Encoder, EncodeError, GpuEncoder, GpuError};
use crate::Format;

pub struct StandardEncoder {
    gpu: GpuEncoder,
    astc: CpuEncoder,
}

impl StandardEncoder {
    pub fn new(jobs: usize) -> Result<Self, GpuError> {
        Ok(Self {
            gpu: GpuEncoder::new()?,
            astc: CpuEncoder::new(jobs),
        })
    }

    /// 复用已经创建好的 GPU 设备（CLI 先探测 adapter 是否存在）。
    pub fn from_gpu(gpu: GpuEncoder, jobs: usize) -> Self {
        Self {
            gpu,
            astc: CpuEncoder::new(jobs),
        }
    }

    pub fn gpu(&self) -> &GpuEncoder {
        &self.gpu
    }
}

impl Encoder for StandardEncoder {
    fn encode(&self, rgba: &[u8], w: u32, h: u32, fmt: Format) -> Result<Vec<u8>, EncodeError> {
        match fmt {
            Format::Bc7 => self.gpu.encode(rgba, w, h, fmt),
            Format::Astc4x4 => self.astc.encode(rgba, w, h, fmt),
        }
    }

    fn encode_batch(
        &self,
        items: &[(&[u8], u32, u32)],
        fmt: Format,
    ) -> Vec<Result<Vec<u8>, EncodeError>> {
        match fmt {
            Format::Bc7 => Encoder::encode_batch(&self.gpu, items, fmt),
            Format::Astc4x4 => Encoder::encode_batch(&self.astc, items, fmt),
        }
    }
}
