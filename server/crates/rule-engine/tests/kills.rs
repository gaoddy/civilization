//! T07：击杀归属。
//!
//! 覆盖攻击击杀、反击击杀、效果伤害击杀、友方击杀只触发 `OnKill`、
//! 同时死亡（互相击杀）不触发、复生后再次击杀，以及无来源伤害不产生击杀。

mod common;

use common::{categories_for, effects_of, minion, setup_with_gold, token_card};
use rule_engine::combat::{CombatAction, CombatLog, CombatSetup, simulate_with};
use rule_engine::data::{Card, CardType, HeroCard, Playstyle, Skill};
use rule_engine::engine::Engine;
use rule_engine::rng::RngStreams;
use rule_engine::state::{InstanceId, MinionInstance, Seat};

/// 把同 id 的随从卡镜像为英雄池条目：战斗开始的英雄技能只从英雄池读取。
fn mirror_as_hero(engine: &mut Engine, id: &str) {
    let card = engine
        .catalog
        .cards
        .get(id)
        .expect("镜像源卡应存在")
        .clone();
    engine.catalog.heroes.insert(
        id.to_string(),
        HeroCard {
            id: id.to_string(),
            categories: card.categories,
            skills: card.skills,
        },
    );
}

/// 插入一张合成随从卡，携带若干 `(keyword, effects)` 技能。
fn add_card(
    engine: &mut Engine,
    id: &str,
    attack: i32,
    health: i32,
    skills: &[(Option<&str>, &str)],
) {
    let mut skill_ids = Vec::new();
    for (index, (keyword, effects)) in skills.iter().enumerate() {
        let skill_id = engine.catalog.insert_skill(Skill {
            id: format!("{id}-skill-{index}"),
            keyword: keyword.map(str::to_string),
            effects: effects_of(effects),
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

fn battle(
    engine: &Engine,
    attacker: &[MinionInstance],
    defender: &[MinionInstance],
    first: Seat,
) -> CombatLog {
    simulate_with(
        engine,
        attacker,
        defender,
        0,
        1,
        first,
        &mut RngStreams::from_seed([17u8; 32]),
        CombatSetup::default(),
    )
}

fn final_minion(log: &CombatLog, id: InstanceId) -> &MinionInstance {
    log.attacker_final
        .iter()
        .chain(log.defender_final.iter())
        .find(|minion| minion.instance_id == id)
        .expect("战斗结束时随从仍在快照中")
}

fn deaths(log: &CombatLog) -> Vec<InstanceId> {
    log.actions
        .iter()
        .filter_map(|action| match action {
            CombatAction::Death { minion } => Some(*minion),
            _ => None,
        })
        .collect()
}

#[test]
fn attack_kill_triggers_on_kill_for_attacker() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "hunter",
        5,
        5,
        &[(None, "OnKill|ModifyStats|self|attack=1,duration=permanent")],
    );
    add_card(&mut engine, "prey", 0, 2, &[]);

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "hunter")],
        &[minion(&engine.catalog, 2, "prey")],
        0,
    );

    assert_eq!(deaths(&log), vec![2]);
    assert_eq!(
        final_minion(&log, 1).stats.attack.permanent,
        1,
        "攻击击杀应触发击杀者的 OnKill"
    );
}

#[test]
fn counterattack_kill_triggers_on_kill_for_defender() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(&mut engine, "weakling", 1, 1, &[]);
    add_card(
        &mut engine,
        "guard",
        3,
        10,
        &[(None, "OnKill|ModifyStats|self|attack=1,duration=permanent")],
    );

    // 防守方先手：弱鸡主动攻击守卫并被反击打死。
    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "weakling")],
        &[minion(&engine.catalog, 2, "guard")],
        1,
    );

    assert_eq!(deaths(&log).first(), Some(&1));
    assert_eq!(
        final_minion(&log, 2).stats.attack.permanent,
        1,
        "反击击杀应归因给反击者"
    );
}

