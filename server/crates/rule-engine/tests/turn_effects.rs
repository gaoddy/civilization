mod common;

use std::collections::BTreeSet;

use common::{card_has_play, categories_for, effects_of, place, test_catalog, test_playstyles};
use rule_engine::combat::{self, CombatAction, CombatLog, CombatSetup};
use rule_engine::data::{Card, CardType, Catalog, Era, Playstyle, Skill, card_in_pool};
use rule_engine::effects::{Effect, EffectCtx, parse_effects};
use rule_engine::engine::Engine;
use rule_engine::events::GameEvent;
use rule_engine::hash::state_hash;
use rule_engine::intent::Intent;
use rule_engine::rng::RngStreams;
use rule_engine::settle::Pairing;
use rule_engine::state::{GameState, HandCard, InstanceId, MinionInstance, Phase, Seat, ShopSlot};

/// 往目录里插入一张合成随从卡，带一个技能（`keyword` + `effects`）。
fn add_card(
    catalog: &mut Catalog,
    id: &str,
    playstyle: Playstyle,
    stats: (u8, i32, i32),
    skill: (Option<&str>, &str),
) {
    let (level, attack, health) = stats;
    let (keyword, effects) = skill;
    let skill_id = format!("{id}-skill");
    catalog.skills.insert(
        skill_id.clone(),
        Skill {
            id: skill_id.clone(),
            keyword: keyword.map(str::to_string),
            effects: effects_of(effects),
            cost: None,
            uses_per_turn: None,
        },
    );
    catalog.cards.insert(
        id.to_string(),
        Card {
            id: id.to_string(),
            card_type: CardType::Minion,
            taxon: "hominidae".to_string(),
            categories: categories_for(playstyle),
            subtype: None,
            level,
            era: None,
            attack: Some(attack),
            health: Some(health),
            skills: vec![skill_id],
            is_scholar: false,
            cost: None,
        },
    );
}

/// 在真实 v0.1 数据上追加测试专用的确定性卡牌。
fn catalog_with_synthetics() -> Catalog {
    let mut catalog = test_catalog();
    add_card(
        &mut catalog,
        "breed2",
        Playstyle::Mammal,
        (1, 1, 2),
        (Some("breed 2"), "[]"),
    );
    add_card(
        &mut catalog,
        "breed1",
        Playstyle::Mammal,
        (1, 1, 2),
        (Some("breed 1"), "[]"),
    );
    add_card(
        &mut catalog,
        "scholar-breed",
        Playstyle::Neutral,
        (1, 0, 4),
        (Some("breed 2"), "[]"),
    );
    add_card(
        &mut catalog,
        "sickly",
        Playstyle::Mammal,
        (1, 1, 2),
        (None, "[]"),
    );
    add_card(
        &mut catalog,
        "turn9",
        Playstyle::Mammal,
        (1, 0, 20),
        (None, "OnTurnStart|DealDamage|friendly:leftmost|amount=9"),
    );
    add_card(
        &mut catalog,
        "turn7",
        Playstyle::Mammal,
        (1, 0, 20),
        (None, "OnTurnStart|DealDamage|friendly:leftmost|amount=7"),
    );
    add_card(
        &mut catalog,
        "contagious",
        Playstyle::Mammal,
        (1, 1, 100),
        (Some("contagion"), "[]"),
    );
    add_card(
        &mut catalog,
        "target",
        Playstyle::Mammal,
        (1, 0, 10),
        (None, "[]"),
    );
    add_card(
        &mut catalog,
        "latent",
        Playstyle::Virus,
        (1, 0, 10),
        (
            Some("contagion"),
            "OnTurnStart|Latent|self|amount=1,threshold=3,count=3",
        ),
    );
    add_card(
        &mut catalog,
        "latent0",
        Playstyle::Virus,
        (1, 0, 10),
        (
            Some("contagion"),
            "OnTurnStart|Latent|self|amount=1,threshold=3,count=0",
        ),
    );
    // 被爆发击杀的目标：亡语给友方全体 +1 攻击，用于证明死亡结算被触发。
    add_card(
        &mut catalog,
        "frail",
        Playstyle::Mammal,
        (1, 0, 2),
        (None, "OnDeath|ModifyStats|friendly:all|attack=1,health=0"),
    );
    add_card(
        &mut catalog,
        "witness",
        Playstyle::Mammal,
        (1, 0, 10),
        (None, "[]"),
    );
    add_card(
        &mut catalog,
        "armored",
        Playstyle::Mammal,
        (1, 0, 10),
        (Some("armor 2"), "[]"),
    );
    add_card(
        &mut catalog,
        "shielded",
        Playstyle::Mammal,
        (1, 0, 10),
        (Some("divine_shield"), "[]"),
    );
    add_card(
        &mut catalog,
        "scavenger",
        Playstyle::Machine,
        (1, 1, 1),
        (None, "OnDeath|Salvage|self|pool=playstyle,count=1"),
    );
    // 非机械载体：仅用于验证随机池跟随载体玩法，而不是写死机械。
    add_card(
        &mut catalog,
        "mammal-salvager",
        Playstyle::Mammal,
        (1, 1, 1),
        (None, "OnDeath|Salvage|self|pool=playstyle,count=1"),
    );
    add_card(
        &mut catalog,
        "killer",
        Playstyle::Machine,
        (1, 10, 10),
        (None, "[]"),
    );
    catalog
}

