//! 100 次重放的确定性校验。

mod common;

use std::collections::BTreeSet;

use common::{catalog, log_hash, run};
use rule_engine::data::Playstyle;
use rule_engine::engine::Engine;
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

#[test]
fn same_seed_uses_same_five_playstyles() {
    for seed in [0u64, 1, 7, 42, u64::MAX] {
        let first = rule_engine::playstyles::draw(seed);
        let second = rule_engine::playstyles::draw(seed);
        assert_eq!(first, second, "种子 {seed} 的 5 个玩法应固定");
        assert_eq!(first.len(), 5);
        let set: BTreeSet<Playstyle> = first.into_iter().collect();
        assert_eq!(set.len(), 5, "5 个玩法互不重复");
        assert!(!set.contains(&Playstyle::Neutral));

        let (_, state) = Engine::new(catalog(), sim::seed_from_u64(seed), first);
        assert_eq!(state.enabled_playstyles, set);
    }

    // 批量第 i 局种子为 `derive_seed(seed, i)`；重放按日志种子重新推导，
    // 两条路径必须得到同一组玩法。
    let batch = sim::run_games(2, 1, 8).expect("批量模拟应能完成");
    for (index, log) in batch.games.iter().enumerate() {
        let game_seed = sim::derive_seed(1, index);
        assert_eq!(log.seed, game_seed);
        let expected: BTreeSet<Playstyle> = rule_engine::playstyles::draw(game_seed)
            .into_iter()
            .collect();
        let (_, batch_state) = Engine::new(
            catalog(),
            sim::seed_from_u64(game_seed),
            rule_engine::playstyles::draw(game_seed),
        );
        let (_, replay_state) = Engine::new(
            catalog(),
            sim::seed_from_u64(log.seed),
            rule_engine::playstyles::draw(log.seed),
        );
        assert_eq!(batch_state.enabled_playstyles, expected);
        assert_eq!(replay_state.enabled_playstyles, expected);
    }
}

#[test]
fn replay_keeps_kill_attribution_and_decompose_consistent() {
    // 种子 7 的玩法含真菌与细菌，卡池会带出分解随从；击杀归属与分解结果都写入
    // 状态哈希，重放两次必须与原局逐位一致。
    let drawing = rule_engine::playstyles::draw(7);
    assert!(drawing.contains(&Playstyle::Fungus), "种子 7 应启用真菌");
    assert!(drawing.contains(&Playstyle::Bacteria), "种子 7 应启用细菌");

    let log = sim::run_game(catalog(), 7).expect("单局应能完成");
    let first = sim::replay(&log).expect("重放应成功");
    let second = sim::replay(&log).expect("再次重放应成功");
    assert_eq!(first, log.final_hash, "重放哈希应与原局一致");
    assert_eq!(second, first, "重放本身应确定");
}

/// 批次最终哈希必须与固定基线一致。
///
/// 本基线因卡片分类与玩法体系落地而重定，之后又因「目标被主伤害击杀后仍反击」
/// 的规则调整再次重定：反击进入更多交换，战斗结果与随机流消耗随之变化。
/// 2026-11-18 玩法扩充（13 个流派共 106 张新卡、共享池随之变化）后第三次重定：
/// `state_hash` 含共享池，卡池变化必然改变每局哈希；同时修复了目标待选在合法池
/// 不含最左侧随从时无法消费的 AI 缺陷（`ai::choose_pending` 沿棋盘后移），
/// 机器人行为随之变化。基线冻结的仍是行为：任何算术、遍历顺序或随机流消耗
/// 变化都会使哈希改变。
///
/// 2026-11-18 玩法扩充收尾后第四次重定：`MinionInstance` 新增延迟战吼结算
/// 标记 `delayed_battlecry_used` 并计入 `state_hash`，修复同一实例每场战斗
/// 重复结算延迟战吼的缺陷；仅哈希构成变化，模拟流程不变。
///
/// 2026-11-18 卡牌参数重排第一步后第五次重定：三张 2 星双技能卡合并技能、
/// 六张无技能随从补技能，参战卡牌效果变化，涉及这些卡的批次哈希随之改变。
///
/// 2026-11-18 第六次重定：模拟器 AI 增加「打出效果牌」步骤（先无目标、被拒后
/// 改试己方最左随从），修复效果牌买入手牌后永不打出、挤占手牌空位的问题。
///
/// 2026-11-18 第七次重定：卡牌参数重排批次 A（哺乳纲与中立非学者随从按玩法
/// 定位重排身材），参战数值变化，对局哈希随之改变。
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
