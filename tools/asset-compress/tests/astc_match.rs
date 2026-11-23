use asset_compress::Format;
use asset_compress::encode::{CpuEncoder, Encoder};
use asset_compress::run_one;
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

fn psnr_alpha_after_decode(fmt: Format, blocks: &[u8], rgba: &[u8], w: u32, h: u32) -> f64 {
    let decoded = match fmt {
        Format::Bc7 => bcn_decode::decode_rgba8_to_vec(
            bcn_decode::Format::Bc7,
            blocks,
            w as usize,
            h as usize,
        )
        .unwrap(),
        Format::Astc4x4 => decode_astc(blocks, w, h),
    };
    psnr(rgba, &decoded)
}

#[test]
fn astc_blocks_match_godot_fixture() {
    let img = image::load_from_memory(&std::fs::read(fx("ch_ammonite_ava.png")).unwrap())
        .unwrap()
        .to_rgba8();
    let (w, h) = (img.width(), img.height());
    let ours = CpuEncoder::new(1)
        .encode(img.as_raw(), w, h, Format::Astc4x4)
        .unwrap();
    let godot = std::fs::read(fx("ch_ammonite_ava.astc.ctex")).unwrap();
    assert_eq!(ours, godot[52..], "ASTC 块与 Godot 产物必须逐字节一致");
}

/// 半透明块必须真的编码 alpha（而不是被当成不透明丢弃）。
///
/// 通道沿同一方向按比例变化：BC7 的 RGB 与 alpha 共享插值索引时能精确表示；
/// alpha 若被丢弃会直接掉到 ~10 dB。
#[test]
fn translucent_blocks_survive_round_trip() {
    let mut rgba = Vec::new();
    for _y in 0..8u8 {
        for x in 0..8u8 {
            rgba.extend_from_slice(&[x * 32, x * 32, 128, x * 16]);
        }
    }
    assert!(rgba.chunks(4).any(|p| p[3] != 255), "样本必须含半透明像素");
    for fmt in [Format::Bc7, Format::Astc4x4] {
        let b = CpuEncoder::new(1).encode(&rgba, 8, 8, fmt).unwrap();
        assert!(!b.is_empty(), "{fmt:?} 编码失败");
        let p = psnr_alpha_after_decode(fmt, &b, &rgba, 8, 8);
        assert!(p >= 40.0, "{fmt:?} PSNR {p:.2} < 40");
    }
}

#[test]
fn run_one_reproduces_godot_import_layout() {
    let root = std::env::temp_dir().join(format!("asset-compress-run-one-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let src_dir = root.join("assets/characters/avatars");
    std::fs::create_dir_all(&src_dir).unwrap();
    let src = src_dir.join("ch_ammonite_ava.png");
    std::fs::copy(fx("ch_ammonite_ava.png"), &src).unwrap();
    std::fs::copy(fx("ch_ammonite_ava.png.import"), src.with_extension("png.import")).unwrap();

    let out_dir = root.join(".godot/imported");
    let res_path = "res://assets/characters/avatars/ch_ammonite_ava.png";
    let outcome = run_one(&src, res_path, &out_dir, &CpuEncoder::new(1)).unwrap();

    assert_eq!(
        std::fs::read_to_string(src.with_extension("png.import")).unwrap(),
        std::fs::read_to_string(fx("ch_ammonite_ava.png.import")).unwrap(),
        ".import 必须原样保留（uid、params 均不变）"
    );

    let base = "ch_ammonite_ava.png-b49c22653db395ac738c396b434cd412";
    let astc = std::fs::read(out_dir.join(format!("{base}.astc.ctex"))).unwrap();
    assert_eq!(astc, std::fs::read(fx("ch_ammonite_ava.astc.ctex")).unwrap());
    let bptc = std::fs::read(out_dir.join(format!("{base}.bptc.ctex"))).unwrap();
    assert_eq!(bptc.len(), 52 + 64 * 64 * 16);

    let md5_text = std::fs::read_to_string(out_dir.join(format!("{base}.md5"))).unwrap();
    assert_eq!(
        asset_compress::godot::md5::read_source_md5(&md5_text).unwrap(),
        outcome.source_md5
    );
    assert_eq!(outcome.source_md5, "ce8296b63346bba3be32f4e4c1e5e96d");

    std::fs::remove_dir_all(&root).unwrap();
}
