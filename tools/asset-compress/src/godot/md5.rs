//! `.md5` 校验文件。
//!
//! 格式（逐字节与 Godot 产物一致）：
//!
//! ```text
//! source_md5="<源 PNG 内容 md5>"
//! dest_md5="<按 dest_files 顺序拼接的产物文件字节 md5>"
//! ```

use md5::{Digest, Md5};

pub fn source_md5(bytes: &[u8]) -> String {
    let mut h = Md5::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

pub fn dest_md5(files: &[Vec<u8>]) -> String {
    let mut h = Md5::new();
    for f in files {
        h.update(f);
    }
    format!("{:x}", h.finalize())
}

/// 从已有 `.md5` 文本里取 `source_md5`，用于增量判定。
pub fn read_source_md5(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        line.strip_prefix("source_md5=\"")
            .and_then(|rest| rest.strip_suffix('"'))
            .map(str::to_string)
    })
}

/// Godot 的 `.md5` 以空行结尾（与现网产物逐字节一致）。
pub fn render_md5(source: &str, dest: &str) -> String {
    format!("source_md5=\"{source}\"\ndest_md5=\"{dest}\"\n\n")
}
