mod common;

use std::collections::BTreeSet;

use common::{effects_of, minion, place, set_shop, setup, test_catalog, test_playstyles};
use rule_engine::achievements::{Achievement, STEAM_ENGINE_CARD_ID};
use rule_engine::data::{Card, CardType, Catalog, Era, Playstyle, Skill, card_has_playstyle};
use rule_engine::effects::Duration;
use rule_engine::engine::Engine;
use rule_engine::hash::state_hash;
use rule_engine::intent::Intent;
use rule_engine::settle::Pairing;
use rule_engine::state::{GameState, HandCard, PendingChoice, Phase};

/// 往目录里插入一张带指定技能的合成随从卡。
fn add_skill_card(
    catalog: &mut Catalog,
    id: &str,
    subtype: Option<&str>,
    attack: i32,
    health: i32,
    raw: &str,
) {
    let skill_id = catalog.insert_skill(Skill {
        id: format!("{id}-skill"),
        keyword: None,
        effects: effects_of(raw),
        cost: None,
        uses_per_turn: None,
    });
    catalog.cards.insert(
        id.to_string(),
        Card {
            id: id.to_string(),
            card_type: CardType::Minion,
            in_shop_pool: true,
            taxon: "hominidae".to_string(),
            categories: Vec::new(),
            subtype: subtype.map(str::to_string),
            level: 1,
            era: None,
            attack: Some(attack),
            health: Some(health),
            skills: vec![skill_id],
            cost: Some(1),
        },
    );
}

/// 把引擎摆到战斗阶段，并设置一场固定配对。
fn combat_pairing(state: &mut GameState) {
    state.phase = Phase::Combat;
    state.first_seat = 0;
    state.last_winner = None;
    state.pairings = vec![Pairing {
        a: 0,
        b: 1,
        ghost: None,
    }];
}

/// 引擎内的成就标识必须与生成数据一一对应，防止实体文档与运行时漂移。
#[test]
fn achievement_ids_match_generated_data() {
    let raw = include_str!("../../../data/v0.2/achievements.json");
    let records: Vec<serde_json::Value> = serde_json::from_str(raw).expect("成就数据应可解析");
    let mut ids: Vec<String> = records
        .iter()
        .map(|record| {
            record
                .get("id")
                .and_then(serde_json::Value::as_str)
                .expect("成就数据缺少 id")
                .to_string()
        })
        .collect();
    ids.sort();
    let engine_ids: Vec<String> = Achievement::ALL
        .into_iter()
        .map(|achievement| achievement.id().to_string())
        .collect();
    assert_eq!(ids, engine_ids);
}

#[test]
fn achievement_id_round_trips() {
    for achievement in Achievement::ALL {
        assert_eq!(Achievement::from_id(achievement.id()), Some(achievement));
    }
    assert_eq!(Achievement::from_id("no-such-achievement"), None);
}

#[test]
fn achievement_progress_participates_in_state_hash() {
    let (_, mut state) = Engine::new(test_catalog(), [9; 32], test_playstyles());
    let before = state_hash(&state);
    state.players[0]
        .achievements
        .unlocked
        .insert(Achievement::Genetics);
    let unlocked = state_hash(&state);
    assert_ne!(before, unlocked);

    state.players[0]
        .achievements
        .grown_subtypes
        .insert("plant".into());
    assert_ne!(unlocked, state_hash(&state));
}

fn play_effect(engine: &Engine, state: &mut GameState, card_id: &str) {
    let instance_id = state.next_instance_id;
    state.next_instance_id = instance_id.saturating_add(1);
    state.players[0]
        .hand
        .push(HandCard::effect(instance_id, card_id));
    let hand_index = u8::try_from(state.players[0].hand.len() - 1).unwrap();
    engine
        .apply(
            state,
            0,
            Intent::PlayCard {
                hand_index,
                position: None,
                target: None,
            },
        )
        .expect("打出效果牌应成功");
}