fn setup(seed: u8) -> (Engine, GameState) {
    let (engine, mut state) = Engine::new(catalog_with_synthetics(), [seed; 32], test_playstyles());
    state.phase = Phase::Recruit;
    state.turn = 1;
    for player in &mut state.players {
        player.hp = 30;
        player.gold = 10;
    }
    (engine, state)
}

fn effect(raw: &str) -> Effect {
    parse_effects(raw)
        .expect("测试效果应可解析")
        .into_iter()
        .next()
        .expect("测试效果至少一段")
}

fn ctx(seat: Seat) -> EffectCtx {
    EffectCtx {
        seat,
        source: None,
        target: None,
        event_target: None,
        golden: false,
        discover_level: None,
    }
}

fn board_minion(state: &GameState, seat: Seat, index: usize) -> &MinionInstance {
    &state.players[usize::from(seat)].board[index]
}

/// 与规则一致的「当前星级排序第一张」筛选，供断言期望值。
fn first_eligible(engine: &Engine, state: &GameState, star: u8) -> Option<String> {
    engine
        .catalog
        .cards
        .values()
        .find(|card| {
            card.card_type == CardType::Minion
                && card.level == star
                && state.pool.get(&card.id).copied().unwrap_or(0) > 0
                && card_in_pool(card, &state.enabled_playstyles)
        })
        .map(|card| card.id.clone())
}

// ---------------------------------------------------------------------------
// 繁衍 / 传染 / 回合开始效果
// ---------------------------------------------------------------------------

#[test]
fn breed_adds_health_each_turn() {
    let (mut engine, mut state) = setup(1);
    place(&engine.catalog, &mut state, 0, 1, "breed2");

    engine.start_turn(&mut state).expect("开始回合应成功");

    let minion = board_minion(&state, 0, 0);
    assert_eq!(minion.stats.health.growth, 2, "繁衍 2 每回合 +2 生命");
    assert_eq!(minion.breed_applied, 2, "记录累计已应用的繁衍量");
    assert_eq!(minion.stats.attack.growth, 0, "繁衍不加攻击");
    assert_eq!(combat::effective_health(&engine.catalog, minion), 4);

    engine.start_turn(&mut state).expect("开始下一回合应成功");
    let minion = board_minion(&state, 0, 0);
    assert_eq!(minion.stats.health.growth, 4, "第二回合继续 +2");
    assert_eq!(minion.breed_applied, 4);
}

#[test]
fn golden_breed_doubles_health() {
    let (mut engine, mut state) = setup(1);
    place(&engine.catalog, &mut state, 0, 1, "breed2");
    state.players[0].board[0].golden = true;

    engine.start_turn(&mut state).expect("开始回合应成功");

    let minion = board_minion(&state, 0, 0);
    assert_eq!(
        minion.stats.health.growth, 4,
        "金色繁衍 2 触发两次，等价于 +4 生命"
    );
    assert_eq!(minion.breed_applied, 4, "记录实际应用的繁衍量");
    assert_eq!(minion.stats.attack.growth, 0);
}

#[test]
fn breed_includes_dynamic_layers() {
    let (mut engine, mut state) = setup(1);
    place(&engine.catalog, &mut state, 0, 1, "breed1");
    state.players[0].board[0].breed = 3;

    engine.start_turn(&mut state).expect("开始回合应成功");

    let minion = board_minion(&state, 0, 0);
    assert_eq!(minion.stats.health.growth, 4, "固定 1 + 授予 3");
    assert_eq!(minion.breed_applied, 4);
    assert_eq!(minion.stats.attack.growth, 0);
}

#[test]
fn scholar_breed_adds_health_only() {
    let (mut engine, mut state) = setup(1);
    place(&engine.catalog, &mut state, 0, 1, "scholar-breed");

    engine.start_turn(&mut state).expect("开始回合应成功");

    let minion = board_minion(&state, 0, 0);
    assert_eq!(minion.stats.health.growth, 2);
    assert_eq!(minion.stats.attack.growth, 0);
    assert_eq!(combat::effective_attack(&engine.catalog, minion), 0);
    assert_eq!(combat::effective_health(&engine.catalog, minion), 6);
}

#[test]
fn breed_resolves_before_contagion() {
    let (mut engine, mut state) = setup(1);
    place(&engine.catalog, &mut state, 0, 1, "breed2");
    state.players[0].board[0].contagion = 2;

    engine.start_turn(&mut state).expect("开始回合应成功");

    let board = &state.players[0].board;
    assert_eq!(board.len(), 1, "先 +2 再 -2，随从应存活");
    assert_eq!(board[0].stats.health.growth, 2);
    assert_eq!(board[0].stats.health.permanent, -2);
    assert_eq!(combat::effective_health(&engine.catalog, &board[0]), 2);
}

