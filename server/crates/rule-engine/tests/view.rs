//! 逐座位视图 `view()` 与 `placement_of()` 的集成测试。
//!
//! 覆盖：商店/手牌/棋盘的操作可用性与禁用原因、隐藏信息边界、
//! 待选队列、配对对手、排名名次，以及 `view()` 负责的阶段门控。

mod common;

use common::{heroes, test_catalog};
use rule_engine::data::CardType;
use rule_engine::engine::Engine;
use rule_engine::intent::RejectCode;
use rule_engine::state::{
    AuraEffect, GameState, HandCard, MinionInstance, PendingChoice, Phase, Seat, ShopSlot,
};
use rule_engine::view::{
    Affordances, PendingChoiceView, PlayerSummaryView, SeatView, SelfView, placement_of, view,
};

fn setup() -> (Engine, GameState) {
    let (mut engine, mut state) = Engine::new(test_catalog(), [7; 32]);
    engine
        .start_game(&mut state, &heroes())
        .expect("开局应成功");
    (engine, state)
}

/// 目录中第一张非英雄随从卡，供构造商店/手牌/棋盘使用。
fn minion_id() -> String {
    test_catalog()
        .cards
        .values()
        .find(|card| card.card_type == CardType::Minion && !card.is_hero)
        .expect("数据中应存在非英雄随从")
        .id
        .clone()
}

/// 目录中第一张非英雄、非学者且攻击力为正的随从卡（光环数值断言需要）。
fn positive_attack_minion_id() -> String {
    test_catalog()
        .cards
        .values()
        .find(|card| {
            card.card_type == CardType::Minion
                && !card.is_hero
                && !card.is_scholar
                && card.attack.is_some_and(|attack| attack > 0)
        })
        .expect("数据中应存在攻击力为正的非学者随从")
        .id
        .clone()
}

/// 构造一个最小可用的棋盘随从实例。
fn dummy(engine: &Engine, instance_id: u64, card_id: &str) -> MinionInstance {
    MinionInstance::new(&engine.catalog, instance_id, card_id, false)
}

fn set_shop(state: &mut GameState, seat: Seat, ids: &[Option<String>]) {
    let len = state.players[usize::from(seat)].shop.len().max(ids.len());
    let mut shop: Vec<Option<ShopSlot>> = ids
        .iter()
        .cloned()
        .map(|card_id| {
            card_id.map(|card_id| ShopSlot {
                card_id,
                frozen: false,
            })
        })
        .collect();
    while shop.len() < len {
        shop.push(None);
    }
    state.players[usize::from(seat)].shop = shop;
}

/// 断言所有操作布尔值均为 `false`（`can_end_recruit` 也包含在内）。
fn assert_all_false(affordances: &Affordances) {
    assert!(affordances.shop_slots.iter().all(|slot| !slot.buyable));
    assert!(affordances.hand_cards.iter().all(|card| !card.playable));
    assert!(
        affordances
            .board_minions
            .iter()
            .all(|minion| !minion.sellable)
    );
    assert!(!affordances.can_reroll);
    assert!(!affordances.can_upgrade_era);
    assert!(!affordances.can_use_hero_power);
    assert!(!affordances.can_end_recruit);
}

#[test]
fn shop_affordance_marks_unaffordable_slots() {
    let (engine, mut state) = setup();
    let card = minion_id();
    set_shop(
        &mut state,
        0,
        &[Some(card.clone()), Some(card), None, None, None, None],
    );
    state.players[0].gold = 0;

    let seat_view = view(&engine, &state, 0).expect("座位 0 的视图应存在");
    let shop = &seat_view.self_view.affordances.shop_slots;
    assert_eq!(shop.len(), 6, "视图应覆盖全部商店槽位");
    for (index, affordance) in shop.iter().enumerate() {
        assert!(!affordance.buyable, "金币为 0 时槽位 {index} 不可购买");
        assert_eq!(
            affordance.disabled_reason,
            Some(if index < 2 {
                RejectCode::NotEnoughGold
            } else {
                RejectCode::ShopSlotEmpty
            }),
            "槽位 {index} 的禁用原因"
        );
    }
}

