//! CPU exact 编码器：astcenc `PRESET_MEDIUM`（与 Godot 同档）+ ISPC BC7。
//!
//! astcenc 的 `Context` 持有裸指针且 `Config` 不可复制，所以这里保存
//! `ConfigBuilder`（可克隆），每次编码新建上下文。上下文分配开销远小于编码本身。

use astcenc_rs::{ConfigBuilder, Context, Extents, Image, Swizzle, PRESET_MEDIUM};
use intel_tex_2::{RgbaSurface, bc7};

use super::{EncodeError, Encoder};
use crate::Format;

pub struct CpuEncoder {
    jobs: usize,
    astc: ConfigBuilder,
}

impl CpuEncoder {
    pub fn new(jobs: usize) -> Self {
        let astc = ConfigBuilder::new()
            .with_block_size(Extents::new(4, 4))
            .with_preset(PRESET_MEDIUM);
        Self {
            jobs: jobs.max(1),
            astc,
        }
    }
}

impl Encoder for CpuEncoder {
    fn encode(&self, rgba: &[u8], w: u32, h: u32, fmt: Format) -> Result<Vec<u8>, EncodeError> {
        if w == 0 || h == 0 || rgba.len() != w as usize * h as usize * 4 {
            return Err(EncodeError::BadSize { w, h });
        }
        match fmt {
            Format::Astc4x4 => {
                let cfg = self
                    .astc
                    .clone()
                    .build()
                    .map_err(|e| EncodeError::Astc(format!("{e:?}")))?;
                let mut ctx = Context::with_threads(self.jobs, cfg)
                    .map_err(|e| EncodeError::Astc(format!("{e:?}")))?;
                let img = Image {
                    extents: Extents::new(w, h),
                    data: vec![rgba.to_vec()],
                };
                ctx.compress::<u8, _, _>(&img, Swizzle::rgba())
                    .map_err(|e| EncodeError::Astc(format!("{e:?}")))
            }
            Format::Bc7 => {
                if w % 4 != 0 || h % 4 != 0 {
                    return Err(EncodeError::BadSize { w, h });
                }
                let surf = RgbaSurface {
                    width: w,
                    height: h,
                    stride: w * 4,
                    data: rgba,
                };
                Ok(bc7::compress_blocks(&bc7::alpha_basic_settings(), &surf))
            }
        }
    }
}
