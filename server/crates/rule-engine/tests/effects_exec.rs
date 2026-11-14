mod common;

use std::collections::BTreeSet;

use common::test_catalog;
use rule_engine::combat;
use rule_engine::effects::{
    Duration, Effect, EffectCtx, Op, Trigger, effect_coverage, parse_effects,
};
use rule_engine::engine::Engine;
use rule_engine::events::GameEvent;
use rule_engine::hash::state_hash;
use rule_engine::intent::{Intent, RejectCode};
use rule_engine::state::{GameState, InstanceId, MinionInstance, Phase, Seat, ShopSlot};

fn setup() -> (Engine, GameState) {
    let (engine, mut state) = Engine::new(test_catalog(), [11u8; 32]);
    state.phase = Phase::Recruit;
    state.turn = 1;
    for player in &mut state.players {
        player.hp = 30;
    }
    (engine, state)
}

fn minion(instance_id: InstanceId, card_id: &str) -> MinionInstance {
    MinionInstance {
        instance_id,
        card_id: card_id.to_string(),
        golden: false,
        growth: 0,
        growth_health_only: 0,
        breed: 0,
        breed_applied: 0,
        contagion: 0,
        granted: Vec::new(),
        mods: Vec::new(),
        shield_used: false,
        reborn_used: false,
        attacks_left: 0,
        cannot_attack: false,
    }
}

fn place(state: &mut GameState, seat: Seat, id: InstanceId, card_id: &str) {
    state.players[usize::from(seat)]
        .board
        .push(minion(id, card_id));
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

#[test]
fn battlecry_growth_adds_permanent_stats() {
    let (mut engine, mut state) = setup();
    place(&mut state, 0, 1, "pink-ape");
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
    assert_eq!(grown.growth, 2, "战吼：获得两次成长");
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
fn scholar_growth_adds_health_only() {
    let (mut engine, mut state) = setup();
    place(&mut state, 0, 1, "aristotle");

    engine.apply_op(
        &mut state,
        &effect("OnPlay|Growth|friendly:all|times=1"),
        &ctx(0),
        None,
    );

    let scholar = board_minion(&state, 0, 0);
    assert_eq!(scholar.growth, 0, "学者成长不加攻击");
    assert_eq!(scholar.growth_health_only, 1, "学者成长只加生命");
    assert_eq!(combat::effective_attack(&engine.catalog, scholar), 0);
    assert_eq!(combat::effective_health(&engine.catalog, scholar), 10);
}

#[test]
fn health_only_growth_adds_no_attack() {
    let (mut engine, mut state) = setup();
    place(&mut state, 0, 1, "yezhu");

    engine.apply_op(
        &mut state,
        &effect("OnPlay|Growth|friendly:all|times=2,healthOnly=true"),
        &ctx(0),
        None,
    );

    let grown = board_minion(&state, 0, 0);
    assert_eq!(grown.growth, 0);
    assert_eq!(grown.growth_health_only, 2);
    assert_eq!(combat::effective_attack(&engine.catalog, grown), 2);
    assert_eq!(combat::effective_health(&engine.catalog, grown), 4);
}

#[test]
fn modify_stats_permanent_and_combat() {
    let (mut engine, mut state) = setup();
    place(&mut state, 0, 1, "yezhu");
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
    assert_eq!(board_minion(&state, 0, 0).mods.len(), 2);

    state.phase = Phase::Settle;
    engine.resolve_settle(&mut state).expect("结算应成功");

    let minion = board_minion(&state, 0, 0);
    assert_eq!(minion.mods.len(), 1, "combat 修正在 Settle 前清除");
    assert_eq!(minion.mods[0].duration, Duration::Permanent);
    assert_eq!(minion.mods[0].attack, 1);
    assert_eq!(minion.mods[0].health, 2);
}

#[test]
fn grant_keyword_respects_duration() {
    let (mut engine, mut state) = setup();
    place(&mut state, 0, 1, "yezhu");
    place(&mut state, 0, 2, "yezhu");
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
        &effect("OnPlay|GrantKeyword|self|keyword=护盾,duration=permanent"),
        &first,
        None,
    );
    engine.apply_op(
        &mut state,
        &effect("OnPlay|GrantKeyword|self|keyword=连击,duration=combat"),
        &second,
        None,
    );
    engine.apply_op(
        &mut state,
        &effect("OnPlay|GrantKeyword|self|keyword=繁衍,value=2,duration=permanent"),
        &first,
        None,
    );

    let shielded = board_minion(&state, 0, 0);
    assert!(
        shielded
            .granted
            .contains(&("护盾".into(), Duration::Permanent))
    );
    assert!(!shielded.shield_used, "授予护盾会重置已用标记");
    assert_eq!(shielded.breed, 2, "繁衍层数写入 breed，不进入 granted");
    assert!(!shielded.granted.iter().any(|(name, _)| name == "繁衍"));

    let windfury = board_minion(&state, 0, 1);
    assert!(
        windfury
            .granted
            .contains(&("连击".into(), Duration::Combat))
    );
    assert!(combat::keywords(&engine.catalog, windfury).contains("连击"));

    state.phase = Phase::Settle;
    engine.resolve_settle(&mut state).expect("结算应成功");
    assert!(
        !combat::keywords(&engine.catalog, board_minion(&state, 0, 1)).contains("连击"),
        "combat 关键词在 Settle 前清除"
    );
    assert!(
        board_minion(&state, 0, 0)
            .granted
            .iter()
            .any(|(name, duration)| name == "护盾" && *duration == Duration::Permanent),
        "permanent 关键词保留"
    );
}

#[test]
fn remove_growth_reduces_growth_count() {
    let (mut engine, mut state) = setup();
    place(&mut state, 0, 1, "yezhu");
    state.players[0].board[0].growth = 5;
    state.players[0].board[0].growth_health_only = 3;

    engine.apply_op(
        &mut state,
        &effect("OnCombatStart|RemoveGrowth|friendly:all"),
        &ctx(0),
        None,
    );

    let minion = board_minion(&state, 0, 0);
    assert_eq!(minion.growth, 0);
    assert_eq!(minion.growth_health_only, 0);
}

#[test]
fn cannot_attack_blocks_attacks() {
    let (mut engine, mut state) = setup();
    place(&mut state, 0, 1, "yezhu");

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
            .any(|(name, duration)| name == "禁攻" && *duration == Duration::Combat)
    );

    state.phase = Phase::Settle;
    engine.resolve_settle(&mut state).expect("结算应成功");
    let minion = board_minion(&state, 0, 0);
    assert!(!minion.cannot_attack, "combat 禁攻在 Settle 前解除");
    assert!(minion.granted.is_empty());
}