#[test]
fn effect_damage_kill_triggers_on_kill_for_caster() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "caster",
        1,
        3,
        &[
            (
                Some("battlecry"),
                "OnCombatStart|DealDamage|enemy:all|amount=9",
            ),
            (None, "OnKill|ModifyStats|self|attack=2,duration=permanent"),
        ],
    );
    add_card(&mut engine, "prey", 0, 2, &[]);

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "caster")],
        &[minion(&engine.catalog, 2, "prey")],
        0,
    );

    // 战斗开始效果打死猎物：击杀归因施放者。
    assert_eq!(deaths(&log).first(), Some(&2));
    assert_eq!(
        final_minion(&log, 1).stats.attack.permanent,
        2,
        "效果伤害击杀应触发施放者的 OnKill"
    );
}

#[test]
fn hero_effect_has_no_kill_attribution() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(&mut engine, "caster", 1, 3, &[]);
    add_card(
        &mut engine,
        "prey",
        0,
        2,
        &[(None, "OnKill|ModifyStats|self|attack=2,duration=permanent")],
    );
    add_card(
        &mut engine,
        "hero",
        0,
        0,
        &[(Some("hero"), "OnCombatStart|DealDamage|enemy:all|amount=9")],
    );
    mirror_as_hero(&mut engine, "hero");

    let log = simulate_with(
        &engine,
        &[minion(&engine.catalog, 1, "caster")],
        &[minion(&engine.catalog, 2, "prey")],
        0,
        1,
        0,
        &mut RngStreams::from_seed([17u8; 32]),
        CombatSetup {
            attacker_hero: Some("hero"),
            ..CombatSetup::default()
        },
    );

    assert!(deaths(&log).contains(&2), "英雄被动打死了猎物");
    assert_eq!(
        final_minion(&log, 2).stats.attack.permanent,
        0,
        "英雄技能不归因随从，不产生击杀"
    );
}

#[test]
fn friendly_kill_triggers_on_kill_but_not_on_ally_kill() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "bomber",
        0,
        5,
        &[
            (
                Some("battlecry"),
                "OnCombatStart|DealDamage|friendly:all:other=true|amount=9",
            ),
            (None, "OnKill|ModifyStats|self|attack=1,duration=permanent"),
            (
                None,
                "OnAllyKill|ModifyStats|self|attack=10,duration=permanent",
            ),
        ],
    );
    add_card(
        &mut engine,
        "ally",
        0,
        1,
        &[(
            None,
            "OnAllyKill|ModifyStats|self|attack=3,duration=permanent",
        )],
    );

    let log = battle(
        &engine,
        &[
            minion(&engine.catalog, 1, "bomber"),
            minion(&engine.catalog, 2, "ally"),
        ],
        &[],
        0,
    );

    assert!(deaths(&log).contains(&2), "友方随从被击杀");
    assert_eq!(
        final_minion(&log, 1).stats.attack.permanent,
        1,
        "击杀友方只触发 OnKill，不触发 OnAllyKill"
    );
}

#[test]
fn ally_kill_triggers_for_all_living_allies_including_killer() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "hunter",
        5,
        5,
        &[(
            None,
            "OnAllyKill|ModifyStats|self|attack=1,duration=permanent",
        )],
    );
    add_card(
        &mut engine,
        "cheerer",
        0,
        5,
        &[(
            None,
            "OnAllyKill|ModifyStats|self|attack=2,duration=permanent",
        )],
    );
    add_card(&mut engine, "prey", 0, 2, &[]);

    let log = battle(
        &engine,
        &[
            minion(&engine.catalog, 1, "hunter"),
            minion(&engine.catalog, 2, "cheerer"),
        ],
        &[minion(&engine.catalog, 3, "prey")],
        0,
    );

    assert!(deaths(&log).contains(&3), "猎物应被击杀");
    assert_eq!(
        final_minion(&log, 1).stats.attack.permanent,
        1,
        "击杀者自身也响应 OnAllyKill"
    );
    assert_eq!(
        final_minion(&log, 2).stats.attack.permanent,
        2,
        "其他存活友方响应 OnAllyKill"
    );
}

