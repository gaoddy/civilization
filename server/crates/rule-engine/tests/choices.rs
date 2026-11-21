//! 发现、复制与待选交互（Task 14）。
//!
//! 覆盖 `Op::Discover` 产生的 `PendingChoice::Discover`、`ChooseDiscover` 的
//! FIFO 消费与拒绝码、`Op::GainMinion`（`from=shop`，`to` / `level` 组合），
//! 以及进化（`Evolve`）与 `OnAllyEvolve` 的招募侧结算。
//!
//! `GainMinion` 的 `from` 只接受 `shop`：其它取值在加载期报错的用例见文件末尾。

mod common;

use common::{
    CARDS_JSON, CONFIG_JSON, HEROES_JSON, SKILLS_JSON, TAXONOMY_JSON, categories_for, effects_of,
    test_catalog, test_playstyles, turn_started,
};
use rule_engine::data::{Card, CardType, Catalog, Playstyle, Skill, SkillId, card_in_pool};
use rule_engine::effects::{Effect, EffectCtx, TargetRef, Zone, parse_effects};
use rule_engine::engine::Engine;
use rule_engine::events::GameEvent;
use rule_engine::hash::state_hash;
use rule_engine::intent::{Intent, RejectCode};
use rule_engine::rng::Stream;
use rule_engine::state::{
    GameState, HandCard, InstanceId, MinionInstance, PendingChoice, Phase, Seat, ShopSlot,
};

// ---------------------------------------------------------------------------
// 测试夹具
// ---------------------------------------------------------------------------

/// 构造一个技能。
fn skill(id: &str, keyword: Option<&str>, effects: &str, cost: Option<u8>) -> Skill {
    Skill {
        id: id.to_string(),
        keyword: keyword.map(str::to_string),
        effects: effects_of(effects),
        cost,
        uses_per_turn: cost.map(|_| 1),
    }
}

/// 把技能写入目录，返回其整数标识。
fn add_skill(catalog: &mut Catalog, skill: Skill) -> SkillId {
    catalog.insert_skill(skill)
}

/// 往目录里插入一张合成随从卡。
fn add_minion(
    catalog: &mut Catalog,
    id: &str,
    playstyle: Playstyle,
    stats: (u8, i32, i32),
    skills: &[&str],
) {
    let (level, attack, health) = stats;
    let resolved: Vec<SkillId> = skills
        .iter()
        .map(|skill| {
            catalog
                .skill_id(skill)
                .expect("测试卡引用的技能应先写入目录")
        })
        .collect();
    catalog.cards.insert(
        id.to_string(),
        Card {
            id: id.to_string(),
            card_type: CardType::Minion,
            in_shop_pool: true,
            taxon: "hominidae".to_string(),
            categories: categories_for(playstyle),
            subtype: None,
            level,
            era: None,
            attack: Some(attack),
            health: Some(health),
            skills: resolved,
            cost: None,
        },
    );
    catalog.config.pool_counts[usize::from(level) - 1] = 16;
}

fn first_minion_of_level(
    catalog: &Catalog,
    level: u8,
    enabled: &std::collections::BTreeSet<Playstyle>,
) -> String {
    catalog
        .cards
        .values()
        .find(|card| {
            card.card_type == CardType::Minion
                && card.in_shop_pool
                && card.level == level
                && card_in_pool(card, enabled)
        })
        .expect("数据中应存在该星级随从")
        .id
        .clone()
}

fn set_shop(state: &mut GameState, seat: usize, slots: Vec<Option<&str>>) {
    state.players[seat].shop = slots
        .into_iter()
        .map(|card_id| {
            card_id.map(|card_id| {
                let instance_id = state.next_instance_id;
                state.next_instance_id = instance_id.saturating_add(1);
                ShopSlot {
                    instance_id,
                    card_id: card_id.to_string(),
                    frozen: false,
                }
            })
        })
        .collect();
}

fn dummy(catalog: &Catalog, instance_id: InstanceId, card_id: &str) -> MinionInstance {
    MinionInstance::new(catalog, instance_id, card_id, false)
}

fn hand_card(card_id: &str) -> HandCard {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(20_000);
    HandCard::Minion(MinionInstance::new(
        &test_catalog(),
        NEXT.fetch_add(1, Ordering::Relaxed),
        card_id,
        false,
    ))
}

fn effect(raw: &str) -> Effect {
    parse_effects(raw)
        .expect("效果应能解析")
        .into_iter()
        .next()
        .expect("应至少有一段效果")
}

fn ctx(seat: u8) -> EffectCtx {
    EffectCtx {
        seat,
        source: None,
        target: None,
        event_target: None,
        golden: false,
        level_override: None,
    }
}

/// 取第 `index` 条待选的发现候选；不是发现待选时返回 `None`。
fn discover_candidates_at(state: &GameState, index: usize) -> Option<Vec<String>> {
    match state.pending.get(index) {
        Some(PendingChoice::Discover { candidates, .. }) => Some(candidates.clone()),
        _ => None,
    }
}

