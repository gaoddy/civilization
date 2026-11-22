//! T03：效果与系统写点记账。
//!
//! 覆盖招募与战斗两条路径的属性写入来源：卡牌 + 技能归因、系统来源
//! （进化、繁衍、传染、复苏、回手、战斗成长）以及临时层 / 光环来源的清除。

mod common;

use common::{categories_for, effects_of, minion, place, setup, setup_with_gold};
use rule_engine::combat::{CombatLog, CombatSetup, simulate_with};
use rule_engine::data::{Card, CardType, Playstyle, Skill};
use rule_engine::effects::{EffectCtx, Trigger};
use rule_engine::engine::Engine;
use rule_engine::intent::Intent;
use rule_engine::rng::RngStreams;
use rule_engine::settle::Pairing;
use rule_engine::state::{
    InstanceId, MinionInstance, PendingChoice, Phase, StatSourceLayer, StatSourceOrigin,
    StatSystemSource,
};

/// 向目录插入一张带若干 `(skill_id, effects)` 技能的合成随从卡。
///
/// 招募测试用 1/1；战斗测试需要可控的基础攻防时改用 [`add_combat_card`]。
fn add_effect_card(engine: &mut Engine, card_id: &str, skills: &[(&str, &str)]) {
    add_combat_card(engine, card_id, 1, 1, skills);
}

