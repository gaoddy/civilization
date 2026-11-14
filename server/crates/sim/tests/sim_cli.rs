//! CLI 模拟器集成测试：单局、重放、批量与报告。

use std::path::PathBuf;
use std::process::Command;

use rule_engine::engine::Engine;
use rule_engine::hash::state_hash;
use rule_engine::state::{GameState, Seat};
use serde_json::Value;
use sim::ai;
use sim::report;

const CARDS_JSON: &str = include_str!("../../../data/v0.1/cards.json");
const SKILLS_JSON: &str = include_str!("../../../data/v0.1/skills.json");
const TOKENS_JSON: &str = include_str!("../../../data/v0.1/tokens.json");
const CONFIG_JSON: &str = include_str!("../../../data/v0.1/config.json");

fn catalog() -> rule_engine::data::Catalog {
    rule_engine::data::Catalog::from_json(CARDS_JSON, SKILLS_JSON, TOKENS_JSON, CONFIG_JSON)
        .expect("已提交的 v0.1 数据应能完整加载")
}

fn run(args: &[&str]) -> (i32, String, String) {
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

fn temp_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("sim-test-{}-{name}", std::process::id()));
    path
}

/// 日志中的 `final_hash`：16 位小写十六进制字符串，解析回数值便于比较。
fn log_hash(json: &Value) -> u64 {
    let text = json["final_hash"]
        .as_str()
        .unwrap_or_else(|| panic!("应含十六进制 final_hash：{json}"));
    u64::from_str_radix(text, 16).expect("final_hash 应为十六进制")
}

fn assert_ranking(ranking: &[Value], label: &str) {
    let mut seats: Vec<u64> = ranking
        .iter()
        .map(|entry| {
            entry["seat"]
                .as_u64()
                .unwrap_or_else(|| panic!("{label}: 排名缺少 seat"))
        })
        .collect();
    seats.sort_unstable();
    assert_eq!(
        seats,
        (0..8).collect::<Vec<u64>>(),
        "{label}: 应有 1..8 排名"
    );
    for (index, entry) in ranking.iter().enumerate() {
        assert_eq!(
            entry["place"].as_u64(),
            Some(index as u64 + 1),
            "{label}: 名次应连续"
        );
    }
}

#[test]
fn single_game_produces_ranking_and_hash() {
    let (code, stdout, stderr) = run(&["play", "--seed", "1"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let json: Value = serde_json::from_str(&stdout).expect("stdout 应是对局 JSON");
    let _ = log_hash(&json);
    assert_eq!(json["seed"].as_u64(), Some(1));
    let ranking = json["ranking"].as_array().expect("应含 ranking");
    assert_ranking(ranking, "single");
    assert!(json["turns"].is_u64(), "应含回合数");
}

#[test]
fn replay_same_log_twice_has_same_hash() {
    let (code, stdout, stderr) = run(&["play", "--seed", "42"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let json: Value = serde_json::from_str(&stdout).expect("stdout 应是对局 JSON");
    let hash = log_hash(&json);
    let log_path = temp_path("replay.json");
    std::fs::write(&log_path, &stdout).expect("应能写日志");

    let first = run(&["replay", "--log", log_path.to_str().unwrap()]);
    let second = run(&["replay", "--log", log_path.to_str().unwrap()]);
    assert_eq!(first.0, 0, "stderr: {}", first.2);
    assert_eq!(second.0, 0, "stderr: {}", second.2);
    let a: Value = serde_json::from_str(&first.1).expect("重放输出应为 JSON");
    let b: Value = serde_json::from_str(&second.1).expect("重放输出应为 JSON");
    assert_eq!(a["final_hash"].as_u64(), Some(hash));
    assert_eq!(b["final_hash"].as_u64(), Some(hash));
    let _ = std::fs::remove_file(&log_path);
}

#[test]
fn batch_of_ten_all_ranked() {
    let json = sim::run_games(10, 7, 8).expect("批量模拟应能完成");
    assert_eq!(json.games.len(), 10);
    for (index, game) in json.games.iter().enumerate() {
        // 第 i 局种子为 seed + i，写入日志便于复现。
        assert_eq!(game.seed, 7 + index as u64);
        assert_eq!(game.ranking.len(), 8, "第 {index} 局应全部排名");
        let mut seats: Vec<Seat> = game.ranking.clone();
        seats.sort_unstable();
        assert_eq!(seats, (0..8).collect::<Vec<Seat>>());
    }
}

#[test]
fn report_contains_required_fields() {
    let batch = sim::run_games(4, 1, 8).expect("批量模拟应能完成");
    let report = report::build(&batch);
    let value = serde_json::to_value(&report).expect("报告应能序列化");
    for field in [
        "games",
        "seed",
        "wins_by_tribe",
        "avg_turns",
        "era_upgrade_turns",
        "triples_per_game",
        "survivors_at_turn_20",
        "avg_damage_per_loss",
        "final_hashes",
    ] {
        assert!(value.get(field).is_some(), "报告缺少字段 {field}");
    }
    assert_eq!(value["games"].as_u64(), Some(4));
    assert_eq!(value["final_hashes"].as_array().map(Vec::len), Some(4));
}

#[test]
fn invalid_args_exit_nonzero() {
    let (code, _, stderr) = run(&["play"]);
    assert_ne!(code, 0, "缺 --seed 应非零退出");
    assert!(stderr.contains("--seed"), "stderr 应提示缺少 --seed");
    let (code, _, _) = run(&["batch", "--games", "2"]);
    assert_ne!(code, 0, "batch 缺 --seed 应非零退出");
    let (code, _, _) = run(&["report", "--log", "/nonexistent/sim-log.json"]);
    assert_ne!(code, 0, "不存在日志应非零退出");
    let (code, _, _) = run(&["bogus"]);
    assert_ne!(code, 0, "未知子命令应非零退出");
}

#[test]
fn ai_never_loops_and_matches_state_hash() {
    // AI 决定论与状态哈希：同种子两局完全一致，且每个回合都能以 EndRecruit 结束。
    let mut hashes = Vec::new();
    for _ in 0..3 {
        let (mut engine, mut state) = Engine::new(catalog(), [3; 32]);
        let log = sim::play_game(&mut engine, &mut state, 3, 8).expect("单局应能完成");
        hashes.push(state_hash(&state));
        assert_eq!(log.final_hash, state_hash(&state));
        assert_eq!(state.phase, rule_engine::state::Phase::GameOver);
    }
    assert_eq!(hashes[0], hashes[1]);
    assert_eq!(hashes[1], hashes[2]);
}

#[test]
fn ai_choose_is_deterministic_for_seat() {
    let (mut engine, mut state) = Engine::new(catalog(), [11; 32]);
    let heroes: std::collections::BTreeMap<Seat, String> = engine
        .catalog
        .cards
        .values()
        .filter(|card| card.is_hero)
        .take(8)
        .enumerate()
        .map(|(index, card)| (index as Seat, card.id.clone()))
        .collect();
    engine.start_game(&mut state, &heroes).expect("开局应成功");
    let a = ai::choose(&state, 0, &engine.catalog);
    let b = ai::choose(&state, 0, &engine.catalog);
    assert_eq!(a, b);
    assert!(a.is_some());
    let _: &GameState = &state;
}
