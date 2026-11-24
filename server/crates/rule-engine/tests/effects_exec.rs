mod common;

use std::collections::BTreeSet;

use common::{categories_for, effects_of, minion, place, setup, test_catalog};
use rule_engine::combat::{self, CombatAction, CombatLog, CombatSetup, simulate_with};
use rule_engine::data::{Card, CardType, Playstyle, Skill};
use rule_engine::effects::{
    Duration, Effect, EffectCtx, Op, Params, TargetRef, Trigger, Zone, effect_coverage,
    parse_effects,
};
use rule_engine::engine::Engine;
use rule_engine::events::GameEvent;
use rule_engine::hash::state_hash;
use rule_engine::intent::{Intent, RejectCode};
use rule_engine::rng::RngStreams;
use rule_engine::state::{GameState, HandCard, MinionInstance, Phase, Seat, ShopSlot};

/// 棋盘目标：测试里的意图目标统一指向本人棋盘。
fn board_target(id: u64) -> TargetRef {
    TargetRef {
        zone: Zone::Board,
        id,
    }
}

/// 往测试目录里插入一张合成随从，避免依赖真实卡面数值。
fn add_card(engine: &mut Engine, id: &str, attack: i32, health: i32, keywords: &[&str]) {
    let mut skill_ids = Vec::new();
    for (index, keyword) in keywords.iter().enumerate() {
        let skill_id = engine.catalog.insert_skill(Skill {
            id: format!("{id}-skill-{index}"),
            keyword: Some((*keyword).to_string()),
            effects: Vec::new(),
            cost: None,
            uses_per_turn: None,
        });
        skill_ids.push(skill_id);
    }
    engine.catalog.cards.insert(
        id.to_string(),
        Card {
            id: id.to_string(),
            card_type: CardType::Minion,
            in_shop_pool: true,
            taxon: "hominidae".to_string(),
            categories: categories_for(Playstyle::Mammal),
            subtype: None,
            level: 1,
            era: None,
            attack: Some(attack),
            health: Some(health),
            skills: skill_ids,
            cost: Some(1),
        },
    );
}

/// 给合成随从追加一组战斗效果技能。
fn add_effect_card(
    engine: &mut Engine,
    id: &str,
    attack: i32,
    health: i32,
    keywords: &[&str],
    effects: &[&str],
) {
    add_card(engine, id, attack, health, keywords);
    for (index, raw) in effects.iter().enumerate() {
        let skill_id = engine.catalog.insert_skill(Skill {
            id: format!("{id}-effect-{index}"),
            keyword: None,
            effects: effects_of(raw),
            cost: None,
            uses_per_turn: None,
        });
        engine
            .catalog
            .cards
            .get_mut(id)
            .expect("刚插入的合成卡应存在")
            .skills
            .push(skill_id);
    }
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
        level_override: None,
    }
}

fn board_minion(state: &GameState, seat: Seat, index: usize) -> &MinionInstance {
    &state.players[usize::from(seat)].board[index]
}

#[test]
fn battlecry_growth_adds_permanent_stats() {
    let (engine, mut state) = setup(11);
    place(&engine.catalog, &mut state, 0, 1, "pink-ape");
    let attack_before = combat::effective_attack(&engine.catalog, board_minion(&state, 0, 0));
    let health_before = combat::effective_health(&engine.catalog, board_minion(&state, 0, 0));

    let events = engine.fire(
        &mut state,
        Trigger::OnPlay,
        EffectCtx {
            source: Some(1),
            ..ctx(0)
        },
    );

    assert!(events.is_empty(), "成长不产生事件");
    let grown = board_minion(&state, 0, 0);
    assert_eq!(grown.stats.attack.growth, 2, "战吼：获得两次成长");
    assert_eq!(grown.stats.health.growth, 2);
    assert_eq!(
        combat::effective_attack(&engine.catalog, grown),
        attack_before + 2
    );
    assert_eq!(
        combat::effective_health(&engine.catalog, grown),
        health_before + 2
    );
}

#[test]
fn growth_applies_attack_and_health_to_zero_attack_minion() {
    let (engine, mut state) = setup(11);
    place(&engine.catalog, &mut state, 0, 1, "aristotle");

    engine.apply_op(
        &mut state,
        &effect("OnPlay|Growth|friendly:all|times=1"),
        &ctx(0),
        None,
    );

    let minion = board_minion(&state, 0, 0);
    assert_eq!(
        minion.stats.attack.growth, 1,
        "攻击力 0 的随从成长照常加攻击"
    );
    assert_eq!(minion.stats.health.growth, 1, "成长同时加生命");
    assert_eq!(combat::effective_attack(&engine.catalog, minion), 1);
    assert_eq!(combat::effective_health(&engine.catalog, minion), 10);
}

#[test]
fn health_only_growth_adds_no_attack() {
    let (mut engine, mut state) = setup(11);
    add_card(&mut engine, "test-growth", 2, 2, &[]);
    place(&engine.catalog, &mut state, 0, 1, "test-growth");

    engine.apply_op(
        &mut state,
        &effect("OnPlay|Growth|friendly:all|times=2,healthOnly=true"),
        &ctx(0),
        None,
    );

    let grown = board_minion(&state, 0, 0);
    assert_eq!(grown.stats.attack.growth, 0);
    assert_eq!(grown.stats.health.growth, 2);
    assert_eq!(combat::effective_attack(&engine.catalog, grown), 2);
    assert_eq!(combat::effective_health(&engine.catalog, grown), 4);
}