/// 第 `index` 条待选的进化剩余次数；不是进化待选时返回 `None`。
fn evolve_remaining(state: &GameState, index: usize) -> Option<u8> {
    match state.pending.get(index) {
        Some(PendingChoice::Evolve { remaining, .. }) => Some(*remaining),
        _ => None,
    }
}

/// 进化夹具中的观察者。
fn watcher(state: &GameState) -> &MinionInstance {
    state.players[0]
        .board
        .iter()
        .find(|minion| minion.card_id == "watcher")
        .expect("观察者仍在场")
}

fn pending_count(state: &GameState) -> usize {
    state.pending.len()
}

fn pool_count(state: &GameState, card_id: &str) -> u16 {
    state.pool.get(card_id).copied().unwrap_or(0)
}

fn discover_effect() -> Effect {
    effect("OnActivate|Discover|self|pool=playstyle,choose=1")
}

// ---------------------------------------------------------------------------
// 发现与待选
// ---------------------------------------------------------------------------

#[test]
fn discover_pool_uses_enabled_playstyles() {
    let (engine, mut state) = turn_started(2);

    let events = engine.apply_op(&mut state, &discover_effect(), &ctx(0), None);

    assert!(events.is_empty(), "发现只登记待选，不直接产生事件");
    assert_eq!(pending_count(&state), 1, "应登记一条发现待选");
    let PendingChoice::Discover {
        seat, candidates, ..
    } = &state.pending[0]
    else {
        panic!("第 0 条待选应为发现");
    };
    assert_eq!(*seat, 0);
    assert!(!candidates.is_empty(), "应至少有候选");
    assert!(candidates.len() <= 3, "三选一最多三个候选");
    let mut unique = candidates.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), candidates.len(), "候选不得重复");

    for card_id in candidates {
        let card = engine.catalog.cards.get(card_id).expect("候选应在目录中");
        assert_eq!(card.card_type, CardType::Minion);
        assert!(
            card_in_pool(card, &state.enabled_playstyles),
            "候选 {card_id} 的玩法 {:?} 未在本局启用（{:?}）",
            card.categories,
            state.enabled_playstyles
        );
        assert!(pool_count(&state, card_id) > 0, "候选必须在共享池中");
    }
}

#[test]
fn discover_follows_injected_random_stream() {
    let (engine, mut state) = Engine::new(test_catalog(), [2; 32], test_playstyles());
    engine
        .start_game(&mut state, &common::heroes())
        .expect("开局应成功");
    engine.start_turn(&mut state).expect("开始回合应成功");
    let before: Vec<u64> = Stream::ALL
        .iter()
        .map(|stream| state.rng.clone().next_u64(*stream))
        .collect();

    engine.apply_op(&mut state, &discover_effect(), &ctx(0), None);

    // 候选来自注入的 Stream::Discover：抽取时推进该流，其余流不变。
    for (index, stream) in Stream::ALL.iter().enumerate() {
        let now = state.rng.clone().next_u64(*stream);
        if *stream == Stream::Discover {
            assert_ne!(before[index], now, "发现应推进 Stream::Discover");
        } else {
            assert_eq!(before[index], now, "{stream:?} 不应被发现推进");
        }
    }

    // 同一起点、同一随机流必须得到同一组候选。
    let (engine_again, mut fresh) = Engine::new(test_catalog(), [2; 32], test_playstyles());
    engine_again
        .start_game(&mut fresh, &common::heroes())
        .expect("开局应成功");
    engine_again.start_turn(&mut fresh).expect("开始回合应成功");
    engine_again.apply_op(&mut fresh, &discover_effect(), &ctx(0), None);
    assert_eq!(state.pending, fresh.pending, "发现候选必须确定");
}

#[test]
fn discover_choose_one_returns_one_card() {
    let (engine, mut state) = turn_started(2);
    let board = state.players[0].board.len();
    engine.apply_op(&mut state, &discover_effect(), &ctx(0), None);
    let candidates = discover_candidates_at(&state, 0).expect("第 0 条待选应为发现");
    let chosen = candidates[1].clone();
    let pool_before = pool_count(&state, &chosen);

    let outcome = engine
        .apply(&mut state, 0, Intent::ChooseDiscover { choice: 1 })
        .expect("选择应成功");

    assert_eq!(state.players[0].hand.len(), 1, "选择后手牌 +1");
    assert_eq!(state.players[0].hand[0].card_id(), chosen);
    assert!(!state.players[0].hand[0].golden());
    assert_eq!(pending_count(&state), 0, "选择后待选 -1");
    assert_eq!(
        pool_count(&state, &chosen),
        pool_before - 1,
        "发现扣 1 份池"
    );
    assert_eq!(state.players[0].board.len(), board, "发现进手牌不进战场");
    assert_eq!(
        outcome.events,
        vec![GameEvent::CardDiscovered {
            seat: 0,
            card_id: chosen
        }]
    );
}