#[test]
fn mutual_death_triggers_no_kill_hooks() {
    let (mut engine, _state) = setup_with_gold(31);
    // 攻击者主伤害击杀目标，目标反击击杀攻击者：同一次交换双方都死亡。
    add_card(
        &mut engine,
        "champion",
        5,
        5,
        &[(None, "OnKill|ModifyStats|self|attack=1,duration=permanent")],
    );
    add_card(
        &mut engine,
        "witness",
        0,
        10,
        &[(
            None,
            "OnAllyKill|ModifyStats|self|attack=1,duration=permanent",
        )],
    );
    add_card(&mut engine, "rival", 5, 5, &[]);

    let log = battle(
        &engine,
        &[
            minion(&engine.catalog, 1, "champion"),
            minion(&engine.catalog, 2, "witness"),
        ],
        &[minion(&engine.catalog, 3, "rival")],
        0,
    );

    assert!(deaths(&log).contains(&1), "攻击者被反击击杀");
    assert!(deaths(&log).contains(&3), "目标被主伤害击杀");
    assert_eq!(
        final_minion(&log, 1).stats.attack.permanent,
        0,
        "互杀时击杀者不触发 OnKill"
    );
    assert_eq!(
        final_minion(&log, 2).stats.attack.permanent,
        0,
        "互杀时友方存活随从不触发 OnAllyKill"
    );
}

#[test]
fn mutual_death_with_reborn_does_not_retrigger_hooks() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "reborn-champion",
        5,
        5,
        &[
            (Some("reborn"), "[]"),
            (None, "OnKill|ModifyStats|self|attack=1,duration=permanent"),
        ],
    );
    add_card(&mut engine, "rival", 5, 5, &[]);

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "reborn-champion")],
        &[minion(&engine.catalog, 2, "rival")],
        0,
    );

    assert!(
        log.actions
            .iter()
            .any(|action| matches!(action, CombatAction::Reborn { minion: 1 })),
        "攻击者应复生"
    );
    assert_eq!(
        final_minion(&log, 1).stats.attack.permanent,
        0,
        "互杀后复生不补发 OnKill"
    );
}

#[test]
fn self_kill_after_kill_triggers_no_hooks() {
    let (mut engine, _state) = setup_with_gold(31);
    // 攻击者击杀目标后死于自身的 OnDealDamage：不属于互杀，但击杀者已死。
    add_card(
        &mut engine,
        "suicide",
        5,
        10,
        &[
            (None, "OnDealDamage|DealDamage|self|amount=99"),
            (None, "OnKill|ModifyStats|self|attack=1,duration=permanent"),
        ],
    );
    add_card(
        &mut engine,
        "witness",
        0,
        10,
        &[(
            None,
            "OnAllyKill|ModifyStats|self|attack=1,duration=permanent",
        )],
    );
    add_card(&mut engine, "victim", 0, 1, &[]);

    let log = battle(
        &engine,
        &[
            minion(&engine.catalog, 1, "suicide"),
            minion(&engine.catalog, 2, "witness"),
        ],
        &[minion(&engine.catalog, 3, "victim")],
        0,
    );

    assert!(deaths(&log).contains(&1), "攻击者自身死亡");
    assert!(deaths(&log).contains(&3), "被击杀者死亡");
    assert_eq!(
        final_minion(&log, 1).stats.attack.permanent,
        0,
        "击杀者已死，不触发 OnKill"
    );
    assert_eq!(
        final_minion(&log, 2).stats.attack.permanent,
        0,
        "击杀者已死，友方不触发 OnAllyKill"
    );
}

#[test]
fn lethal_target_does_not_trigger_own_deal_damage() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "bomb",
        3,
        1,
        &[(None, "OnDealDamage|DealDamage|enemy:random|amount=9")],
    );
    add_card(&mut engine, "striker", 5, 10, &[]);

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "striker")],
        &[minion(&engine.catalog, 2, "bomb")],
        0,
    );

    assert!(deaths(&log).contains(&2), "目标被主伤害击杀");
    assert!(
        !deaths(&log).contains(&1),
        "攻击者不应死于目标的 OnDealDamage"
    );
    assert_eq!(
        final_minion(&log, 1).stats.health.current,
        7,
        "攻击者只承受 3 点反击，未触发目标的 OnDealDamage"
    );
}

#[test]
fn surviving_target_triggers_own_deal_damage() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "bomb",
        3,
        5,
        &[(None, "OnDealDamage|DealDamage|enemy:random|amount=9")],
    );
    add_card(&mut engine, "striker", 2, 10, &[]);

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "striker")],
        &[minion(&engine.catalog, 2, "bomb")],
        0,
    );

    assert!(
        deaths(&log).contains(&1),
        "目标的 OnDealDamage 应击杀攻击者"
    );
    assert!(!deaths(&log).contains(&2), "目标承受 2 点主伤害后仍存活");
    assert_eq!(
        final_minion(&log, 2).stats.health.current,
        3,
        "目标剩余 3 点生命"
    );
}