#[test]
fn modify_stats_permanent_and_combat() {
    let (engine, mut state) = setup(11);
    place(&engine.catalog, &mut state, 0, 1, "yezhu");
    let source_ctx = EffectCtx {
        source: Some(1),
        ..ctx(0)
    };

    engine.apply_op(
        &mut state,
        &effect("OnPlay|ModifyStats|self|attack=1,health=2,duration=permanent"),
        &source_ctx,
        None,
    );
    engine.apply_op(
        &mut state,
        &effect("OnPlay|ModifyStats|self|health=4,duration=combat"),
        &source_ctx,
        None,
    );
    assert_eq!(board_minion(&state, 0, 0).stats.health.turn, 4);

    state.phase = Phase::Settle;
    engine.resolve_settle(&mut state).expect("结算应成功");

    let minion = board_minion(&state, 0, 0);
    assert_eq!(minion.stats.health.turn, 0, "combat 修正在 Settle 前清除");
    assert_eq!(minion.stats.attack.permanent, 1);
    assert_eq!(minion.stats.health.permanent, 2);
}

#[test]
fn grant_keyword_respects_duration() {
    let (engine, mut state) = setup(11);
    place(&engine.catalog, &mut state, 0, 1, "yezhu");
    place(&engine.catalog, &mut state, 0, 2, "yezhu");
    state.players[0].board[0].shield_used = true;
    let first = EffectCtx {
        source: Some(1),
        ..ctx(0)
    };
    let second = EffectCtx {
        source: Some(2),
        ..ctx(0)
    };

    engine.apply_op(
        &mut state,
        &effect("OnPlay|GrantKeyword|self|keyword=divine_shield,duration=permanent"),
        &first,
        None,
    );
    engine.apply_op(
        &mut state,
        &effect("OnPlay|GrantKeyword|self|keyword=windfury,duration=combat"),
        &second,
        None,
    );
    engine.apply_op(
        &mut state,
        &effect("OnPlay|GrantKeyword|self|keyword=breed,value=2,duration=permanent"),
        &first,
        None,
    );

    let shielded = board_minion(&state, 0, 0);
    assert!(
        shielded
            .granted
            .contains(&("divine_shield".into(), Duration::Permanent))
    );
    assert!(!shielded.shield_used, "授予护盾会重置已用标记");
    assert_eq!(shielded.breed, 2, "繁衍层数写入 breed，不进入 granted");
    assert!(!shielded.granted.iter().any(|(name, _)| name == "breed"));

    let windfury = board_minion(&state, 0, 1);
    assert!(
        windfury
            .granted
            .contains(&("windfury".into(), Duration::Combat))
    );
    assert!(combat::keyword_set(&engine.catalog, windfury).contains_name("windfury"));

    state.phase = Phase::Settle;
    engine.resolve_settle(&mut state).expect("结算应成功");
    assert!(
        !combat::keyword_set(&engine.catalog, board_minion(&state, 0, 1)).contains_name("windfury"),
        "combat 关键词在 Settle 前清除"
    );
    assert!(
        board_minion(&state, 0, 0)
            .granted
            .iter()
            .any(|(name, duration)| name == "divine_shield" && *duration == Duration::Permanent),
        "permanent 关键词保留"
    );
}

#[test]
fn remove_growth_reduces_growth_count() {
    let (engine, mut state) = setup(11);
    place(&engine.catalog, &mut state, 0, 1, "yezhu");
    state.players[0].board[0].stats.attack.add_growth(5);
    state.players[0].board[0].stats.health.add_growth(5);
    state.players[0].board[0].stats.health.add_growth(3);

    engine.apply_op(
        &mut state,
        &effect("OnCombatStart|RemoveGrowth|friendly:all"),
        &ctx(0),
        None,
    );

    let minion = board_minion(&state, 0, 0);
    assert_eq!(minion.stats.attack.growth, 0);
    assert_eq!(minion.stats.health.growth, 0);
}

#[test]
fn cannot_attack_blocks_attacks() {
    let (engine, mut state) = setup(11);
    place(&engine.catalog, &mut state, 0, 1, "yezhu");

    engine.apply_op(
        &mut state,
        &effect("OnCombatStart|CannotAttack|friendly:all|duration=combat"),
        &ctx(0),
        None,
    );

    let minion = board_minion(&state, 0, 0);
    assert!(minion.cannot_attack);
    assert!(
        minion
            .granted
            .iter()
            .any(|(name, duration)| name == "cannot_attack" && *duration == Duration::Combat)
    );

    state.phase = Phase::Settle;
    engine.resolve_settle(&mut state).expect("结算应成功");
    let minion = board_minion(&state, 0, 0);
    assert!(!minion.cannot_attack, "combat 禁攻在 Settle 前解除");
    assert!(minion.granted.is_empty());
}

