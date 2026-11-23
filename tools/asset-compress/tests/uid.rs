//! 新 uid 的跨资源去重：生成前扫描项目内全部 `.import`。
//!
//! uid 冲突不会报错，只会让 Godot 重编该资源并留下歧义引用
//! （见 `output/gpu-texture-phase0/UID_VERIFICATION.md` 约束 3）。

use asset_compress::godot::import;
use asset_compress::uid::UidBook;
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
