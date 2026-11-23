//! 增量判定：源 PNG 的 md5 与现有 `.md5` 一致时跳过重编。
//!
//! 只比对源，产物缺失或损坏由调用方另行处理（`--check` 模式负责校验）。

use crate::godot::md5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Work {
    Skip,
    Encode,
}

pub fn decide(source_bytes: &[u8], existing_md5: Option<&str>) -> Work {
    let Some(text) = existing_md5 else {
        return Work::Encode;
    };
    match md5::read_source_md5(text) {
        Some(found) if found == md5::source_md5(source_bytes) => Work::Skip,
        _ => Work::Encode,
    }
}