#[test]
fn biological_evolution_counts_growth_events_once_per_game() {
    let (engine, mut state) = setup(3);
    place(&engine.catalog, &mut state, 0, 1, "gear");
    let gold = state.players[0].gold;

    for _ in 0..5 {
        play_effect(&engine, &mut state, "nahan");
    }

    let progress = &state.players[0].achievements;
    assert!(
        progress
            .unlocked
            .contains(&Achievement::BiologicalEvolution)
    );
    assert_eq!(progress.growth_events, 5, "每次成长结算计 1 次");
    assert_eq!(
        state.players[0].gold,
        gold.saturating_add(1),
        "生物进化奖励只发一次"
    );
    assert!(!progress.unlocked.contains(&Achievement::EducationForAll));
}

#[test]
fn growth_layers_do_not_repeat_count() {
    let (engine, mut state) = setup(9);
    place(&engine.catalog, &mut state, 0, 1, "gear");

    // 一次 2 层成长仍只记 1 次结算。
    play_effect(&engine, &mut state, "wenming-feiyue");

    assert_eq!(state.players[0].achievements.growth_events, 1);
}

#[test]
fn education_for_all_requires_five_distinct_grown_subtypes() {
    let (engine, mut state) = setup(4);
    for (index, card_id) in ["qinghao", "juhua", "maitian", "zhulin", "tai"]
        .into_iter()
        .enumerate()
    {
        place(&engine.catalog, &mut state, 0, 100 + index as u64, card_id);
    }

    play_effect(&engine, &mut state, "nahan");

    let progress = &state.players[0].achievements;
    assert!(
        progress
            .unlocked
            .contains(&Achievement::BiologicalEvolution)
    );
    assert!(progress.unlocked.contains(&Achievement::EducationForAll));
    assert_eq!(progress.grown_subtypes.len(), 5);
    assert_eq!(state.players[0].hand.len(), 1, "奖励一张效果牌");
    assert!(matches!(state.players[0].hand[0], HandCard::Effect { .. }));
}

#[test]
fn breed_growth_counts_and_grants_biological_gold_this_turn() {
    let (engine, mut state) = setup(8);
    for index in 0..3u64 {
        place(&engine.catalog, &mut state, 0, 1 + index, "baiyi");
    }

    engine.start_turn(&mut state).expect("开始回合应成功");

    let progress = &state.players[0].achievements;
    assert_eq!(progress.growth_events, 3, "繁衍属于成长结算");
    assert!(
        progress
            .unlocked
            .contains(&Achievement::BiologicalEvolution)
    );
    let expected = engine.catalog.config.gold_curve[1].saturating_add(1);
    assert_eq!(
        state.players[0].gold, expected,
        "繁衍触发的 +1 金币当回合可用"
    );
    assert_eq!(progress.next_turn_gold_bonus, 0);
}

#[test]
fn genetics_triggers_on_plant_triple_and_offers_plant_discover() {
    let (engine, mut state) = setup(5);
    state.enabled_playstyles = BTreeSet::from([Playstyle::Plant]);
    for index in 0..3u64 {
        state.players[0].hand.push(HandCard::Minion(minion(
            &engine.catalog,
            200 + index,
            "ganlanshu",
        )));
    }

    engine.check_triple(&mut state, 0);

    assert!(
        state.players[0]
            .achievements
            .unlocked
            .contains(&Achievement::Genetics)
    );
    let candidates = state
        .pending
        .iter()
        .find_map(|choice| match choice {
            PendingChoice::Discover { candidates, .. } => Some(candidates.clone()),
            _ => None,
        })
        .expect("遗传学应登记发现待选");
    assert!(!candidates.is_empty());
    for candidate in candidates {
        let card = engine
            .catalog
            .cards
            .get(&candidate)
            .expect("候选应在目录中");
        assert!(
            card_has_playstyle(card, Playstyle::Plant),
            "候选 {candidate} 应为植物"
        );
    }
}