fn add_combat_card(
    engine: &mut Engine,
    card_id: &str,
    attack: i32,
    health: i32,
    skills: &[(&str, &str)],
) {
    let skill_ids = skills
        .iter()
        .map(|(skill_id, raw)| {
            engine.catalog.insert_skill(Skill {
                id: (*skill_id).to_string(),
                keyword: None,
                effects: effects_of(raw),
                cost: None,
                uses_per_turn: None,
            })
        })
        .collect();
    engine.catalog.cards.insert(
        card_id.to_string(),
        Card {
            id: card_id.to_string(),
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

fn ctx() -> EffectCtx {
    EffectCtx {
        seat: 0,
        source: None,
        target: None,
        event_target: None,
        golden: false,
        level_override: None,
    }
}

fn card_origin(card_id: &str) -> StatSourceOrigin {
    StatSourceOrigin::Card {
        card_id: Some(card_id.to_string()),
        skill_id: None,
    }
}

fn system(origin: StatSystemSource) -> StatSourceOrigin {
    StatSourceOrigin::System(origin)
}

fn battle(engine: &Engine, attacker: &[MinionInstance], defender: &[MinionInstance]) -> CombatLog {
    let mut rng = RngStreams::from_seed([13u8; 32]);
    simulate_with(
        engine,
        attacker,
        defender,
        0,
        1,
        0,
        &mut rng,
        CombatSetup::default(),
    )
}

fn final_minion(log: &CombatLog, id: InstanceId) -> &MinionInstance {
    log.attacker_final
        .iter()
        .chain(log.defender_final.iter())
        .find(|minion| minion.instance_id == id)
        .expect("战斗结束快照应包含该随从")
}

/// 某一层来源的攻防合计。
fn layer_sum(minion: &MinionInstance, layer: StatSourceLayer) -> (i32, i32) {
    minion
        .stat_sources
        .iter()
        .filter(|source| source.layer == layer)
        .fold((0, 0), |(attack, health), source| {
            (attack + source.attack, health + source.health)
        })
}

/// 不变式：各层来源之和必须等于对应聚合层的数值。
fn assert_layer_sums(minion: &MinionInstance) {
    assert_eq!(
        layer_sum(minion, StatSourceLayer::Growth),
        (minion.stats.attack.growth, minion.stats.health.growth),
        "成长层来源和应等于成长层"
    );
    assert_eq!(
        layer_sum(minion, StatSourceLayer::Permanent),
        (minion.stats.attack.permanent, minion.stats.health.permanent),
        "永久层来源和应等于永久层"
    );
    assert_eq!(
        layer_sum(minion, StatSourceLayer::Turn),
        (minion.stats.attack.turn, minion.stats.health.turn),
        "本回合层来源和应等于本回合层"
    );
    assert_eq!(
        layer_sum(minion, StatSourceLayer::Aura),
        (minion.stats.attack.aura, minion.stats.health.aura),
        "光环层来源和应等于光环层"
    );
}

// ---------------------------------------------------------------------------
// 招募：卡牌 + 技能归因
// ---------------------------------------------------------------------------

#[test]
fn same_skill_twice_merges_one_row() {
    let (mut engine, mut state) = setup(5);
    place(&engine.catalog, &mut state, 0, 10, "jianchihu");
    add_effect_card(
        &mut engine,
        "buffer",
        &[(
            "buffer-skill",
            "OnPlay|ModifyStats|friendly:all|attack=1,health=2",
        )],
    );

    engine.fire_card(&mut state, "buffer", Trigger::OnPlay, ctx());
    engine.fire_card(&mut state, "buffer", Trigger::OnPlay, ctx());

    let rows: Vec<_> = state.players[0].board[0]
        .stat_sources
        .iter()
        .filter(|source| source.layer == StatSourceLayer::Permanent)
        .collect();
    assert_eq!(rows.len(), 1, "同一技能触发两次只占一行");
    assert_eq!((rows[0].attack, rows[0].health), (2, 4));
    assert_eq!(
        rows[0].origin,
        StatSourceOrigin::Card {
            card_id: Some("buffer".to_string()),
            skill_id: Some(engine.catalog.skill_id("buffer-skill").unwrap()),
        }
    );
}

#[test]
fn same_card_different_skills_stay_separate() {
    let (mut engine, mut state) = setup(5);
    place(&engine.catalog, &mut state, 0, 10, "jianchihu");
    add_effect_card(
        &mut engine,
        "multi",
        &[
            ("multi-a", "OnPlay|ModifyStats|friendly:all|attack=1"),
            ("multi-b", "OnPlay|ModifyStats|friendly:all|health=3"),
        ],
    );

    engine.fire_card(&mut state, "multi", Trigger::OnPlay, ctx());

    let rows: Vec<_> = state.players[0].board[0]
        .stat_sources
        .iter()
        .filter(|source| source.layer == StatSourceLayer::Permanent)
        .collect();
    assert_eq!(rows.len(), 2, "同一张卡的不同技能各占一行");
    assert!(rows.iter().any(|row| row.origin
        == StatSourceOrigin::Card {
            card_id: Some("multi".to_string()),
            skill_id: Some(engine.catalog.skill_id("multi-a").unwrap()),
        }));
    assert!(rows.iter().any(|row| row.origin
        == StatSourceOrigin::Card {
            card_id: Some("multi".to_string()),
            skill_id: Some(engine.catalog.skill_id("multi-b").unwrap()),
        }));
}

#[test]
fn growth_sources_clear_on_remove_growth() {
    let (mut engine, mut state) = setup(5);
    place(&engine.catalog, &mut state, 0, 10, "jianchihu");
    add_effect_card(
        &mut engine,
        "grower",
        &[("grower-skill", "OnPlay|Growth|friendly:all|times=2")],
    );
    add_effect_card(
        &mut engine,
        "stripper",
        &[("stripper-skill", "OnPlay|RemoveGrowth|friendly:all")],
    );

    engine.fire_card(&mut state, "grower", Trigger::OnPlay, ctx());
    assert!(
        state.players[0].board[0]
            .stat_sources
            .iter()
            .any(|source| source.layer == StatSourceLayer::Growth)
    );

    engine.fire_card(&mut state, "stripper", Trigger::OnPlay, ctx());
    assert!(
        !state.players[0].board[0]
            .stat_sources
            .iter()
            .any(|source| source.layer == StatSourceLayer::Growth)
    );
}

#[test]
fn combat_duration_writes_turn_layer_and_settle_clears_it() {
    let (mut engine, mut state) = setup(5);
    place(&engine.catalog, &mut state, 0, 10, "jianchihu");
    add_effect_card(
        &mut engine,
        "temp-buffer",
        &[(
            "temp-skill",
            "OnPlay|ModifyStats|friendly:all|attack=1,health=1,duration=combat",
        )],
    );

    engine.fire_card(&mut state, "temp-buffer", Trigger::OnPlay, ctx());
    assert!(
        state.players[0].board[0]
            .stat_sources
            .iter()
            .any(|source| source.layer == StatSourceLayer::Turn)
    );

    state.phase = Phase::Settle;
    engine.resolve_settle(&mut state).expect("结算应成功");

    assert!(
        !state.players[0].board[0]
            .stat_sources
            .iter()
            .any(|source| source.layer == StatSourceLayer::Turn)
    );
}

// ---------------------------------------------------------------------------
// 招募：系统来源
// ---------------------------------------------------------------------------

#[test]
fn evolve_records_system_source() {
    let (engine, mut state) = setup(5);
    place(&engine.catalog, &mut state, 0, 10, "jianchihu");
    state.pending.push_back(PendingChoice::Evolve {
        seat: 0,
        source: 10,
        remaining: 1,
    });

    engine
        .apply(&mut state, 0, Intent::ChooseDiscover { choice: 0 })
        .expect("进化 stats 选项应结算");

    let minion = &state.players[0].board[0];
    let row = minion
        .stat_sources
        .iter()
        .find(|source| source.origin == system(StatSystemSource::Evolve))
        .expect("进化应记录一条系统来源");
    assert_eq!(row.layer, StatSourceLayer::Permanent);
    assert_eq!((row.attack, row.health), (2, 2));
}

#[test]
fn breed_records_system_source() {
    let (mut engine, mut state) = setup(5);
    add_combat_card(&mut engine, "breeder", 1, 10, &[]);
    place(&engine.catalog, &mut state, 0, 10, "breeder");
    state.players[0].board[0].breed = 2;

    engine.start_turn(&mut state).expect("开始回合应成功");

    let minion = &state.players[0].board[0];
    let row = minion
        .stat_sources
        .iter()
        .find(|source| source.origin == system(StatSystemSource::Breed))
        .expect("繁衍应记录一条系统来源");
    assert_eq!(row.layer, StatSourceLayer::Growth);
    assert_eq!((row.attack, row.health), (0, 2));
}

#[test]
fn contagion_records_system_source() {
    let (mut engine, mut state) = setup(5);
    add_combat_card(&mut engine, "sick", 1, 10, &[]);
    place(&engine.catalog, &mut state, 0, 10, "sick");
    state.players[0].board[0].contagion = 2;

    engine.start_turn(&mut state).expect("开始回合应成功");

    let minion = &state.players[0].board[0];
    let row = minion
        .stat_sources
        .iter()
        .find(|source| source.origin == system(StatSystemSource::Contagion))
        .expect("传染应记录一条系统来源");
    assert_eq!(row.layer, StatSourceLayer::Permanent);
    assert_eq!((row.attack, row.health), (0, -2));
}

#[test]
fn latent_burst_records_permanent_damage_source() {
    let (mut engine, mut state) = setup(5);
    add_combat_card(
        &mut engine,
        "dengue",
        4,
        9,
        &[(
            "latent-3",
            "OnTurnStart|Latent|self|amount=1,count=3,threshold=3",
        )],
    );
    add_combat_card(&mut engine, "tank", 1, 50, &[]);
    place(&engine.catalog, &mut state, 0, 10, "dengue");
    place(&engine.catalog, &mut state, 1, 20, "tank");
    state.players[0].board[0].contagion = 3;

    engine.start_turn(&mut state).expect("开始回合应成功");

    let victim = &state.players[1].board[0];
    assert_eq!(
        victim.stats.health.permanent, -3,
        "潜伏爆发造成 3 点永久伤害"
    );
    let rows: Vec<_> = victim
        .stat_sources
        .iter()
        .filter(|source| source.layer == StatSourceLayer::Permanent)
        .collect();
    assert_eq!(rows.len(), 1, "潜伏爆发应记一条永久层来源");
    assert_eq!((rows[0].attack, rows[0].health), (0, -3));
    assert_eq!(
        rows[0].origin,
        StatSourceOrigin::Card {
            card_id: Some("dengue".to_string()),
            skill_id: Some(engine.catalog.skill_id("latent-3").unwrap()),
        }
    );
    assert_layer_sums(victim);
}

// ---------------------------------------------------------------------------
// 战斗：复苏、回手、移除成长
// ---------------------------------------------------------------------------

#[test]
fn revive_records_system_source() {
    let (mut engine, _state) = setup_with_gold(31);
    add_combat_card(
        &mut engine,
        "reviver",
        1,
        1,
        &[("reviver-skill", "OnDeath|Revive|self|attack=2,health=2")],
    );
    add_combat_card(&mut engine, "brute", 10, 100, &[]);

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "reviver")],
        &[minion(&engine.catalog, 2, "brute")],
    );

    let revived = final_minion(&log, 1);
    let row = revived
        .stat_sources
        .iter()
        .find(|source| source.origin == system(StatSystemSource::Revive))
        .expect("复苏应记录一条系统来源");
    assert_eq!(row.layer, StatSourceLayer::Permanent);
    assert_eq!((row.attack, row.health), (2, 2));
}

