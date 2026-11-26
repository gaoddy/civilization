//! 固定种子评测：同一权重与种子两次结果一致，报告 95% 置信区间、配对名次优势，
//! 且默认以启发式机器人为锚点。

#![cfg(feature = "train")]

use ai_core::CardIndex;
use ai_model::{ModelConfig, Weights};
use ai_train::eval_full::{Anchor, EVAL_SEED_BASE, EvalConfig, eval_full};
use ai_train::league::{League, PolicyId};
use ai_train::ppo::TrainModel;
use rule_engine::data::Catalog;

fn catalog() -> Catalog {
    Catalog::from_json(
        include_str!("../../../data/v0.1/cards.json"),
        include_str!("../../../data/v0.1/heroes.json"),
        include_str!("../../../data/v0.1/skills.json"),
        include_str!("../../../data/v0.1/config.json"),
        include_str!("../../../data/v0.1/taxonomy.json"),
    )
    .expect("卡表")
}

fn tiny_catalog() -> Catalog {
    let mut catalog = catalog();
    catalog.config.turn_limit = 2;
    catalog
}

fn weights_for(catalog: &Catalog, seed: u64) -> Weights {
    let index = CardIndex::build(catalog);
    let model = ModelConfig {
        obs_dim: ai_core::OBS_DIM,
        card_emb: 16,
        hidden: 64,
        layers: 2,
    };
    TrainModel::new(
        &model,
        index.card_count(),
        index.hero_count(),
        index.skill_count(),
        seed,
    )
    .expect("模型")
    .weights()
    .expect("权重")
}

#[test]
fn eval_is_deterministic_and_reports_confidence_interval() {
    let catalog = tiny_catalog();
    let learner = weights_for(&catalog, 1);
    let anchor = weights_for(&catalog, 2);
    let mut league = League::new(0, 1);
    league.set_anchor(anchor);
    let cfg = EvalConfig {
        seeds: (0..8).collect(),
        learner_seats: 4,
        anchors: vec![Anchor::Weights(PolicyId::Anchor)],
        max_decisions_per_turn: 16,
        max_steps_per_game: 200_000,
    };

    let first = eval_full(&catalog, &learner, &league, &cfg).expect("第一次评测");
    let second = eval_full(&catalog, &learner, &league, &cfg).expect("第二次评测");

    assert_eq!(first, second, "同权重同种子必须逐字段一致");
    assert_eq!(first.games, 8, "每个种子一局");
    assert!(first.mean_placement >= 1.0 && first.mean_placement <= 8.0);
    assert!(first.ci95 > 0.0, "名次有波动时置信区间宽度应大于 0");
}

/// 默认锚点是启发式机器人：整局跑完不中断，优势按局配对计算。
#[test]
fn heuristic_anchor_plays_whole_games_and_reports_paired_advantage() {
    let catalog = tiny_catalog();
    let learner = weights_for(&catalog, 3);
    let league = League::new(0, 0);
    let cfg = EvalConfig {
        seeds: (EVAL_SEED_BASE..EVAL_SEED_BASE + 4).collect(),
        learner_seats: 8,
        anchors: vec![Anchor::Heuristic],
        max_decisions_per_turn: 16,
        max_steps_per_game: 200_000,
    };

    let first = eval_full(&catalog, &learner, &league, &cfg).expect("启发式锚点评测");
    let second = eval_full(&catalog, &learner, &league, &cfg).expect("第二次评测");

    assert_eq!(
        first, second,
        "启发式锚点不引入额外随机，两次必须逐字段一致"
    );
    assert_eq!(first.games, 4, "每个种子一局");
    assert!(
        first.anchor_mean >= 1.0 && first.anchor_mean <= 8.0,
        "锚点平均名次应在 1..=8：{}",
        first.anchor_mean
    );
    assert!(
        (first.advantage - (first.anchor_mean - first.mean_placement)).abs() < 1e-4,
        "优势应等于锚点均名次减学习者均名次：{} vs {} - {}",
        first.advantage,
        first.anchor_mean,
        first.mean_placement
    );
    assert_eq!(
        first.anchor_wins + first.anchor_losses,
        first.games,
        "8 人局名次互不相同，学习者每局要么优于要么劣于锚点均值"
    );
    assert!(first.advantage_ci95 >= 0.0);
}

/// 单局被步数上限截断时跳过该局并计数；整批都截断则报错，而不是返回一个空报告。
#[test]
fn truncated_games_are_skipped_and_counted() {
    let catalog = tiny_catalog();
    let learner = weights_for(&catalog, 5);
    let league = League::new(0, 0);
    let cfg = EvalConfig {
        seeds: (EVAL_SEED_BASE..EVAL_SEED_BASE + 2).collect(),
        learner_seats: 8,
        anchors: vec![Anchor::Heuristic],
        max_decisions_per_turn: 16,
        max_steps_per_game: 4,
    };

    let error = eval_full(&catalog, &learner, &league, &cfg).expect_err("全部截断应报错");
    assert!(
        format!("{error}").contains("截断"),
        "错误信息应说明截断：{error}"
    );
}

/// 评测报告统计学习者座位的决策数与站位决策数，用于观察候选数量偏置。
#[test]
fn report_counts_learner_rearrange_decisions() {
    let catalog = tiny_catalog();
    let learner = weights_for(&catalog, 7);
    let league = League::new(0, 0);
    let cfg = EvalConfig {
        seeds: (0..2).collect(),
        learner_seats: 1,
        anchors: vec![Anchor::Heuristic],
        max_decisions_per_turn: 16,
        max_steps_per_game: 200_000,
    };

    let report = eval_full(&catalog, &learner, &league, &cfg).expect("评测");
    assert!(report.decisions > 0, "学习者应有决策");
    assert!(
        report.rearrange_decisions <= report.decisions,
        "站位决策不能多于总决策"
    );
}