#[test]
fn deal_damage_uses_selector_and_amount() {
    let (mut engine, mut state) = setup(11);
    state.phase = Phase::Combat;
    add_card(&mut engine, "test-damage", 2, 2, &[]);
    place(&engine.catalog, &mut state, 1, 10, "test-damage");

    let events = engine.apply_op(
        &mut state,
        &effect("OnCombatStart|DealDamage|enemy:random|amount=2"),
        &ctx(0),
        None,
    );

    assert_eq!(events, vec![GameEvent::DamageTaken { seat: 1, amount: 2 }]);
    let victim = board_minion(&state, 1, 0);
    assert_eq!(victim.stats.health.turn, -2);
    assert_eq!(combat::effective_health(&engine.catalog, victim), 0);
}

#[test]
fn deal_damage_cannot_kill_floors_health_at_one() {
    let (engine, mut state) = setup(11);
    state.phase = Phase::Combat;
    place(&engine.catalog, &mut state, 1, 10, "yanhuashi");

    let mut damage = effect("OnCombatStart|DealDamage|enemy:all|amount=10");
    let Params::DealDamage(params) = &mut damage.params else {
        panic!("期望 DealDamage 参数");
    };
    params.can_kill = Some(false);
    let events = engine.apply_op(&mut state, &damage, &ctx(0), None);

    assert_eq!(events, vec![GameEvent::DamageTaken { seat: 1, amount: 4 }]);
    let victim = board_minion(&state, 1, 0);
    assert_eq!(combat::effective_health(&engine.catalog, victim), 1);
}

#[test]
fn enemy_selectors_are_empty_during_recruit() {
    let (engine, mut state) = setup(11);
    place(&engine.catalog, &mut state, 1, 10, "yezhu");
    place(&engine.catalog, &mut state, 0, 1, "yezhu");

    let events = engine.apply_op(
        &mut state,
        &effect("OnCombatStart|DealDamage|enemy:all|amount=2"),
        &ctx(0),
        None,
    );

    assert!(events.is_empty(), "招募阶段敌方选择器无目标");
    assert_eq!(board_minion(&state, 1, 0).stats.health.turn, 0);
    let enemy_effect = effect("OnCombatStart|DealDamage|enemy:all|amount=2");
    assert!(
        engine
            .resolve_selector(&state, &enemy_effect.selector, &ctx(0))
            .is_empty()
    );
}

#[test]
fn times_if_changes_times_by_playstyle() {
    let (engine, mut state) = setup(11);
    place(&engine.catalog, &mut state, 0, 1, "tengman");
    place(&engine.catalog, &mut state, 0, 2, "yezhu");
    let growth_effect = effect("OnPlay|Growth|friendly:single|times=1,timesIf=playstyle:plant:2");

    let plant_ctx = EffectCtx {
        target: Some(1),
        ..ctx(0)
    };
    engine.apply_op(&mut state, &growth_effect, &plant_ctx, None);
    assert_eq!(
        board_minion(&state, 0, 0).stats.attack.growth,
        2,
        "植物目标改为两次"
    );

    let beast_ctx = EffectCtx {
        target: Some(2),
        ..ctx(0)
    };
    engine.apply_op(&mut state, &growth_effect, &beast_ctx, None);
    assert_eq!(
        board_minion(&state, 0, 1).stats.attack.growth,
        1,
        "非植物目标按基础 times=1 结算"
    );
}

#[test]
fn condition_gates_effect() {
    let (engine, mut state) = setup(11);
    place(&engine.catalog, &mut state, 0, 1, "buluozhanshi");
    place(&engine.catalog, &mut state, 0, 2, "bruce-lee");
    let gated = effect(
        "OnActivate|Growth|friendly:single:playstyle=mammal|times=1,timesIf=playstyle:mammal:2,condition=count:friendly:playstyle=mammal:>=3",
    );
    let target_ctx = EffectCtx {
        target: Some(1),
        ..ctx(0)
    };

    engine.apply_op(&mut state, &gated, &target_ctx, None);
    assert_eq!(
        board_minion(&state, 0, 0).stats.attack.growth,
        0,
        "条件不满足不执行"
    );

    place(&engine.catalog, &mut state, 0, 3, "buluozhanshi");
    engine.apply_op(&mut state, &gated, &target_ctx, None);
    assert_eq!(
        board_minion(&state, 0, 0).stats.attack.growth,
        2,
        "三名人类时 timesIf 生效"
    );
}

#[test]
fn golden_keeps_condition_threshold_but_doubles_output() {
    let (engine, mut state) = setup(11);
    place(&engine.catalog, &mut state, 0, 1, "buluozhanshi");
    place(&engine.catalog, &mut state, 0, 2, "bruce-lee");
    let gated = effect(
        "OnActivate|Growth|friendly:single:playstyle=mammal|times=1,condition=count:friendly:playstyle=mammal:>=3",
    );
    let golden_ctx = EffectCtx {
        source: Some(1),
        golden: true,
        target: Some(1),
        ..ctx(0)
    };

    engine.apply_op(&mut state, &gated, &golden_ctx, None);
    assert_eq!(
        board_minion(&state, 0, 0).stats.attack.growth,
        0,
        "条件阈值不翻倍：两名人类仍不满足 >=3"
    );

    place(&engine.catalog, &mut state, 0, 3, "buluozhanshi");
    engine.apply_op(&mut state, &gated, &golden_ctx, None);
    assert_eq!(
        board_minion(&state, 0, 0).stats.attack.growth,
        2,
        "条件仍按 3 判定，成长次数 ×2"
    );
}

