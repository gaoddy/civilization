#![cfg(feature = "train")]

use ai_core::{CardIndex, OBS_DIM};
use ai_model::board_value;
use ai_train::data_c0::{BoardPair, BoardSample};
use ai_train::model::{C0Config, C0Net, bce_with_logits, board_observation, extract_weights};
use ai_train::train_c0::{calibrate, split_pairs, train_c0};
use candle_core::{DType, Device, Tensor};
use candle_nn::{VarBuilder, VarMap};
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

fn minion_card(catalog: &Catalog) -> String {
    catalog
        .cards
        .values()
        .find(|card| card.in_shop_pool && card.attack.is_some())
        .expect("有随从卡")
        .id
        .clone()
}

fn hero_id(catalog: &Catalog) -> String {
    catalog.heroes.keys().next().expect("有英雄").clone()
}

fn board(catalog: &Catalog, card: &str, attack: i32, health: i32) -> BoardSample {
    let board = (0..3)
        .map(|index| {
            let mut minion = MinionInstance::new(catalog, index + 1, card, false);
            minion.stats = MinionStats::new(attack, health);
            minion
        })
        .collect();
    BoardSample {
        hero: hero_id(catalog),
        playstyles: 1,
        board,
    }
}

/// 强弱分明的合成数据：训练后留出集校准必须达标。
#[test]
fn separable_data_meets_calibration_targets() {
    let catalog = catalog();
    let index = CardIndex::build(&catalog);
    let card = minion_card(&catalog);
    let strong = board(&catalog, &card, 40, 40);
    let weak = board(&catalog, &card, 1, 1);
    let pairs: Vec<BoardPair> = (0..400)
        .map(|round| {
            if round % 2 == 0 {
                BoardPair {
                    a: strong.clone(),
                    b: weak.clone(),
                    a_wins: true,
                }
            } else {
                BoardPair {
                    a: weak.clone(),
                    b: strong.clone(),
                    a_wins: false,
                }
            }
        })
        .collect();

    let cfg = C0Config {
        steps: 400,
        batch: 64,
        lr: 0.001,
        hidden: 32,
        layers: 1,
        card_emb: 1,
        seed: 3,
        held_out: 0.2,
    };
    let weights = train_c0(&pairs, &index, &catalog, &cfg).expect("训练");
    for (name, data) in &weights.tensors {
        assert!(
            data.iter().all(|value| value.is_finite()),
            "{name} 含非有限值，训练发散"
        );
    }
    let (_, held) = split_pairs(&pairs, cfg.held_out);
    assert!(!held.is_empty());
    let calibration = calibrate(&weights, &cfg, &index, &catalog, &held);
    assert!(calibration.brier <= 0.25, "Brier 过高：{calibration:?}");
    assert!(calibration.ece <= 0.03, "ECE 过高：{calibration:?}");
}

/// 同一份数据、同一个种子必须得到同一份权重，否则产物不可复现。
#[test]
fn same_seed_trains_same_weights() {
    let catalog = catalog();
    let index = CardIndex::build(&catalog);
    let card = minion_card(&catalog);
    let pairs = vec![
        BoardPair {
            a: board(&catalog, &card, 30, 30),
            b: board(&catalog, &card, 2, 2),
            a_wins: true,
        },
        BoardPair {
            a: board(&catalog, &card, 2, 2),
            b: board(&catalog, &card, 30, 30),
            a_wins: false,
        },
    ];
    let cfg = C0Config {
        steps: 2,
        batch: 2,
        lr: 0.001,
        hidden: 8,
        layers: 1,
        card_emb: 1,
        seed: 7,
        held_out: 0.0,
    };
    let first = train_c0(&pairs, &index, &catalog, &cfg).expect("第一次训练");
    let second = train_c0(&pairs, &index, &catalog, &cfg).expect("第二次训练");
    assert_eq!(
        first.tensors, second.tensors,
        "同一随机种子的训练结果不一致"
    );
}

/// candle 训练侧与纯 Rust 推理侧必须给出同一个价值。
#[test]
fn candle_and_handwritten_forward_agree() {
    let catalog = catalog();
    let index = CardIndex::build(&catalog);
    let cfg = C0Config {
        steps: 0,
        batch: 1,
        lr: 0.0,
        hidden: 8,
        layers: 2,
        card_emb: 1,
        seed: 0,
        held_out: 0.0,
    };
    let device = Device::Cpu;
    let var_map = VarMap::new();
    let vb = VarBuilder::from_varmap(&var_map, DType::F32, &device);
    let net = C0Net::build(vb, &cfg).expect("建网");
    {
        let mut data = var_map.data().lock().expect("锁");
        for (table, (_, var)) in data.iter_mut().enumerate() {
            let dims = var.as_tensor().dims().to_vec();
            let size = var.as_tensor().elem_count();
            let values: Vec<f32> = (0..size)
                .map(|position| ((table * 37 + position * 13) % 17) as f32 * 0.01 - 0.08)
                .collect();
            var.set(&Tensor::from_vec(values, dims, &device).expect("张量"))
                .expect("赋值");
        }
    }
    let weights = extract_weights(&var_map).expect("导出");

    let sample = board(&catalog, &minion_card(&catalog), 7, 9);
    let obs = board_observation(&sample, &index, &catalog);
    let input = Tensor::from_vec(obs.to_vec(), (1, OBS_DIM), &device).expect("输入");
    let candle_value = net
        .forward(&input)
        .expect("candle 前向")
        .to_vec1::<f32>()
        .expect("取值")[0];
    let handwritten = board_value(&weights, &cfg.model_config(), &obs).expect("手写前向");
    assert!(
        (candle_value - handwritten).abs() < 1e-5,
        "candle {candle_value} 与手写 {handwritten} 不一致"
    );
}

/// 极端 logit 下损失必须有限且等于 |z|（y 与预测相反时）。
#[test]
fn bce_is_stable_on_extreme_logits() {
    let device = Device::Cpu;
    let logits = Tensor::from_vec(vec![1000.0f32, -1000.0], 2, &device).expect("logits");
    let targets = Tensor::from_vec(vec![0.0f32, 1.0], 2, &device).expect("targets");
    let loss = bce_with_logits(&logits, &targets)
        .expect("损失")
        .to_scalar::<f32>()
        .expect("取值");
    assert!(loss.is_finite());
    assert!((loss - 1000.0).abs() < 1.0, "loss = {loss}");
}
