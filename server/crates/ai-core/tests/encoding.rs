use std::collections::BTreeMap;

use ai_core::{Action, BOARD_SLOTS, CardIndex, Encoder, History, OBS_DIM, SEATS, SHOP_SLOTS};
use rule_engine::data::Catalog;
use rule_engine::engine::Engine;
use rule_engine::legal::legal_actions;
use rule_engine::playstyles;
use rule_engine::state::GameState;
use rule_engine::view::view;

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

/// 开局并进入第一回合招募的固定夹具。
fn started() -> (Engine, GameState, Catalog) {
    let catalog = catalog();
    let (engine, mut state) = Engine::new(catalog.clone(), [7; 32], playstyles::draw(7));
    let heroes: BTreeMap<u8, String> = catalog
        .heroes
        .values()
        .take(8)
        .enumerate()
        .map(|(index, hero)| (index as u8, hero.id.clone()))
        .collect();
    engine.start_game(&mut state, &heroes).expect("开局");
    (engine, state, catalog)
}

fn fnv(values: &[f32]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for value in values {
        hash ^= u64::from(value.to_bits());
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[test]
fn card_index_is_stable_and_complete() {
    let catalog = catalog();
    let index = CardIndex::build(&catalog);
    assert!(index.card("bisheng") >= 0);
    assert_eq!(index.card("bisheng"), index.card("bisheng"));
    assert_eq!(index.card("not-a-card"), -1);
    assert_eq!(index.card_count(), catalog.cards.len());
    assert_eq!(index.hero_count(), catalog.heroes.len());
    assert!(index.hero(catalog.heroes.values().next().expect("英雄").id.as_str()) >= 0);
}

#[test]
fn observation_uses_only_public_information() {
    let (engine, state, catalog) = started();
    let seat_view = view(&engine, &state, 0).expect("视图");
    let obs = Encoder::new(CardIndex::build(&catalog)).encode(&seat_view, &History::new());

    assert_eq!(obs.shop.len(), SHOP_SLOTS);
    assert_eq!(obs.players.len(), SEATS);
    assert_eq!(obs.board.len(), BOARD_SLOTS);
    // 没记录过对手棋盘时，所有历史槽位都是 -1。
    assert!(
        obs.players
            .iter()
            .all(|player| player.last_board.iter().all(|card| *card == -1))
    );
    assert_eq!(obs.hero, obs.players[0].hero);
    assert_eq!(obs.players.iter().filter(|p| p.alive > 0.0).count(), 8);
}

#[test]
fn history_only_fills_the_recorded_opponent() {
    let (engine, state, catalog) = started();
    let index = CardIndex::build(&catalog);
    let bisheng = index.card("bisheng");
    let mut history = History::new();
    history.record(3, &[bisheng, -1, -1, -1, -1, -1, -1]);

    let obs = Encoder::new(index).encode(&view(&engine, &state, 0).expect("视图"), &history);
    assert_eq!(obs.players[3].last_board[0], bisheng);
    assert!(
        obs.players[2].last_board.iter().all(|card| *card == -1),
        "没交手过的座位不应出现棋盘"
    );
}

#[test]
fn every_legal_action_round_trips() {
    let (engine, state, _) = started();
    let actions = legal_actions(&engine, &state, 0);
    assert!(!actions.is_empty());
    for intent in actions {
        let action = Action::from_intent(&intent).expect("意图应能转成动作");
        assert_eq!(action.to_intent(), Some(intent));
    }
}

#[test]
fn observation_layout_is_stable() {
    let (engine, state, catalog) = started();
    let obs = Encoder::new(CardIndex::build(&catalog))
        .encode(&view(&engine, &state, 0).expect("视图"), &History::new());
    let values = obs.to_vec();
    assert_eq!(values.len(), OBS_DIM);
    assert_eq!(ai_core::OBS_DIM, 431);
    assert_eq!(ai_core::SCHEMA_VERSION, 2);
    assert_eq!(values[0], 0.0);
    assert_eq!(values[1], 1.0 / 20.0);
    assert_eq!(values[2], 1.0);
    // 金样本：字段顺序或归一化常数一改，这里必然失败。
    assert_eq!(fnv(&values), 0x3d3a_a15c_bc52_4da1);
}

/// 观测只能来自公开信息：改动其它座位的手牌与金币后，同一个座位的观测必须不变。
#[test]
fn observation_ignores_hidden_state_of_other_seats() {
    use rule_engine::state::{HandCard, MinionInstance};

    let (engine, state, catalog) = started();
    let encoder = Encoder::new(CardIndex::build(&catalog));
    let history = History::new();
    let before = encoder.encode(&view(&engine, &state, 0).expect("视图"), &history);

    let mut mutated = state.clone();
    let other = usize::from(3u8);
    let card = catalog
        .cards
        .values()
        .find(|card| card.in_shop_pool)
        .expect("卡表应有可购买卡")
        .id
        .clone();
    mutated.players[other]
        .hand
        .push(HandCard::Minion(MinionInstance::new(
            &catalog, 90_001, &card, false,
        )));
    mutated.players[other].gold = 99;
    let after = encoder.encode(&view(&engine, &mutated, 0).expect("视图"), &history);

    assert_eq!(before, after, "座位 0 的观测不应看到座位 3 的隐藏状态");
}

/// 手工拼一个覆盖新增字段的视图，只测编码，不依赖对局推进。
fn view_stub(catalog: &Catalog) -> rule_engine::view::SeatView {
    use rule_engine::view::{
        Affordances, BoardMinionView, HeroPowerView, MinionAbilityView, MinionStatsView,
        PendingChoiceView, SeatView, SelfView, ShopAffordanceView, ShopSlotView,
    };
    let card = catalog.cards.values().next().expect("卡表").id.clone();
    let skill = catalog.skills().first().expect("技能").id.clone();
    let hero = catalog.heroes.values().next().expect("英雄").id.clone();
    SeatView {
        phase: rule_engine::state::Phase::Recruit,
        turn: 1,
        seat: 0,
        opponent_seat: Some(5),
        self_view: SelfView {
            hp: 30,
            era: 2,
            gold: 5,
            hero_card_id: Some(hero),
            hero_powers: vec![HeroPowerView {
                skill_id: skill.clone(),
                uses_left: 2,
                requires_target: false,
                target_candidates: Vec::new(),
                can_use: true,
                disabled_reason: None,
                voyage_remaining: None,
                voyage_pending: 0,
            }],
            shop: vec![
                ShopSlotView {
                    index: 0,
                    instance_id: Some(1),
                    card_id: Some(card.clone()),
                    frozen: false,
                },
                ShopSlotView {
                    index: 1,
                    instance_id: Some(2),
                    card_id: Some(card.clone()),
                    frozen: false,
                },
            ],
            hand: Vec::new(),
            board: vec![BoardMinionView {
                index: 0,
                minion_id: 7,
                card_id: card.clone(),
                golden: false,
                attack: 2,
                health: 3,
                stats: MinionStatsView::default(),
                keywords: vec![
                    "taunt".to_string(),
                    "breed 2".to_string(),
                    "armor 3".to_string(),
                ],
                stealth: false,
                abilities: vec![MinionAbilityView {
                    skill_id: skill,
                    cost: 1,
                    uses_left: 1,
                    requires_target: false,
                    target_candidates: Vec::new(),
                    can_use: false,
                    disabled_reason: None,
                }],
            }],
            reroll_cost: 1,
            upgrade_cost: None,
            recruited: false,
            pending_choices: vec![PendingChoiceView::Discover {
                candidates: vec![card.clone(), card],
            }],
            affordances: Affordances {
                shop_slots: vec![
                    ShopAffordanceView {
                        index: 0,
                        buyable: true,
                        disabled_reason: None,
                    },
                    ShopAffordanceView {
                        index: 1,
                        buyable: false,
                        disabled_reason: None,
                    },
                ],
                hand_cards: Vec::new(),
                board_minions: Vec::new(),
                can_reroll: true,
                can_unfreeze: false,
                can_upgrade_era: false,
                upgrade_disabled_reason: None,
                can_end_recruit: true,
            },
        },
        players: Vec::new(),
        enabled_playstyles: Vec::new(),
    }
}

#[test]
fn encoder_reads_public_shop_keyword_and_skill_state() {
    let catalog = catalog();
    let index = CardIndex::build(&catalog);
    let obs =
        Encoder::new(CardIndex::build(&catalog)).encode(&view_stub(&catalog), &History::new());

    // 可购买性来自引擎 affordance，不是金币自行推算。
    assert_eq!(obs.shop_buyable[0], 1.0);
    assert_eq!(obs.shop_buyable[1], 0.0);

    // 关键词基名置位；breed / armor 的层数单独入向量。
    let taunt = rule_engine::keyword::Keyword::Taunt.index();
    let breed = rule_engine::keyword::Keyword::Breed.index();
    assert_eq!(obs.board_keywords[0][taunt], 1.0);
    assert_eq!(obs.board_keywords[0][breed], 1.0);
    assert_eq!(obs.board_keyword_layers[0][0], 2.0 / 3.0);
    assert_eq!(obs.board_keyword_layers[0][2], 1.0);

    // 英雄与随从主动技能的剩余次数、可用性。
    let skill = catalog.skills().first().expect("技能").id.clone();
    assert_eq!(obs.hero_powers[0].skill, index.skill(&skill));
    assert_eq!(obs.hero_power_count, 1);
    assert_eq!(obs.hero_powers[0].uses, 1.0);
    assert_eq!(obs.hero_powers[0].can_use, 1.0);
    assert_eq!(obs.board_abilities[0].has, 1.0);
    assert_eq!(obs.board_abilities[0].skill, index.skill(&skill));
    assert_eq!(obs.board_abilities[0].uses, 0.5);
    assert_eq!(obs.board_abilities[0].can_use, 0.0);

    // 发现候选与本回合对手座位。
    let card = catalog.cards.values().next().expect("卡表").id.clone();
    assert_eq!(obs.discover[0], index.card(&card));
    assert_eq!(obs.discover[1], index.card(&card));
    assert_eq!(obs.discover[2], -1);
    assert_eq!(obs.discover_count, 2);
    assert_eq!(obs.opponent_seat, 5);

    // 新增段的顺序与位置固定：250 起商店可买，429 起对手座位。
    let values = obs.to_vec();
    assert_eq!(values[250], 1.0);
    assert_eq!(values[255], 0.0);
    assert_eq!(values[429], 1.0);
    assert_eq!(values[430], 5.0 / 7.0);
}

#[test]
fn target_pending_carries_only_the_candidate_count() {
    use rule_engine::effects::{TargetRef, Zone};
    use rule_engine::view::PendingChoiceView;

    let catalog = catalog();
    let mut view = view_stub(&catalog);
    view.self_view.pending_choices = vec![PendingChoiceView::Target {
        source_minion_id: 7,
        candidates: vec![
            TargetRef {
                zone: Zone::Board,
                id: 7,
            },
            TargetRef {
                zone: Zone::Board,
                id: 8,
            },
        ],
    }];
    let obs = Encoder::new(CardIndex::build(&catalog)).encode(&view, &History::new());
    assert_eq!(obs.discover_count, 0);
    assert_eq!(obs.discover, [-1, -1, -1]);
    assert_eq!(obs.target_candidates, 2);
    assert_eq!(obs.to_vec()[428], 2.0 / 23.0);
}