#[test]
fn view_never_exposes_other_seats_private_state() {
    let (engine, mut state) = setup();
    let private_card = "private-card-seat-1".to_string();
    let private_hand = "private-hand-seat-1".to_string();

    state.players[0].gold = 7;
    set_shop(&mut state, 0, &[Some("self-shop-seat-0".to_string())]);
    state.players[1].gold = 9;
    set_shop(
        &mut state,
        1,
        &[Some(private_card.clone()), Some(private_card.clone())],
    );
    state.players[1].hand.push(HandCard {
        card_id: private_hand.clone(),
        golden: false,
    });

    let seat_view = view(&engine, &state, 0).expect("座位 0 的视图应存在");

    // 结构断言：`players` 的元素类型就是 `PlayerSummaryView`，其字段到此为止。
    let _: &Vec<PlayerSummaryView> = &seat_view.players;
    assert_eq!(seat_view.players.len(), 8, "应包含全部座位的公开摘要");
    for summary in &seat_view.players {
        // 穷尽解构：一旦 `PlayerSummaryView` 增加字段，这里会编译失败，
        // 从而迫使新增字段重新做隐私审查。
        let PlayerSummaryView {
            seat: _,
            hp: _,
            era: _,
            alive: _,
            board_count: _,
            hero_card_id: _,
            recruited: _,
            placement: _,
            rank: _,
        } = summary;
    }

    // `SelfView` 也只有自己座位的私密数据；`gold` 字段仅此一处。
    let SelfView {
        hp: _,
        era: _,
        gold,
        hero_card_id: _,
        hero_power_skill_id: _,
        hero_power_uses_left: _,
        shop: _,
        hand: _,
        board: _,
        reroll_cost: _,
        upgrade_cost: _,
        recruited: _,
        pending_choices: _,
        affordances: _,
    } = &seat_view.self_view;
    assert_eq!(*gold, 7, "自身视图只读取本座位金币");
    assert!(
        seat_view
            .self_view
            .shop
            .iter()
            .any(|slot| slot.card_id.as_deref() == Some("self-shop-seat-0")),
        "自身商店可见"
    );

    // 遍历视图中所有字符串，断言对手的私密数据无法从任何字段读到。
    let strings = collect_strings(&seat_view);
    assert!(
        !strings.iter().any(|value| value == &private_card),
        "视图中不得出现对手商店卡牌 {private_card}"
    );
    assert!(
        !strings.iter().any(|value| value == &private_hand),
        "视图中不得出现对手手牌 {private_hand}"
    );
}

/// 收集 `SeatView` 里所有可见字符串，用于隐私边界断言。
fn collect_strings(seat_view: &SeatView) -> Vec<String> {
    let mut strings = Vec::new();
    strings.extend(seat_view.enabled_tribes.iter().cloned());
    for slot in &seat_view.self_view.shop {
        strings.extend(slot.card_id.iter().cloned());
    }
    for card in &seat_view.self_view.hand {
        strings.push(card.card_id.clone());
    }
    for minion in &seat_view.self_view.board {
        strings.push(minion.card_id.clone());
        strings.extend(minion.keywords.iter().cloned());
    }
    if let Some(hero) = &seat_view.self_view.hero_card_id {
        strings.push(hero.clone());
    }
    if let Some(skill) = &seat_view.self_view.hero_power_skill_id {
        strings.push(skill.clone());
    }
    for summary in &seat_view.players {
        strings.extend(summary.hero_card_id.iter().cloned());
    }
    for pending in &seat_view.self_view.pending_choices {
        match pending {
            PendingChoiceView::Discover { candidates } => {
                strings.extend(candidates.iter().cloned());
            }
            PendingChoiceView::Evolve { options } => strings.extend(options.iter().cloned()),
            PendingChoiceView::Target { .. } => {}
        }
    }
    strings
}

#[test]
fn pending_choices_reflect_engine_queue() {
    let (engine, mut state) = setup();
    state.pending.push_back(PendingChoice::Discover {
        seat: 0,
        candidates: vec!["candidate-a".into(), "candidate-b".into()],
        source: 1,
    });
    state.pending.push_back(PendingChoice::Discover {
        seat: 1,
        candidates: vec!["other-seat-candidate".into()],
        source: 2,
    });

    let seat_view = view(&engine, &state, 0).expect("座位 0 的视图应存在");
    assert_eq!(
        seat_view.self_view.pending_choices,
        vec![PendingChoiceView::Discover {
            candidates: vec!["candidate-a".into(), "candidate-b".into()],
        }],
        "只返回本座位的第一条待选"
    );
    assert_eq!(
        view(&engine, &state, 1)
            .expect("座位 1 的视图应存在")
            .self_view
            .pending_choices
            .len(),
        1,
        "其他座位的待选不出现在座位 0 视图中"
    );
}

