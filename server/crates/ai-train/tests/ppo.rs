#![cfg(feature = "train")]

use ai_core::{ACTION_FEATURE_DIM, ActionFeatures, BOARD_SLOTS, OBS_DIM, Observation, Slot};
use ai_model::ModelConfig;
use ai_train::ppo::{
    Batch, PpoConfig, TrainModel, log_prob, masked_probs, sample_action, train_step,
};
use ai_train::random_policy::RandomPolicy;

fn model_config() -> ModelConfig {
    ModelConfig {
        obs_dim: OBS_DIM,
        card_emb: 4,
        hidden: 16,
        layers: 1,
    }
}

fn zero_obs() -> Observation {
    Observation {
        seat: 0,
        turn: 0,
        phase: 1,
        hero: 0,
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

fn feature(kind: u8) -> ActionFeatures {
    let mut values = [0.0; ACTION_FEATURE_DIM];
    values[usize::from(kind)] = 1.0;
    ActionFeatures {
        kind,
        card: -1,
        target_card: -1,
        skill: -1,
        hero: -1,
        values,
    }
}

/// 两臂老虎机：动作 1 拿 1 分，训练后策略必须收敛到动作 1。
#[test]
fn ppo_learns_two_armed_bandit() {
    let cfg = model_config();
    let mut model = TrainModel::new(&cfg, 2, 1, 1, 7).expect("建模型");
    let obs = zero_obs();
    let candidates = vec![feature(0), feature(1)];
    let ppo = PpoConfig {
        gamma: 1.0,
        lambda: 1.0,
        clip: 0.2,
        lr: 0.01,
        entropy: 0.0,
        epochs: 4,
    };
    let mut rng = RandomPolicy::new(3);
    for step in 0..200 {
        let (logits, value) = model.logits(&obs, &candidates).expect("前向");
        let mask = vec![true, true];
        let probs = masked_probs(&logits, &mask);
        let action = sample_action(&probs, &mut rng);
        let batch = Batch {
            obs: vec![obs],
            actions: vec![action],
            masks: vec![mask.clone()],
            logps: vec![log_prob(&logits, &mask, action)],
            values: vec![value],
            rewards: vec![if action == 1 { 1.0 } else { 0.0 }],
            dones: vec![true],
            candidates: vec![candidates.clone()],
        };
        let metrics = train_step(&mut model, &batch, &ppo).expect("训练步");
        assert!(
            metrics.policy_loss.is_finite() && metrics.value_loss.is_finite(),
            "第 {step} 步损失非有限：{metrics:?}"
        );
    }
    let (logits, _) = model.logits(&obs, &candidates).expect("前向");
    let probs = masked_probs(&logits, &[true, true]);
    assert!(probs[1] > 0.9, "动作 1 的概率只有 {}", probs[1]);
}

/// 被掩码的动作概率严格为 0，其余按 softmax 归一。
#[test]
fn masked_actions_get_zero_probability() {
    let logits = vec![0.5, 1.5, -0.5];
    let probs = masked_probs(&logits, &[true, false, true]);
    assert_eq!(probs[1], 0.0);
    assert!((probs[0] + probs[2] - 1.0).abs() < 1e-6);
    assert!(probs[0] > 0.0 && probs[2] > 0.0);

    let none_valid = masked_probs(&logits, &[false, false, false]);
    assert!(none_valid.iter().all(|probability| *probability == 0.0));
    assert_eq!(sample_action(&none_valid, &mut RandomPolicy::new(1)), 0);
}

/// 训练侧网络与纯 Rust 推理侧必须给出同一组 logit 与价值，权重才能直接迁移。
#[test]
fn training_forward_matches_inference_forward() {
    let cfg = model_config();
    let model = TrainModel::new(&cfg, 5, 4, 3, 11).expect("建模型");
    let weights = model.weights().expect("导出权重");
    let mut obs = zero_obs();
    obs.hero = 3;
    let candidates = vec![
        ActionFeatures {
            kind: 0,
            card: 1,
            target_card: -1,
            skill: 1,
            hero: -1,
            values: {
                let mut values = [0.0; ACTION_FEATURE_DIM];
                values[0] = 1.0;
                values
            },
        },
        ActionFeatures {
            kind: 5,
            card: 4,
            target_card: 2,
            skill: -1,
            hero: 2,
            values: {
                let mut values = [0.0; ACTION_FEATURE_DIM];
                values[5] = 1.0;
                values
            },
        },
    ];
    let (trained_logits, trained_value) = model.logits(&obs, &candidates).expect("训练侧前向");
    let (inference_logits, inference_value) =
        ai_model::forward(&weights, &cfg, &obs, &candidates).expect("推理侧前向");
    assert_eq!(trained_logits.len(), inference_logits.len());
    for (trained, inference) in trained_logits.iter().zip(&inference_logits) {
        assert!(
            (trained - inference).abs() < 1e-5,
            "logit 不一致：训练侧 {trained}，推理侧 {inference}"
        );
    }
    assert!(
        (trained_value - inference_value).abs() < 1e-5,
        "价值不一致：训练侧 {trained_value}，推理侧 {inference_value}"
    );
}
