//! 纯 Rust 神经策略：确定性零权重在真实开局状态上给出引擎可接受的意图。

use std::collections::BTreeMap;

use ai_core::OBS_DIM;
use ai_model::{ModelConfig, Weights};
use bot_ai::Policy;
use bot_ai::neural::NeuralPolicy;
use rule_engine::data::Catalog;
use rule_engine::engine::Engine;
use rule_engine::playstyles;
use rule_engine::rng::seed_from_u64;
use rule_engine::state::Seat;

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

fn model_config() -> ModelConfig {
    ModelConfig {
        obs_dim: OBS_DIM,
        card_emb: 16,
        hidden: 64,
        layers: 2,
    }
}

/// 全零权重：logits 全为 0，argmax 取第一个合法动作，正好验证「合法且可执行」。
fn zero_weights(catalog: &Catalog) -> Weights {
    let cfg = model_config();
    let index = ai_core::CardIndex::build(catalog);
    let hidden = cfg.hidden;
    let candidate_input = cfg.candidate_input();
    let mut tensors = BTreeMap::new();
    let mut put = |name: &str, len: usize| {
        tensors.insert(name.to_string(), vec![0.0f32; len]);
    };
    put("trunk.0.weight", hidden * cfg.obs_dim);
    put("trunk.0.bias", hidden);
    put("trunk.1.weight", hidden * hidden);
    put("trunk.1.bias", hidden);
    put("value.0.weight", hidden * hidden);
    put("value.0.bias", hidden);
    put("value.1.weight", hidden);
    put("value.1.bias", 1);
    put("cand.0.weight", hidden * candidate_input);
    put("cand.0.bias", hidden);
    put("cand.1.weight", hidden);
    put("cand.1.bias", 1);
    put("card.emb", index.card_count() * cfg.card_emb);
    put("hero.emb", index.hero_count() * cfg.card_emb);
    put("skill.emb", index.skill_count() * cfg.card_emb);
    Weights {
        tensors,
        card_count: index.card_count(),
        hero_count: index.hero_count(),
        skill_count: index.skill_count(),
    }
}

fn started_state(catalog: &Catalog) -> (Engine, rule_engine::state::GameState) {
    let engine = Engine::from_catalog(catalog.clone());
    let mut state = engine.initial_state(seed_from_u64(7), playstyles::draw(7));
    let heroes: BTreeMap<Seat, String> = catalog
        .heroes
        .keys()
        .take(8)
        .enumerate()
        .map(|(index, hero)| (index as Seat, hero.clone()))
        .collect();
    engine.start_game(&mut state, &heroes).expect("开局");
    (engine, state)
}

#[test]
fn neural_policy_returns_legal_action_on_fixture() {
    let catalog = catalog();
    let (engine, mut state) = started_state(&catalog);
    let policy = NeuralPolicy::from_weights(zero_weights(&catalog), &catalog).expect("策略");
    let intent = policy.plan(&state, 0, &engine, &[]).expect("应有动作");
    engine.apply(&mut state, 0, intent).expect("引擎接受");
}

#[test]
fn neural_policy_loads_weights_from_dir() {
    let catalog = catalog();
    let dir = std::env::temp_dir().join(format!("bot-ai-neural-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("建目录");
    zero_weights(&catalog)
        .save(&dir.join("weights.json"))
        .expect("写权重");
    let policy = NeuralPolicy::load(&dir, &catalog).expect("加载");
    let (engine, state) = started_state(&catalog);
    assert!(policy.plan(&state, 0, &engine, &[]).is_some());
    std::fs::remove_dir_all(&dir).expect("清理");
}

#[test]
fn policy_trait_wraps_heuristic() {
    let catalog = catalog();
    let (engine, state) = started_state(&catalog);
    let wrapped = bot_ai::HeuristicPolicy;
    assert_eq!(
        wrapped.plan(&state, 0, &engine, &[]),
        bot_ai::plan(&state, 0, &engine, &[])
    );
}