#[test]
fn contagion_decrements_health_and_can_kill() {
    let (mut engine, mut state) = setup(1);
    place(&engine.catalog, &mut state, 0, 1, "sickly");
    place(&engine.catalog, &mut state, 0, 2, "sickly");
    state.players[0].board[0].contagion = 5;
    state.players[0].board[1].contagion = 1;

    engine.start_turn(&mut state).expect("开始回合应成功");

    let board = &state.players[0].board;
    assert_eq!(board.len(), 1, "生命降至 0 的随从在回合开始时死亡");
    assert_eq!(board[0].instance_id, 2);
    assert_eq!(board[0].stats.health.permanent, -1);
    assert_eq!(combat::effective_health(&engine.catalog, &board[0]), 1);
}

#[test]
fn turn_start_effect_runs_left_to_right() {
    let (mut engine, mut state) = setup(1);
    place(&engine.catalog, &mut state, 0, 1, "turn9");
    place(&engine.catalog, &mut state, 0, 2, "turn7");

    let events = engine.start_turn(&mut state).expect("开始回合应成功");

    let amounts: Vec<i32> = events
        .into_iter()
        .filter_map(|event| match event {
            GameEvent::DamageTaken { amount, .. } => Some(amount),
            _ => None,
        })
        .collect();
    assert_eq!(amounts, vec![9, 7], "按站位从左到右结算");
}

#[test]
fn golden_turn_start_effect_runs_twice() {
    let (mut engine, mut state) = setup(1);
    place(&engine.catalog, &mut state, 0, 1, "turn9");
    state.players[0].board[0].golden = true;

    let events = engine.start_turn(&mut state).expect("开始回合应成功");

    let amounts: Vec<i32> = events
        .into_iter()
        .filter_map(|event| match event {
            GameEvent::DamageTaken { amount, .. } => Some(amount),
            _ => None,
        })
        .collect();
    assert_eq!(amounts, vec![9, 9], "金色随从的回合开始效果连续触发两次");
}

#[test]
fn turn_start_skips_eliminated_players() {
    let (mut engine, mut state) = setup(1);
    place(&engine.catalog, &mut state, 0, 1, "breed2");
    state.players[0].eliminated = true;

    engine.start_turn(&mut state).expect("开始回合应成功");

    assert_eq!(state.players[0].board[0].stats.health.growth, 0);
}

// ---------------------------------------------------------------------------
// 首次刷新
// ---------------------------------------------------------------------------

#[test]
fn first_refresh_adds_extra_slot_once() {
    let (mut engine, mut state) = setup(1);
    state.players[0].hero = Some("galileo".to_string());
    state.players[0].first_refresh_used = false;
    state.players[0].first_refresh_pending = true;

    engine
        .apply(&mut state, 0, Intent::RerollShop)
        .expect("首次刷新应成功");

    let player = &state.players[0];
    assert_eq!(player.shop.len(), 7, "观测额外增加 1 个槽位");
    assert!(!player.first_refresh_pending, "首次刷新触发后清除标记");
    let extra = player.shop[6].as_ref().expect("额外槽位应有随从");
    let card = engine.catalog.cards.get(&extra.card_id).expect("卡牌存在");
    assert_eq!(card.level, 1, "额外槽位使用当前时代星级");

    let gold_before = player.gold;
    engine
        .apply(&mut state, 0, Intent::RerollShop)
        .expect("第二次刷新应成功");
    let player = &state.players[0];
    assert_eq!(player.shop.len(), 6, "本回合不再追加额外槽位");
    assert_eq!(player.gold, gold_before - 1, "第二次刷新消耗 1 金币");
}

#[test]
fn first_refresh_reads_hero_pool() {
    let (mut engine, mut state) = setup(1);
    state.players[0].hero = Some("galileo".to_string());
    state.players[0].first_refresh_used = false;
    state.players[0].first_refresh_pending = true;
    // 从同名随从卡移除首次刷新技能，英雄池保留。
    engine
        .catalog
        .cards
        .get_mut("galileo")
        .expect("伽利略随从卡应存在")
        .skills
        .retain(|skill| skill != "observation");

    engine
        .apply(&mut state, 0, Intent::RerollShop)
        .expect("首次刷新应成功");

    assert_eq!(
        state.players[0].shop.len(),
        7,
        "英雄首次刷新技能只从英雄池读取"
    );
}

#[test]
fn first_refresh_fires_only_once_per_turn() {
    let (mut engine, mut state) = setup(1);
    state.players[0].hero = Some("galileo".to_string());
    state.players[0].first_refresh_used = false;
    state.players[0].first_refresh_pending = true;

    engine
        .apply(&mut state, 0, Intent::RerollShop)
        .expect("首次刷新应成功");
    engine
        .apply(&mut state, 0, Intent::RerollShop)
        .expect("第二次刷新应成功");
    assert_eq!(state.players[0].shop.len(), 6);

    // 进入下一回合后重新允许触发。
    engine.start_turn(&mut state).expect("开始下一回合应成功");
    assert!(state.players[0].first_refresh_pending);
    engine
        .apply(&mut state, 0, Intent::RerollShop)
        .expect("新回合首次刷新应成功");
    assert_eq!(state.players[0].shop.len(), 7);
    assert!(!state.players[0].first_refresh_pending);
}