#[test]
fn discover_consumes_pending_fifo() {
    let (engine, mut state) = turn_started(2);
    engine.apply_op(&mut state, &discover_effect(), &ctx(0), None);
    engine.apply_op(&mut state, &discover_effect(), &ctx(0), None);
    let first = discover_candidates_at(&state, 0).expect("第 0 条待选应为发现")[0].clone();
    let second = discover_candidates_at(&state, 1).expect("第 1 条待选应为发现")[0].clone();

    engine
        .apply(&mut state, 0, Intent::ChooseDiscover { choice: 0 })
        .expect("第一次选择应成功");
    engine
        .apply(&mut state, 0, Intent::ChooseDiscover { choice: 0 })
        .expect("第二次选择应成功");

    assert_eq!(
        state.players[0]
            .hand
            .iter()
            .map(|card| card.card_id())
            .collect::<Vec<_>>(),
        vec![first.as_str(), second.as_str()],
        "按 FIFO 消费；池被扣后第二组候选不受影响"
    );
    assert_eq!(pending_count(&state), 0);
}

#[test]
fn choose_without_pending_rejects_invalid_stage() {
    let (engine, mut state) = turn_started(2);
    let before = state_hash(&state);

    let reject = engine
        .apply(&mut state, 0, Intent::ChooseDiscover { choice: 0 })
        .expect_err("没有待选时必须拒绝");

    assert_eq!(reject.code, RejectCode::InvalidStage);
    assert_eq!(before, state_hash(&state), "拒绝时不改变状态");
}

#[test]
fn choose_out_of_range_rejects_invalid_choice() {
    let (engine, mut state) = turn_started(2);
    engine.apply_op(&mut state, &discover_effect(), &ctx(0), None);
    let index = discover_candidates_at(&state, 0)
        .expect("第 0 条待选应为发现")
        .len();
    let before = state_hash(&state);

    let reject = engine
        .apply(
            &mut state,
            0,
            Intent::ChooseDiscover {
                choice: index as u8,
            },
        )
        .expect_err("下标越界必须拒绝");

    assert_eq!(reject.code, RejectCode::InvalidChoice);
    assert_eq!(before, state_hash(&state), "拒绝时不消费待选");
}

#[test]
fn choose_with_full_hand_rejects_no_space() {
    let (engine, mut state) = turn_started(2);
    let hand_slots = engine.catalog.config.hand_slots;
    state.players[0].hand = (0..hand_slots).map(|_| hand_card("sickly")).collect();
    engine.apply_op(&mut state, &discover_effect(), &ctx(0), None);
    let before = state_hash(&state);

    let reject = engine
        .apply(&mut state, 0, Intent::ChooseDiscover { choice: 0 })
        .expect_err("手牌满必须拒绝");

    assert_eq!(reject.code, RejectCode::NoSpace);
    assert_eq!(before, state_hash(&state));
}

#[test]
fn choose_in_combat_rejects_invalid_stage() {
    let (engine, mut state) = turn_started(2);
    engine.apply_op(&mut state, &discover_effect(), &ctx(0), None);
    state.phase = Phase::Combat;

    let reject = engine
        .apply(&mut state, 0, Intent::ChooseDiscover { choice: 0 })
        .expect_err("战斗阶段不接受选择");

    assert_eq!(reject.code, RejectCode::InvalidStage);
    assert_eq!(pending_count(&state), 1);
}

#[test]
fn discover_fires_from_hero_power() {
    let (mut engine, mut state) = turn_started(2);
    let skill_id = add_skill(
        &mut engine.catalog,
        skill(
            "test-discover",
            None,
            "OnActivate|Discover|self|pool=playstyle,choose=1",
            Some(0),
        ),
    );
    let hero = engine
        .catalog
        .heroes
        .values()
        .next()
        .expect("数据中应有英雄")
        .id
        .clone();
    engine
        .catalog
        .heroes
        .get_mut(&hero)
        .expect("英雄存在")
        .skills
        .push(skill_id);
    state.players[0].hero = Some(hero);
    state.players[0].gold = 10;

    engine
        .apply(
            &mut state,
            0,
            Intent::UseHeroPower {
                skill_id: "test-discover".to_string(),
                target: None,
            },
        )
        .expect("主动技能应成功");

    assert_eq!(pending_count(&state), 1, "主动发现应登记待选");
    assert!(
        state
            .pending
            .iter()
            .all(|pending| matches!(pending, PendingChoice::Discover { seat: 0, .. }))
    );
}