#[test]
fn reborn_then_kill_triggers_on_kill_again() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "reaper",
        1,
        10,
        &[(None, "OnKill|ModifyStats|self|attack=1,duration=permanent")],
    );
    add_card(
        &mut engine,
        "phoenix",
        0,
        1,
        &[(Some("reborn"), "[]"), (Some("taunt"), "[]")],
    );

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "reaper")],
        &[minion(&engine.catalog, 2, "phoenix")],
        0,
    );

    assert!(
        log.actions
            .iter()
            .any(|action| matches!(action, CombatAction::Reborn { minion: 2 })),
        "被击杀者应复生"
    );
    assert_eq!(
        deaths(&log).iter().filter(|id| **id == 2).count(),
        2,
        "复生后再次被击杀"
    );
    assert_eq!(
        final_minion(&log, 1).stats.attack.permanent,
        2,
        "两次击杀各触发一次 OnKill"
    );
}

#[test]
fn kill_hook_chain_processes_new_kill_records() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "archer",
        5,
        5,
        &[(None, "OnKill|DealDamage|enemy:random|amount=9")],
    );
    add_card(
        &mut engine,
        "witness",
        0,
        10,
        &[(
            None,
            "OnAllyKill|ModifyStats|self|attack=1,duration=permanent",
        )],
    );
    add_card(&mut engine, "prey-a", 0, 2, &[]);
    add_card(&mut engine, "prey-b", 0, 2, &[]);

    let log = battle(
        &engine,
        &[
            minion(&engine.catalog, 1, "archer"),
            minion(&engine.catalog, 2, "witness"),
        ],
        &[
            minion(&engine.catalog, 3, "prey-a"),
            minion(&engine.catalog, 4, "prey-b"),
        ],
        0,
    );

    assert!(deaths(&log).contains(&3));
    assert!(
        deaths(&log).contains(&4),
        "OnKill 效果造成的第二次击杀应进入死亡队列"
    );
    assert_eq!(
        final_minion(&log, 2).stats.attack.permanent,
        2,
        "两条击杀记录都触发 OnAllyKill"
    );
}

/// 给随从设置初始传染层数。
fn infected(mut minion: MinionInstance, layers: i32) -> MinionInstance {
    minion.contagion = layers;
    minion
}

/// 注册一个 2/2 细菌衍生物，供转化召唤测试使用。
fn add_germ_token(engine: &mut Engine) {
    engine
        .catalog
        .cards
        .insert("germ".to_string(), token_card("germ", 2, 2));
}

#[test]
fn detonate_consumes_layers_and_deals_per_layer_damage() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "bomber",
        0,
        10,
        &[(
            None,
            "OnCombatStart|Detonate|enemy:single|amount=2,threshold=99",
        )],
    );
    add_card(&mut engine, "carrier", 0, 20, &[]);

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "bomber")],
        &[infected(minion(&engine.catalog, 2, "carrier"), 3)],
        0,
    );

    assert!(
        log.actions.iter().any(|action| matches!(
            action,
            CombatAction::Detonated {
                target: 2,
                layers: 3,
                damage: 6,
            }
        )),
        "3 层 × 每层 2 点应记录 6 点引爆伤害"
    );
    assert_eq!(
        final_minion(&log, 2).stats.health.current,
        14,
        "目标承受 6 点伤害后剩余 14 点生命"
    );
    assert_eq!(final_minion(&log, 2).contagion, 0, "引爆移除全部层数");
    assert!(!deaths(&log).contains(&2), "未达阈值时目标存活");
}

#[test]
fn detonate_threshold_destroys_even_through_shield() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "bomber",
        0,
        10,
        &[(
            None,
            "OnCombatStart|Detonate|enemy:single|amount=1,threshold=3",
        )],
    );
    add_card(
        &mut engine,
        "carrier",
        0,
        20,
        &[(Some("divine_shield"), "[]")],
    );

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "bomber")],
        &[infected(minion(&engine.catalog, 2, "carrier"), 3)],
        0,
    );

    let carrier = final_minion(&log, 2);
    assert!(carrier.shield_used, "护盾抵消按层伤害并被消耗");
    assert_eq!(
        log.actions.iter().find_map(|action| match action {
            CombatAction::Detonated { damage, .. } => Some(*damage),
            _ => None,
        }),
        Some(0),
        "护盾抵消后记录的伤害为 0"
    );
    assert!(deaths(&log).contains(&2), "达到阈值时护盾不能阻止消灭");
    assert_eq!(carrier.stats.health.current, 0);
}

