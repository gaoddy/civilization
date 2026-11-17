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

/// 批次最终哈希必须与改动前的固定基线一致。
///
/// 基线由结构化 effects 改造前的代码生成；它冻结的是行为，不只是自洽性。
#[test]
fn batch_final_hashes_match_golden_baseline() {
    let golden: Value = serde_json::from_str(include_str!("golden/seed-1-games-20.json"))
        .expect("基线文件应为 JSON");
    let mut path = std::env::temp_dir();
    path.push(format!("sim-test-{}-batch20.json", std::process::id()));
    let (code, _stdout, stderr) = run(&[
        "batch",
        "--games",
        "20",
        "--seed",
        "1",
        "--out",
        path.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let actual: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("应能读取批次输出"))
            .expect("批次输出应为 JSON");
    let _ = std::fs::remove_file(&path);

    let expected = golden["hashes"].as_array().expect("基线应含 hashes");
    let games = actual["games"].as_array().expect("输出应含 games");
    assert_eq!(games.len(), expected.len(), "局数不一致");
    for (index, (game, hash)) in games.iter().zip(expected).enumerate() {
        assert_eq!(game["final_hash"], *hash, "第 {index} 局最终哈希不一致");
    }
}