#[test]
fn golden_battlecry_doubles_output() {
    let (engine, mut state) = setup(11);
    place(&engine.catalog, &mut state, 0, 1, "pink-ape");

    engine.fire(
        &mut state,
        Trigger::OnPlay,
        EffectCtx {
            source: Some(1),
            golden: true,
            ..ctx(0)
        },
    );

    assert_eq!(
        board_minion(&state, 0, 0).stats.attack.growth,
        4,
        "金色数值 ×2、单次结算"
    );
}

#[test]
fn on_activate_respects_cost_and_uses() {
    let (engine, mut state) = setup(11);
    state.players[0].hero = Some("luxun".to_string());
    state.players[0].gold = 5;
    place(&engine.catalog, &mut state, 0, 1, "yezhu");

    engine
        .apply(
            &mut state,
            0,
            Intent::UseHeroPower {
                skill_id: "shout".to_string(),
                target: Some(board_target(1)),
            },
        )
        .expect("首次使用应成功");
    assert_eq!(state.players[0].gold, 4);
    assert_eq!(board_minion(&state, 0, 0).stats.attack.growth, 1);

    engine
        .apply(
            &mut state,
            0,
            Intent::UseHeroPower {
                skill_id: "shout".to_string(),
                target: Some(board_target(1)),
            },
        )
        .expect("每回合可用两次");
    assert_eq!(state.players[0].gold, 3);
    assert_eq!(board_minion(&state, 0, 0).stats.attack.growth, 2);

    let before = state_hash(&state);
    let rejected = engine.apply(
        &mut state,
        0,
        Intent::UseHeroPower {
            skill_id: "shout".to_string(),
            target: Some(board_target(1)),
        },
    );
    assert_eq!(rejected.unwrap_err().code, RejectCode::HeroPowerLimit);
    assert_eq!(before, state_hash(&state));
}

#[test]
fn buy_enters_hand_then_play_fires_on_play() {
    let (engine, mut state) = setup(11);
    state.players[0].gold = 10;
    state.players[0].shop = vec![Some(ShopSlot {
        instance_id: 4_242,
        card_id: "yezhu".to_string(),
        frozen: false,
    })];

    let outcome = engine
        .apply(&mut state, 0, Intent::BuyMinion { slot: 0 })
        .expect("购买应成功");

    assert_eq!(
        outcome.events[0],
        GameEvent::MinionBought {
            seat: 0,
            slot: 0,
            card_id: "yezhu".to_string(),
        }
    );
    assert_eq!(state.players[0].hand.len(), 1, "买入只入手牌");
    assert!(state.players[0].board.is_empty(), "买入不入场");

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

    let played = board_minion(&state, 0, 0);
    assert_eq!(played.stats.health.permanent, 1, "战吼：获得 +1 生命");
}

#[test]
fn play_effect_card_fires_effect_then_on_use() {
    let (engine, mut state) = setup(11);
    place(&engine.catalog, &mut state, 0, 1, "yanhuashi");
    state.players[0]
        .hand
        .push(HandCard::effect(9_000, "aolinpike-jidian"));

    engine
        .apply(
            &mut state,
            0,
            Intent::PlayCard {
                hand_index: 0,
                position: None,
                target: Some(board_target(1)),
            },
        )
        .expect("打出效果牌应成功");

    let responder = board_minion(&state, 0, 0);
    assert_eq!(
        responder.stats.health.growth, 1,
        "效果牌自身：获得一次成长（+1/+1）"
    );
    assert_eq!(
        responder.stats.health.permanent, 1,
        "OnUse 响应：目标 +1 生命"
    );
}

#[test]
fn unimplemented_triggers_return_empty() {
    let (engine, mut state) = setup(11);
    place(&engine.catalog, &mut state, 0, 1, "pink-ape");

    let events = engine.fire(
        &mut state,
        Trigger::OnDeath,
        EffectCtx {
            source: Some(1),
            ..ctx(0)
        },
    );
    assert!(events.is_empty());
    assert_eq!(board_minion(&state, 0, 0).stats.attack.growth, 0);
}

#[test]
fn coverage_counts_all_effect_segments() {
    let catalog = test_catalog();
    let coverage = effect_coverage(&catalog);

    let total: usize = coverage.values().sum();
    // 英雄主动技能各带一段 `OnActivate` 效果，效果段总数随之上升。
    assert_eq!(total, 246, "已提交数据的全部效果段都应被统计");
    assert_eq!(coverage.get(&(Trigger::OnPlay, Op::Growth)), Some(&20));
    assert_eq!(coverage.get(&(Trigger::OnPlay, Op::ModifyStats)), Some(&12));
    assert_eq!(coverage.get(&(Trigger::OnUse, Op::ModifyStats)), Some(&1));
    assert_eq!(coverage.get(&(Trigger::OnActivate, Op::Growth)), Some(&13));
    assert_eq!(
        coverage.get(&(Trigger::OnActivate, Op::ModifyStats)),
        Some(&1)
    );
    assert_eq!(
        coverage.get(&(Trigger::OnActivate, Op::SevenVoyages)),
        Some(&1)
    );

    let executable: BTreeSet<Op> = [
        Op::Growth,
        Op::ModifyStats,
        Op::GrantKeyword,
        Op::RemoveGrowth,
        Op::CannotAttack,
        Op::DealDamage,
        Op::Summon,
        Op::Copy,
        Op::Discover,
        Op::RefreshShop,
        Op::AddShopSlot,
        Op::GainMinion,
        Op::GrantMinion,
        Op::Decompose,
        Op::Mimic,
        Op::Devour,
        Op::Detonate,
        Op::Convert,
        Op::Revive,
        Op::Recall,
        Op::Salvage,
        Op::Latent,
        Op::SevenVoyages,
    ]
    .into_iter()
    .collect();
    assert!(
        coverage.keys().all(|(_, op)| executable.contains(op)),
        "所有出现过的动作都应被 apply_op 覆盖"
    );
}