#[test]
fn era_upgrade_reroll_does_not_fire_first_refresh() {
    let (mut engine, mut state) = setup(1);
    state.players[0].hero = Some("galileo".to_string());
    state.players[0].first_refresh_pending = true;

    engine
        .apply(&mut state, 0, Intent::UpgradeEra)
        .expect("升时代应成功");

    assert_eq!(state.players[0].shop.len(), 6, "升时代重抽不追加额外槽位");
    assert!(
        state.players[0].first_refresh_pending,
        "升时代不消耗首次刷新触发"
    );
}

#[test]
fn first_refresh_pending_is_hashed_and_reset_each_turn() {
    let (mut engine, mut state) = setup(1);
    assert!(state.players[0].first_refresh_pending, "开局应为待触发");

    let before = state_hash(&state);
    state.players[0].first_refresh_pending = false;
    assert_ne!(before, state_hash(&state), "首次刷新标记计入状态哈希");

    engine.start_turn(&mut state).expect("开始回合应成功");
    assert!(state.players[0].first_refresh_pending, "每回合重置为待触发");
}

// ---------------------------------------------------------------------------
// 商店类原语
// ---------------------------------------------------------------------------

#[test]
fn refresh_shop_free_sets_cost_zero() {
    let (mut engine, mut state) = setup(1);
    state.players[0].gold = 5;
    state.players[0].first_refresh_used = true;
    state.players[0].first_refresh_pending = false;

    let events = engine.apply_op(
        &mut state,
        &effect("OnActivate|RefreshShop|self|free=true"),
        &ctx(0),
        None,
    );

    assert!(events.contains(&GameEvent::ShopRolled { seat: 0 }));
    assert_eq!(state.players[0].gold, 5, "免费刷新不扣金币");
    assert!(
        state.players[0].first_refresh_used,
        "免费刷新不消耗本回合免费额度"
    );
}

#[test]
fn effect_refresh_fires_first_refresh_once() {
    let (mut engine, mut state) = setup(1);
    state.players[0].hero = Some("galileo".to_string());
    state.players[0].gold = 5;
    state.players[0].first_refresh_pending = true;

    let events = engine.apply_op(
        &mut state,
        &effect("OnActivate|RefreshShop|self|free=true"),
        &ctx(0),
        None,
    );

    assert!(events.contains(&GameEvent::ShopRolled { seat: 0 }));
    assert_eq!(state.players[0].shop.len(), 7, "效果刷新也触发观测");
    assert!(!state.players[0].first_refresh_pending);

    let events = engine.apply_op(
        &mut state,
        &effect("OnActivate|RefreshShop|self|free=true"),
        &ctx(0),
        None,
    );
    assert!(events.contains(&GameEvent::ShopRolled { seat: 0 }));
    assert_eq!(state.players[0].shop.len(), 6, "首次刷新每回合只触发一次");
}

#[test]
fn refresh_shop_paid_costs_one_gold() {
    let (mut engine, mut state) = setup(1);
    state.players[0].gold = 5;
    state.players[0].first_refresh_used = true;
    state.players[0].first_refresh_pending = false;

    let events = engine.apply_op(
        &mut state,
        &effect("OnActivate|RefreshShop|self"),
        &ctx(0),
        None,
    );

    assert!(events.contains(&GameEvent::ShopRolled { seat: 0 }));
    assert_eq!(state.players[0].gold, 4, "非免费刷新消耗 1 金币");
}

#[test]
fn refresh_shop_without_gold_is_a_no_op() {
    let (mut engine, mut state) = setup(1);
    state.players[0].gold = 0;
    state.players[0].first_refresh_used = true;

    let before = state_hash(&state);
    let events = engine.apply_op(
        &mut state,
        &effect("OnActivate|RefreshShop|self"),
        &ctx(0),
        None,
    );

    assert!(events.is_empty());
    assert_eq!(before, state_hash(&state), "金币不足时状态不变");
}

#[test]
fn add_shop_slot_uses_current_era() {
    let (mut engine, mut state) = setup(2);
    state.players[0].era = Era::Primitive;
    state.players[0].shop = vec![None; 6];

    let expected = first_eligible(&engine, &state, 2).expect("原始时代应有可选随从");
    let pool_before = state.pool.get(&expected).copied().unwrap_or(0);

    let events = engine.apply_op(
        &mut state,
        &effect("OnActivate|AddShopSlot|self|era=current,count=1"),
        &ctx(0),
        None,
    );

    assert!(events.is_empty());
    assert_eq!(state.players[0].shop.len(), 7);
    assert_eq!(
        state.players[0].shop[6]
            .as_ref()
            .map(|slot| slot.card_id.as_str()),
        Some(expected.as_str())
    );
    assert_eq!(
        state.pool.get(&expected).copied().unwrap_or(0),
        pool_before - 1,
        "追加槽位从共享池扣 1 张"
    );
}