#[test]
fn machines_roar_triggers_and_discounts_machine_purchases_this_turn() {
    let (engine, mut state) = setup(6);
    state.players[0].hand.push(HandCard::Minion(minion(
        &engine.catalog,
        900,
        STEAM_ENGINE_CARD_ID,
    )));
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
        .expect("打出蒸汽机应成功");

    let progress = &state.players[0].achievements;
    assert!(progress.unlocked.contains(&Achievement::MachinesRoar));
    assert_eq!(progress.machine_discount_turn, Some(state.turn));

    set_shop(&mut state, 0, &[Some("gear".to_string())]);
    state.players[0].gold = 5;
    engine
        .apply(&mut state, 0, Intent::BuyMinion { slot: 0 })
        .expect("购买机械随从应成功");
    assert_eq!(state.players[0].gold, 3, "机械随从本回合便宜 1 金币");
}

#[test]
fn one_small_step_triggers_on_future_era_and_grants_next_turn_gold() {
    let (engine, mut state) = setup(7);
    state.players[0].era = Era::Modern;
    state.players[0].gold = u8::try_from(engine.catalog.config.era_costs[5]).unwrap();

    engine
        .apply(&mut state, 0, Intent::UpgradeEra)
        .expect("升级到未来时代应成功");

    let progress = &state.players[0].achievements;
    assert_eq!(state.players[0].era, Era::Future);
    assert!(progress.unlocked.contains(&Achievement::OneSmallStep));
    assert_eq!(progress.next_turn_gold_bonus, 1);

    let expected = engine.catalog.config.gold_curve[1].saturating_add(1);
    engine.start_turn(&mut state).expect("开始回合应成功");
    assert_eq!(state.players[0].gold, expected);
    assert_eq!(state.players[0].achievements.next_turn_gold_bonus, 0);
}

#[test]
fn turing_moment_triggers_when_machine_has_ignore_taunt_and_windfury() {
    let (engine, mut state) = setup(10);
    place(&engine.catalog, &mut state, 0, 1, "gear");
    let gear = &mut state.players[0].board[0];
    gear.granted
        .push(("ignore_taunt".to_string(), Duration::Permanent));
    gear.granted
        .push(("windfury".to_string(), Duration::Permanent));

    engine
        .apply(&mut state, 0, Intent::FreezeShop { slot: None })
        .expect("任意招募意图都应触发关键词检查");

    assert!(
        state.players[0]
            .achievements
            .unlocked
            .contains(&Achievement::TuringMoment)
    );
    let gear = &state.players[0].board[0];
    assert_eq!(gear.stats.attack.permanent, 2);
    assert_eq!(gear.stats.health.permanent, 2);
}

#[test]
fn combat_growth_counts_once_and_defers_biological_gold() {
    let (mut engine, mut state) = setup(11);
    add_skill_card(
        &mut engine.catalog,
        "combat-grower",
        None,
        10,
        10,
        "OnCombatStart|Growth|friendly:all|times=1",
    );
    add_skill_card(&mut engine.catalog, "plain-a", None, 10, 10, "[]");
    add_skill_card(&mut engine.catalog, "plain-b", None, 10, 10, "[]");
    add_skill_card(&mut engine.catalog, "punching-bag", None, 1, 1, "[]");

    state.turn = 5;
    state.players[0].gold = 3;
    state.players[0].board = vec![
        minion(&engine.catalog, 10, "combat-grower"),
        minion(&engine.catalog, 11, "plain-a"),
        minion(&engine.catalog, 12, "plain-b"),
    ];
    state.players[1].board = vec![minion(&engine.catalog, 20, "punching-bag")];
    combat_pairing(&mut state);

    engine.resolve_combat(&mut state).expect("战斗结算应成功");

    let progress = &state.players[0].achievements;
    assert_eq!(
        progress.growth_events, 3,
        "每个被成长的随从计 1 次，战斗写回不重复计"
    );
    assert!(
        progress
            .unlocked
            .contains(&Achievement::BiologicalEvolution)
    );
    assert_eq!(state.players[0].gold, 3, "战斗中的金币奖励不顺延到当回合");
    assert_eq!(progress.next_turn_gold_bonus, 1);

    let expected = engine.catalog.config.gold_curve[5].saturating_add(1);
    engine.start_turn(&mut state).expect("开始回合应成功");
    assert_eq!(state.players[0].gold, expected, "战斗奖励在下一回合到账");
}