#[test]
fn selector_picks_cover_leftmost_adjacent_top_attack_and_other() {
    let (engine, mut state) = setup(11);
    place(&engine.catalog, &mut state, 0, 1, "yezhu");
    place(&engine.catalog, &mut state, 0, 2, "bruce-lee");
    place(&engine.catalog, &mut state, 0, 3, "tengman");

    let leftmost = effect("OnPlay|Growth|friendly:leftmost|times=1");
    assert_eq!(
        engine.resolve_selector(&state, &leftmost.selector, &ctx(0)),
        vec![1]
    );

    let adjacent = effect("OnPlay|Growth|friendly:leftmost+adjacent|times=1");
    assert_eq!(
        engine.resolve_selector(&state, &adjacent.selector, &ctx(0)),
        vec![1, 2]
    );

    let top = effect("OnPlay|Growth|friendly:topAttack|times=1");
    assert_eq!(
        engine.resolve_selector(&state, &top.selector, &ctx(0)),
        vec![2],
        "攻击力最高的随从"
    );

    let others = effect("OnPlay|Growth|friendly:all:other=true|times=1");
    let source_ctx = EffectCtx {
        source: Some(2),
        ..ctx(0)
    };
    assert_eq!(
        engine.resolve_selector(&state, &others.selector, &source_ctx),
        vec![1, 3],
        "other=true 排除来源"
    );
}

#[test]
fn grant_contagion_stacks_layer() {
    let (engine, mut state) = setup(11);
    place(&engine.catalog, &mut state, 0, 1, "yezhu");

    engine.apply_op(
        &mut state,
        &effect("OnCombatStart|GrantKeyword|friendly:all|keyword=contagion,value=2"),
        &ctx(0),
        None,
    );

    assert_eq!(
        board_minion(&state, 0, 0).contagion,
        2,
        "传染层数写入 contagion，不进入 granted"
    );
}

#[test]
fn grant_armor_keeps_layer_in_keyword_string() {
    let (engine, mut state) = setup(11);
    place(&engine.catalog, &mut state, 0, 1, "yezhu");

    engine.apply_op(
        &mut state,
        &effect("OnCombatStart|GrantKeyword|friendly:all|keyword=armor 2,duration=combat"),
        &ctx(0),
        None,
    );

    let minion = board_minion(&state, 0, 0);
    assert!(
        minion.granted.iter().any(|(name, _)| name == "armor 2"),
        "armor 层数应写入关键词字符串，而不是裸 armor"
    );
    assert_eq!(
        combat::armor_value(&engine.catalog, minion),
        2,
        "战斗侧读取到授予的 2 点减伤"
    );
}

#[test]
fn grant_armor_stacks_repeated_same_layer() {
    let (engine, mut state) = setup(11);
    place(&engine.catalog, &mut state, 0, 1, "yezhu");

    for keyword in ["armor 1", "armor 1", "armor 2"] {
        engine.apply_op(
            &mut state,
            &effect(&format!(
                "OnCombatStart|GrantKeyword|friendly:all|keyword={keyword},duration=combat"
            )),
            &ctx(0),
            None,
        );
    }

    let minion = board_minion(&state, 0, 0);
    assert_eq!(
        combat::armor_value(&engine.catalog, minion),
        4,
        "同层与不同层的授予减伤都应累加"
    );
    assert_eq!(
        minion
            .granted
            .iter()
            .filter(|(name, _)| name.starts_with("armor"))
            .count(),
        1,
        "多次授予应合并为一条 armor 关键词"
    );
}

#[test]
fn golden_minion_played_from_hand_doubles_battlecry() {
    let (engine, mut state) = setup(11);
    place(&engine.catalog, &mut state, 0, 1, "yezhu");
    state.next_instance_id = 100;
    state.players[0]
        .hand
        .push(HandCard::Minion(MinionInstance::new(
            &engine.catalog,
            9_000,
            "xinge",
            true,
        )));

    engine
        .apply(
            &mut state,
            0,
            Intent::PlayCard {
                hand_index: 0,
                position: None,
                target: Some(board_target(1)),
            },
        )
        .expect("打出金色随从应成功");

    assert!(state.players[0].board[1].golden, "手牌金色标记保留到棋盘");
    assert_eq!(
        board_minion(&state, 0, 0).stats.attack.growth,
        2,
        "金色战吼数值 ×2，+1/+1 变为 +2/+2"
    );
}