#[test]
fn recall_preserves_growth_as_recall_source_and_drops_combat_layers() {
    let (mut engine, _state) = setup_with_gold(31);
    add_combat_card(
        &mut engine,
        "recallee",
        1,
        1,
        &[
            ("recallee-growth", "OnCombatStart|Growth|self|times=2"),
            (
                "recallee-temp",
                "OnCombatStart|ModifyStats|self|attack=1,duration=combat",
            ),
            ("recallee-recall", "OnDeath|Recall|self|to=hand"),
        ],
    );
    add_combat_card(&mut engine, "brute", 10, 100, &[]);

    let mut recalled = minion(&engine.catalog, 1, "recallee");
    // 模拟持久棋盘折叠进来的光环；回手时必须清掉，不能漏进手牌。
    recalled.stats.attack.set_aura(3);
    recalled.record_stat_source(StatSourceLayer::Aura, card_origin("aura-card"), 3, 0);

    let log = battle(&engine, &[recalled], &[minion(&engine.catalog, 2, "brute")]);

    assert_eq!(log.hand_minions.len(), 1, "死亡后回手一张");
    let hand = &log.hand_minions[0].minion;
    assert!(
        hand.stat_sources
            .iter()
            .all(|source| source.layer != StatSourceLayer::Aura),
        "光环来源不能漏进手牌"
    );
    assert!(
        hand.stat_sources
            .iter()
            .all(|source| source.layer != StatSourceLayer::Turn),
        "本回合来源不能漏进手牌"
    );
    let growth: Vec<_> = hand
        .stat_sources
        .iter()
        .filter(|source| source.layer == StatSourceLayer::Growth)
        .collect();
    assert_eq!(growth.len(), 1, "回手成长合并为一条系统来源");
    assert_eq!(growth[0].origin, system(StatSystemSource::Recall));
    assert_eq!((growth[0].attack, growth[0].health), (2, 2));
}