#[test]
fn detonate_with_zero_layers_is_silent() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "bomber",
        0,
        10,
        &[(
            None,
            "OnCombatStart|Detonate|enemy:single|amount=2,threshold=1",
        )],
    );
    add_card(&mut engine, "carrier", 0, 20, &[]);

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "bomber")],
        &[minion(&engine.catalog, 2, "carrier")],
        0,
    );

    assert!(
        !log.actions
            .iter()
            .any(|action| matches!(action, CombatAction::Detonated { .. })),
        "0 层引爆静默落空"
    );
    assert_eq!(final_minion(&log, 2).stats.health.current, 20);
    assert!(!deaths(&log).contains(&2));
}

#[test]
fn convert_requires_threshold_and_summons_token_for_source() {
    let (mut engine, _state) = setup_with_gold(31);
    add_germ_token(&mut engine);
    add_card(
        &mut engine,
        "converter",
        0,
        10,
        &[(
            None,
            "OnCombatStart|Convert|enemy:single|threshold=3,token=germ,count=1",
        )],
    );
    add_card(&mut engine, "carrier", 0, 20, &[]);

    let below = battle(
        &engine,
        &[minion(&engine.catalog, 1, "converter")],
        &[infected(minion(&engine.catalog, 2, "carrier"), 2)],
        0,
    );
    assert!(
        !below
            .actions
            .iter()
            .any(|action| matches!(action, CombatAction::Converted { .. })),
        "2 层未达阈值，不转化"
    );
    assert_eq!(below.attacker_final.len(), 1, "未达阈值不召唤 Token");
    assert_eq!(final_minion(&below, 2).contagion, 2, "未转化不消耗层数");

    let reached = battle(
        &engine,
        &[minion(&engine.catalog, 1, "converter")],
        &[infected(minion(&engine.catalog, 2, "carrier"), 3)],
        0,
    );
    assert!(
        reached.actions.iter().any(|action| matches!(
            action,
            CombatAction::Converted {
                target: 2,
                token,
            } if token == "germ"
        )),
        "达到阈值时记录转化"
    );
    assert!(deaths(&reached).contains(&2), "转化致死");
    let token = reached
        .attacker_final
        .iter()
        .find(|minion| minion.card_id == "germ")
        .expect("来源方棋盘应召唤 Token");
    assert_eq!(token.stats.attack.current, 2);
    assert_eq!(token.stats.health.current, 2, "召唤的是 2/2 细菌 Token");
    assert_eq!(
        final_minion(&reached, 2).contagion,
        3,
        "转化是门槛，不消耗层数"
    );
}

#[test]
fn convert_destroys_through_shield_and_armor() {
    let (mut engine, _state) = setup_with_gold(31);
    add_germ_token(&mut engine);
    add_card(
        &mut engine,
        "converter",
        0,
        10,
        &[(
            None,
            "OnCombatStart|Convert|enemy:single|threshold=1,token=germ,count=1",
        )],
    );
    add_card(
        &mut engine,
        "tank",
        0,
        10,
        &[(Some("divine_shield"), "[]"), (Some("armor 4"), "[]")],
    );

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "converter")],
        &[infected(minion(&engine.catalog, 2, "tank"), 1)],
        0,
    );

    assert!(deaths(&log).contains(&2), "护盾与减伤不能阻止转化");
    let tank = final_minion(&log, 2);
    assert_eq!(tank.stats.health.current, 0);
    assert!(!tank.shield_used, "转化不消耗护盾");
}

