//! CLI 模拟器集成测试：单局、重放、批量与报告。

mod common;

use common::{catalog, log_hash, run, temp_path};
use rule_engine::engine::Engine;
use rule_engine::hash::state_hash;
use rule_engine::state::{GameState, Seat};
use serde_json::Value;
use sim::ai;
use sim::report;

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
        "wins_by_playstyle",
        "avg_turns",
        "win_rate_permille_by_playstyle",
        "era_upgrade_turns_by_seat",
        "avg_era_upgrade_turn_by_seat",
        "avg_era_upgrade_turn",
        "era_upgrade_count",
        "triples_per_game",
        "games_reaching_turn_20",
        "survivors_at_turn_20",
        "games_ended_on_turn_20",
        "avg_survivors_at_end",
        "avg_damage_per_loss",
        "final_hashes",
    ] {
        assert!(value.get(field).is_some(), "报告缺少字段 {field}");
    }
    assert_eq!(value["games"].as_u64(), Some(4));
    assert_eq!(value["final_hashes"].as_array().map(Vec::len), Some(4));
    assert_eq!(
        value["era_upgrade_turns_by_seat"].as_array().map(Vec::len),
        Some(8)
    );
    assert_eq!(
        value["win_rate_permille_by_playstyle"]
            .as_object()
            .map(|m| m.len()),
        Some(14),
        "13 个玩法与中立都应出现夺冠率"
    );
    let rates = value["win_rate_permille_by_playstyle"]
        .as_object()
        .expect("夺冠率为对象");
    let permille_sum: u64 = rates
        .values()
        .map(|rate| rate.as_u64().expect("夺冠率为整数"))
        .sum();
    let games = value["games"].as_u64().expect("games 为整数");
    // 按注释口径断言上界；下界由「夺冠局数之和 = 局数」间接保证，并核对每个流派与计数一致。
    assert!(
        permille_sum <= games * 1_000,
        "各玩法夺冠率千分比之和不应超过局数 * 1000（实际 {permille_sum}，局数 {games}）"
    );
    let wins_total: u64 = value["wins_by_playstyle"]
        .as_object()
        .expect("wins_by_playstyle 为对象")
        .values()
        .map(|count| count.as_u64().expect("计数为整数"))
        .sum();
    assert_eq!(wins_total, games, "各玩法夺冠局数之和应等于局数");
    let wins = value["wins_by_playstyle"]
        .as_object()
        .expect("wins_by_playstyle 为对象");
    for (playstyle, count) in wins {
        let expected = count.as_u64().expect("计数为整数") * 1_000 / games;
        assert_eq!(
            rates[playstyle].as_u64(),
            Some(expected),
            "玩法 {playstyle} 夺冠率应为 夺冠局数 * 1000 / 局数（向下取整）"
        );
    }
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
        let (mut engine, mut state) =
            Engine::new(catalog(), [3; 32], rule_engine::playstyles::draw(3));
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
    let (mut engine, mut state) =
        Engine::new(catalog(), [11; 32], rule_engine::playstyles::draw(11));
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

#[test]
fn assign_heroes_filters_by_enabled_playstyles_and_cycles() {
    use rule_engine::data::{Playstyle, card_in_pool};
    use std::collections::BTreeSet;

    let catalog = catalog();
    // 植物玩法只有少量英雄：过滤后不足 8 张，按 id 升序循环取。
    let plant: BTreeSet<Playstyle> = [Playstyle::Plant].into_iter().collect();
    let heroes = sim::assign_heroes(&catalog, &plant).expect("植物玩法应有可用英雄");
    assert_eq!(heroes.len(), 8);
    let ids: Vec<&str> = heroes.values().map(String::as_str).collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert!(
        sorted.windows(2).any(|pair| pair[0] == pair[1]),
        "可用英雄不足 8 张时应循环取"
    );
    for id in &ids {
        let card = catalog.cards.get(*id).expect("英雄卡应存在");
        assert!(card.is_hero, "{id} 不是英雄卡");
        assert!(card_in_pool(card, &plant), "{id} 不属于植物玩法卡池");
    }

    // 哺乳玩法英雄充足：按 id 升序取前 8 张，不重复。
    let mammal: BTreeSet<Playstyle> = [Playstyle::Mammal].into_iter().collect();
    let heroes = sim::assign_heroes(&catalog, &mammal).expect("哺乳玩法应有可用英雄");
    let ids: Vec<&str> = heroes.values().map(String::as_str).collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(ids, sorted, "英雄应按 id 升序分配");
    let distinct: BTreeSet<&str> = ids.iter().copied().collect();
    assert_eq!(distinct.len(), 8, "英雄池足够时每个座位不同");
}

fn sample_game_json(seed: u64) -> Value {
    let (code, stdout, stderr) = run(&["play", "--seed", &seed.to_string()]);
    assert_eq!(code, 0, "stderr: {stderr}");
    serde_json::from_str(&stdout).expect("stdout 应是对局 JSON")
}

