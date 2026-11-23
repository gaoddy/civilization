use asset_compress::godot::md5;
use asset_compress::plan::{self, Work};

#[test]
fn skips_when_source_md5_matches() {
    let src = b"png-bytes";
    let existing = format!(
        "source_md5=\"{}\"\ndest_md5=\"x\"\n\n",
        md5::source_md5(src)
    );
    assert_eq!(plan::decide(src, Some(&existing)), Work::Skip);
}

#[test]
fn encodes_when_md5_missing_or_different() {
    assert_eq!(plan::decide(b"a", None), Work::Encode);
    assert_eq!(
        plan::decide(b"a", Some("source_md5=\"deadbeef\"\n\n")),
        Work::Encode
    );
    assert_eq!(plan::decide(b"a", Some("下载中\n")), Work::Encode);
}
