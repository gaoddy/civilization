//! 新 uid 的跨资源去重：生成前扫描项目内全部 `.import`。
//!
//! uid 冲突不会报错，只会让 Godot 重编该资源并留下歧义引用
//! （见 `tools/asset-compress/docs/phase0/UID_VERIFICATION.md` 约束 3）。

use asset_compress::encode::CpuEncoder;
use asset_compress::godot::{import, md5};
use asset_compress::uid::UidBook;
use asset_compress::run_one;
use std::path::{Path, PathBuf};

fn temp_root(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "asset-compress-uid-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn write_import(dir: &Path, name: &str, uid: &str) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(
        dir.join(format!("{name}.import")),
        format!("[remap]\n\nimporter=\"texture\"\nuid=\"{uid}\"\n"),
    )
    .unwrap();
}

#[test]
fn allocate_skips_uid_already_in_project() {
    let root = temp_root("skip");
    let seed = "res://assets/a.png:deadbeef";
    let taken = import::new_uid(seed);
    write_import(&root.join("assets/cards"), "other.png", &taken);

    let mut book = UidBook::new(&root);
    let first = book.allocate(seed);
    assert_ne!(first, taken, "必须避开项目内已占用的 uid");
    assert_eq!(first.len(), "uid://".len() + 13, "{first}");

    let second = book.allocate(seed);
    assert_ne!(second, first, "同一 seed 再次分配必须给出新值");

    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn allocation_is_deterministic() {
    let root = temp_root("det");
    let seed = "res://assets/a.png:deadbeef";
    let taken = import::new_uid(seed);
    write_import(&root.join("assets"), "other.png", &taken);

    let mut first_book = UidBook::new(&root);
    let a = first_book.allocate(seed);
    let b = first_book.allocate("res://assets/b.png:cafe");

    // 同一项目、同一 seed 两次运行结果一致（重跑不会换来换去）。
    let mut second_book = UidBook::new(&root);
    assert_eq!(second_book.allocate(seed), a);
    assert_eq!(second_book.allocate("res://assets/b.png:cafe"), b);

    std::fs::remove_dir_all(&root).unwrap();
}

/// 端到端：新资源的 `.import` 不能拿到项目里别的资源已经用着的 uid。
#[test]
fn run_one_avoids_uid_used_by_another_asset() {
    let root = temp_root("run-one");
    std::fs::write(root.join("project.godot"), "").unwrap();
    let src_dir = root.join("assets");
    std::fs::create_dir_all(&src_dir).unwrap();
    let src = src_dir.join("a.png");
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/odd_5x7.png"),
        &src,
    )
    .unwrap();

    let res = "res://assets/a.png";
    let source = std::fs::read(&src).unwrap();
    let seed = format!("{res}:{}", md5::source_md5(&source));
    let would_be = import::new_uid(&seed);
    write_import(&root.join("assets/other"), "decoy.png", &would_be);

    let out_dir = root.join(".godot/imported");
    run_one(&src, res, &out_dir, &CpuEncoder::new(1)).unwrap();

    let text = std::fs::read_to_string(src.with_extension("png.import")).unwrap();
    let uid = import::existing_uid(&text).unwrap();
    assert_ne!(uid, would_be, "新 uid 不能与项目内已有资源冲突");

    std::fs::remove_dir_all(&root).unwrap();
}
