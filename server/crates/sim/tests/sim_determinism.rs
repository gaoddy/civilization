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

#[test]
fn replay_rejects_event_hash_mismatch() {
    let mut log = sim::run_game(catalog(), 7).expect("单局应能完成");
    let expected = log.event_hash;
    log.event_hash ^= 1;

    let error = sim::replay(&log).expect_err("事件序列哈希不一致必须拒绝");

    assert!(
        error.to_string().contains("事件序列"),
        "错误应指出事件序列不一致：{error}"
    );
    assert_ne!(expected, log.event_hash);
}

#[test]
fn same_seed_reproduces_event_sequence_hash() {
    let first = sim::run_game(catalog(), 11).expect("第一次运行应成功");
    let second = sim::run_game(catalog(), 11).expect("第二次运行应成功");

    assert_eq!(first.final_hash, second.final_hash, "终局哈希应一致");
    assert_eq!(first.event_hash, second.event_hash, "事件序列哈希应一致");
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
///
/// 2026-11-18 第八次重定：卡牌参数重排批次 B（鸟、爬行、鱼、昆虫、头足、水螅
/// 六个玩法重排身材）。
///
/// 2026-11-18 第九次重定：批次 A/B/C 统一按玩法目标攻血比重新分配角色形状
/// （坦克取攻击下限/生命上限、输出取攻击上限/生命下限），13 个玩法平均攻血比
/// 从 0.43–0.47 拉开到 0.316–0.520，参战数值全面变化。
///
/// 2026-11-18 第十次重定：效果牌扩充到 100 张并加入定向玩法关联，共享池
/// 与商店抽取同步变化；战斗哈希不受影响，只重定模拟对局基线。
///
/// 2026-11-18 第十一次重定：AI 定向效果牌购买过滤，机器人不再购买与自身
/// 主导玩法不匹配的定向效果牌，购买决策与随机流消耗随之改变。
///
/// 2026-11-18 第十二次重定：卡牌参数重排最终调参（1–3 星为主、4–6 星少量），
/// 参战数值变化；随后冻结数值，本基线同时作为最终基线。
///
/// 2026-11-18 第十三次重定：AI 打出定向效果牌时遍历全部己方随从（原只试最左），
/// 带玩法筛选的效果牌不再长期卡在手牌，动作与随机流消耗改变。本次只改决策候选，
/// 不涉及卡牌数值。
///
/// 2026-11-19 合并 develop 的「无视嘲讽」替换（原 16 张最优攻击卡改用
/// `keyword-ignore-taunt`）：本批次 20 局哈希与第十三次基线逐位一致，说明这
/// 20 局未抽取到受影响的卡；选目标行为变化由 rule-engine 的战斗用例覆盖，
/// 基线保持原值。
///
/// 2026-11-19 修正 12 张多技能卡的表外第二技能（技能组合与部分卡的光环/繁衍
/// 变化）：本批次 20 局哈希逐位一致，说明这些卡未进入本批对局；技能效果
/// 变化由 rule-engine 用例覆盖，基线保持原值。
///
/// 2026-11-19 第十四次重定：协议层下发目标候选池。三张「复制手牌随从」卡
/// （达·芬奇 / 鹦鹉 / 印刷术）改为玩家选目标，引擎与机器人按 `zone:id` 候选
/// 决策，涉及这些卡的批次哈希随之改变；前两局未抽取到即保持不变。
///
/// 2026-11-20 第十五次重定：任何未消费待选都禁止手动 `EndRecruit`；模拟器 AI
/// 无可消费待选时按超时处理并交给 `finish_recruit` 清空，发现/进化不再跨回合
/// 保留。此前允许带发现待选结束招募并保留到下一回合，行为与随机流消耗改变。
///
/// 2026-11-20 第十六次重定：带目标战吼的随从改为两段式入场（放置先提交，目标
/// 由 `ChooseTarget` 另行提交，新增 `AbandonTarget` 撤销入场）。金色奖励、
/// 三连合并与入场观察者因此从「入场当次意图」推迟到「选定目标之后」；本批次
/// 20 局中第 1、11 局走到该路径，最终哈希改变，其余局保持原值。
///
/// 2026-11-20 第十七次重定：英雄卡从普通卡池拆出为独立英雄卡，原 32 张英雄随从
/// 保留在随从池且不再带 `is_hero` 标记，因此这些卡重新进入商店与随机池；本批次
/// 20 局哈希随之全部改变。
///
/// 2026-11-20 第十八次重定：玩家状态新增 `froze_this_turn`（解冻只在冻结的当回合
/// 可用），状态哈希为每名玩家多写一个布尔值；机器人不冻结，行为不变，但本批次
/// 20 局哈希全部改变。
///
/// 2026-11-20 第十九次重定：卡牌玩法归属从单值 `playstyle` 迁移为多值
/// `categories`，部分随从与工具卡按 `play:machine` 等额外归属进入更多玩法卡池，
/// 效果牌也按指派后的玩法进入商店；AI 的主导玩法改为逐条 `play:` 计数，
/// 定向效果牌购买与随机流消耗随之改变。本批次 20 局哈希全部改变。
///
/// 2026-11-21 第二十次重定：合并多值 `categories` 与英雄主动技能两条基线变更。
/// 模拟器 AI 使用英雄主动技能，按引擎下发的合法目标池选择目标，「全选友方」
/// 「筛选后单体」与「战斗期关键词」三种技能形状不再跳过；与多玩法卡池、
/// 商店抽取叠加后技能结算与随机流消耗再次改变，本批次 20 局哈希全部改变。
///
/// 2026-11-21 第二十一次重定：Token 合入卡牌实体并新增 `in_shop_pool`，
/// 三连奖励改为直接随机发放下一等级池内随从（`GrantMinion`，消耗
/// `Stream::Effects`），共享池、商店候选与发现候选统一按池内卡过滤；
/// AI 的对局路径与随机流消耗随之改变，本批次 20 局哈希全部改变。
///
/// 2026-11-21 第二十二次重定：移除 `is_scholar` 与学者机制，攻击力 0 的随从
/// 改为按普通随从成长（+1/+1），4 个学者专属技能替换为通用技能，同时移除
/// `match_text`。成长结果与技能组合变化涉及参战卡牌，本批次 20 局哈希全部改变。
///
/// 2026-11-21 第二十三次重定：商店抽取即从共享牌池扣除并占用，刷新归还未保留
/// 的份，随从实例新增 `pool_copies` 记录实际扣池份数并计入状态哈希；商店内容、
/// 发现候选与 AI 购买路径随之改变，本批次 20 局哈希全部改变。
///
/// 2026-11-22 第二十四次重定：郑和新增主动技能「七下西洋」，玩家状态新增
/// `seven_voyages` 并计入状态哈希；AI 多一个可用的主动技能，对局路径与
/// 随机流消耗随之改变，本批次 20 局哈希全部改变。
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