#[test]
fn triple_reward_is_played_and_grants_next_level_minion() {
    let (engine, mut state) = turn_started(2);
    let card_id = first_minion_of_level(&engine.catalog, 1, &state.enabled_playstyles);
    state.players[0].hand = (0..2).map(|_| hand_card(&card_id)).collect();

    // 手牌已有两张，从商店买下第三张即形成三连：金色入手牌，不再登记发现待选。
    set_shop(
        &mut state,
        0,
        vec![Some(&card_id), None, None, None, None, None],
    );
    let outcome = engine
        .apply(&mut state, 0, Intent::BuyMinion { slot: 0 })
        .expect("购买应成功");
    assert!(
        outcome
            .events
            .iter()
            .any(|event| matches!(event, GameEvent::TripleFormed { .. }))
    );
    assert_eq!(pending_count(&state), 0, "三连不再立即登记发现待选");

    // 打出金色：发放奖励牌。
    engine
        .apply(
            &mut state,
            0,
            Intent::PlayCard {
                hand_index: 0,
                position: None,
                target: None,
            },
        )
        .expect("打出金色应成功");
    assert_eq!(pending_count(&state), 0, "奖励牌入手时还没有待选");

    // 打出奖励牌：直接随机获得一张下一等级、命中本局玩法的池内随从。
    engine
        .apply(
            &mut state,
            0,
            Intent::PlayCard {
                hand_index: 0,
                position: None,
                target: None,
            },
        )
        .expect("打出奖励牌应成功");
    assert_eq!(pending_count(&state), 0, "奖励牌不再登记发现待选");
    assert_eq!(state.players[0].hand.len(), 1, "奖励换成一张手牌随从");
    let granted = engine
        .catalog
        .cards
        .get(state.players[0].hand[0].card_id())
        .expect("获得的随从应在目录中");
    assert_eq!(granted.level, 2, "来源 1 星 +1 后为 2 星");
    assert!(granted.in_shop_pool, "奖励只会发出池内随从");
}

// ---------------------------------------------------------------------------
// GainMinion
// ---------------------------------------------------------------------------

#[test]
fn gain_minion_level_current_uses_player_era() {
    let (mut engine, mut state) = turn_started(2);
    // 1 星与 2 星同时在货，玩家处于原始：若误用「回合对应的时代」会取 1 星。
    // 史前的商店等级同样是 1 星，因此必须把玩家放到原始才能区分两种口径。
    let one_star = first_minion_of_level(&engine.catalog, 1, &state.enabled_playstyles);
    let enabled = *state
        .enabled_playstyles
        .iter()
        .next()
        .expect("至少一个启用玩法");
    add_minion(
        &mut engine.catalog,
        "probe-two-star",
        enabled,
        (2, 4, 4),
        &[],
    );
    set_shop(
        &mut state,
        0,
        vec![
            Some("probe-two-star"),
            Some(&one_star),
            None,
            None,
            None,
            None,
        ],
    );
    state.players[0].era = rule_engine::data::Era::Primitive;

    let outcome = engine.apply_op(
        &mut state,
        &effect("OnActivate|GainMinion|self|from=shop,level=current,to=hand"),
        &ctx(0),
        None,
    );

    assert!(outcome.is_empty(), "获得随从不是打出，不产生事件");
    let gained = state.players[0].hand.first().expect("应获得一张随从");
    let card = engine
        .catalog
        .cards
        .get(gained.card_id())
        .expect("获得卡应在目录中");
    assert_eq!(
        card.id, "probe-two-star",
        "level=current 取玩家当前时代（原始 2 星）"
    );
}

#[test]
fn gain_minion_from_shop_removes_shop_slot() {
    let (engine, mut state) = turn_started(2);
    let card_id = first_minion_of_level(&engine.catalog, 1, &state.enabled_playstyles);
    set_shop(
        &mut state,
        0,
        vec![Some(&card_id), None, Some(&card_id), None, None, None],
    );
    let pool_before = pool_count(&state, &card_id);

    engine.apply_op(
        &mut state,
        &effect("OnActivate|GainMinion|self|from=shop,level=1,to=hand"),
        &ctx(0),
        None,
    );

    let shop = &state.players[0].shop;
    assert_eq!(shop[0], None, "获得商店槽位里的随从会清空该槽位");
    let untouched = shop[2].as_ref().expect("其余槽位不受影响");
    assert_eq!(untouched.card_id, card_id);
    assert!(!untouched.frozen);
    assert_eq!(state.players[0].hand.len(), 1);
    assert_eq!(state.players[0].hand[0].card_id(), card_id);
    assert_eq!(
        state.players[0].hand[0].minion().unwrap().pool_copies,
        1,
        "商店槽位已在抽取时扣池，获得时不再扣池"
    );
    assert_eq!(
        pool_count(&state, &card_id),
        pool_before,
        "池只扣一次（抽取时）"
    );
}

#[test]
fn gain_minion_from_empty_shop_is_a_no_op() {
    let (engine, mut state) = turn_started(2);
    set_shop(&mut state, 0, vec![None; 6]);
    state.players[0].shop.clear();
    let before = state_hash(&state);

    engine.apply_op(
        &mut state,
        &effect("OnActivate|GainMinion|self|from=shop,level=1,to=hand"),
        &ctx(0),
        None,
    );

    assert_eq!(before, state_hash(&state), "商店为空时不凭空获得");
}