#[test]
fn combat_remove_growth_clears_growth_sources() {
    let (mut engine, _state) = setup_with_gold(31);
    add_combat_card(
        &mut engine,
        "stripper",
        1,
        10,
        &[("stripper-skill", "OnCombatStart|RemoveGrowth|self")],
    );
    add_combat_card(&mut engine, "brute", 0, 100, &[]);

    let mut stripped = minion(&engine.catalog, 1, "stripper");
    stripped.stats.attack.add_growth(2);
    stripped.stats.health.add_growth(2);
    stripped.record_stat_source(StatSourceLayer::Growth, card_origin("prior"), 2, 2);

    let log = battle(&engine, &[stripped], &[minion(&engine.catalog, 2, "brute")]);

    let result = final_minion(&log, 1);
    assert_eq!(result.stats.attack.growth, 0);
    assert_eq!(result.stats.health.growth, 0);
    assert!(
        result
            .stat_sources
            .iter()
            .all(|source| source.layer != StatSourceLayer::Growth),
        "移除成长后成长来源也必须清空"
    );
}

// ---------------------------------------------------------------------------
// 复制：清临时层与光环来源
// ---------------------------------------------------------------------------

#[test]
fn copy_clears_turn_and_aura_sources() {
    let (mut engine, mut state) = setup(5);
    place(&engine.catalog, &mut state, 0, 10, "jianchihu");
    add_effect_card(
        &mut engine,
        "copier",
        &[("copier-skill", "OnPlay|Copy|self|state=current,to=hand")],
    );

    let source = &mut state.players[0].board[0];
    source.stats.attack.add_turn(1);
    source.stats.health.add_turn(1);
    source.record_stat_source(StatSourceLayer::Turn, card_origin("temp"), 1, 1);
    source.stats.attack.set_aura(1);
    source.stats.health.set_aura(1);
    source.record_stat_source(StatSourceLayer::Aura, card_origin("aura"), 1, 1);
    source.stats.attack.add_permanent(2);
    source.stats.health.add_permanent(2);
    source.record_stat_source(StatSourceLayer::Permanent, card_origin("prior"), 2, 2);

    engine.fire_card(
        &mut state,
        "copier",
        Trigger::OnPlay,
        EffectCtx {
            source: Some(10),
            ..ctx()
        },
    );

    let copy = state.players[0]
        .hand
        .iter()
        .find_map(|card| card.minion())
        .expect("复制应进入手牌");
    assert!(
        copy.stat_sources
            .iter()
            .all(|source| source.layer != StatSourceLayer::Turn),
        "复制品不携带本回合来源"
    );
    assert!(
        copy.stat_sources
            .iter()
            .all(|source| source.layer != StatSourceLayer::Aura),
        "复制品不携带光环来源"
    );
    assert!(
        copy.stat_sources
            .iter()
            .any(|source| source.origin == card_origin("prior")),
        "永久来源照常保留"
    );
}