#[test]
fn golden_on_use_responder_doubles_output() {
    fn responder_effect_count(golden: bool) -> usize {
        let (engine, mut state) = setup(11);
        let mut responder = minion(&engine.catalog, 1, "yanhuashi");
        responder.golden = golden;
        state.players[0].board.push(responder);
        state.players[0]
            .hand
            .push(HandCard::effect(9_000, "aolinpike-jidian"));
        engine
            .apply(
                &mut state,
                0,
                Intent::PlayCard {
                    hand_index: 0,
                    position: None,
                    target: Some(board_target(1)),
                },
            )
            .expect("打出效果牌应成功");
        assert_eq!(state.players[0].board[0].card_id, "yanhuashi");
        state.players[0].board[0].stats.health.permanent as usize
    }

    assert_eq!(responder_effect_count(false), 1, "普通随从响应一次");
    assert_eq!(responder_effect_count(true), 2, "金色随从响应时数值 ×2");
}

#[test]
fn both_side_merges_in_ascending_seat_order() {
    let (engine, mut state) = setup(11);
    state.phase = Phase::Combat;
    place(&engine.catalog, &mut state, 1, 11, "yezhu");
    place(&engine.catalog, &mut state, 3, 31, "yezhu");
    place(&engine.catalog, &mut state, 5, 51, "yezhu");

    let both = effect("OnCombatStart|DealDamage|both:all|amount=1");
    let seat_five = EffectCtx { seat: 5, ..ctx(5) };
    assert_eq!(
        engine.resolve_selector(&state, &both.selector, &seat_five),
        vec![11, 31, 51],
        "双方池按座位升序合并，己方座位处于自然位置"
    );
}

#[test]
fn cannot_attack_without_duration_clears_at_settle() {
    let (engine, mut state) = setup(11);
    place(&engine.catalog, &mut state, 0, 1, "yezhu");

    engine.apply_op(
        &mut state,
        &effect("OnCombatStart|CannotAttack|friendly:all"),
        &ctx(0),
        None,
    );

    let minion = board_minion(&state, 0, 0);
    assert!(minion.cannot_attack);
    assert!(
        minion
            .granted
            .iter()
            .any(|(name, duration)| name == "cannot_attack" && *duration == Duration::Combat),
        "省略 duration 时按 combat 记录"
    );

    state.phase = Phase::Settle;
    engine.resolve_settle(&mut state).expect("结算应成功");
    let minion = board_minion(&state, 0, 0);
    assert!(!minion.cannot_attack, "无 duration 的禁攻在 Settle 前解除");
    assert!(minion.granted.is_empty());
}

#[test]
fn mimic_copies_current_stats_and_keywords_not_growth() {
    let (mut engine, mut state) = setup(11);
    add_card(&mut engine, "mimic-source", 0, 0, &[]);
    add_card(&mut engine, "mimic-target", 0, 0, &["taunt"]);
    place(&engine.catalog, &mut state, 0, 1, "mimic-source");
    place(&engine.catalog, &mut state, 1, 10, "mimic-target");
    // 目标当前 5/5：卡面 0/0 加成长层；拟态只读当前值，不复制成长层。
    state.players[1].board[0].stats.attack.add_growth(5);
    state.players[1].board[0].stats.health.add_growth(5);
    state.phase = Phase::Combat;

    let mimic = effect("OnCombatStart|Mimic|enemy:single|duration=combat");
    let mimic_ctx = EffectCtx {
        source: Some(1),
        target: Some(10),
        ..ctx(0)
    };
    let events = engine.apply_op(&mut state, &mimic, &mimic_ctx, None);

    assert!(events.is_empty(), "拟态不产生事件");
    let source = board_minion(&state, 0, 0);
    assert_eq!(combat::effective_attack(&engine.catalog, source), 5);
    assert_eq!(combat::effective_health(&engine.catalog, source), 5);
    assert_eq!(source.stats.attack.growth, 0, "不复制目标成长层");
    assert_eq!(source.stats.health.growth, 0, "不复制目标成长层");
    assert!(
        source
            .granted
            .contains(&("taunt".to_string(), Duration::Combat)),
        "关键词按 duration 写入 granted"
    );
    assert!(
        combat::keyword_set(&engine.catalog, source).contains_name("taunt"),
        "拟态后获得嘲讽"
    );

    // 重复结算：仍以同一目标当前值为准，不应叠加数值。
    engine.apply_op(&mut state, &mimic, &mimic_ctx, None);
    let source = board_minion(&state, 0, 0);
    assert_eq!(
        combat::effective_attack(&engine.catalog, source),
        5,
        "重复结算仍为目标当前值"
    );
    assert_eq!(
        combat::effective_health(&engine.catalog, source),
        5,
        "重复结算仍为目标当前值"
    );

    let target = board_minion(&state, 1, 0);
    assert_eq!(
        combat::effective_attack(&engine.catalog, target),
        5,
        "目标本身不被修改"
    );

    state.phase = Phase::Settle;
    engine.resolve_settle(&mut state).expect("结算应成功");
    let source = board_minion(&state, 0, 0);
    assert_eq!(
        combat::effective_attack(&engine.catalog, source),
        0,
        "combat 拟态在本回合结束后回滚"
    );
    assert!(
        !combat::keyword_set(&engine.catalog, source).contains_name("taunt"),
        "combat 关键词在结算后移除"
    );
}