#[test]
fn add_shop_slot_is_capped_at_seven() {
    let (mut engine, mut state) = setup(2);
    state.players[0].era = Era::Primitive;
    state.players[0].shop = vec![None; 7];

    let before = state_hash(&state);
    engine.apply_op(
        &mut state,
        &effect("OnActivate|AddShopSlot|self|era=current,count=1"),
        &ctx(0),
        None,
    );

    assert_eq!(before, state_hash(&state), "已达 7 槽时静默跳过");
}

#[test]
fn add_shop_slot_skips_when_no_eligible_card() {
    let (mut engine, mut state) = setup(2);
    state.players[0].era = Era::Primitive;
    state.players[0].shop = vec![None; 6];
    let level_two: Vec<String> = engine
        .catalog
        .cards
        .values()
        .filter(|card| card.level == 2)
        .map(|card| card.id.clone())
        .collect();
    for card_id in level_two {
        state.pool.insert(card_id, 0);
    }

    let before = state_hash(&state);
    engine.apply_op(
        &mut state,
        &effect("OnActivate|AddShopSlot|self|era=current,count=1"),
        &ctx(0),
        None,
    );

    assert_eq!(before, state_hash(&state), "没有合格卡牌时静默跳过");
}

#[test]
fn gain_minion_from_shop_moves_to_hand() {
    let (mut engine, mut state) = setup(2);
    state.players[0].era = Era::Primitive;
    // 商店里放一张 2 星随从：获得它时清空槽位、扣 1 份池。
    let expected = engine
        .catalog
        .cards
        .values()
        .find(|card| {
            card.card_type == CardType::Minion
                && card.level == 2
                && card_in_pool(card, &state.enabled_playstyles)
        })
        .expect("原始时代应有 2 星随从")
        .id
        .clone();
    state.players[0].shop = vec![
        Some(ShopSlot {
            instance_id: 4_242,
            card_id: expected.clone(),
            frozen: true,
        });
        6
    ];
    let pool_before = state.pool.get(&expected).copied().unwrap_or(0);

    let events = engine.apply_op(
        &mut state,
        &effect("OnActivate|GainMinion|self|from=shop,level=current,to=hand"),
        &ctx(0),
        None,
    );

    assert!(events.is_empty(), "获得随从不是打出，不触发 OnPlay");
    let player = &state.players[0];
    assert_eq!(player.hand.len(), 1);
    assert_eq!(player.hand[0].card_id(), expected);
    assert_eq!(
        state.pool.get(&expected).copied().unwrap_or(0),
        pool_before - 1,
        "共享池只扣一次"
    );
    assert_eq!(
        player.shop[0], None,
        "from=shop 获得的是商店槽位本身，槽位随之清空"
    );
}

#[test]
fn gain_minion_runs_triple_check() {
    let (mut engine, mut state) = setup(2);
    state.players[0].era = Era::Primitive;
    let expected = engine
        .catalog
        .cards
        .values()
        .find(|card| {
            card.card_type == CardType::Minion
                && card.level == 2
                && card_in_pool(card, &state.enabled_playstyles)
        })
        .expect("原始时代应有 2 星随从")
        .id
        .clone();
    state.players[0].hand = (0..2)
        .map(|index| {
            HandCard::Minion(MinionInstance::new(
                &test_catalog(),
                9_000 + index as InstanceId,
                &expected,
                false,
            ))
        })
        .collect();
    state.players[0].shop = vec![
        Some(ShopSlot {
            instance_id: 4_242,
            card_id: expected.clone(),
            frozen: false,
        });
        6
    ];

    let events = engine.apply_op(
        &mut state,
        &effect("OnActivate|GainMinion|self|from=shop,level=current,to=hand"),
        &ctx(0),
        None,
    );

    assert!(
        events.iter().any(|event| matches!(
            event,
            GameEvent::TripleFormed { seat: 0, card_id, .. } if card_id == &expected
        )),
        "获得第三张同名随从应触发三连"
    );
    let golden = state.players[0]
        .hand
        .iter()
        .filter_map(HandCard::minion)
        .find(|minion| minion.card_id == expected && minion.golden);
    assert!(golden.is_some(), "三连应合成金色随从并进入手牌");
}

#[test]
fn gain_minion_with_full_hand_is_a_no_op() {
    let (mut engine, mut state) = setup(2);
    state.players[0].era = Era::Primitive;
    let hand_slots = engine.catalog.config.hand_slots;
    state.players[0].hand = (0..hand_slots)
        .map(|_| HandCard::Minion(MinionInstance::new(&test_catalog(), 9_000, "sickly", false)))
        .collect();

    let before = state_hash(&state);
    engine.apply_op(
        &mut state,
        &effect("OnActivate|GainMinion|self|from=shop,level=current,to=hand"),
        &ctx(0),
        None,
    );

    assert_eq!(before, state_hash(&state), "手牌已满时不扣池");
}

// ---------------------------------------------------------------------------
// 传染在战斗间的持久化
// ---------------------------------------------------------------------------

