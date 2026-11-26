use std::collections::BTreeMap;

use ai_core::features::{
    ActionFeatures, FEATURE_ENTITY_ATTACK, FEATURE_ENTITY_HEALTH, FEATURE_HAS_TARGET,
};
use ai_core::{Action, CardIndex};
use rule_engine::data::Catalog;
use rule_engine::engine::Engine;
use rule_engine::playstyles;
use rule_engine::state::{MinionInstance, PendingChoice, Seat};

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

fn heroes(catalog: &Catalog) -> BTreeMap<Seat, String> {
    catalog
        .heroes
        .values()
        .take(8)
        .enumerate()
        .map(|(index, hero)| (index as Seat, hero.id.clone()))
        .collect()
}

/// 每个动作类别都必须点亮 one-hot 的对应位，且只点亮一位。
#[test]
fn every_action_kind_has_a_single_hot_slot() {
    let actions = [
        Action::Buy { slot: 0 },
        Action::Sell { minion: 0 },
        Action::Reroll,
        Action::Upgrade,
        Action::End,
        Action::PlayCard {
            hand: 0,
            position: None,
            target: None,
        },
        Action::Rearrange { from: 0, to: 1 },
        Action::Freeze { slot: None },
        Action::Unfreeze { slot: None },
        Action::HeroPower {
            skill: "x".to_string(),
            target: None,
        },
        Action::MinionAbility {
            minion: 0,
            skill: "x".to_string(),
            target: None,
        },
        Action::ChooseDiscover { choice: 0 },
        Action::ChooseTarget {
            target: rule_engine::effects::TargetRef {
                zone: rule_engine::effects::Zone::Board,
                id: 0,
            },
        },
        Action::AbandonTarget,
        Action::PickHero {
            hero: "x".to_string(),
        },
    ];
    for action in &actions {
        let features = ActionFeatures::of(action);
        let kind = usize::from(action.kind_index());
        assert_eq!(features.values[kind], 1.0, "{action:?}");
        let total: f32 = features.values[..ai_core::features::ACTION_KIND_SLOTS]
            .iter()
            .sum();
        assert_eq!(total, 1.0, "{action:?}");
    }
}

/// 卖出与目标类动作需要从状态解析实体与目标。
#[test]
fn state_aware_features_resolve_entities() {
    let catalog = catalog();
    let engine = Engine::from_catalog(catalog.clone());
    let mut state = engine.initial_state([0; 32], playstyles::draw(0));
    engine
        .start_game(&mut state, &heroes(&catalog))
        .expect("开局");
    let index = CardIndex::build(&catalog);
    let card_id = catalog
        .cards
        .values()
        .find(|card| card.in_shop_pool && card.attack.is_some())
        .expect("有随从卡")
        .id
        .clone();
    let minion = MinionInstance::new(&catalog, 4242, &card_id, false);
    let (attack, health) = (minion.attack(), minion.health());
    state.players[0].board.push(minion);

    let sell = ActionFeatures::from_action(&Action::Sell { minion: 4242 }, &state, 0, &index);
    assert_eq!(sell.card, index.card(&card_id));
    assert_eq!(sell.values[FEATURE_ENTITY_ATTACK], attack as f32 / 50.0);
    assert_eq!(sell.values[FEATURE_ENTITY_HEALTH], health as f32 / 50.0);

    let target = rule_engine::effects::TargetRef {
        zone: rule_engine::effects::Zone::Board,
        id: 4242,
    };
    let choose = ActionFeatures::from_action(&Action::ChooseTarget { target }, &state, 0, &index);
    assert_eq!(choose.target_card, index.card(&card_id));
    assert_eq!(choose.values[FEATURE_HAS_TARGET], 1.0);
}