#[test]
fn game_from_json_rejects_missing_final_hash() {
    let mut json = sample_game_json(3);
    json.as_object_mut().unwrap().remove("final_hash");
    let error = report::game_from_json(&json).expect_err("缺 final_hash 应报错");
    assert!(
        error.contains("final_hash"),
        "错误应指出 final_hash：{error}"
    );
}

#[test]
fn game_from_json_rejects_missing_event_hash() {
    let mut json = sample_game_json(5);
    json.as_object_mut().unwrap().remove("event_hash");
    let error = report::game_from_json(&json).expect_err("缺 event_hash 应报错");
    assert!(
        error.contains("event_hash"),
        "错误应指出 event_hash：{error}"
    );
}

#[test]
fn game_from_json_rejects_missing_or_unknown_winner_fields() {
    let base = sample_game_json(4);

    let mut no_playstyle = base.clone();
    no_playstyle["stats"]
        .as_object_mut()
        .unwrap()
        .remove("winner_playstyle");
    let error = report::game_from_json(&no_playstyle).expect_err("缺 winner_playstyle 应报错");
    assert!(
        error.contains("winner_playstyle"),
        "错误应指出 winner_playstyle：{error}"
    );

    let mut unknown_playstyle = base.clone();
    unknown_playstyle["stats"]["winner_playstyle"] = Value::from("龙");
    let error =
        report::game_from_json(&unknown_playstyle).expect_err("未知 winner_playstyle 应报错");
    assert!(error.contains("龙"), "错误应带上未知玩法名：{error}");

    let mut no_seat = base.clone();
    no_seat["stats"]
        .as_object_mut()
        .unwrap()
        .remove("winner_seat");
    let error = report::game_from_json(&no_seat).expect_err("缺 winner_seat 应报错");
    assert!(
        error.contains("winner_seat"),
        "错误应指出 winner_seat：{error}"
    );

    report::game_from_json(&base).expect("完整日志应能解析");
}

#[test]
fn check_accepts_batch_and_reports_counts() {
    let batch = sim::run_games(3, 1, 8).expect("批量模拟应能完成");
    let json = report::batch_to_json(&batch);
    let path = temp_path("check-ok.json");
    std::fs::write(&path, serde_json::to_string(&json).unwrap()).expect("应能写日志");
    let path_text = path.to_str().unwrap();

    let (code, stdout, stderr) = run(&["check", "--log", path_text, "--expect-games", "3"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stdout.contains("已校验 3 局（期望 3 局）"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("排名条目 24"), "stdout: {stdout}");

    // report 同样校验，校验摘要写 stderr，stdout 仍是纯 JSON。
    let (code, stdout, stderr) = run(&["report", "--log", path_text]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let _: Value = serde_json::from_str(&stdout).expect("report stdout 应是纯 JSON");
    assert!(stderr.contains("已校验 3 局"), "stderr: {stderr}");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn check_fails_on_expect_games_mismatch() {
    let batch = sim::run_games(2, 1, 8).expect("批量模拟应能完成");
    let json = report::batch_to_json(&batch);
    let path = temp_path("check-mismatch.json");
    std::fs::write(&path, serde_json::to_string(&json).unwrap()).expect("应能写日志");

    let (code, stdout, stderr) = run(&[
        "check",
        "--log",
        path.to_str().unwrap(),
        "--expect-games",
        "1000",
    ]);
    assert_ne!(code, 0, "局数不符应非零退出");
    assert!(stdout.is_empty(), "失败时 stdout 应为空");
    assert!(stderr.contains("局数不符"), "stderr: {stderr}");
    assert!(stderr.contains("2 局，期望 1000 局"), "stderr: {stderr}");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn check_fails_on_incomplete_ranking() {
    let mut batch = sim::run_games(4, 1, 8).expect("批量模拟应能完成");
    batch.games[2].ranking.truncate(7);
    let error = report::check_m0_batch(&batch, None).expect_err("排名不足应报错");
    assert!(error.contains("1/4 局异常"), "错误应给出计数：{error}");
    assert!(error.contains("第 2 局"), "错误应指出局序：{error}");

    let mut batch = sim::run_games(4, 1, 8).expect("批量模拟应能完成");
    batch.games[1].ranking.swap(0, 1);
    batch.games[1].ranking[1] = batch.games[1].ranking[0];
    let error = report::check_m0_batch(&batch, None).expect_err("座位重复应报错");
    assert!(error.contains("座位重复"), "错误应指出座位重复：{error}");

    let batch = sim::run_games(4, 1, 8).expect("批量模拟应能完成");
    let check = report::check_m0_batch(&batch, Some(4)).expect("完整批量应通过");
    assert_eq!(check.ranking_entries, 32);
}
