//! 非 4 倍数尺寸：按边缘复制补齐到块边界，纹理尺寸仍保留原值。
//!
//! Godot 对这类素材是双线性放大到块边界再编码（会把边缘拉糊），本工具保留
//! 锐利边缘；`.ctex` 头形制与 Godot 一致：u32 写原尺寸、u16 写块对齐尺寸
//! （实测 Godot 对 5×7 写 5/7 + 8/8）。

use asset_compress::encode::{CpuEncoder, Encoder, GpuEncoder};
use asset_compress::{run_one, Format};
use std::path::PathBuf;

fn fx(name: &str) -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
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

fn odd_rgba() -> (Vec<u8>, u32, u32) {
    let img = image::load_from_memory(&std::fs::read(fx("odd_5x7.png")).unwrap())
        .unwrap()
        .to_rgba8();
    let (w, h) = (img.width(), img.height());
    assert_eq!((w, h), (5, 7));
    (img.into_raw(), w, h)
}

fn decode_astc(blocks: &[u8], w: u32, h: u32) -> Vec<u8> {
    use astcenc_rs::{ConfigBuilder, Context, Extents, Flags, Swizzle};
    let cfg = ConfigBuilder::new()
        .with_block_size(Extents::new(4, 4))
        .with_preset(astcenc_rs::PRESET_MEDIUM)
        .with_flags(Flags::DECOMPRESS_ONLY)
        .build()
        .unwrap();
    let mut ctx = Context::with_threads(1, cfg).unwrap();
    ctx.decompress::<u8>(blocks, Extents::new(w, h), Swizzle::rgba())
        .unwrap()
        .data
        .remove(0)
}

/// 5×7 的 ASTC 按 ceil 块数输出，头写原尺寸 + 块对齐尺寸，纹理范围内像素不糊。
#[test]
fn astc_non_multiple_of_4_keeps_original_size() {
    let (rgba, w, h) = odd_rgba();
    let ours = CpuEncoder::new(1).encode(&rgba, w, h, Format::Astc4x4).unwrap();
    assert_eq!(ours.len(), 4 * 16, "5×7 应为 2×2 个块");

    let ctex = asset_compress::godot::ctex::encode_ctex(Format::Astc4x4, w, h, &ours).unwrap();
    assert_eq!(u32::from_le_bytes(ctex[8..12].try_into().unwrap()), 5);
    assert_eq!(u32::from_le_bytes(ctex[12..16].try_into().unwrap()), 7);
    assert_eq!(u16::from_le_bytes(ctex[40..42].try_into().unwrap()), 8);
    assert_eq!(u16::from_le_bytes(ctex[42..44].try_into().unwrap()), 8);

    let d = decode_astc(&ours, w, h);
    let p = psnr(&rgba, &d);
    assert!(p >= 40.0, "ASTC 5×7 PSNR {p:.2} < 40");
}

/// CPU BC7 不再拒绝非 4 倍数，补齐后按 ceil 块数输出。
#[test]
fn cpu_bc7_encodes_non_multiple_of_4() {
    let (rgba, w, h) = odd_rgba();
    let blocks = CpuEncoder::new(1).encode(&rgba, w, h, Format::Bc7).unwrap();
    assert_eq!(blocks.len(), 4 * 16);
    let d = bcn_decode::decode_rgba8_to_vec(
        bcn_decode::Format::Bc7,
        &blocks,
        w as usize,
        h as usize,
    )
    .unwrap();
    let p = psnr(&rgba, &d);
    assert!(p >= 40.0, "BC7 5×7 PSNR {p:.2} < 40");
}

/// GPU BC7 同样接受非 4 倍数。
#[test]
fn gpu_bc7_encodes_non_multiple_of_4() {
    let gpu = GpuEncoder::new().expect("需要 Metal adapter");
    let (rgba, w, h) = odd_rgba();
    let blocks = gpu.encode(&rgba, w, h, Format::Bc7).unwrap();
    assert_eq!(blocks.len(), 4 * 16);
    let d = bcn_decode::decode_rgba8_to_vec(
        bcn_decode::Format::Bc7,
        &blocks,
        w as usize,
        h as usize,
    )
    .unwrap();
    let p = psnr(&rgba, &d);
    assert!(p >= 40.0, "GPU BC7 5×7 PSNR {p:.2} < 40");
}

/// `.md5` 是「产物完整」的提交标记：`.import` 缺失时必须重编，
/// 否则中断后工具会永久跳过、留下 Godot 会重编的残缺状态。
#[test]
fn needs_encode_when_import_missing() {
    let root = std::env::temp_dir().join(format!(
        "asset-compress-needs-encode-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let src_dir = root.join("assets");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(root.join("project.godot"), "").unwrap();
    let src = src_dir.join("a.png");
    std::fs::copy(fx("ch_ammonite_ava.png"), &src).unwrap();
    let source = std::fs::read(&src).unwrap();
    let out_dir = root.join(".godot/imported");

    run_one(&src, "res://assets/a.png", &out_dir, &CpuEncoder::new(1)).unwrap();
    assert!(!asset_compress::run::needs_encode(&root, &src, &source).unwrap());

    std::fs::remove_file(src.with_extension("png.import")).unwrap();
    assert!(
        asset_compress::run::needs_encode(&root, &src, &source).unwrap(),
        "缺 .import 时必须重编"
    );

    std::fs::remove_dir_all(&root).unwrap();
}
