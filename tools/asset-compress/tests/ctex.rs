use asset_compress::godot::ctex;
use asset_compress::Format;

fn fx(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn header_matches_godot_fixture() {
    let bytes = std::fs::read(fx("ch_ammonite_ava.astc.ctex")).unwrap();
    let h = ctex::parse_header(&bytes).unwrap();
    assert_eq!((h.width, h.height, h.format), (256, 256, 35));
    assert_eq!(h.payload_len, 64 * 64 * 16);
    let rebuilt = ctex::encode_ctex(Format::Astc4x4, h.width, h.height, &bytes[52..]).unwrap();
    assert_eq!(rebuilt, bytes, "ASTC GST2 必须逐字节复刻 Godot 产物");
}

#[test]
fn bc7_format_field_is_22() {
    let bytes = std::fs::read(fx("ch_ammonite_ava.bptc.ctex")).unwrap();
    let h = ctex::parse_header(&bytes).unwrap();
    assert_eq!(h.format, 22);
    let rebuilt = ctex::encode_ctex(Format::Bc7, h.width, h.height, &bytes[52..]).unwrap();
    assert_eq!(rebuilt, bytes, "BC7 GST2 必须逐字节复刻 Godot 产物");
}

#[test]
fn rejects_payload_of_wrong_length() {
    assert!(ctex::encode_ctex(Format::Astc4x4, 256, 256, &[0u8; 10]).is_err());
}

#[test]
fn rejects_oversized_dimensions() {
    let blocks = vec![0u8; 17_500 * 16];
    assert!(ctex::encode_ctex(Format::Bc7, 70_000, 4, &blocks).is_err());
}