#[test]
fn opponent_seat_follows_pairings() {
    let (engine, mut state) = setup();
    state.pairings = vec![rule_engine::state::Pairing {
        a: 0,
        b: 3,
        ghost: None,
    }];

    assert_eq!(
        view(&engine, &state, 0)
            .expect("座位 0 的视图应存在")
            .opponent_seat,
        Some(3),
        "配对 (0,3) 时对手为 3"
    );
    assert_eq!(
        view(&engine, &state, 3)
            .expect("座位 3 的视图应存在")
            .opponent_seat,
        Some(0),
        "配对 (0,3) 时座位 3 的对手为 0"
    );

    state.pairings = vec![rule_engine::state::Pairing {
        a: 0,
        b: 0,
        ghost: Some(0),
    }];
    assert_eq!(
        view(&engine, &state, 0)
            .expect("座位 0 的视图应存在")
            .opponent_seat,
        None,
        "镜像配对没有对手座位"
    );

    state.pairings.clear();
    assert_eq!(
        view(&engine, &state, 0)
            .expect("座位 0 的视图应存在")
            .opponent_seat,
        None,
        "尚未配对时为 None"
    );
}

#[test]
fn opponent_seat_none_for_solo_without_ghost() {
    let (engine, mut state) = setup();
    // 落单但没有出局者可当幽灵：设计表格规定 `opponentPlayerId = null`。
    state.pairings = vec![rule_engine::state::Pairing {
        a: 0,
        b: 0,
        ghost: None,
    }];

    assert_eq!(
        view(&engine, &state, 0)
            .expect("座位 0 的视图应存在")
            .opponent_seat,
        None,
        "空幽灵的落单配对没有对手座位"
    );
}

#[test]
fn opponent_seat_points_at_ghost_owner() {
    let (engine, mut state) = setup();
    // 座位 0 落单并对战幽灵归属座位 3（已出局）；座位 1 与座位 2 真实配对。
    state.players[3].eliminated = true;
    state.pairings = vec![
        rule_engine::state::Pairing {
            a: 0,
            b: 0,
            ghost: Some(3),
        },
        rule_engine::state::Pairing {
            a: 1,
            b: 2,
            ghost: None,
        },
    ];

    assert_eq!(
        view(&engine, &state, 0)
            .expect("座位 0 的视图应存在")
            .opponent_seat,
        Some(3),
        "幽灵战指向幽灵归属座位"
    );
    assert_eq!(
        view(&engine, &state, 3)
            .expect("座位 3 的视图应存在")
            .opponent_seat,
        None,
        "幽灵归属者本人观战，没有对手座位"
    );
    assert_eq!(
        view(&engine, &state, 1)
            .expect("座位 1 的视图应存在")
            .opponent_seat,
        Some(2),
        "真实配对时的对手为配对中的另一座位"
    );
}

#[test]
fn board_view_folds_auras_into_stats_and_keywords() {
    let (engine, mut state) = setup();
    let card = positive_attack_minion_id();
    state.players[0].board.push(dummy(&engine, 1, &card));

    let before = view(&engine, &state, 0)
        .expect("座位 0 的视图应存在")
        .self_view
        .board[0]
        .clone();
    assert!(
        !before.keywords.contains(&"光环测试词".to_string()),
        "前提：未加光环时不存在该关键词"
    );

    state.auras.insert(
        1,
        AuraEffect {
            attack: 2,
            health: 1,
            keywords: ["divine_shield".to_string(), "光环测试词".to_string()]
                .into_iter()
                .collect(),
            floor_one: true,
        },
    );

    let after = view(&engine, &state, 0)
        .expect("座位 0 的视图应存在")
        .self_view
        .board[0]
        .clone();
    assert_eq!(after.attack, before.attack + 2, "攻击力应折叠光环加成");
    assert_eq!(after.health, before.health + 1, "生命值应折叠光环加成");
    assert_eq!(after.stats.attack.aura, 2, "光环进入攻击的 aura 层");
    assert_eq!(after.stats.health.aura, 1, "光环进入生命的 aura 层");
    assert_eq!(
        after.stats.attack.current, after.attack,
        "构成当前量与原字段一致"
    );
    assert_eq!(
        after.stats.health.current, after.health,
        "构成当前量与原字段一致"
    );
    assert_eq!(
        after.stats.attack.initial
            + after.stats.attack.permanent
            + after.stats.attack.turn
            + after.stats.attack.aura
            - after.stats.attack.damage,
        after.stats.attack.current,
        "攻击各层与受击自洽"
    );
    assert!(
        after.keywords.contains(&"divine_shield".to_string()),
        "关键词应包含光环授予的护盾"
    );
    assert!(
        after.keywords.contains(&"光环测试词".to_string()),
        "关键词应包含光环授予的自定义关键词"
    );
}

