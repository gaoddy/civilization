mod common;

use common::{CARDS_JSON, CONFIG_JSON, SKILLS_JSON, TAXONOMY_JSON, test_catalog};
use rule_engine::data::{CardType, Catalog};
use rule_engine::effects::{Compare, Effect, Op, Pick, Selector, Side, Trigger, validate_effects};

#[test]
fn loads_all_committed_data() {
    let catalog = test_catalog();
    assert_eq!(catalog.cards.len(), 409);
    assert_eq!(catalog.heroes.len(), 32);
    assert_eq!(catalog.skills.len(), 247);
    assert_eq!(catalog.tokens.len(), 23);
}

#[test]
fn hero_ids_also_exist_in_the_minion_pool() {
    let catalog = test_catalog();
    for hero in catalog.heroes.values() {
        let card = catalog
            .cards
            .get(&hero.id)
            .unwrap_or_else(|| panic!("英雄 {} 缺少同名随从卡", hero.id));
        assert_eq!(
            card.card_type,
            CardType::Minion,
            "英雄 {} 的同 id 卡应为随从",
            hero.id
        );
    }
}

#[test]
fn heroes_reject_missing_matching_minion() {
    let heroes = r#"[{"id":"no-such-card","card_type":"hero","playstyle":"mammal","skills":[]}]"#;
    assert!(Catalog::from_json("[]", heroes, "[]", "[]", CONFIG_JSON, TAXONOMY_JSON).is_err());
}

#[test]
fn heroes_reject_matching_effect_card() {
    let heroes =
        r#"[{"id":"aolinpike-jidian","card_type":"hero","playstyle":"neutral","skills":[]}]"#;
    let error = Catalog::from_json(
        CARDS_JSON,
        heroes,
        SKILLS_JSON,
        "[]",
        CONFIG_JSON,
        TAXONOMY_JSON,
    )
    .expect_err("同 id 的效果牌不能充当随从卡");
    assert!(error.to_string().contains("不是随从卡"), "{error}");
}

#[test]
fn heroes_reject_skills_that_diverge_from_minion() {
    let heroes = r#"[{"id":"adam-smith","card_type":"hero","playstyle":"mammal","skills":[]}]"#;
    let error = Catalog::from_json(
        CARDS_JSON,
        heroes,
        SKILLS_JSON,
        "[]",
        CONFIG_JSON,
        TAXONOMY_JSON,
    )
    .expect_err("英雄与同名随从的技能必须一致");
    assert!(error.to_string().contains("不一致"), "{error}");
}

#[test]
fn heroes_reject_non_hero_card_type() {
    let bad = r#"[{"id":"x","card_type":"minion","playstyle":"mammal","skills":[]}]"#;
    assert!(Catalog::from_json("[]", bad, "[]", "[]", CONFIG_JSON, TAXONOMY_JSON).is_err());
}

#[test]
fn heroes_reject_unknown_skill() {
    let bad = r#"[{"id":"x","card_type":"hero","playstyle":"mammal","skills":["no-such-skill"]}]"#;
    assert!(Catalog::from_json("[]", bad, "[]", "[]", CONFIG_JSON, TAXONOMY_JSON).is_err());
}

#[test]
fn cards_reject_hero_card_type() {
    let bad = r#"[{"id":"x","card_type":"hero","playstyle":"mammal","skills":[]}]"#;
    assert!(Catalog::from_json(bad, "[]", "[]", "[]", CONFIG_JSON, TAXONOMY_JSON).is_err());
}

#[test]
fn every_card_skill_exists() {
    let catalog = test_catalog();
    for card in catalog.cards.values() {
        for skill in &card.skills {
            assert!(
                catalog.skills.contains_key(skill),
                "{} -> {}",
                card.id,
                skill
            );
        }
    }
}

#[test]
fn unknown_field_fails() {
    let bad = r#"[{"id":"x","card_type":"minion","playstyle":"mammal","taxon":"hominidae","level":1,"era":"prehistoric","attack":1,"health":2,"is_scholar":false,"oops":1}]"#;
    assert!(Catalog::from_json(bad, "[]", "[]", "[]", CONFIG_JSON, TAXONOMY_JSON).is_err());
}

#[test]
fn deserializes_a_structured_effect() {
    let raw = r#"[{"trigger":"on_activate","op":"growth","selector":{"kind":"side","side":"friendly","pick":"single","filters":[{"key":"playstyle","value":"mammal"}]},"params":{"times":1,"times_if":{"key":"playstyle","value":"mammal","times":2},"condition":{"side":"friendly","filter":{"key":"playstyle","value":"mammal"},"compare":"ge","count":3}}}]"#;
    let effects: Vec<Effect> = serde_json::from_str(raw).expect("结构化效果应能反序列化");
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].trigger, Trigger::OnActivate);
    assert_eq!(effects[0].op, Op::Growth);
    assert!(matches!(
        effects[0].selector,
        Selector::Side {
            side: Side::Friendly,
            pick: Pick::Single,
            ..
        }
    ));
    assert_eq!(effects[0].params.times, Some(1));
    assert_eq!(effects[0].params.times_if.as_ref().unwrap().times, 2);
    assert_eq!(
        effects[0].params.condition.as_ref().unwrap().compare,
        Compare::Ge
    );
}

