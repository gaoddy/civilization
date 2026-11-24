# 阶段 0 原型留档

本目录保留纹理压缩方案阶段 0 的实测结论与原始 shader，供 `tools/asset-compress` 追溯：

- `UID_VERIFICATION.md`：Godot `.import` 的 uid 语义实测（`src/uid.rs`、`src/godot/import.rs`、`tests/uid.rs` 引用）。
- `ASTC_NOTES.md`：ASTC 编码实测记录。
- `BASELINE.md`：阶段 0 基线数据。
- `bc7_mode1.wgsl`：`src/shaders/bc7-standard.wgsl` 的移植来源原型。

其余原型代码（探针二进制、Cargo 工程、参考实现）是一次性实验，未随仓库保留；需要重跑时按 `ASTC_NOTES.md` 与 `BASELINE.md` 的步骤重建。