/// 发现候选各自对应一张牌：候选特征必须带上该牌的卡索引，否则三个选项在
/// 特征上完全一样，策略只能学到一个位置偏好。
#[test]
fn discover_candidates_carry_their_card_index() {
    let catalog = catalog();
    let engine = Engine::from_catalog(catalog.clone());
    let mut state = engine.initial_state([0; 32], playstyles::draw(0));
    engine
        .start_game(&mut state, &heroes(&catalog))
        .expect("开局");
    let index = CardIndex::build(&catalog);
    let candidates: Vec<String> = catalog
        .cards
        .values()
        .filter(|card| card.in_shop_pool)
        .take(3)
        .map(|card| card.id.clone())
        .collect();
    assert_eq!(candidates.len(), 3, "卡表应有足够候选");
    state.pending.push_back(PendingChoice::Discover {
        seat: 0,
        candidates: candidates.clone(),
        source: 0,
    });

    for (choice, card_id) in candidates.iter().enumerate() {
        let features = ActionFeatures::from_action(
            &Action::ChooseDiscover {
                choice: choice as u8,
            },
            &state,
            0,
            &index,
        );
        assert_eq!(
            features.card,
            index.card(card_id),
            "候选 {choice}（{card_id}）应带自己的卡索引"
        );
    }
}

/// 位置与选项特征都按各自的合法上界归一化：最大合法取值恒为 1.0，
/// 发现/进化的候选数不同也不能让特征尺度漂移。
#[test]
fn position_and_choice_features_reach_one_at_the_last_legal_value() {
    use ai_core::features::{BOARD_POSITION_DIVISOR, FEATURE_CHOICE, FEATURE_POSITION};

    let last_position = Action::PlayCard {
        hand: 0,
        position: Some(BOARD_POSITION_DIVISOR as u8),
        target: None,
    };
    let features = ActionFeatures::of(&last_position);
    assert_eq!(
        features.values[FEATURE_POSITION], 1.0,
        "最后一个合法插入点应为 1.0"
    );

    let catalog = catalog();
    let engine = Engine::from_catalog(catalog.clone());
    let mut state = engine.initial_state([0; 32], playstyles::draw(0));
    engine
        .start_game(&mut state, &heroes(&catalog))
        .expect("开局");
    let index = CardIndex::build(&catalog);

    for count in [3usize, 4] {
        let candidates: Vec<String> = catalog
            .cards
            .values()
            .filter(|card| card.in_shop_pool)
            .take(count)
            .map(|card| card.id.clone())
            .collect();
        state.pending.clear();
        state.pending.push_back(PendingChoice::Discover {
            seat: 0,
            candidates,
            source: count as u64,
        });
        let last = ActionFeatures::from_action(
            &Action::ChooseDiscover {
                choice: (count - 1) as u8,
            },
            &state,
            0,
            &index,
        );
        assert_eq!(
            last.values[FEATURE_CHOICE], 1.0,
            "候选数 {count} 时最后一个选项应为 1.0"
        );
    }
}

/// 站位预算只限制站位，不限制其他动作；达到上限后站位退出训练采样候选。
#[test]
fn rearrange_budget_caps_only_rearrange() {
    use ai_core::{MAX_REARRANGE_PER_TURN, rearrange_within_budget};

    let rearrange = Action::Rearrange { from: 0, to: 1 };
    for used in 0..MAX_REARRANGE_PER_TURN {
        assert!(
            rearrange_within_budget(&rearrange, used),
            "已用 {used} 次时站位仍应可候选"
        );
    }
    assert!(
        !rearrange_within_budget(&rearrange, MAX_REARRANGE_PER_TURN),
        "达到上限后站位不应再进候选"
    );

    for used in [0, MAX_REARRANGE_PER_TURN, MAX_REARRANGE_PER_TURN + 1] {
        assert!(
            rearrange_within_budget(&Action::Buy { slot: 0 }, used),
            "买入不受站位预算影响"
        );
        assert!(
            rearrange_within_budget(&Action::End, used),
            "结束招募不受站位预算影响"
        );
    }
}
