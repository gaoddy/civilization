//! C2 训练：演员加载的报错口径、C1 混训配比与短预算冒烟。

#![cfg(feature = "train")]

use ai_core::CardIndex;
use ai_model::ModelConfig;
use ai_train::ppo::TrainModel;
use ai_train::train_c2::{
    C2Config, EnvMix, MixSampler, STEP_COST, decision_reward, load_actor, train_c2,
};
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

fn model() -> ModelConfig {
    ModelConfig {
        obs_dim: ai_core::OBS_DIM,
        card_emb: 16,
        hidden: 64,
        layers: 2,
    }
}

#[test]
fn missing_tensor_fails_with_clear_error() {
    let catalog = catalog();
    let index = CardIndex::build(&catalog);
    let mut train = TrainModel::new(
        &model(),
        index.card_count(),
        index.hero_count(),
        index.skill_count(),
        0,
    )
    .expect("模型");
    let mut weights = train.weights().expect("权重");
    assert!(weights.tensors.remove("trunk.0.weight").is_some());

    let error = load_actor(&mut train, &weights).expect_err("缺张量必须报错");
    assert!(
        error.to_string().contains("trunk.0.weight"),
        "报错要指出张量名：{error}"
    );
}

/// 零成本且可逆的动作（站位、冻结、重排）要付时间成本，`End` 免费。
///
/// 没有这一步成本时，策略会在一回合里把 40 次决策全花在站位上调来调去：实测 C2
/// 快照的动作分布里 `Rearrange` 占八成以上，训练数据被无效动作淹没。
#[test]
fn step_cost_is_charged_for_every_action_but_end() {
    assert_eq!(decision_reward(0.0, true), 0.0, "End 不应付费");
    assert!(
        (decision_reward(0.0, false) + STEP_COST).abs() < 1e-7,
        "非 End 决策应扣 {STEP_COST}"
    );
    assert!(
        (decision_reward(0.5, false) - (0.5 - STEP_COST)).abs() < 1e-7,
        "名次奖励与时间成本应叠加"
    );
}

#[test]
fn c1_mix_ratio_is_honored() {
    let mut sampler = MixSampler::new(0.3);
    let mut c1 = 0usize;
    const ROUNDS: usize = 1000;
    for _ in 0..ROUNDS {
        if sampler.pick() == EnvMix::C1 {
            c1 += 1;
        }
    }
    let ratio = c1 as f32 / ROUNDS as f32;
    assert!((ratio - 0.3).abs() <= 0.02, "C1 占比应为 0.3，实际 {ratio}");
}

#[test]
fn train_c2_runs_a_short_budget_and_returns_weights() {
    let catalog = tiny_catalog();
    let cfg = C2Config {
        episodes: 2,
        batch_episodes: 2,
        c1_mix: 0.0,
        league_size: 2,
        snapshot_every: 1,
        seeds: 0..64,
        seed: 3,
        ..C2Config::default()
    };
    let output = train_c2(&catalog, &[], None, None, &cfg).expect("C2 训练");
    assert_eq!(output.episodes, 2);
    assert!(!output.weights.tensors.is_empty(), "产出权重");
    assert!(
        (1.0..=8.0).contains(&output.mean_placement),
        "名次均值应在 1..=8：{}",
        output.mean_placement
    );
}

/// 长时间训练要在训练中把权重落盘：崩了还能从最近的检查点接着评测。
#[test]
fn training_writes_periodic_checkpoints() {
    let catalog = tiny_catalog();
    let dir = std::env::temp_dir().join(format!("c2-checkpoints-{}", std::process::id()));
    let cfg = C2Config {
        episodes: 2,
        batch_episodes: 1,
        c1_mix: 0.0,
        league_size: 2,
        snapshot_every: 1,
        seeds: 0..64,
        seed: 3,
        checkpoint_dir: Some(dir.clone()),
        checkpoint_every: 1,
        ..C2Config::default()
    };

    train_c2(&catalog, &[], None, None, &cfg).expect("C2 训练");

    let mut steps: Vec<String> = std::fs::read_dir(&dir)
        .expect("检查点目录应存在")
        .map(|entry| {
            entry
                .expect("目录项")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    steps.sort();
    assert_eq!(steps.len(), 2, "每批次一个检查点，实际 {steps:?}");
    for step in &steps {
        assert!(
            dir.join(step).join("weights.json").is_file(),
            "检查点 {step} 应有 weights.json"
        );
    }

    std::fs::remove_dir_all(&dir).expect("清理");
}