#[test]
fn devour_only_accepts_target_at_or_below_limit() {
    let (mut engine, mut state) = setup(11);
    add_card(&mut engine, "devour-source", 0, 0, &[]);
    add_card(&mut engine, "devour-small", 3, 5, &[]);
    add_card(&mut engine, "devour-big", 4, 5, &[]);
    place(&engine.catalog, &mut state, 0, 1, "devour-source");
    place(&engine.catalog, &mut state, 1, 10, "devour-small");
    place(&engine.catalog, &mut state, 1, 11, "devour-big");
    state.phase = Phase::Combat;

    let devour = effect("OnCombatStart|Devour|enemy:single|value=3");
    let small_ctx = EffectCtx {
        source: Some(1),
        target: Some(10),
        ..ctx(0)
    };
    engine.apply_op(&mut state, &devour, &small_ctx, None);

    assert_eq!(
        board_minion(&state, 1, 0).stats.health.current,
        0,
        "攻击力 3 的目标被致死"
    );
    let source = board_minion(&state, 0, 0);
    assert_eq!(source.stats.attack.permanent, 3, "永久获得目标卡面攻击力");
    assert_eq!(source.stats.health.permanent, 5, "永久获得目标卡面生命值");

    let big_ctx = EffectCtx {
        source: Some(1),
        target: Some(11),
        ..ctx(0)
    };
    engine.apply_op(&mut state, &devour, &big_ctx, None);

    let big = board_minion(&state, 1, 1);
    assert_eq!(
        big.stats.health.current, 5,
        "攻击力 4 超过 value=3，目标不受影响"
    );
    let source = board_minion(&state, 0, 0);
    assert_eq!(source.stats.attack.permanent, 3, "超限时不获得属性");
    assert_eq!(source.stats.health.permanent, 5, "超限时不获得属性");
}

#[test]
fn devour_and_mimic_are_silent_when_target_is_missing() {
    let (mut engine, mut state) = setup(11);
    add_card(&mut engine, "quiet-source", 0, 0, &[]);
    add_card(&mut engine, "quiet-friend", 5, 5, &["taunt"]);
    place(&engine.catalog, &mut state, 0, 1, "quiet-source");
    place(&engine.catalog, &mut state, 0, 2, "quiet-friend");
    state.phase = Phase::Combat;

    let effects = [
        // 目标实例不存在时拟态落空。
        effect("OnCombatStart|Mimic|target|duration=combat"),
        effect("OnCombatStart|Devour|enemy:single|value=3"),
        // 吞噬仍只接受敌方目标；指向友方的显式目标落空。
        effect("OnCombatStart|Devour|target|value=3"),
    ];
    for effect in effects {
        let target = if effect.op == Op::Mimic {
            Some(99)
        } else {
            Some(2)
        };
        let events = engine.apply_op(
            &mut state,
            &effect,
            &EffectCtx {
                source: Some(1),
                target,
                ..ctx(0)
            },
            None,
        );
        assert!(events.is_empty(), "落空不产生事件");
    }

    let source = board_minion(&state, 0, 0);
    assert_eq!(combat::effective_attack(&engine.catalog, source), 0);
    assert_eq!(combat::effective_health(&engine.catalog, source), 0);
    assert!(source.granted.is_empty(), "友方目标不复制关键词");
    assert_eq!(source.stats.attack.permanent, 0, "友方目标不触发永久获得");
    let friend = board_minion(&state, 0, 1);
    assert_eq!(friend.stats.health.current, 5, "友方目标不被消灭");
}

#[test]
fn mimic_allows_friendly_target_and_stats_only_skips_keywords() {
    let (mut engine, mut state) = setup(11);
    add_card(&mut engine, "mimic-friendly-source", 0, 0, &[]);
    add_card(&mut engine, "mimic-friendly-target", 5, 5, &["taunt"]);
    place(&engine.catalog, &mut state, 0, 1, "mimic-friendly-source");
    place(&engine.catalog, &mut state, 0, 2, "mimic-friendly-target");
    state.phase = Phase::Combat;

    let mimic = effect("OnCombatStart|Mimic|friendly:single|duration=combat,statsOnly=true");
    let mimic_ctx = EffectCtx {
        source: Some(1),
        target: Some(2),
        ..ctx(0)
    };
    let events = engine.apply_op(&mut state, &mimic, &mimic_ctx, None);

    assert!(events.is_empty(), "拟态不产生事件");
    let source = board_minion(&state, 0, 0);
    assert_eq!(combat::effective_attack(&engine.catalog, source), 5);
    assert_eq!(combat::effective_health(&engine.catalog, source), 5);
    assert!(
        source.granted.is_empty(),
        "statsOnly 只复制攻防，不复制关键词"
    );

    // 重复结算仍以目标当前值为准，不叠加。
    engine.apply_op(&mut state, &mimic, &mimic_ctx, None);
    let source = board_minion(&state, 0, 0);
    assert_eq!(combat::effective_attack(&engine.catalog, source), 5);
    assert_eq!(combat::effective_health(&engine.catalog, source), 5);
    assert!(source.granted.is_empty());
}

