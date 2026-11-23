use asset_compress::godot::{import, md5};

fn fx(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn fixtures_reproduce_godot_md5_file() {
    let png = std::fs::read(fx("ch_ammonite_ava.png")).unwrap();
    let bptc = std::fs::read(fx("ch_ammonite_ava.bptc.ctex")).unwrap();
    let astc = std::fs::read(fx("ch_ammonite_ava.astc.ctex")).unwrap();
    let want = std::fs::read_to_string(fx("ch_ammonite_ava.md5")).unwrap();

    let src = md5::source_md5(&png);
    let dst = md5::dest_md5(&[bptc, astc]);
    assert_eq!(md5::render_md5(&src, &dst), want);
}

#[test]
fn dest_md5_is_order_sensitive() {
    let a = md5::dest_md5(&[b"aaa".to_vec(), b"bbb".to_vec()]);
    let b = md5::dest_md5(&[b"bbb".to_vec(), b"aaa".to_vec()]);
    assert_ne!(a, b);
    assert_eq!(a, md5::source_md5(b"aaabbb"));
}

#[test]
fn import_round_trips_uid() {
    let t = import::render_import(
        "res://assets/a.png",
        "uid://b5sf4yux3vulg",
        "res://.godot/imported/a.png-1.bptc.ctex",
        "res://.godot/imported/a.png-1.astc.ctex",
        "[params]\ncompress/mode=2\n",
    );
    assert_eq!(
        import::existing_uid(&t).as_deref(),
        Some("uid://b5sf4yux3vulg")
    );
    assert!(t.contains("path.bptc=\"res://.godot/imported/a.png-1.bptc.ctex\""));
    assert!(t.contains("dest_files=[") && t.contains(".astc.ctex\"]"));
}

#[test]
fn existing_uid_absent_is_none() {
    assert_eq!(import::existing_uid("[remap]\nimporter=\"texture\"\n"), None);
}

#[test]
fn uid_is_stable_and_well_shaped() {
    let u = import::new_uid("res://assets/a.png:deadbeef");
    assert_eq!(u, import::new_uid("res://assets/a.png:deadbeef"));
    assert_ne!(u, import::new_uid("res://assets/b.png:deadbeef"));
    let body = u.strip_prefix("uid://").expect("uid:// 前缀");
    assert_eq!(body.len(), 13, "{u}");
    assert!(body.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()), "{u}");
}

#[test]
fn fixture_import_uid_is_reused_verbatim() {
    let t = std::fs::read_to_string(fx("ch_ammonite_ava.png.import")).unwrap();
    let uid = import::existing_uid(&t).unwrap();
    // Godot 的 uid 是 64 位数的 base32 文本，长度 12–13 取决于前导零
    let body = uid.strip_prefix("uid://").unwrap();
    assert!((12..=13).contains(&body.len()), "{uid}");
}
