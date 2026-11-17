mod common;

use common::test_catalog;
use rule_engine::combat;
use rule_engine::data::{Card, CardType, Catalog, Era, Skill, Tribe};
use rule_engine::effects::{Effect, EffectCtx, parse_effects};
use rule_engine::engine::Engine;
use rule_engine::events::GameEvent;
use rule_engine::hash::state_hash;
use rule_engine::intent::Intent;
use rule_engine::settle::Pairing;
use rule_engine::state::{GameState, HandCard, InstanceId, MinionInstance, Phase, Seat, ShopSlot};

/// 往目录里插入一张合成随从卡，带一个技能（`keyword` + `effects`）。
fn add_card(
    catalog: &mut Catalog,
    id: &str,
    tribe: Tribe,
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
            tribe,
            subtype: None,
            level,
            era: None,
            attack: Some(attack),
            health: Some(health),
            skills: vec![skill_id],
            is_scholar: tribe == Tribe::Scholar,
            is_hero: false,
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
        Tribe::Human,
        (1, 1, 2),
        (Some("breed 2"), "[]"),
    );
    add_card(
        &mut catalog,
        "breed1",
        Tribe::Human,
        (1, 1, 2),
        (Some("breed 1"), "[]"),
    );
    add_card(
        &mut catalog,
        "scholar-breed",
        Tribe::Scholar,
        (1, 0, 4),
        (Some("breed 2"), "[]"),
    );
    add_card(
        &mut catalog,
        "sickly",
        Tribe::Human,
        (1, 1, 2),
        (None, "[]"),
    );
    add_card(
        &mut catalog,
        "turn9",
        Tribe::Human,
        (1, 0, 20),
        (None, "OnTurnStart|DealDamage|friendly:leftmost|amount=9"),
    );
    add_card(
        &mut catalog,
        "turn7",
        Tribe::Human,
        (1, 0, 20),
        (None, "OnTurnStart|DealDamage|friendly:leftmost|amount=7"),
    );
    add_card(
        &mut catalog,
        "contagious",
        Tribe::Human,
        (1, 1, 100),
        (Some("contagion"), "[]"),
    );
    add_card(
        &mut catalog,
        "target",
        Tribe::Human,
        (1, 0, 10),
        (None, "[]"),
    );
    catalog
}

fn setup(seed: u8) -> (Engine, GameState) {
    let (engine, mut state) = Engine::new(catalog_with_synthetics(), [seed; 32]);
    state.phase = Phase::Recruit;
    state.turn = 1;
    for player in &mut state.players {
        player.hp = 30;
        player.gold = 10;
    }
    (engine, state)
}

fn minion(catalog: &Catalog, instance_id: InstanceId, card_id: &str) -> MinionInstance {
    MinionInstance::new(catalog, instance_id, card_id, false)
}

fn place(catalog: &Catalog, state: &mut GameState, seat: Seat, id: InstanceId, card_id: &str) {
    state.players[usize::from(seat)]
        .board
        .push(minion(catalog, id, card_id));
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
                && !card.is_hero
                && card.level == star
                && state.pool.get(&card.id).copied().unwrap_or(0) > 0
                && (card.tribe == Tribe::General || state.enabled_tribes.contains(&card.tribe))
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
                && !card.is_hero
                && card.level == 2
                && (card.tribe == Tribe::General || state.enabled_tribes.contains(&card.tribe))
        })
        .expect("原始时代应有 2 星随从")
        .id
        .clone();
    state.players[0].shop = vec![
        Some(ShopSlot {
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
    assert_eq!(player.hand[0].card_id, expected);
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
                && !card.is_hero
                && card.level == 2
                && (card.tribe == Tribe::General || state.enabled_tribes.contains(&card.tribe))
        })
        .expect("原始时代应有 2 星随从")
        .id
        .clone();
    state.players[0].hand = (0..2)
        .map(|_| HandCard {
            card_id: expected.clone(),
            golden: false,
        })
        .collect();
    state.players[0].shop = vec![
        Some(ShopSlot {
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
            GameEvent::TripleFormed { seat: 0, card_id } if card_id == &expected
        )),
        "获得第三张同名随从应触发三连"
    );
    let golden = state.players[0]
        .board
        .iter()
        .find(|minion| minion.card_id == expected && minion.golden);
    assert!(golden.is_some(), "三连应合成金色随从");
}

#[test]
fn gain_minion_with_full_hand_is_a_no_op() {
    let (mut engine, mut state) = setup(2);
    state.players[0].era = Era::Primitive;
    let hand_slots = engine.catalog.config.hand_slots;
    state.players[0].hand = (0..hand_slots)
        .map(|_| HandCard {
            card_id: "sickly".to_string(),
            golden: false,
        })
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
        Tribe::Human,
        (1, 2, 20),
        (None, "OnCombatStart|Growth|self|times=2"),
    );
    add_card(
        &mut catalog,
        "ghost-contagious",
        Tribe::Human,
        (1, 1, 20),
        (Some("contagion"), "[]"),
    );
    let (mut engine, mut state) = Engine::new(catalog, [3u8; 32]);
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

/// 测试夹具用 DSL 构造结构化效果；加载链路本身不再解析 DSL。
fn effects_of(raw: &str) -> Vec<rule_engine::effects::Effect> {
    rule_engine::effects::parse_effects(raw).expect("测试效果应可解析")
}
