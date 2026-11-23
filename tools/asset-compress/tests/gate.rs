use asset_compress::Format;
use asset_compress::encode::{Encoder, EncodeError, GpuEncoder};
use asset_compress::gate::{self, GateFailure};
use std::path::{Path, PathBuf};

fn assets_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../client/assets")
}

fn first_card() -> (Vec<u8>, u32, u32) {
    let mut files: Vec<PathBuf> = std::fs::read_dir(assets_dir().join("cards/faces"))
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "png"))
        .collect();
    files.sort();
    let img = image::open(&files[0]).unwrap().to_rgba8();
    let (w, h) = (img.width(), img.height());
    (img.into_raw(), w, h)
}

/// 全零块的编码器：解码出来必然是黑图，必须被门禁拦住。
struct BlackEncoder;

impl Encoder for BlackEncoder {
    fn encode(&self, _rgba: &[u8], w: u32, h: u32, fmt: Format) -> Result<Vec<u8>, EncodeError> {
        let _ = fmt;
        Ok(vec![0u8; (w.div_ceil(4) * h.div_ceil(4) * 16) as usize])
    }
}

#[test]
fn gate_rejects_low_quality() {
    let (rgba, w, h) = first_card();
    let err = gate::check_quality(&BlackEncoder, Format::Bc7, &[(&rgba, w, h)]).unwrap_err();
    match err {
        GateFailure::Below { report, .. } => {
            assert!(report.mean < 20.0, "black blocks mean {:.2}", report.mean);
        }
        other => panic!("期望 Below，得到 {other:?}"),
    }
}

#[test]
fn gate_passes_on_gpu_card() {
    let gpu = GpuEncoder::new().unwrap();
    let (rgba, w, h) = first_card();
    let r = gate::check_quality(&gpu, Format::Bc7, &[(&rgba, w, h)]).unwrap();
    assert_eq!(r.count, 1);
    assert!(r.mean >= 43.08, "mean {:.2}", r.mean);
}