#[test]
fn placement_is_hidden_before_game_over() {
    let (engine, mut state) = setup();
    // 对局进行中的真实形态：淘汰 2 人后新出局者前插，下标 +1 并非名次。
    state.ranking = vec![2, 0, 7, 6, 5, 4];
    assert_eq!(state.phase, Phase::Recruit);

    let seat_view = view(&engine, &state, 0).expect("座位 0 的视图应存在");
    assert!(
        seat_view
            .players
            .iter()
            .all(|summary| summary.placement.is_none()),
        "对局结束前所有 placement 都必须为 None"
    );
    assert_eq!(placement_of(&state, 2), None);
    assert_eq!(placement_of(&state, 0), None);

    state.phase = Phase::GameOver;
    state.ranking = vec![1, 3, 2, 0, 7, 6, 5, 4];
    assert_eq!(placement_of(&state, 1), Some(1));
    assert_eq!(placement_of(&state, 3), Some(2));
    assert_eq!(placement_of(&state, 0), Some(4));

    // `GameOver` 阶段 `rank` 与 `placement` 完全一致。
    let seat_view = view(&engine, &state, 0).expect("座位 0 的视图应存在");
    for summary in &seat_view.players {
        assert_eq!(
            summary.rank,
            placement_of(&state, summary.seat),
            "座位 {} 的 rank 应等于 placement",
            summary.seat
        );
    }
}

#[test]
fn placement_comes_from_ranking() {
    let (engine, mut state) = setup();
    // 名次只在 `GameOver` 阶段公开；此处构造完整榜单。
    state.phase = Phase::GameOver;
    state.ranking = vec![5, 2];

    assert_eq!(placement_of(&state, 5), Some(1));
    assert_eq!(placement_of(&state, 2), Some(2));
    assert_eq!(placement_of(&state, 0), None);

    let seat_view = view(&engine, &state, 0).expect("座位 0 的视图应存在");
    let placement = |seat: Seat| {
        seat_view
            .players
            .iter()
            .find(|summary| summary.seat == seat)
            .expect("摘要应包含该座位")
            .placement
    };
    assert_eq!(placement(5), Some(1));
    assert_eq!(placement(2), Some(2));
    assert_eq!(placement(0), None);
}

#[test]
fn lobby_phase_disables_all_affordances() {
    let (engine, state) = Engine::new(test_catalog(), [7; 32]);
    assert_eq!(state.phase, Phase::Lobby);

    let seat_view = view(&engine, &state, 0).expect("座位 0 的视图应存在");
    assert_all_false(&seat_view.self_view.affordances);
    assert!(
        seat_view
            .self_view
            .affordances
            .shop_slots
            .iter()
            .all(|slot| slot.disabled_reason == Some(RejectCode::InvalidStage)),
        "非招募阶段所有商店槽位的禁用原因是 InvalidStage"
    );
    assert_eq!(
        seat_view.self_view.affordances.upgrade_disabled_reason,
        Some(RejectCode::InvalidStage)
    );
    assert_eq!(
        seat_view.self_view.affordances.hero_power_disabled_reason,
        Some(RejectCode::InvalidStage)
    );
}

#[test]
fn recruited_seat_cannot_act_but_can_end_recruit() {
    let (engine, mut state) = setup();
    let card = minion_id();
    set_shop(&mut state, 0, &[Some(card.clone())]);
    state.players[0].hand.push(HandCard {
        card_id: card.clone(),
        golden: false,
    });
    state.players[0].board.push(dummy(&engine, 1, &card));
    state.players[0].gold = 10;
    state.players[0].recruit_done = true;

    let affordances = view(&engine, &state, 0)
        .expect("座位 0 的视图应存在")
        .self_view
        .affordances;
    assert!(affordances.shop_slots.iter().all(|slot| !slot.buyable));
    assert!(affordances.hand_cards.iter().all(|card| !card.playable));
    assert!(
        affordances
            .board_minions
            .iter()
            .all(|minion| !minion.sellable)
    );
    assert!(!affordances.can_reroll);
    assert!(!affordances.can_upgrade_era);
    assert!(!affordances.can_use_hero_power);
    assert!(affordances.can_end_recruit, "重复 EndRecruit 幂等成功");
}