#[test]
fn contagion_persists_across_combat() {
    let (mut engine, mut state) = setup(3);
    state.phase = Phase::Combat;
    state.turn = 2;
    state.first_seat = 0;
    state.last_winner = None;
    state.pairings = vec![Pairing {
        a: 0,
        b: 1,
        ghost: None,
    }];
    place(&engine.catalog, &mut state, 0, 1, "contagious");
    place(&engine.catalog, &mut state, 1, 2, "target");

    engine.resolve_combat(&mut state).expect("战斗结算应成功");

    assert_eq!(
        state.players[1].board[0].contagion, 5,
        "多轮行动中每次主动攻击都叠一层传染，层数写回持久随从"
    );
    assert_eq!(state.players[1].board[0].stats.attack.growth, 0);
    assert_eq!(state.players[1].board[0].stats.health.growth, 0);
    // 战斗内光环折叠与伤害产生的临时层都不写回持久随从。
    let defender = board_minion(&state, 1, 0);
    assert_eq!(defender.stats.attack.turn, 0);
    assert_eq!(defender.stats.health.turn, 0);
    assert_eq!(defender.stats.attack.aura, 0);
    assert_eq!(defender.stats.health.aura, 0);
    let attacker = board_minion(&state, 0, 0);
    assert_eq!(attacker.stats.attack.turn, 0);
    assert_eq!(attacker.stats.health.turn, 0);
    assert_eq!(attacker.stats.attack.aura, 0);
    assert_eq!(attacker.stats.health.aura, 0);
}

#[test]
fn ghost_combat_writes_back_attacker_only() {
    let mut catalog = catalog_with_synthetics();
    // 落单座位：开战成长 +2；幽灵带「传染」，会在交手中给落单座位叠加传染。
    add_card(
        &mut catalog,
        "grower",
        Playstyle::Mammal,
        (1, 2, 20),
        (None, "OnCombatStart|Growth|self|times=2"),
    );
    add_card(
        &mut catalog,
        "ghost-contagious",
        Playstyle::Mammal,
        (1, 1, 20),
        (Some("contagion"), "[]"),
    );
    let (mut engine, mut state) = Engine::new(catalog, [3u8; 32], test_playstyles());
    state.phase = Phase::Combat;
    state.turn = 2;
    for player in &mut state.players {
        player.hp = 30;
        player.gold = 10;
    }
    state.first_seat = 0;
    state.last_winner = None;
    state.pairings = vec![Pairing {
        a: 0,
        b: 0,
        ghost: Some(3),
    }];
    place(&engine.catalog, &mut state, 0, 1, "grower");
    state.players[3].eliminated = true;
    place(&engine.catalog, &mut state, 3, 2, "ghost-contagious");
    let ghost_before = state.players[3].board.clone();

    engine.resolve_combat(&mut state).expect("幽灵战斗应成功");

    let own = &state.players[0].board[0];
    assert_eq!(
        own.stats.attack.growth, 2,
        "落单座位的战斗成长应写回持久随从"
    );
    assert_eq!(
        own.stats.health.growth, 2,
        "落单座位的战斗成长应写回持久随从"
    );
    assert!(own.contagion > 0, "幽灵施加的传染应写回落单座位的持久随从");
    assert_eq!(
        state.players[3].board, ghost_before,
        "幽灵座位棋盘不得被写回"
    );
}

#[test]
fn turn_start_keeps_b_tree_ordering() {
    // 座位顺序固定：先结算座位 0，再结算座位 1。
    let (mut engine, mut state) = setup(1);
    place(&engine.catalog, &mut state, 1, 2, "turn7");
    place(&engine.catalog, &mut state, 0, 1, "turn9");
    state.players[1].board[0].breed = 0;

    let events = engine.start_turn(&mut state).expect("开始回合应成功");
    let seats: Vec<Seat> = events
        .into_iter()
        .filter_map(|event| match event {
            GameEvent::DamageTaken { seat, .. } => Some(seat),
            _ => None,
        })
        .collect();
    assert_eq!(seats, vec![0, 1], "座位升序结算");
}

// ---------------------------------------------------------------------------
// 潜伏 / 零件回收
// ---------------------------------------------------------------------------

#[test]
fn latent_adds_attack_per_contagion_layer_and_bursts_at_threshold() {
    let (mut engine, mut state) = setup(1);
    place(&engine.catalog, &mut state, 0, 1, "latent");
    place(&engine.catalog, &mut state, 1, 2, "target");
    state.players[0].board[0].contagion = 2;

    engine.start_turn(&mut state).expect("开始回合应成功");
    let latent = &state.players[0].board[0];
    assert_eq!(
        combat::effective_attack(&engine.catalog, latent),
        2,
        "2 层获得 +2 攻击"
    );
    assert_eq!(latent.contagion, 2, "未达阈值时层数保留");
    assert!(!latent.latent_used, "未爆发时不置标记");
    assert_eq!(
        combat::effective_health(&engine.catalog, &state.players[1].board[0]),
        10,
        "未达阈值不对敌方造成伤害"
    );

    // 手动补到 3 层：下一次回合开始应爆发。
    state.players[0].board[0].contagion = 3;
    engine.start_turn(&mut state).expect("开始回合应成功");
    let latent = &state.players[0].board[0];
    assert_eq!(
        combat::effective_attack(&engine.catalog, latent),
        5,
        "3 层再获得 +3 攻击（累计 5）"
    );
    assert_eq!(latent.contagion, 0, "爆发后移除全部层数");
    assert!(latent.latent_used, "爆发后置 latent_used");
    assert_eq!(
        combat::effective_health(&engine.catalog, &state.players[1].board[0]),
        7,
        "爆发对敌方全体造成 3 点伤害"
    );

    // 再次达到阈值：攻击照常按层增长，但爆发每局只触发一次。
    state.players[0].board[0].contagion = 3;
    engine.start_turn(&mut state).expect("开始回合应成功");
    let latent = &state.players[0].board[0];
    assert_eq!(
        combat::effective_attack(&engine.catalog, latent),
        8,
        "爆发用尽后仍按层获得攻击力"
    );
    assert_eq!(latent.contagion, 3, "未爆发时不清除层数");
    assert_eq!(
        combat::effective_health(&engine.catalog, &state.players[1].board[0]),
        7,
        "爆发每局只触发一次"
    );
}

