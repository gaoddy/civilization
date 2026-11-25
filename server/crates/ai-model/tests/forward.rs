use std::path::PathBuf;

use ai_core::observe::Opponent;
use ai_core::{ACTION_FEATURE_DIM, Action, ActionFeatures, CardIndex, OBS_DIM, Observation, Slot};
use ai_model::{ModelConfig, Weights, forward, forward_hero};
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

fn zero_obs() -> Observation {
    Observation {
        seat: 0,
        turn: 0,
        phase: 0,
        hero: -1,
        playstyles: 0,
        hp: 0,
        gold: 0,
        era: 0,
        shop: [Slot::default(); 6],
        hand: [Slot::default(); 10],
        board: [Slot::default(); 7],
        players: [Opponent::default(); 8],
        pending: 0,
    }
}

/// 构造一份只有骨架张量的权重：卡/英雄/技能表按给定数量置零。
fn zeros(cfg: &ModelConfig, cards: usize, heroes: usize, skills: usize) -> Weights {
    let mut weights = Weights {
        tensors: Default::default(),
        card_count: cards,
        hero_count: heroes,
        skill_count: skills,
    };
    weights.set("card.emb", vec![0.0; cards * cfg.card_emb]);
    weights.set("hero.emb", vec![0.0; heroes * cfg.card_emb]);
    weights.set("skill.emb", vec![0.0; skills * cfg.card_emb]);
    let mut input = cfg.obs_dim;
    for layer in 0..cfg.layers {
        weights.set(
            &format!("trunk.{layer}.weight"),
            vec![0.0; cfg.hidden * input],
        );
        weights.set(&format!("trunk.{layer}.bias"), vec![0.0; cfg.hidden]);
        input = cfg.hidden;
    }
    let cand_input = 4 * cfg.card_emb + ACTION_FEATURE_DIM + cfg.hidden;
    weights.set("cand.0.weight", vec![0.0; cfg.hidden * cand_input]);
    weights.set("cand.0.bias", vec![0.0; cfg.hidden]);
    weights.set("cand.1.weight", vec![0.0; cfg.hidden]);
    weights.set("cand.1.bias", vec![0.0]);
    weights.set("value.0.weight", vec![0.0; cfg.hidden * cfg.hidden]);
    weights.set("value.0.bias", vec![0.0; cfg.hidden]);
    weights.set("value.1.weight", vec![0.0; cfg.hidden]);
    weights.set("value.1.bias", vec![0.0]);
    let hero_inner = cfg.card_emb + cfg.hidden;
    weights.set("hero_head.0.weight", vec![0.0; cfg.hidden * hero_inner]);
    weights.set("hero_head.0.bias", vec![0.0; cfg.hidden]);
    weights.set("hero_head.1.weight", vec![0.0; cfg.hidden]);
    weights.set("hero_head.1.bias", vec![0.0]);
    weights
}

#[test]
fn known_weights_produce_known_logits() {
    let cfg = ModelConfig {
        obs_dim: OBS_DIM,
        card_emb: 1,
        hidden: 2,
        layers: 1,
    };
    let mut weights = zeros(&cfg, 4, 2, 2);
    // 主干权重全零，观测不影响主干：输出 relu([1, -2]) = [1, 0]。
    weights.set("trunk.0.bias", vec![1.0, -2.0]);
    // 候选头输入：[实体, 目标, 技能, 英雄, 数值 24, 主干 2]，共 30 维。
    let cand_input = 4 + ACTION_FEATURE_DIM + 2;
    let mut cand_w = vec![0.0; 2 * cand_input];
    cand_w[4 + Action::Reroll.kind_index() as usize] = 1.0;
    cand_w[cand_input + 4 + Action::End.kind_index() as usize] = 1.0;
    weights.set("cand.0.weight", cand_w);
    weights.set("cand.1.weight", vec![2.0, 3.0]);
    // 价值头：主干 [1, 0] → 4。
    weights.set("value.0.weight", vec![1.0, 0.0, 0.0, 1.0]);
    weights.set("value.1.weight", vec![4.0, 5.0]);

    let features = vec![
        ActionFeatures::of(&Action::Reroll),
        ActionFeatures::of(&Action::End),
    ];
    let (logits, value) = forward(&weights, &cfg, &zero_obs(), &features).expect("前向");
    assert_eq!(logits, vec![2.0, 3.0]);
    assert_eq!(value, 4.0);
}