#[test]
fn education_for_all_grants_effect_immediately_after_combat() {
    let (mut engine, mut state) = setup(12);
    let skill_id = engine.catalog.insert_skill(Skill {
        id: "combat-growth-all".to_string(),
        keyword: None,
        effects: effects_of("OnCombatStart|Growth|friendly:all|times=1"),
        cost: None,
        uses_per_turn: None,
    });
    engine
        .catalog
        .cards
        .get_mut("qinghao")
        .expect("测试卡应存在")
        .skills
        .push(skill_id);
    add_skill_card(&mut engine.catalog, "punching-bag", None, 1, 1, "[]");

    state.turn = 4;
    for (index, card_id) in ["qinghao", "juhua", "maitian", "zhulin", "tai"]
        .into_iter()
        .enumerate()
    {
        state.players[0]
            .board
            .push(minion(&engine.catalog, 30 + index as u64, card_id));
    }
    state.players[1].board = vec![minion(&engine.catalog, 40, "punching-bag")];
    combat_pairing(&mut state);

    engine.resolve_combat(&mut state).expect("战斗结算应成功");

    let progress = &state.players[0].achievements;
    assert!(progress.unlocked.contains(&Achievement::EducationForAll));
    assert_eq!(progress.grown_subtypes.len(), 5);
    assert_eq!(state.players[0].hand.len(), 1, "战斗结算后立刻发放效果牌");
    assert!(matches!(state.players[0].hand[0], HandCard::Effect { .. }));
    assert_eq!(progress.next_turn_gold_bonus, 1);
}

#[test]
fn machines_roar_triggers_when_steam_engine_is_summoned_in_recruit() {
    let (mut engine, mut state) = setup(13);
    add_skill_card(
        &mut engine.catalog,
        "summoner",
        None,
        2,
        2,
        "OnPlay|Summon|self|card=steam-engine,count=1",
    );
    state.players[0]
        .hand
        .push(HandCard::Minion(minion(&engine.catalog, 50, "summoner")));
    state.pool.insert(STEAM_ENGINE_CARD_ID.to_string(), 5);

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
        .expect("打出召唤随从应成功");

    let progress = &state.players[0].achievements;
    assert!(progress.unlocked.contains(&Achievement::MachinesRoar));
    assert_eq!(progress.machine_discount_turn, Some(state.turn));
}

#[test]
fn machines_roar_triggers_when_steam_engine_is_summoned_in_combat() {
    let (mut engine, mut state) = setup(14);
    add_skill_card(
        &mut engine.catalog,
        "mech-bomb",
        None,
        1,
        1,
        "OnDeath|Summon|self|card=steam-engine,count=1",
    );
    add_skill_card(&mut engine.catalog, "club", None, 10, 10, "[]");

    state.turn = 7;
    state.players[0].board = vec![minion(&engine.catalog, 60, "mech-bomb")];
    state.players[1].board = vec![minion(&engine.catalog, 61, "club")];
    state.pool.insert(STEAM_ENGINE_CARD_ID.to_string(), 5);
    combat_pairing(&mut state);

    engine.resolve_combat(&mut state).expect("战斗结算应成功");

    let progress = &state.players[0].achievements;
    assert!(progress.unlocked.contains(&Achievement::MachinesRoar));
    assert_eq!(
        progress.machine_discount_turn,
        Some(8),
        "战斗内触发的折扣留到下一次招募"
    );
}
