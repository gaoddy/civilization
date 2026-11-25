#![cfg(feature = "train")]

use std::fs;
use std::path::PathBuf;

use ai_core::{ACTION_FEATURE_DIM, ActionFeatures, BOARD_SLOTS, OBS_DIM, Observation, Slot};
use ai_model::{ModelConfig, Weights};
use ai_train::data_c0::BoardSample;
use ai_train::eval::eval_c1;
use ai_train::ppo::TrainModel;
use rule_engine::data::Catalog;
use rule_engine::state::{MinionInstance, MinionStats};

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

/// 测试用短局：把回合上限压到 2，评测只关心确定性与可复现，不需要跑满 20 回合。
fn short_game_catalog() -> Catalog {
    let mut catalog = catalog();
    catalog.config.turn_limit = 2;
    catalog
}

fn minion_card(catalog: &Catalog) -> String {
    catalog
        .cards
        .values()
        .find(|card| card.in_shop_pool && card.attack.is_some())
        .expect("有随从卡")
        .id
        .clone()
}

fn heroes(catalog: &Catalog) -> Vec<String> {
    catalog.heroes.keys().cloned().collect()
}

fn board(catalog: &Catalog, hero: &str, attack: i32, health: i32) -> BoardSample {
    let card = minion_card(catalog);
    BoardSample {
        hero: hero.to_string(),
        playstyles: 1,
        board: (0..3)
            .map(|index| {
                let mut minion = MinionInstance::new(catalog, index + 1, &card, false);
                minion.stats = MinionStats::new(attack, health);
                minion
            })
            .collect(),
    }
}

fn model_config() -> ModelConfig {
    ModelConfig {
        obs_dim: OBS_DIM,
        card_emb: 8,
        hidden: 32,
        layers: 2,
    }
}

fn seeded_weights(catalog: &Catalog) -> Weights {
    let index = ai_core::CardIndex::build(catalog);
    let model = TrainModel::new(
        &model_config(),
        index.card_count(),
        index.hero_count(),
        index.skill_count(),
        5,
    )
    .expect("建模型");
    model.weights().expect("导出")
}

/// 同一权重、同一 seeds、同一分布必须给出同一份评测。
#[test]
fn c1_eval_is_deterministic_for_fixed_seeds() {
    let catalog = short_game_catalog();
    let heroes = heroes(&catalog);
    let distribution = vec![
        board(&catalog, &heroes[0], 8, 8),
        board(&catalog, &heroes[1], 20, 20),
    ];
    let weights = seeded_weights(&catalog);
    let seeds: Vec<u64> = (0..2).collect();
    let first = eval_c1(&catalog, &weights, &distribution, &seeds);
    let second = eval_c1(&catalog, &weights, &distribution, &seeds);
    assert_eq!(first, second, "同输入评测不稳定");
    assert_eq!(first.games, 2);
    assert!((0.0..=1.0).contains(&first.win_rate));
    assert!(first.mean_margin.is_finite());
}

/// 权重存盘后读回：评测结果与推理输出完全一致。
#[test]
fn checkpoint_round_trips() {
    let catalog = short_game_catalog();
    let heroes = heroes(&catalog);
    let distribution = vec![board(&catalog, &heroes[0], 10, 10)];
    let weights = seeded_weights(&catalog);
    let seeds: Vec<u64> = vec![0];
    let before = eval_c1(&catalog, &weights, &distribution, &seeds);

    let directory = unique_dir();
    fs::create_dir_all(&directory).expect("建目录");
    let path = directory.join("weights.json");
    weights.save(&path).expect("保存");
    let loaded = Weights::load(&path).expect("读取");
    assert_eq!(loaded.tensors, weights.tensors);
    let after = eval_c1(&catalog, &loaded, &distribution, &seeds);
    assert_eq!(before, after, "读回的权重评测不一致");

    let index = ai_core::CardIndex::build(&catalog);
    let model = model_config();
    let obs = zero_obs(0);
    let candidates = vec![ActionFeatures {
        kind: 0,
        card: 0,
        target_card: -1,
        skill: -1,
        hero: -1,
        values: [1.0; ACTION_FEATURE_DIM],
    }];
    let (first_logits, first_value) =
        ai_model::forward(&weights, &model, &obs, &candidates).expect("前向");
    let (second_logits, second_value) =
        ai_model::forward(&loaded, &model, &obs, &candidates).expect("前向");
    assert_eq!(first_logits, second_logits);
    assert_eq!(first_value, second_value);
    assert_eq!(index.card_count(), weights.card_count);

    fs::remove_dir_all(&directory).expect("清理");
}

fn zero_obs(hero: i32) -> Observation {
    Observation {
        seat: 0,
        turn: 0,
        phase: 1,
        hero,
        playstyles: 0,
        hp: 0,
        gold: 0,
        era: 0,
        shop: [Slot::default(); 6],
        hand: [Slot::default(); 10],
        board: [Slot::default(); BOARD_SLOTS],
        players: [Default::default(); 8],
        pending: 0,
    }
}

fn unique_dir() -> PathBuf {
    std::env::temp_dir().join(format!(
        "ai-train-c1-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("时钟")
            .as_nanos()
    ))
}