#[test]
fn eliminated_seat_disables_all_affordances() {
    let (engine, mut state) = setup();
    let card = minion_id();
    set_shop(&mut state, 0, &[Some(card.clone())]);
    state.players[0].hand.push(HandCard {
        card_id: card.clone(),
        golden: false,
    });
    state.players[0].board.push(dummy(&engine, 1, &card));
    state.players[0].gold = 10;
    state.players[0].eliminated = true;

    // 前提：金币充足、商店有货、手牌与棋盘均非空——仅看意图校验本应全部可操作。
    let seat_view = view(&engine, &state, 0).expect("座位 0 的视图应存在");
    assert_all_false(&seat_view.self_view.affordances);
}

#[test]
fn unknown_seat_has_no_view() {
    let (engine, state) = setup();
    assert!(view(&engine, &state, 8).is_none());
}

#[test]
fn hand_card_requires_target_for_target_effect_card() {
    let (engine, mut state) = setup();
    state.players[0].hand.push(HandCard {
        card_id: "chengbang-faling".to_string(),
        golden: false,
    });

    let seat_view = view(&engine, &state, 0).expect("座位 0 的视图应存在");
    assert!(
        seat_view.self_view.hand[0].requires_target,
        "friendly:single 的效果牌需要目标"
    );
}

#[test]
fn hand_card_requires_target_false_for_plain_effect_card() {
    let (engine, mut state) = setup();
    state.players[0].hand.push(HandCard {
        card_id: "aolinpike-jidian".to_string(),
        golden: false,
    });

    let seat_view = view(&engine, &state, 0).expect("座位 0 的视图应存在");
    assert!(
        !seat_view.self_view.hand[0].requires_target,
        "friendly:all 不需要指定单一目标"
    );
}

#[test]
fn hand_card_requires_target_false_for_minion_card() {
    let (engine, mut state) = setup();
    state.players[0].hand.push(HandCard {
        card_id: "archimedes".to_string(),
        golden: false,
    });

    let seat_view = view(&engine, &state, 0).expect("座位 0 的视图应存在");
    assert!(
        !seat_view.self_view.hand[0].requires_target,
        "视图口径为 OnPlayEffect，随从牌恒为 false"
    );
}

#[test]
fn pending_choice_view_target_carries_source() {
    let (engine, mut state) = setup();
    state.pending.push_back(PendingChoice::Target {
        seat: 0,
        source: 42,
        card_id: "fangzhiji".to_string(),
        golden: false,
    });

    let seat_view = view(&engine, &state, 0).expect("座位 0 的视图应存在");
    assert_eq!(
        seat_view.self_view.pending_choices,
        vec![PendingChoiceView::Target {
            source_minion_id: 42
        }]
    );
}

#[test]
fn affordances_blocked_while_target_pending() {
    let (engine, mut state) = setup();
    let card = minion_id();
    set_shop(&mut state, 0, &[Some(card.clone())]);
    state.players[0].hand.push(HandCard {
        card_id: card.clone(),
        golden: false,
    });
    state.players[0].board.push(dummy(&engine, 1, &card));
    state.players[0].gold = 10;
    state.pending.push_back(PendingChoice::Target {
        seat: 0,
        source: 1,
        card_id: card,
        golden: false,
    });

    let affordances = view(&engine, &state, 0)
        .expect("座位 0 的视图应存在")
        .self_view
        .affordances;
    assert!(affordances.shop_slots.iter().all(|slot| !slot.buyable));
    assert!(affordances.hand_cards.iter().all(|card| !card.playable));
    assert!(
        affordances
            .board_minions
            .iter()
            .all(|minion| !minion.sellable)
    );
    assert!(!affordances.can_reroll);
    assert!(!affordances.can_upgrade_era);
    assert!(!affordances.can_use_hero_power);
    assert!(!affordances.can_end_recruit, "目标待选期间不能结束招募");
}
