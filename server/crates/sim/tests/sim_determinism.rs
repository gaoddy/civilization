//! 100 次重放的确定性校验。

use serde_json::Value;

fn run(args: &[&str]) -> (i32, String, String) {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_sim"))
        .args(args)
        .output()
        .expect("应能启动 sim 二进制");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn replay_same_log_100_times_has_same_hash() {
    let (code, stdout, stderr) = run(&["play", "--seed", "42"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let log: Value = serde_json::from_str(&stdout).expect("stdout 应是对局 JSON");
    // 日志中的 `final_hash` 是 16 位十六进制字符串，`replay` 输出数值哈希，二者比较前统一解析。
    let expected = u64::from_str_radix(
        log["final_hash"]
            .as_str()
            .expect("日志应含十六进制 final_hash"),
        16,
    )
    .expect("final_hash 应为十六进制");
    let mut path = std::env::temp_dir();
    path.push(format!("sim-test-{}-replay100.json", std::process::id()));
    std::fs::write(&path, &stdout).expect("应能写日志");

    for attempt in 0..100 {
        let (code, stdout, stderr) = run(&["replay", "--log", path.to_str().unwrap()]);
        assert_eq!(code, 0, "第 {attempt} 次重放失败: {stderr}");
        let value: Value = serde_json::from_str(&stdout).expect("重放输出应为 JSON");
        assert_eq!(
            value["final_hash"].as_u64(),
            Some(expected),
            "第 {attempt} 次重放哈希不一致"
        );
    }

    let _ = std::fs::remove_file(&path);
}