#[test]
fn gain_minion_to_board_places_new_minion() {
    let (engine, mut state) = turn_started(2);
    let card_id = first_minion_of_level(&engine.catalog, 1, &state.enabled_playstyles);
    set_shop(
        &mut state,
        0,
        vec![Some(&card_id), None, None, None, None, None],
    );

    engine.apply_op(
        &mut state,
        &effect("OnActivate|GainMinion|self|from=shop,level=1,to=board"),
        &ctx(0),
        None,
    );

    assert_eq!(state.players[0].board.len(), 1, "to=board 直接进入战场");
    assert_eq!(state.players[0].board[0].card_id, card_id);
    assert_eq!(state.players[0].shop[0], None, "来源槽位被清空");
    assert!(state.players[0].hand.is_empty());
    assert!(!state.players[0].board[0].golden);
}

#[test]
fn gain_minion_to_board_with_full_board_is_a_no_op() {
    let (engine, mut state) = turn_started(2);
    let board_slots = engine.catalog.config.board_slots;
    state.players[0].board = (1..=board_slots as InstanceId)
        .map(|id| dummy(&engine.catalog, id, "sickly"))
        .collect();
    state.next_instance_id = board_slots as InstanceId + 1;
    let before = state_hash(&state);

    engine.apply_op(
        &mut state,
        &effect("OnActivate|GainMinion|self|from=shop,level=current,to=board"),
        &ctx(0),
        None,
    );

    assert_eq!(before, state_hash(&state), "棋盘满时不扣池");
}

#[test]
fn gain_minion_to_board_runs_triple_check() {
    let (engine, mut state) = turn_started(2);
    let card_id = first_minion_of_level(&engine.catalog, 1, &state.enabled_playstyles);
    state.players[0].board = vec![
        dummy(&engine.catalog, 1, &card_id),
        dummy(&engine.catalog, 2, &card_id),
    ];
    state.next_instance_id = 3;
    set_shop(
        &mut state,
        0,
        vec![Some(&card_id), None, None, None, None, None],
    );

    let events = engine.apply_op(
        &mut state,
        &effect("OnActivate|GainMinion|self|from=shop,level=1,to=board"),
        &ctx(0),
        None,
    );

    assert!(
        events
            .iter()
            .any(|event| matches!(event, GameEvent::TripleFormed { .. })),
        "获得后应执行三连判定"
    );
}

#[test]
fn copy_hand_minion_to_hand_respects_hand_limit() {
    let (engine, mut state) = turn_started(2);
    let hand_slots = engine.catalog.config.hand_slots;
    state.players[0].hand = (0..hand_slots).map(|_| hand_card("sickly")).collect();
    let target = state.players[0].hand[0].instance_id();
    let before = state_hash(&state);
    let mut ctx = ctx(0);
    ctx.target = Some(target);

    engine.apply_op(
        &mut state,
        &effect("OnActivate|Copy|target:hand:type=minion|from=hand,type=minion,to=hand"),
        &ctx,
        None,
    );

    assert_eq!(before, state_hash(&state), "手牌满时复制为 no-op");
    assert_eq!(state.players[0].hand.len(), hand_slots);
}

#[test]
fn copy_hand_minion_to_hand_with_room_adds_copy() {
    let (engine, mut state) = turn_started(2);
    let card_id = first_minion_of_level(&engine.catalog, 2, &state.enabled_playstyles);
    state.players[0].hand = vec![hand_card(&card_id)];
    let target = state.players[0].hand[0].instance_id();
    let pool_before = pool_count(&state, &card_id);
    let mut ctx = ctx(0);
    ctx.target = Some(target);

    engine.apply_op(
        &mut state,
        &effect("OnActivate|Copy|target:hand:type=minion|from=hand,type=minion,to=hand"),
        &ctx,
        None,
    );

    assert_eq!(state.players[0].hand.len(), 2);
    assert_eq!(state.players[0].hand[1].card_id(), card_id);
    assert_eq!(
        pool_count(&state, &card_id),
        pool_before,
        "复制不扣牌池（核心规则 §6.8）"
    );
}

#[test]
fn target_candidates_list_only_the_selector_zone() {
    let (engine, mut state) = turn_started(2);
    state.players[0].board = vec![dummy(&engine.catalog, 1, "amoeba")];
    state.players[0].hand = vec![hand_card("e-coli")];
    set_shop(&mut state, 0, vec![Some("sickly")]);
    let hand_id = state.players[0].hand[0].instance_id();
    let shop_id = state.players[0].shop[0]
        .as_ref()
        .expect("商店槽应有卡")
        .instance_id;

    let hand_copy = effect("OnActivate|Copy|target:hand:type=minion|from=hand,to=hand");
    assert_eq!(
        engine.target_candidates(&state, &[hand_copy], &ctx(0), false),
        vec![TargetRef {
            zone: Zone::Hand,
            id: hand_id
        }],
        "手牌选择器只下发手牌候选"
    );

    let shop_stats = effect("OnActivate|ModifyStats|target:shop|attack=1");
    assert_eq!(
        engine.target_candidates(&state, &[shop_stats], &ctx(0), false),
        vec![TargetRef {
            zone: Zone::Shop,
            id: shop_id
        }],
        "商店选择器只下发商店候选"
    );

    let board_stats = effect("OnActivate|ModifyStats|target|attack=1");
    assert_eq!(
        engine.target_candidates(&state, &[board_stats], &ctx(0), false),
        vec![TargetRef {
            zone: Zone::Board,
            id: 1
        }],
        "裸 target 只下发我方棋盘候选"
    );
}

