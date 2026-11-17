#![allow(dead_code)]

use std::path::PathBuf;
use std::process::Command;

use rule_engine::data::Catalog;
use serde_json::Value;

/// 加载仓库内嵌的 v0.1 数据。
pub fn catalog() -> Catalog {
    sim::embedded_catalog().expect("内嵌卡表应能加载")
}

/// 运行 `sim` 二进制并取回退出码、stdout 与 stderr。
pub fn run(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_sim"))
        .args(args)
        .output()
        .expect("应能启动 sim 二进制");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// 本进程专用的临时文件路径。
pub fn temp_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("sim-test-{}-{name}", std::process::id()));
    path
}

/// 日志中的 `final_hash`：16 位小写十六进制字符串，解析回数值便于比较。
pub fn log_hash(json: &Value) -> u64 {
    let text = json["final_hash"]
        .as_str()
        .unwrap_or_else(|| panic!("应含十六进制 final_hash：{json}"));
    u64::from_str_radix(text, 16).expect("final_hash 应为十六进制")
}