#[test]
fn empty_candidates_still_produce_value() {
    let cfg = ModelConfig {
        obs_dim: OBS_DIM,
        card_emb: 1,
        hidden: 2,
        layers: 1,
    };
    let mut weights = zeros(&cfg, 4, 2, 2);
    weights.set("trunk.0.bias", vec![1.0, 2.0]);
    weights.set("value.0.weight", vec![1.0, 0.0, 0.0, 1.0]);
    weights.set("value.1.weight", vec![1.0, 1.0]);
    let (logits, value) = forward(&weights, &cfg, &zero_obs(), &[]).expect("前向");
    assert!(logits.is_empty());
    assert_eq!(value, 3.0);
}

#[test]
fn out_of_range_embedding_index_is_error() {
    let cfg = ModelConfig {
        obs_dim: OBS_DIM,
        card_emb: 1,
        hidden: 2,
        layers: 1,
    };
    let weights = zeros(&cfg, 4, 2, 2);
    let mut features = ActionFeatures::of(&Action::Reroll);
    features.card = 99;
    assert!(forward(&weights, &cfg, &zero_obs(), &[features]).is_err());
}

#[test]
fn weights_round_trip_and_reject_catalog_mismatch() {
    let cfg = ModelConfig {
        obs_dim: OBS_DIM,
        card_emb: 2,
        hidden: 4,
        layers: 2,
    };
    let mut weights = zeros(&cfg, 8, 3, 5);
    weights.set("cand.0.bias", vec![0.1, 0.2, 0.3, 0.4]);

    let path: PathBuf = std::env::temp_dir().join(format!("ai-model-{}", std::process::id()));
    let manifest = path.join("weights.json");
    weights.save(&manifest).expect("保存");
    let loaded = Weights::load(&manifest).expect("加载");
    assert_eq!(loaded, weights);

    let index = CardIndex::build(&catalog());
    assert!(loaded.validate(&index).is_err(), "卡表数量不同必须拒绝");

    std::fs::remove_dir_all(&path).expect("清理");
}

#[test]
fn hero_head_scores_candidates_independently() {
    let cfg = ModelConfig {
        obs_dim: OBS_DIM,
        card_emb: 1,
        hidden: 2,
        layers: 1,
    };
    let mut weights = zeros(&cfg, 4, 2, 2);
    weights.set("trunk.0.bias", vec![1.0, 0.0]);
    // 英雄嵌入：0 → 1.0，1 → 2.0。
    weights.set("hero.emb", vec![1.0, 2.0]);
    // 英雄头输入 [嵌入 1, 主干 2] = 3 维。
    weights.set("hero_head.0.weight", vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    weights.set("hero_head.1.weight", vec![1.0, 1.0]);
    let logits = forward_hero(&weights, &cfg, &zero_obs(), &[0, 1]).expect("英雄头");
    assert_eq!(logits, vec![2.0, 4.0]);
}

/// 选英雄走独立的 `hero_head`（设计 D2）：只改英雄头的参数，PickHero 候选的
/// logit 必须跟着变，否则说明选英雄仍然走的是通用候选头。
#[test]
fn pick_hero_candidates_are_scored_by_the_hero_head() {
    let cfg = ModelConfig {
        obs_dim: OBS_DIM,
        card_emb: 1,
        hidden: 2,
        layers: 1,
    };
    let obs = zero_obs();
    let candidates: Vec<ActionFeatures> = (1..=4)
        .map(|hero: i32| {
            let mut features = ActionFeatures::of(&Action::PickHero {
                hero: format!("hero-{hero}"),
            });
            features.hero = hero;
            features
        })
        .collect();

    let weights = zeros(&cfg, 8, 8, 2);
    let (base, _) = forward(&weights, &cfg, &obs, &candidates).expect("前向");

    let mut changed = weights.clone();
    changed.set("hero_head.1.bias", vec![5.0]);
    let (after, _) = forward(&changed, &cfg, &obs, &candidates).expect("前向");

    assert_ne!(
        base, after,
        "PickHero 的 logit 应由 hero_head 决定，改 hero_head 必须改变结果"
    );
}