#[test]
fn target_candidates_empty_when_selector_zone_is_empty() {
    let (engine, mut state) = turn_started(2);
    state.players[0].hand = vec![HandCard::effect(9_001, "yinshuashu")];
    let hand_copy = effect("OnActivate|Copy|target:hand:type=minion|from=hand,to=hand");
    assert!(
        engine
            .target_candidates(&state, &[hand_copy], &ctx(0), false)
            .is_empty(),
        "手牌没有随从时没有候选"
    );
}

// ---------------------------------------------------------------------------
// 进化与 OnAllyEvolve
// ---------------------------------------------------------------------------

fn evolve_fixture() -> (Engine, GameState) {
    let (mut engine, mut state) = turn_started(2);
    add_skill(
        &mut engine.catalog,
        skill("keyword-evolve", Some("evolve"), "[]", None),
    );
    add_skill(
        &mut engine.catalog,
        skill(
            "trigger-ally-evolve",
            None,
            "OnAllyEvolve|ModifyStats|self|attack=1,health=1,duration=permanent",
            None,
        ),
    );
    add_minion(
        &mut engine.catalog,
        "evolver",
        Playstyle::Bacteria,
        (1, 2, 2),
        &["keyword-evolve"],
    );
    add_minion(
        &mut engine.catalog,
        "watcher",
        Playstyle::Bacteria,
        (1, 1, 1),
        &["trigger-ally-evolve"],
    );
    state.players[0].hand = vec![hand_card("evolver")];
    state.players[0].board = vec![dummy(&engine.catalog, 1, "watcher")];
    state.next_instance_id = 2;
    (engine, state)
}

fn play_evolver(engine: &mut Engine, state: &mut GameState, card_id: &str) -> InstanceId {
    engine
        .apply(
            state,
            0,
            Intent::PlayCard {
                hand_index: 0,
                position: None,
                target: None,
            },
        )
        .expect("打出进化随从应成功");
    state.players[0]
        .board
        .iter()
        .find(|minion| minion.card_id == card_id)
        .expect("进化随从应在战场")
        .instance_id
}

#[test]
fn evolve_on_play_registers_choice() {
    let (mut engine, mut state) = evolve_fixture();
    play_evolver(&mut engine, &mut state, "evolver");

    assert_eq!(pending_count(&state), 1, "进化应先登记选择再结算");
}

#[test]
fn evolve_choice_stats_is_applied_and_triggers_ally_evolve() {
    let (mut engine, mut state) = evolve_fixture();
    let evolver = play_evolver(&mut engine, &mut state, "evolver");

    assert_eq!(evolve_remaining(&state, 0), Some(1), "进化待选择");
    assert_eq!(
        watcher(&state).stats.attack.permanent,
        0,
        "选择结算前不得触发 OnAllyEvolve"
    );

    engine
        .apply(&mut state, 0, Intent::ChooseDiscover { choice: 0 })
        .expect("选择 +2/+2 应成功");

    let minion = state.players[0]
        .board
        .iter()
        .find(|minion| minion.instance_id == evolver)
        .expect("进化随从仍在场");
    assert_eq!(minion.stats.attack.permanent, 2);
    assert_eq!(minion.stats.health.permanent, 2);
    assert_eq!(pending_count(&state), 0);

    let watcher = watcher(&state);
    assert_eq!(
        (
            watcher.stats.attack.permanent,
            watcher.stats.health.permanent
        ),
        (1, 1),
        "选择结算后恰好触发一次 OnAllyEvolve"
    );
}

#[test]
fn evolve_choice_divine_shield_grants_keyword() {
    let (mut engine, mut state) = evolve_fixture();
    let evolver = play_evolver(&mut engine, &mut state, "evolver");

    engine
        .apply(&mut state, 0, Intent::ChooseDiscover { choice: 1 })
        .expect("选择护盾应成功");

    let minion = state.players[0]
        .board
        .iter()
        .find(|minion| minion.instance_id == evolver)
        .expect("进化随从仍在场");
    assert!(
        rule_engine::combat::keyword_set(&engine.catalog, minion).contains_name("divine_shield"),
        "进化选护盾应获得护盾关键词"
    );
}

