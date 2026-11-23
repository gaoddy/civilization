//! 质量门禁：把编码产物解码回像素，统计 PSNR。
//!
//! 门限口径见设计文档第 6 节：`standard` 档 BC7 ≥ 43.08 dB（Godot −0.3），
//! ASTC ≥ 44.24 dB。min 只对 BC7 设限（Godot 单张最低再减 0.5）。

use crate::encode::{EncodeError, Encoder};
use crate::format::Format;

#[derive(Clone, Copy, Debug)]
pub struct Gate {
    pub mean: f64,
    pub min: f64,
}

pub const BC7_GATE: Gate = Gate {
    mean: 43.08,
    min: 38.91,
};
pub const ASTC_GATE: Gate = Gate {
    mean: 44.24,
    min: f64::NEG_INFINITY,
};

pub fn gate_for(fmt: Format) -> Gate {
    match fmt {
        Format::Bc7 => BC7_GATE,
        Format::Astc4x4 => ASTC_GATE,
    }
}

#[derive(Clone, Debug)]
pub struct Report {
    pub format: Format,
    pub count: usize,
    pub mean: f64,
    pub min: f64,
}

#[derive(Debug)]
pub enum GateFailure {
    Encode(EncodeError),
    Decode(String),
    Below { report: Report, gate: Gate },
}

impl std::fmt::Display for GateFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GateFailure::Encode(e) => write!(f, "{e}"),
            GateFailure::Decode(e) => write!(f, "解码产物失败：{e}"),
            GateFailure::Below { report, gate } => write!(
                f,
                "{:?} 质量未过门禁：mean {:.2}（要求 ≥{:.2}），min {:.2}（要求 ≥{:.2}）",
                report.format, report.mean, gate.mean, report.min, gate.min
            ),
        }
    }
}

impl std::error::Error for GateFailure {}

/// 只统计、不判门限（用于打印报告）。
pub fn measure(
    enc: &dyn Encoder,
    fmt: Format,
    images: &[(&[u8], u32, u32)],
) -> Result<Report, GateFailure> {
    let out = enc.encode_batch(images, fmt);
    let mut sum = 0.0;
    let mut min = f64::INFINITY;
    let mut count = 0;
    for (&(rgba, w, h), blocks) in images.iter().zip(out) {
        let blocks = blocks.map_err(GateFailure::Encode)?;
        let decoded = decode(fmt, &blocks, w, h)?;
        let p = psnr(rgba, &decoded);
        sum += p;
        min = min.min(p);
        count += 1;
    }
    Ok(Report {
        format: fmt,
        count,
        mean: if count == 0 { 0.0 } else { sum / count as f64 },
        min,
    })
}

pub fn check_quality(
    enc: &dyn Encoder,
    fmt: Format,
    images: &[(&[u8], u32, u32)],
) -> Result<Report, GateFailure> {
    let report = measure(enc, fmt, images)?;
    let gate = gate_for(fmt);
    if report.mean < gate.mean || report.min < gate.min {
        return Err(GateFailure::Below { report, gate });
    }
    Ok(report)
}

pub fn psnr(a: &[u8], b: &[u8]) -> f64 {
    let mse = a
        .iter()
        .zip(b)
        .map(|(x, y)| {
            let d = *x as f64 - *y as f64;
            d * d
        })
        .sum::<f64>()
        / a.len() as f64;
    if mse == 0.0 {
        99.0
    } else {
        10.0 * (255.0 * 255.0 / mse).log10()
    }
}

fn decode(fmt: Format, blocks: &[u8], w: u32, h: u32) -> Result<Vec<u8>, GateFailure> {
    match fmt {
        Format::Bc7 => bcn_decode::decode_rgba8_to_vec(
            bcn_decode::Format::Bc7,
            blocks,
            w as usize,
            h as usize,
        )
        .map_err(|e| GateFailure::Decode(format!("{e:?}"))),
        Format::Astc4x4 => {
            let cfg = astcenc_rs::ConfigBuilder::new()
                .with_block_size(astcenc_rs::Extents::new(4, 4))
                .with_flags(astcenc_rs::Flags::DECOMPRESS_ONLY)
                .build()
                .map_err(|e| GateFailure::Decode(format!("{e:?}")))?;
            let mut ctx = astcenc_rs::Context::with_threads(1, cfg)
                .map_err(|e| GateFailure::Decode(format!("{e:?}")))?;
            let img = ctx
                .decompress::<u8>(blocks, astcenc_rs::Extents::new(w, h), astcenc_rs::Swizzle::rgba())
                .map_err(|e| GateFailure::Decode(format!("{e:?}")))?;
            Ok(img.data.into_iter().next().unwrap_or_default())
        }
    }
}