#[test]
fn deal_damage_uses_selector_and_amount() {
    let (mut engine, mut state) = setup();
    state.phase = Phase::Combat;
    place(&mut state, 1, 10, "yezhu");

    let events = engine.apply_op(
        &mut state,
        &effect("OnCombatStart|DealDamage|enemy:random|amount=2"),
        &ctx(0),
        None,
    );

    assert_eq!(events, vec![GameEvent::DamageTaken { seat: 1, amount: 2 }]);
    let victim = board_minion(&state, 1, 0);
    assert_eq!(victim.mods.len(), 1);
    assert_eq!(victim.mods[0].duration, Duration::Combat);
    assert_eq!(victim.mods[0].health, -2);
    assert_eq!(combat::effective_health(&engine.catalog, victim), 0);
}

#[test]
fn deal_damage_cannot_kill_floors_health_at_one() {
    let (mut engine, mut state) = setup();
    state.phase = Phase::Combat;
    place(&mut state, 1, 10, "yanhuashi");

    let mut damage = effect("OnCombatStart|DealDamage|enemy:all|amount=10");
    damage.params.can_kill = Some(false);
    let events = engine.apply_op(&mut state, &damage, &ctx(0), None);

    assert_eq!(events, vec![GameEvent::DamageTaken { seat: 1, amount: 4 }]);
    let victim = board_minion(&state, 1, 0);
    assert_eq!(combat::effective_health(&engine.catalog, victim), 1);
}