#[test]
fn evolve_golden_chooses_twice() {
    let (mut engine, mut state) = evolve_fixture();
    add_minion(
        &mut engine.catalog,
        "golden-evolver",
        Playstyle::Bacteria,
        (1, 4, 4),
        &["keyword-evolve"],
    );
    state.players[0].hand = vec![HandCard::Minion(MinionInstance::new(
        &engine.catalog,
        9_000,
        "golden-evolver",
        true,
    ))];
    let evolver = play_evolver(&mut engine, &mut state, "golden-evolver");

    assert_eq!(evolve_remaining(&state, 0), Some(2), "金色进化连续选两次");
    assert_eq!(
        watcher(&state).stats.attack.permanent,
        0,
        "第一次选择前不得触发 OnAllyEvolve"
    );
    engine
        .apply(&mut state, 0, Intent::ChooseDiscover { choice: 0 })
        .expect("第一次选择");
    assert_eq!(evolve_remaining(&state, 0), Some(1), "第一次选择后仍待选");
    assert_eq!(
        watcher(&state).stats.attack.permanent,
        0,
        "进化未完成前不得触发 OnAllyEvolve"
    );
    engine
        .apply(&mut state, 0, Intent::ChooseDiscover { choice: 0 })
        .expect("第二次选择可重复选同一项");

    let minion = state.players[0]
        .board
        .iter()
        .find(|minion| minion.instance_id == evolver)
        .expect("进化随从仍在场");
    assert_eq!(
        minion.stats.attack.permanent, 4,
        "同一项可重复选择，+2/+2 累计两次"
    );
    let watcher = watcher(&state);
    assert_eq!(
        (
            watcher.stats.attack.permanent,
            watcher.stats.health.permanent
        ),
        (1, 1),
        "金色进化是一次入场进化：观察者只触发一次（观察者非金色）"
    );
}

#[test]
fn evolve_golden_watcher_triggers_twice() {
    let (mut engine, mut state) = evolve_fixture();
    add_minion(
        &mut engine.catalog,
        "golden-evolver",
        Playstyle::Bacteria,
        (1, 4, 4),
        &["keyword-evolve"],
    );
    state.players[0].hand = vec![HandCard::Minion(MinionInstance::new(
        &engine.catalog,
        9_000,
        "golden-evolver",
        true,
    ))];
    state.players[0].board[0].golden = true;
    play_evolver(&mut engine, &mut state, "golden-evolver");
    assert_eq!(watcher(&state).stats.attack.permanent, 0);

    engine
        .apply(&mut state, 0, Intent::ChooseDiscover { choice: 0 })
        .expect("第一次选择");
    engine
        .apply(&mut state, 0, Intent::ChooseDiscover { choice: 0 })
        .expect("第二次选择");

    // 一次进化事件；金色观察者自身「触发两次」连续执行，加两段 +1/+1。
    assert_eq!(watcher(&state).stats.attack.permanent, 2);
}

#[test]
fn evolve_skipped_before_choice_does_not_trigger_ally_evolve() {
    let (mut engine, mut state) = evolve_fixture();
    let evolver = play_evolver(&mut engine, &mut state, "evolver");

    // 战吼结算后来源被移出战场（例如出售）：进化不再发生，观察者不触发。
    state.players[0]
        .board
        .retain(|minion| minion.instance_id != evolver);
    assert_eq!(watcher(&state).stats.attack.permanent, 0);
    engine
        .apply(&mut state, 0, Intent::ChooseDiscover { choice: 0 })
        .expect("来源离场时选择被丢弃");
    assert_eq!(
        watcher(&state).stats.attack.permanent,
        0,
        "未完成的进化不触发 OnAllyEvolve"
    );
    assert_eq!(pending_count(&state), 0);
}

#[test]
fn evolve_bonus_does_not_trigger_onsummon_twice() {
    let (mut engine, mut state) = turn_started(2);
    add_skill(
        &mut engine.catalog,
        skill("keyword-evolve", Some("evolve"), "[]", None),
    );
    add_skill(
        &mut engine.catalog,
        skill(
            "trigger-ally-summon",
            None,
            "OnAllySummon|ModifyStats|self|attack=1,duration=permanent",
            None,
        ),
    );
    add_minion(
        &mut engine.catalog,
        "evolver",
        Playstyle::Bacteria,
        (1, 2, 2),
        &["keyword-evolve"],
    );
    add_minion(
        &mut engine.catalog,
        "summon-watcher",
        Playstyle::Machine,
        (1, 1, 1),
        &["trigger-ally-summon"],
    );
    state.players[0].board = vec![
        dummy(&engine.catalog, 1, "evolver"),
        dummy(&engine.catalog, 2, "summon-watcher"),
    ];
    state.next_instance_id = 3;
    // 只观察「选择进化」这一步：观察者已经入场，不计它自己带出的入场触发。
    state.pending.push_back(PendingChoice::Evolve {
        seat: 0,
        source: 1,
        remaining: 1,
    });

    engine
        .apply(&mut state, 0, Intent::ChooseDiscover { choice: 0 })
        .expect("选择应成功");

    let watcher = state.players[0]
        .board
        .iter()
        .find(|minion| minion.card_id == "summon-watcher")
        .expect("观察者仍在场");
    assert_eq!(
        watcher.stats.attack.permanent, 0,
        "选择进化不会再次触发 OnAllySummon"
    );
}

#[test]
fn evolve_cards_without_evolve_do_not_register_choice() {
    let (engine, mut state) = turn_started(2);
    let card_id = first_minion_of_level(&engine.catalog, 1, &state.enabled_playstyles);
    state.players[0].hand = vec![hand_card(&card_id)];

    engine
        .apply(
            &mut state,
            0,
            Intent::PlayCard {
                hand_index: 0,
                position: None,
                target: None,
            },
        )
        .expect("打出应成功");

    assert_eq!(pending_count(&state), 0);
}