#[test]
fn latent_burst_removes_killed_minion_and_fires_deathrattle() {
    // 潜伏在座位 1，受害者座位 0：座位 0 的传染结算已结束，无法替爆发「清理」
    // 尸体，因此这条用例能区分爆发自身是否结算了离场与亡语。
    let (mut engine, mut state) = setup(1);
    place(&engine.catalog, &mut state, 1, 1, "latent");
    place(&engine.catalog, &mut state, 0, 2, "frail");
    place(&engine.catalog, &mut state, 0, 3, "witness");
    state.players[1].board[0].contagion = 3;

    engine.start_turn(&mut state).expect("开始回合应成功");

    let board = &state.players[0].board;
    assert_eq!(board.len(), 1, "2 血目标被击杀后立即离场");
    assert_eq!(board[0].instance_id, 3);
    assert_eq!(
        combat::effective_attack(&engine.catalog, &board[0]),
        1,
        "击杀触发亡语（友方全体 +1 攻击）"
    );
}

#[test]
fn latent_burst_respects_armor() {
    let (mut engine, mut state) = setup(1);
    place(&engine.catalog, &mut state, 0, 1, "latent");
    place(&engine.catalog, &mut state, 1, 2, "armored");
    state.players[0].board[0].contagion = 3;

    engine.start_turn(&mut state).expect("开始回合应成功");

    assert_eq!(
        combat::effective_health(&engine.catalog, &state.players[1].board[0]),
        9,
        "减伤 2 使 3 点爆发只造成 1 点伤害"
    );
}

#[test]
fn latent_burst_consumes_divine_shield() {
    let (mut engine, mut state) = setup(1);
    place(&engine.catalog, &mut state, 0, 1, "latent");
    place(&engine.catalog, &mut state, 1, 2, "shielded");
    state.players[0].board[0].contagion = 3;

    engine.start_turn(&mut state).expect("开始回合应成功");

    let target = &state.players[1].board[0];
    assert_eq!(
        combat::effective_health(&engine.catalog, target),
        10,
        "护盾抵消爆发的全部伤害"
    );
    assert!(target.shield_used, "护盾被消耗一次");
}

#[test]
fn latent_burst_with_zero_count_keeps_layers_and_marker() {
    let (mut engine, mut state) = setup(1);
    place(&engine.catalog, &mut state, 0, 1, "latent0");
    place(&engine.catalog, &mut state, 1, 2, "target");
    state.players[0].board[0].contagion = 3;

    engine.start_turn(&mut state).expect("开始回合应成功");

    let latent = &state.players[0].board[0];
    assert_eq!(latent.contagion, 3, "count=0 不清除层数");
    assert!(!latent.latent_used, "count=0 不消耗爆发次数");
    assert_eq!(
        combat::effective_health(&engine.catalog, &state.players[1].board[0]),
        10,
        "count=0 不造成伤害"
    );
}

#[test]
fn golden_latent_doubles_attack_but_bursts_once() {
    // 金色按通用规则触发两遍：未达阈值时每层攻击力获得双倍；达到阈值时
    // 第一遍爆发并清零层数、置 latent_used，第二遍层数为 0，因此只爆发一次。
    let (mut engine, mut state) = setup(1);
    place(&engine.catalog, &mut state, 0, 1, "latent");
    place(&engine.catalog, &mut state, 1, 2, "target");
    state.players[0].board[0].golden = true;
    state.players[0].board[0].contagion = 2;

    engine.start_turn(&mut state).expect("开始回合应成功");

    let latent = &state.players[0].board[0];
    assert_eq!(
        combat::effective_attack(&engine.catalog, latent),
        4,
        "金色触发两遍，2 层获得 2 × 2 = 4 攻击力"
    );
    assert!(!latent.latent_used);

    state.players[0].board[0].contagion = 3;
    engine.start_turn(&mut state).expect("开始回合应成功");
    let latent = &state.players[0].board[0];
    assert_eq!(latent.contagion, 0, "第一遍爆发后清零层数");
    assert!(latent.latent_used, "第一遍爆发后置 latent_used");
    assert_eq!(
        combat::effective_attack(&engine.catalog, latent),
        7,
        "爆发遍按 3 层 +3，第二遍层数已清零只加 0"
    );
    assert_eq!(
        combat::effective_health(&engine.catalog, &state.players[1].board[0]),
        7,
        "爆发只结算一次，敌方全体只受到 3 点伤害"
    );
}