// ---------------------------------------------------------------------------
// 战斗回写：只补战斗成长差值
// ---------------------------------------------------------------------------

#[test]
fn write_back_records_growth_difference_only() {
    let (mut engine, mut state) = setup_with_gold(5);
    add_combat_card(
        &mut engine,
        "fighter",
        1,
        4,
        &[("fighter-skill", "OnCombatStart|Growth|self|times=2")],
    );
    add_combat_card(&mut engine, "tank", 0, 50, &[]);
    place(&engine.catalog, &mut state, 0, 10, "fighter");
    place(&engine.catalog, &mut state, 1, 20, "tank");

    // 持久棋盘已有 1 点成长及其来源；回写后必须保留。
    let stored = &mut state.players[0].board[0];
    stored.stats.attack.add_growth(1);
    stored.stats.health.add_growth(1);
    stored.record_stat_source(StatSourceLayer::Growth, card_origin("prior"), 1, 1);

    state.phase = Phase::Combat;
    state.pairings = vec![Pairing {
        a: 0,
        b: 1,
        ghost: None,
    }];
    engine.resolve_combat(&mut state).expect("战斗结算应成功");

    let stored = &state.players[0].board[0];
    assert_eq!(
        (stored.stats.attack.growth, stored.stats.health.growth),
        (3, 3),
        "原有 1 点成长 + 战斗内 2 点成长"
    );
    let combat_growth: Vec<_> = stored
        .stat_sources
        .iter()
        .filter(|source| source.origin == system(StatSystemSource::CombatGrowth))
        .collect();
    assert_eq!(combat_growth.len(), 1, "成长差值只记一条战斗成长");
    assert_eq!((combat_growth[0].attack, combat_growth[0].health), (2, 2));
    assert!(
        stored
            .stat_sources
            .iter()
            .any(|source| source.origin == card_origin("prior")),
        "持久棋盘原有成长来源保持"
    );
    assert!(
        !stored.stat_sources.iter().any(|source| matches!(
            source.origin,
            StatSourceOrigin::Card {
                skill_id: Some(_),
                ..
            }
        )),
        "战斗克隆里的技能级来源不回写持久棋盘"
    );
}