#[test]
fn rejects_unknown_enum_and_param() {
    let unknown_op =
        r#"[{"trigger":"on_play","op":"explode","selector":{"kind":"self"},"params":{}}]"#;
    assert!(serde_json::from_str::<Vec<Effect>>(unknown_op).is_err());

    let unknown_param = r#"[{"trigger":"on_play","op":"growth","selector":{"kind":"self"},"params":{"times":1,"oops":2}}]"#;
    assert!(serde_json::from_str::<Vec<Effect>>(unknown_param).is_err());
}

#[test]
fn accepts_neutral_playstyle_filter() {
    // 中立卡是可被筛中的实体玩法取值：从 13 个可启用玩法里取不到，但筛选合法。
    let raw = r#"[{"trigger":"on_play","op":"growth","selector":{"kind":"side","side":"friendly","pick":"all","filters":[{"key":"playstyle","value":"neutral"}]},"params":{"times":1}}]"#;
    let effects: Vec<Effect> = serde_json::from_str(raw).unwrap();
    assert!(validate_effects(&effects).is_ok());
}

#[test]
fn rejects_aura_with_damage() {
    let raw = r#"[{"trigger":"aura","op":"deal_damage","selector":{"kind":"side","side":"friendly","pick":"all","filters":[]},"params":{"amount":1}}]"#;
    let effects: Vec<Effect> = serde_json::from_str(raw).unwrap();
    assert!(validate_effects(&effects).is_err());
}

#[test]
fn rejects_gain_minion_from_board() {
    let raw = r#"[{"trigger":"on_activate","op":"gain_minion","selector":{"kind":"self"},"params":{"from":"board","level":"current","to":"hand"}}]"#;
    let effects: Vec<Effect> = serde_json::from_str(raw).unwrap();
    assert!(validate_effects(&effects).is_err());
}

#[test]
fn omits_duration_by_default() {
    let raw =
        r#"[{"trigger":"on_play","op":"growth","selector":{"kind":"self"},"params":{"times":1}}]"#;
    let effects: Vec<Effect> = serde_json::from_str(raw).unwrap();
    assert_eq!(effects[0].params.duration, None);
}

#[test]
fn rejects_invalid_summon_and_discover() {
    let summon = r#"[{"trigger":"on_play","op":"summon","selector":{"kind":"self"},"params":{"count":1,"token":"token-x","card":"baiyi"}}]"#;
    let effects: Vec<Effect> = serde_json::from_str(summon).unwrap();
    assert!(validate_effects(&effects).is_err());

    let discover = r#"[{"trigger":"on_activate","op":"discover","selector":{"kind":"self"},"params":{"pool":"playstyle","choose":2}}]"#;
    let effects: Vec<Effect> = serde_json::from_str(discover).unwrap();
    assert!(validate_effects(&effects).is_err());
}

#[test]
fn committed_data_is_structured() {
    let catalog = test_catalog();
    let skill = catalog.skills.get("division-of-labor").expect("技能存在");
    assert_eq!(skill.effects.len(), 1);
    assert_eq!(skill.effects[0].trigger, Trigger::OnActivate);
    assert!(
        catalog
            .skills
            .values()
            .all(|skill| validate_effects(&skill.effects).is_ok())
    );
}

#[test]
fn string_effects_are_rejected_at_load() {
    let bad = r#"[{"id":"x","effects":"OnPlay|Growth|self|times=1"}]"#;
    assert!(Catalog::from_json("[]", "[]", bad, "[]", CONFIG_JSON, TAXONOMY_JSON).is_err());
}

#[test]
fn rejects_unknown_selector_field() {
    let raw = r#"[{"trigger":"on_activate","op":"growth","selector":{"kind":"side","side":"friendly","pick":"all","filter":[{"key":"playstyle","value":"mammal"}]},"params":{"times":1}}]"#;
    assert!(serde_json::from_str::<Vec<Effect>>(raw).is_err());
}

#[test]
fn rejects_discover_without_choose() {
    let raw = r#"[{"trigger":"on_activate","op":"discover","selector":{"kind":"self"},"params":{"pool":"playstyle"}}]"#;
    let effects: Vec<Effect> = serde_json::from_str(raw).unwrap();
    assert!(validate_effects(&effects).is_err());
}

#[test]
fn rejects_explicit_duration_on_growth() {
    let raw = r#"[{"trigger":"on_play","op":"growth","selector":{"kind":"self"},"params":{"times":1,"duration":"instant"}}]"#;
    let effects: Vec<Effect> = serde_json::from_str(raw).unwrap();
    assert!(validate_effects(&effects).is_err());
}
