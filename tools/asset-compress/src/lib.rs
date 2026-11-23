//! asset-compress：把 PNG 编码成 Godot 可直接识别的 VRAM 压缩产物。
//!
//! 分层：CLI/编排 → 编码器（CPU exact / GPU standard）→ Godot 产物 writer。

pub mod cli;
pub mod format;
pub mod godot;

pub use cli::{Args, Backend, Quality};
pub use format::Format;