#[test]
fn convert_target_that_leaves_after_selection_is_silent() {
    let (mut engine, _state) = setup_with_gold(31);
    add_germ_token(&mut engine);
    add_card(
        &mut engine,
        "converter",
        0,
        10,
        &[
            (
                None,
                "OnCombatStart|Convert|enemy:all|threshold=1,token=germ,count=1",
            ),
            // 第一次转化召唤后，来源随从吞噬剩余敌人，使第二个已选目标在
            // 轮到它结算前离场。
            (None, "OnAllySummon|Devour|enemy:all|value=99"),
        ],
    );
    add_card(&mut engine, "prey-a", 0, 10, &[]);
    add_card(&mut engine, "prey-b", 0, 10, &[]);

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "converter")],
        &[
            infected(minion(&engine.catalog, 2, "prey-a"), 1),
            infected(minion(&engine.catalog, 3, "prey-b"), 1),
        ],
        0,
    );

    let converted: Vec<InstanceId> = log
        .actions
        .iter()
        .filter_map(|action| match action {
            CombatAction::Converted { target, .. } => Some(*target),
            _ => None,
        })
        .collect();
    assert_eq!(converted, vec![2], "第二个目标已离场，转化静默落空");
    assert_eq!(final_minion(&log, 3).contagion, 1, "未结算的目标不消耗层数");
    assert_eq!(
        log.attacker_final
            .iter()
            .filter(|minion| minion.card_id == "germ")
            .count(),
        1,
        "已离场的目标不再召唤 Token"
    );
}

#[test]
fn convert_summons_token_on_real_source_side_in_ghost_battle() {
    let (mut engine, _state) = setup_with_gold(31);
    add_germ_token(&mut engine);
    add_card(
        &mut engine,
        "ghost-converter",
        0,
        10,
        &[(
            None,
            "OnCombatStart|Convert|both:all|threshold=1,token=germ,count=1",
        )],
    );
    add_card(&mut engine, "prey", 0, 10, &[]);

    // 幽灵战两侧共用同一座位，side 1 的转化仍应把 Token 召回到 side 1。
    // `both:all` 让幽灵侧能选中真实玩家（side 0）的单位；自身会被阵营检查跳过。
    let log = simulate_with(
        &engine,
        &[infected(minion(&engine.catalog, 1, "prey"), 1)],
        &[minion(&engine.catalog, 2, "ghost-converter")],
        0,
        0,
        0,
        &mut RngStreams::from_seed([17u8; 32]),
        CombatSetup::default(),
    );

    assert!(
        log.defender_final
            .iter()
            .any(|minion| minion.card_id == "germ"),
        "Token 应召唤在来源方（side 1）棋盘"
    );
    assert!(
        !log.attacker_final
            .iter()
            .any(|minion| minion.card_id == "germ"),
        "幽灵侧转化不得把 Token 召唤到真实玩家棋盘"
    );
}

#[test]
fn detonate_and_convert_are_silent_when_target_is_missing() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "bomber",
        0,
        10,
        &[(
            None,
            "OnCombatStart|Detonate|enemy:single|amount=2,threshold=1",
        )],
    );
    add_card(
        &mut engine,
        "converter",
        0,
        10,
        &[(
            None,
            "OnCombatStart|Convert|enemy:single|threshold=3,token=germ,count=1",
        )],
    );
    add_card(&mut engine, "sneaky", 0, 20, &[(Some("stealth"), "[]")]);

    // 潜行目标不能被敌方指向性效果选中，等同目标缺失。
    let detonated = battle(
        &engine,
        &[minion(&engine.catalog, 1, "bomber")],
        &[infected(minion(&engine.catalog, 2, "sneaky"), 3)],
        0,
    );
    assert!(
        !detonated
            .actions
            .iter()
            .any(|action| matches!(action, CombatAction::Detonated { .. })),
        "目标缺失时引爆落空"
    );
    assert_eq!(final_minion(&detonated, 2).contagion, 3, "不消耗层数");
    assert!(!deaths(&detonated).contains(&2));

    let converted = battle(
        &engine,
        &[minion(&engine.catalog, 3, "converter")],
        &[infected(minion(&engine.catalog, 4, "sneaky"), 3)],
        0,
    );
    assert!(
        !converted
            .actions
            .iter()
            .any(|action| matches!(action, CombatAction::Converted { .. })),
        "目标缺失时转化落空"
    );
    assert_eq!(converted.attacker_final.len(), 1, "不召唤 Token");
    assert_eq!(final_minion(&converted, 4).contagion, 3, "不消耗层数");
    assert!(!deaths(&converted).contains(&4));
}
