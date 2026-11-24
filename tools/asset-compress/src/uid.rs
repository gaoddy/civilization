//! 跨资源唯一的 uid 分配。
//!
//! `.import` 的 uid 必须全局唯一：冲突不会报错，只会让 Godot 重编该资源并
//! 留下歧义引用（见 `tools/asset-compress/docs/phase0/UID_VERIFICATION.md` 约束 3）。
//! 已有 `.import` 的资源直接复用其 uid、不经过这里；只有全新资源才需要分配，
//! 分配前扫描项目内全部 `.import`，撞上已占用的值就按确定性加盐重算。

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::godot::import;

#[derive(Debug)]
pub struct UidBook {
    root: PathBuf,
    /// 扫描是惰性的：项目里全是有 `.import` 的老资源时，一次也不需要读盘。
    used: Option<BTreeSet<String>>,
}

impl UidBook {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            used: None,
        }
    }

    /// 分配一个项目内未占用的 uid。同一项目、同一 seed 的结果确定，
    /// 重跑不会在几个候选值之间跳来跳去。
    pub fn allocate(&mut self, seed: &str) -> String {
        let root = self.root.clone();
        let used = self.used.get_or_insert_with(|| scan(&root));
        for n in 0u64.. {
            let candidate = if n == 0 {
                import::new_uid(seed)
            } else {
                import::new_uid(&format!("{seed}#{n}"))
            };
            if used.insert(candidate.clone()) {
                return candidate;
            }
        }
        unreachable!("候选空间远大于项目资源数")
    }
}

/// 收集 `root` 下全部 `.import` 的 uid；解析不了的文件直接跳过。
/// `.godot` 是导入缓存，里面的产物不携带 uid。
fn scan(root: &Path) -> BTreeSet<String> {
    let mut used = BTreeSet::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if dir.file_name().is_some_and(|n| n == ".godot") {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "import")
                && let Ok(text) = std::fs::read_to_string(&path)
                && let Some(uid) = import::existing_uid(&text)
            {
                used.insert(uid);
            }
        }
    }
    used
}