#[test]
fn mimic_stats_only_in_combat_skips_keywords() {
    let (mut engine, _state) = setup(11);
    add_effect_card(
        &mut engine,
        "combat-mimic-stats",
        1,
        1,
        &[],
        &["OnCombatStart|Mimic|enemy:single|duration=combat,statsOnly=true"],
    );
    add_card(&mut engine, "combat-mimic-target", 3, 4, &["taunt"]);

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "combat-mimic-stats")],
        &[minion(&engine.catalog, 2, "combat-mimic-target")],
    );

    assert!(
        log.actions.iter().any(|action| matches!(
            action,
            CombatAction::Mimicked {
                source: 1,
                target: 2
            }
        )),
        "战斗内拟态仍记录动作"
    );
    let source = final_minion(&log, 1);
    assert_eq!(source.stats.attack.turn, 2, "复制攻击力增量");
    assert_eq!(source.stats.health.turn, 3, "复制生命值增量");
    assert!(
        !source.granted.iter().any(|(name, _)| name == "taunt"),
        "statsOnly 不复制关键词"
    );
}

fn battle(engine: &Engine, attacker: &[MinionInstance], defender: &[MinionInstance]) -> CombatLog {
    simulate_with(
        engine,
        attacker,
        defender,
        0,
        1,
        0,
        &mut RngStreams::from_seed([23u8; 32]),
        CombatSetup::default(),
    )
}

fn final_minion(log: &CombatLog, id: rule_engine::state::InstanceId) -> &MinionInstance {
    log.attacker_final
        .iter()
        .chain(log.defender_final.iter())
        .find(|minion| minion.instance_id == id)
        .expect("战斗结束后随从仍在快照中")
}

#[test]
fn mimic_in_combat_records_action_and_copies_keywords() {
    let (mut engine, _state) = setup(11);
    add_effect_card(
        &mut engine,
        "combat-mimic",
        1,
        1,
        &[],
        &["OnCombatStart|Mimic|enemy:single|duration=combat"],
    );
    add_card(&mut engine, "combat-victim", 2, 3, &["taunt"]);

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "combat-mimic")],
        &[minion(&engine.catalog, 2, "combat-victim")],
    );

    assert!(
        log.actions.iter().any(|action| matches!(
            action,
            CombatAction::Mimicked {
                source: 1,
                target: 2
            }
        )),
        "战斗内拟态记录 CombatAction::Mimicked"
    );
    let source = final_minion(&log, 1);
    assert_eq!(source.stats.attack.turn, 1, "写入的是达到目标值的增量");
    assert_eq!(source.stats.health.turn, 2, "写入的是达到目标值的增量");
    assert!(
        source
            .granted
            .contains(&("taunt".to_string(), Duration::Combat)),
        "战斗内同样复制关键词"
    );
}

#[test]
fn devour_in_combat_ignores_shield_and_records_action() {
    let (mut engine, _state) = setup(11);
    add_effect_card(
        &mut engine,
        "combat-devour",
        1,
        1,
        &[],
        &["OnCombatStart|Devour|enemy:single|value=3"],
    );
    add_card(&mut engine, "combat-prey", 3, 10, &["divine_shield"]);

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "combat-devour")],
        &[minion(&engine.catalog, 2, "combat-prey")],
    );

    assert!(
        log.actions.iter().any(|action| matches!(
            action,
            CombatAction::Devoured {
                source: 1,
                target: 2
            }
        )),
        "战斗内吞噬记录 CombatAction::Devoured"
    );
    assert!(
        log.actions
            .iter()
            .any(|action| matches!(action, CombatAction::Death { minion: 2 })),
        "护盾不能阻止吞噬"
    );
    let source = final_minion(&log, 1);
    assert_eq!(source.stats.attack.permanent, 3);
    assert_eq!(source.stats.health.permanent, 10);
    let prey = final_minion(&log, 2);
    assert!(!prey.shield_used, "吞噬不消耗护盾，也不被护盾抵消");
    assert_eq!(prey.stats.health.current, 0);
}

#[test]
fn mimic_in_combat_excludes_aura_stats_and_keywords() {
    let (mut engine, _state) = setup(11);
    add_effect_card(
        &mut engine,
        "combat-mimic-plain",
        1,
        1,
        &[],
        &["OnCombatStart|Mimic|enemy:single|duration=combat"],
    );
    add_card(&mut engine, "combat-aura-target", 2, 3, &["taunt"]);

    // 战斗克隆已折叠光环：目标卡面 2/3，光环 +3/+3 后当前值 5/6；
    // 光环还授予「无视嘲讽」。拟态只应复制不含光环的 2/3 与卡面关键词。
    let mut target = minion(&engine.catalog, 2, "combat-aura-target");
    target.stats.attack.set_aura(3);
    target.stats.health.set_aura(3);
    target
        .granted
        .push(("ignore_taunt".to_string(), Duration::Aura));

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "combat-mimic-plain")],
        &[target],
    );

    let source = final_minion(&log, 1);
    assert_eq!(
        source.stats.attack.turn, 1,
        "复制 2 点攻击力而非光环后的 5 点"
    );
    assert_eq!(
        source.stats.health.turn, 2,
        "复制 3 点生命值而非光环后的 6 点"
    );
    assert!(
        source
            .granted
            .contains(&("taunt".to_string(), Duration::Combat)),
        "卡面关键词照常复制"
    );
    assert!(
        !combat::keyword_set(&engine.catalog, source).contains_name("ignore_taunt"),
        "光环授予的关键词不复制"
    );
}