#[test]
fn enemy_selectors_are_empty_during_recruit() {
    let (mut engine, mut state) = setup();
    place(&mut state, 1, 10, "yezhu");
    place(&mut state, 0, 1, "yezhu");

    let events = engine.apply_op(
        &mut state,
        &effect("OnCombatStart|DealDamage|enemy:all|amount=2"),
        &ctx(0),
        None,
    );

    assert!(events.is_empty(), "招募阶段敌方选择器无目标");
    assert!(board_minion(&state, 1, 0).mods.is_empty());
    let enemy_effect = effect("OnCombatStart|DealDamage|enemy:all|amount=2");
    assert!(
        engine
            .resolve_selector(&state, &enemy_effect.selector, &ctx(0))
            .is_empty()
    );
}

#[test]
fn times_if_changes_times_by_tribe() {
    let (mut engine, mut state) = setup();
    place(&mut state, 0, 1, "tengman");
    place(&mut state, 0, 2, "yezhu");
    let growth_effect = effect("OnPlay|Growth|friendly:single|times=1,timesIf=植物:2");

    let plant_ctx = EffectCtx {
        target: Some(1),
        ..ctx(0)
    };
    engine.apply_op(&mut state, &growth_effect, &plant_ctx, None);
    assert_eq!(board_minion(&state, 0, 0).growth, 2, "植物目标改为两次");

    let beast_ctx = EffectCtx {
        target: Some(2),
        ..ctx(0)
    };
    engine.apply_op(&mut state, &growth_effect, &beast_ctx, None);
    assert_eq!(
        board_minion(&state, 0, 1).growth,
        1,
        "其他目标仍是两次基础值"
    );
}

#[test]
fn condition_gates_effect() {
    let (mut engine, mut state) = setup();
    place(&mut state, 0, 1, "buluozhanshi");
    place(&mut state, 0, 2, "bruce-lee");
    let gated = effect(
        "OnActivate|Growth|friendly:single:tribe=人类|times=1,timesIf=人类:2,condition=count:friendly:tribe=人类:>=3",
    );
    let target_ctx = EffectCtx {
        target: Some(1),
        ..ctx(0)
    };

    engine.apply_op(&mut state, &gated, &target_ctx, None);
    assert_eq!(board_minion(&state, 0, 0).growth, 0, "条件不满足不执行");

    place(&mut state, 0, 3, "buluozhanshi");
    engine.apply_op(&mut state, &gated, &target_ctx, None);
    assert_eq!(
        board_minion(&state, 0, 0).growth,
        2,
        "三名人类时 timesIf 生效"
    );
}

#[test]
fn golden_triggers_battlecry_twice() {
    let (mut engine, mut state) = setup();
    place(&mut state, 0, 1, "pink-ape");

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
        board_minion(&state, 0, 0).growth,
        4,
        "金色整段效果连续执行两次"
    );
}

#[test]
fn on_activate_respects_cost_and_uses() {
    let (mut engine, mut state) = setup();
    state.players[0].hero = Some("luxun".to_string());
    state.players[0].gold = 5;
    place(&mut state, 0, 1, "yezhu");

    engine
        .apply(
            &mut state,
            0,
            Intent::UseHeroPower {
                skill_id: "shout".to_string(),
                target: Some(1),
            },
        )
        .expect("首次使用应成功");
    assert_eq!(state.players[0].gold, 4);
    assert_eq!(board_minion(&state, 0, 0).growth, 1);

    engine
        .apply(
            &mut state,
            0,
            Intent::UseHeroPower {
                skill_id: "shout".to_string(),
                target: Some(1),
            },
        )
        .expect("每回合可用两次");
    assert_eq!(state.players[0].gold, 3);
    assert_eq!(board_minion(&state, 0, 0).growth, 2);

    let before = state_hash(&state);
    let rejected = engine.apply(
        &mut state,
        0,
        Intent::UseHeroPower {
            skill_id: "shout".to_string(),
            target: Some(1),
        },
    );
    assert_eq!(rejected.unwrap_err().code, RejectCode::HeroPowerLimit);
    assert_eq!(before, state_hash(&state));
}

