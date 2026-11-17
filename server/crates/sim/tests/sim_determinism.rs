//! 100 次重放的确定性校验。

mod common;

use common::{log_hash, run};
use serde_json::Value;

#[test]
fn replay_same_log_100_times_has_same_hash() {
    let (code, stdout, stderr) = run(&["play", "--seed", "42"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let log: Value = serde_json::from_str(&stdout).expect("stdout 应是对局 JSON");
    let expected = log_hash(&log);
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

/// 批次最终哈希必须与固定基线一致。
///
/// 本基线因 i18n 关键词标识英文化而重定：状态哈希会写入 `granted` 关键词与
/// `aura.keywords` 中的关键词字符串，标识由中文改为英文 id 后哈希输入随之变化，
/// 故哈希值变化不代表结算行为变化。基线冻结的仍是行为：任何算术、遍历顺序或
/// 随机流消耗变化都会使哈希改变。
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