// ---------------------------------------------------------------------------
// GainMinion 的 from 只接受 shop（加载期校验）
// ---------------------------------------------------------------------------

/// 用已提交数据构造一个把指定技能 `effects` 替换掉的 skills.json。
fn skills_with_effects_json(skill_id: &str, effects: &str) -> String {
    let mut skills: serde_json::Value = serde_json::from_str(SKILLS_JSON).expect("技能数据合法");
    let target = skills
        .as_array_mut()
        .expect("技能数据应为数组")
        .iter_mut()
        .find(|skill| skill["id"] == skill_id)
        .expect("技能应存在于已提交数据");
    target["effects"] = serde_json::from_str(effects).expect("测试效果 JSON 合法");
    skills.to_string()
}

#[test]
fn gain_minion_from_board_fails_at_load() {
    let skills = skills_with_effects_json(
        "efficient-algorithm",
        r#"[{"trigger":"on_activate","op":"refresh_shop","selector":{"kind":"self"},"params":{"free":true}},{"trigger":"on_activate","op":"gain_minion","selector":{"kind":"self"},"params":{"from":"board","level":"current","to":"hand"}}]"#,
    );
    let error = Catalog::from_json(CARDS_JSON, HEROES_JSON, &skills, CONFIG_JSON, TAXONOMY_JSON)
        .expect_err("from=board 必须在加载期失败");
    let message = error.to_string();
    assert!(
        message.contains("efficient-algorithm"),
        "错误应带技能 id：{message}"
    );
    assert!(message.contains("from"), "错误应指出参数：{message}");
}

#[test]
fn gain_minion_from_hand_fails_at_load() {
    let skills = skills_with_effects_json(
        "efficient-algorithm",
        r#"[{"trigger":"on_activate","op":"gain_minion","selector":{"kind":"self"},"params":{"from":"hand","level":"current","to":"hand"}}]"#,
    );
    assert!(
        Catalog::from_json(CARDS_JSON, HEROES_JSON, &skills, CONFIG_JSON, TAXONOMY_JSON).is_err(),
        "from=hand 必须在加载期失败"
    );
}

#[test]
fn gain_minion_from_shop_or_omitted_loads() {
    for effects in [
        r#"[{"trigger":"on_activate","op":"gain_minion","selector":{"kind":"self"},"params":{"from":"shop","level":"current","to":"hand"}}]"#,
        r#"[{"trigger":"on_activate","op":"gain_minion","selector":{"kind":"self"},"params":{"level":"3","to":"board"}}]"#,
    ] {
        let skills = skills_with_effects_json("efficient-algorithm", effects);
        Catalog::from_json(CARDS_JSON, HEROES_JSON, &skills, CONFIG_JSON, TAXONOMY_JSON)
            .unwrap_or_else(|error| panic!("{effects} 应能加载：{error}"));
    }
}

#[test]
fn copy_from_hand_requires_target_selector() {
    // 从手牌复制必须由玩家选目标：裸 `self` 选择器在解析期被拒绝。
    let raw = "OnActivate|Copy|self|from=hand,to=hand";
    assert!(
        parse_effects(raw).is_err(),
        "from=hand 必须使用 target:hand 选择器"
    );
    for raw in [
        "OnActivate|Copy|target:hand:type=minion|from=hand,to=hand",
        "OnActivate|Copy|self|from=board,to=hand",
    ] {
        parse_effects(raw).unwrap_or_else(|errors| panic!("{raw} 应能解析：{errors:?}"));
    }
}

#[test]
fn hash_covers_target_pending() {
    let (engine, state) = turn_started(2);
    let target = |seat: Seat,
                  source: InstanceId,
                  card_id: &str,
                  golden: bool,
                  hand_index: u8,
                  fire_ally_evolve: bool| PendingChoice::Target {
        seat,
        source,
        hand_card: HandCard::Minion(MinionInstance::new(
            &engine.catalog,
            source,
            card_id,
            golden,
        )),
        hand_index,
        fire_ally_evolve,
    };
    let mut with_base = state.clone();
    with_base
        .pending
        .push_back(target(0, 1, "fangzhiji", false, 0, false));
    let base_hash = state_hash(&with_base);

    let variants: Vec<PendingChoice> = vec![
        target(1, 1, "fangzhiji", false, 0, false),
        target(0, 2, "fangzhiji", false, 0, false),
        target(0, 1, "bisheng", false, 0, false),
        target(0, 1, "fangzhiji", true, 0, false),
        target(0, 1, "fangzhiji", false, 3, false),
        target(0, 1, "fangzhiji", false, 0, true),
    ];
    for variant in variants {
        let mut other = state.clone();
        other.pending.push_back(variant.clone());
        assert_ne!(
            base_hash,
            state_hash(&other),
            "待选字段不同应产生不同哈希：{variant:?}"
        );
    }
}