#[test]
fn buy_fires_on_play_after_bought_event() {
    let (mut engine, mut state) = setup();
    state.players[0].gold = 10;
    state.players[0].shop = vec![Some(ShopSlot {
        card_id: "yezhu".to_string(),
        frozen: false,
    })];

    let outcome = engine
        .apply(
            &mut state,
            0,
            Intent::BuyMinion {
                slot: 0,
                target: None,
            },
        )
        .expect("购买应成功");

    assert_eq!(
        outcome.events[0],
        GameEvent::MinionBought {
            seat: 0,
            slot: 0,
            card_id: "yezhu".to_string(),
        }
    );
    let bought = board_minion(&state, 0, 0);
    assert_eq!(bought.mods.len(), 1, "战吼：获得 +1 生命");
    assert_eq!(bought.mods[0].health, 1);
    assert_eq!(bought.mods[0].duration, Duration::Permanent);
}

#[test]
fn play_effect_card_fires_effect_then_on_use() {
    let (mut engine, mut state) = setup();
    place(&mut state, 0, 1, "yanhuashi");
    state.players[0].hand.push(rule_engine::state::HandCard {
        card_id: "aolinpike-jidian".to_string(),
        golden: false,
    });

    engine
        .apply(
            &mut state,
            0,
            Intent::PlayCard {
                hand_index: 0,
                target: Some(1),
            },
        )
        .expect("打出效果牌应成功");

    let responder = board_minion(&state, 0, 0);
    assert_eq!(
        responder.growth_health_only, 1,
        "效果牌自身：学者获得一次成长（+0/+1）"
    );
    assert_eq!(responder.mods.len(), 1, "OnUse 响应：目标 +1 生命");
    assert_eq!(responder.mods[0].health, 1);
}

#[test]
fn unimplemented_triggers_return_empty() {
    let (mut engine, mut state) = setup();
    place(&mut state, 0, 1, "pink-ape");

    let events = engine.fire(
        &mut state,
        Trigger::OnDeath,
        EffectCtx {
            source: Some(1),
            ..ctx(0)
        },
    );
    assert!(events.is_empty());
    assert_eq!(board_minion(&state, 0, 0).growth, 0);
}

#[test]
fn coverage_counts_all_effect_segments() {
    let catalog = test_catalog();
    let coverage = effect_coverage(&catalog);

    let total: usize = coverage.values().sum();
    assert_eq!(total, 109, "已提交数据的全部效果段都应被统计");
    assert_eq!(coverage.get(&(Trigger::OnPlay, Op::Growth)), Some(&13));
    assert_eq!(coverage.get(&(Trigger::OnPlay, Op::ModifyStats)), Some(&10));
    assert_eq!(coverage.get(&(Trigger::OnUse, Op::ModifyStats)), Some(&1));

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
    ]
    .into_iter()
    .collect();
    assert!(
        coverage.keys().all(|(_, op)| executable.contains(op)),
        "所有出现过的动作都应被 apply_op 覆盖"
    );
}

#[test]
fn recompute_stats_drops_empty_modifiers() {
    let (mut engine, mut state) = setup();
    place(&mut state, 0, 1, "yezhu");
    state.players[0].board[0]
        .mods
        .push(rule_engine::state::Modifier {
            attack: 0,
            health: 0,
            duration: Duration::Permanent,
        });
    state.players[0].board[0]
        .mods
        .push(rule_engine::state::Modifier {
            attack: 2,
            health: 0,
            duration: Duration::Permanent,
        });

    let mut minion = state.players[0].board[0].clone();
    engine.recompute_stats(&mut state, &mut minion);
    assert_eq!(minion.mods.len(), 1);
    assert_eq!(minion.mods[0].attack, 2);
}

#[test]
fn selector_picks_cover_leftmost_adjacent_top_attack_and_other() {
    let (engine, mut state) = setup();
    place(&mut state, 0, 1, "yezhu");
    place(&mut state, 0, 2, "bruce-lee");
    place(&mut state, 0, 3, "tengman");

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
