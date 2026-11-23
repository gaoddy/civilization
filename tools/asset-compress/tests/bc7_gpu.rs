use asset_compress::Format;
use asset_compress::encode::{Encoder, GpuEncoder};
use std::path::{Path, PathBuf};

fn assets_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../client/assets")
}

fn first_n(dir: &Path, n: usize) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "png"))
        .collect();
    files.sort();
    files.truncate(n);
    files
}

fn psnr(a: &[u8], b: &[u8]) -> f64 {
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

#[test]
fn gpu_bc7_meets_gate_on_cards() {
    let gpu = GpuEncoder::new().expect("需要 Metal adapter");
    let mut sum = 0.0;
    let mut min = f64::INFINITY;
    let mut n = 0.0;
    for f in first_n(&assets_dir().join("cards/faces"), 60) {
        let img = image::open(&f).unwrap().to_rgba8();
        let (w, h) = (img.width(), img.height());
        let blocks = gpu.encode(img.as_raw(), w, h, Format::Bc7).unwrap();
        let d =
            bcn_decode::decode_rgba8_to_vec(bcn_decode::Format::Bc7, &blocks, w as usize, h as usize)
                .unwrap();
        let p = psnr(img.as_raw(), &d);
        sum += p;
        min = min.min(p);
        n += 1.0;
    }
    let mean = sum / n;
    assert!(mean >= 43.08, "BC7 mean {mean:.2} < 43.08");
    assert!(min >= 38.91, "BC7 min {min:.2} < Godot min-0.5");
}

#[test]
fn batched_output_equals_single_output() {
    let gpu = GpuEncoder::new().unwrap();
    let items: Vec<(Vec<u8>, u32, u32)> = first_n(&assets_dir().join("characters/avatars"), 3)
        .iter()
        .map(|f| {
            let i = image::open(f).unwrap().to_rgba8();
            let (w, h) = (i.width(), i.height());
            (i.into_raw(), w, h)
        })
        .collect();
    let refs: Vec<(&[u8], u32, u32)> = items.iter().map(|(r, w, h)| (r.as_slice(), *w, *h)).collect();
    let batch = gpu.encode_batch(&refs);
    assert_eq!(batch.len(), items.len());
    for ((rgba, w, h), b) in items.iter().zip(batch.iter()) {
        assert_eq!(&gpu.encode(rgba, *w, *h, Format::Bc7).unwrap(), b);
    }
}

#[test]
fn gpu_bc7_keeps_alpha_on_translucent_blocks() {
    // 现网素材全是不透明 PNG（实测 0 张带 alpha），用合成图验证 GPU 路径不会
    // 把半透明块当不透明编码。
    let gpu = GpuEncoder::new().unwrap();
    let (w, h) = (64u32, 64u32);
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for _y in 0..h {
        for x in 0..w {
            let a = (255 - x * 4) as u8;
            rgba.extend_from_slice(&[200, 120, 80, a]);
        }
    }
    assert!(rgba.chunks(4).any(|p| p[3] != 255), "样本必须含半透明像素");
    let blocks = gpu.encode(&rgba, w, h, Format::Bc7).unwrap();
    let d = bcn_decode::decode_rgba8_to_vec(bcn_decode::Format::Bc7, &blocks, w as usize, h as usize)
        .unwrap();
    let alpha_src: Vec<u8> = rgba.chunks(4).map(|p| p[3]).collect();
    let alpha_dst: Vec<u8> = d.chunks(4).map(|p| p[3]).collect();
    let p = psnr(&alpha_src, &alpha_dst);
    assert!(p >= 40.0, "alpha PSNR {p:.2} < 40");
}