/// 本局启用玩法（含机械），供零件回收的随机池筛选使用。
fn machine_playstyles() -> BTreeSet<Playstyle> {
    let mut enabled: BTreeSet<Playstyle> = test_playstyles().into_iter().collect();
    enabled.insert(Playstyle::Machine);
    enabled
}

/// 用一名必然阵亡的随从触发 `OnDeath | Salvage`，返回目录与战斗日志。
fn run_salvage(seed: u8, hand: usize, hand_slots: usize, golden: bool) -> (Engine, CombatLog) {
    let (engine, _) = setup(seed);
    let attacker = vec![MinionInstance::new(&engine.catalog, 1, "scavenger", golden)];
    let defender = vec![MinionInstance::new(&engine.catalog, 2, "killer", false)];
    let mut rng = RngStreams::from_seed([seed; 32]);
    let playstyles = machine_playstyles();
    let log = combat::simulate_with(
        &engine,
        &attacker,
        &defender,
        0,
        1,
        0,
        &mut rng,
        CombatSetup {
            attacker_hand: hand,
            defender_hand: 0,
            hand_slots,
            playstyles: Some(&playstyles),
            ..CombatSetup::default()
        },
    );
    (engine, log)
}

#[test]
fn salvage_adds_only_machine_minions_to_hand() {
    for seed in 0..8 {
        let (engine, log) = run_salvage(seed, 0, 10, false);
        assert_eq!(log.hand_copies.len(), 1, "死亡后应获得 1 张牌");
        let copy = &log.hand_copies[0];
        assert_eq!(copy.seat, 0, "牌进入来源座位的手牌");
        assert_eq!(copy.side, 0, "记录来源阵营");
        let card = engine
            .catalog
            .cards
            .get(&copy.card_id)
            .expect("获得的牌应在目录中");
        assert!(
            card_has_play(card, "machine"),
            "随机池只含带 play:machine 的牌"
        );
        assert!(
            !engine.catalog.tokens.contains_key(&copy.card_id),
            "排除 Token"
        );
        assert_eq!(
            card.card_type,
            CardType::Minion,
            "奖励牌（效果牌）不会进入随机池"
        );
        assert!(
            log.actions
                .iter()
                .any(|action| matches!(action, CombatAction::Salvaged { seat: 0, .. })),
            "记录 Salvaged 动作"
        );
    }
}

#[test]
fn salvage_pool_follows_carrier_playstyle() {
    let (engine, _) = setup(3);
    let attacker = vec![MinionInstance::new(
        &engine.catalog,
        1,
        "mammal-salvager",
        false,
    )];
    let defender = vec![MinionInstance::new(&engine.catalog, 2, "killer", false)];
    let mut rng = RngStreams::from_seed([3; 32]);
    let mut playstyles = machine_playstyles();
    playstyles.insert(Playstyle::Mammal);
    let log = combat::simulate_with(
        &engine,
        &attacker,
        &defender,
        0,
        1,
        0,
        &mut rng,
        CombatSetup {
            attacker_hand: 0,
            defender_hand: 0,
            hand_slots: 10,
            playstyles: Some(&playstyles),
            ..CombatSetup::default()
        },
    );

    assert_eq!(log.hand_copies.len(), 1, "死亡后应获得 1 张牌");
    let card = engine
        .catalog
        .cards
        .get(&log.hand_copies[0].card_id)
        .expect("获得的牌应在目录中");
    assert!(
        card_has_play(card, "mammal"),
        "随机池应跟随载体随从的玩法，而不是写死机械"
    );
}

#[test]
fn salvage_with_full_hand_adds_nothing() {
    let (_, log) = run_salvage(3, 1, 1, false);

    assert!(log.hand_copies.is_empty(), "手牌已满时不获得");
    assert!(
        !log.actions
            .iter()
            .any(|action| matches!(action, CombatAction::Salvaged { .. })),
        "手牌已满时不记录 Salvaged"
    );
}

#[test]
fn golden_salvage_adds_two_cards() {
    let (_, log) = run_salvage(3, 0, 10, true);

    assert_eq!(log.hand_copies.len(), 2, "金色触发两次，获得两张牌");
    assert!(
        log.actions
            .iter()
            .filter(|action| matches!(action, CombatAction::Salvaged { .. }))
            .count()
            == 2,
        "每次获得各记录一条 Salvaged"
    );
}

#[test]
fn salvage_is_deterministic_for_same_seed() {
    let (_, first) = run_salvage(9, 0, 10, false);
    let (_, second) = run_salvage(9, 0, 10, false);

    let ids = |log: &CombatLog| -> Vec<String> {
        log.hand_copies
            .iter()
            .map(|copy| copy.card_id.clone())
            .collect()
    };
    assert_eq!(ids(&first), ids(&second), "同一随机种子结果一致");
}
