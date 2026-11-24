mod common;

use common::{CARDS_JSON, CONFIG_JSON, SKILLS_JSON, TAXONOMY_JSON, test_catalog};
use rule_engine::data::{CardType, Catalog};
use rule_engine::effects::{
    Compare, Effect, Op, Params, Pick, Selector, Side, Trigger, validate_effects,
};

#[test]
fn loads_all_committed_data() {
    let catalog = test_catalog();
    assert_eq!(catalog.cards.len(), 432);
    assert_eq!(catalog.heroes.len(), 32);
    assert_eq!(catalog.skills().len(), 262);
    assert_eq!(
        catalog
            .cards
            .values()
            .filter(|card| !card.in_shop_pool)
            .count(),
        24
    );
}

/// 最小合法卡牌夹具；只替换 `taxon` 与 `categories`。
fn card_json(taxon: &str, categories: &str) -> String {
    format!(
        r#"[{{"id":"x","card_type":"minion","taxon":"{taxon}","categories":{categories},"level":1,"skills":[]}}]"#
    )
}

fn load_cards(cards: &str) -> Result<Catalog, rule_engine::error::CatalogError> {
    Catalog::from_json(cards, "[]", "[]", CONFIG_JSON, TAXONOMY_JSON)
}

#[test]
fn cards_reject_missing_play_category() {
    let cards = card_json("hominidae", r#"[{"kind":"prey","value":"hominidae"}]"#);
    let error = load_cards(&cards).expect_err("没有 play: 的卡必须拒绝加载");
    assert!(error.to_string().contains("至少要有 1 条 play:"), "{error}");
}

#[test]
fn cards_reject_neutral_play_category() {
    let cards = card_json("hominidae", r#"[{"kind":"play","value":"neutral"}]"#);
    let error = load_cards(&cards).expect_err("play:neutral 必须拒绝加载");
    assert!(error.to_string().contains("不是可启用的玩法"), "{error}");
}

#[test]
fn cards_reject_duplicate_categories() {
    let cards = card_json(
        "hominidae",
        r#"[{"kind":"play","value":"mammal"},{"kind":"play","value":"mammal"}]"#,
    );
    let error = load_cards(&cards).expect_err("重复条目必须拒绝加载");
    assert!(error.to_string().contains("关联分类条目重复"), "{error}");
}

#[test]
fn cards_reject_prey_on_non_biological_taxon() {
    let cards = card_json(
        "automation",
        r#"[{"kind":"play","value":"machine"},{"kind":"prey","value":"hominidae"}]"#,
    );
    let error = load_cards(&cards).expect_err("造物卡的 prey: 必须拒绝加载");
    assert!(error.to_string().contains("只允许生物卡"), "{error}");
}

#[test]
fn cards_reject_unknown_prey_node() {
    let cards = card_json(
        "hominidae",
        r#"[{"kind":"play","value":"mammal"},{"kind":"prey","value":"no-such-node"}]"#,
    );
    let error = load_cards(&cards).expect_err("分类树外的节点必须拒绝加载");
    assert!(error.to_string().contains("不在分类树中"), "{error}");
}

#[test]
fn heroes_reject_diverging_categories() {
    let cards = card_json("hominidae", r#"[{"kind":"play","value":"mammal"}]"#);
    let heroes = r#"[{"id":"x","categories":[{"kind":"play","value":"bird"}],"skills":[]}]"#;
    let error = Catalog::from_json(&cards, heroes, "[]", CONFIG_JSON, TAXONOMY_JSON)
        .expect_err("英雄与同名随从的分类不一致必须拒绝加载");
    assert!(error.to_string().contains("不一致"), "{error}");
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
    let heroes =
        r#"[{"id":"no-such-card","categories":[{"kind":"play","value":"mammal"}],"skills":[]}]"#;
    assert!(Catalog::from_json("[]", heroes, "[]", CONFIG_JSON, TAXONOMY_JSON).is_err());
}

#[test]
fn heroes_reject_matching_effect_card() {
    let heroes = r#"[{"id":"aolinpike-jidian","categories":[{"kind":"play","value":"mammal"}],"skills":[]}]"#;
    let error = Catalog::from_json(CARDS_JSON, heroes, SKILLS_JSON, CONFIG_JSON, TAXONOMY_JSON)
        .expect_err("同 id 的效果牌不能充当随从卡");
    assert!(error.to_string().contains("不是随从卡"), "{error}");
}

#[test]
fn heroes_allow_skills_that_diverge_from_minion() {
    // 英雄技能与同名随从卡各自维护：同名随从卡带战吼时，英雄可以不带技能。
    let heroes =
        r#"[{"id":"adam-smith","categories":[{"kind":"play","value":"mammal"}],"skills":[]}]"#;
    Catalog::from_json(CARDS_JSON, heroes, SKILLS_JSON, CONFIG_JSON, TAXONOMY_JSON)
        .expect("英雄与同名随从卡的 skills 不要求一致");
}

#[test]
fn heroes_reject_categories_that_diverge_from_minion() {
    let heroes =
        r#"[{"id":"adam-smith","categories":[{"kind":"play","value":"bird"}],"skills":[]}]"#;
    let error = Catalog::from_json(CARDS_JSON, heroes, SKILLS_JSON, CONFIG_JSON, TAXONOMY_JSON)
        .expect_err("英雄与同名随从的关联分类必须一致");
    assert!(error.to_string().contains("不一致"), "{error}");
}

#[test]
fn heroes_reject_unknown_field() {
    // 英雄卡不再有 card_type；写入该字段按未知字段拒绝，防止把随从卡复制进英雄池。
    let bad = r#"[{"id":"x","card_type":"minion","categories":[{"kind":"play","value":"mammal"}],"skills":[]}]"#;
    assert!(Catalog::from_json("[]", bad, "[]", CONFIG_JSON, TAXONOMY_JSON).is_err());
}

#[test]
fn heroes_reject_unknown_skill() {
    let bad = r#"[{"id":"x","categories":[{"kind":"play","value":"mammal"}],"skills":["no-such-skill"]}]"#;
    assert!(Catalog::from_json("[]", bad, "[]", CONFIG_JSON, TAXONOMY_JSON).is_err());
}

#[test]
fn cards_reject_hero_card_type() {
    // CardType 已无 hero 变体，cards.json 写入 hero 在反序列化阶段即被拒绝。
    let bad = r#"[{"id":"x","card_type":"hero","categories":[{"kind":"play","value":"mammal"}],"skills":[]}]"#;
    assert!(Catalog::from_json(bad, "[]", "[]", CONFIG_JSON, TAXONOMY_JSON).is_err());
}

#[test]
fn every_card_skill_exists() {
    let catalog = test_catalog();
    for card in catalog.cards.values() {
        for skill in &card.skills {
            let name = catalog.skill_name(*skill);
            assert_eq!(
                catalog.skill_id(name),
                Some(*skill),
                "{} -> {}",
                card.id,
                name
            );
        }
    }
}

#[test]
fn unknown_field_fails() {
    let bad = r#"[{"id":"x","card_type":"minion","categories":[{"kind":"play","value":"mammal"}],"taxon":"hominidae","level":1,"era":"prehistoric","attack":1,"health":2,"oops":1}]"#;
    assert!(Catalog::from_json(bad, "[]", "[]", CONFIG_JSON, TAXONOMY_JSON).is_err());
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
    let Params::Growth(params) = &effects[0].params else {
        panic!("期望 Growth 参数");
    };
    assert_eq!(params.times, 1);
    assert_eq!(params.times_if.as_ref().unwrap().times, 2);
    assert_eq!(params.condition.as_ref().unwrap().compare, Compare::Ge);
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
    // `neutral` 保留给 Token 与内部哨兵：从 13 个可启用玩法里取不到，但筛选合法。
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
    let raw = r#"[{"trigger":"on_play","op":"modify_stats","selector":{"kind":"self"},"params":{"attack":1}}]"#;
    let effects: Vec<Effect> = serde_json::from_str(raw).unwrap();
    let Params::ModifyStats(params) = &effects[0].params else {
        panic!("期望 ModifyStats 参数");
    };
    assert_eq!(params.duration, None);
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
fn rejects_seven_voyages_off_activate_or_self() {
    let off_activate = r#"[{"trigger":"on_play","op":"seven_voyages","selector":{"kind":"self"},"params":{"turns_max":7}}]"#;
    let effects: Vec<Effect> = serde_json::from_str(off_activate).unwrap();
    assert!(validate_effects(&effects).is_err());

    let off_self = r#"[{"trigger":"on_activate","op":"seven_voyages","selector":{"kind":"side","side":"friendly","pick":"all","filters":[]},"params":{"turns_max":7}}]"#;
    let effects: Vec<Effect> = serde_json::from_str(off_self).unwrap();
    assert!(validate_effects(&effects).is_err());

    let out_of_range = r#"[{"trigger":"on_activate","op":"seven_voyages","selector":{"kind":"self"},"params":{"turns_max":8}}]"#;
    let effects: Vec<Effect> = serde_json::from_str(out_of_range).unwrap();
    assert!(validate_effects(&effects).is_err());
}

#[test]
fn committed_data_is_structured() {
    let catalog = test_catalog();
    let skill = catalog
        .skill_by_name("division-of-labor")
        .expect("技能存在");
    assert_eq!(skill.effects.len(), 1);
    assert_eq!(skill.effects[0].trigger, Trigger::OnActivate);
    assert!(
        catalog
            .skills()
            .iter()
            .all(|skill| validate_effects(&skill.effects).is_ok())
    );
}

#[test]
fn string_effects_are_rejected_at_load() {
    let bad = r#"[{"id":"x","effects":"OnPlay|Growth|self|times=1"}]"#;
    assert!(Catalog::from_json("[]", "[]", bad, CONFIG_JSON, TAXONOMY_JSON).is_err());
}

#[test]
fn rejects_unknown_selector_field() {
    let raw = r#"[{"trigger":"on_activate","op":"growth","selector":{"kind":"side","side":"friendly","pick":"all","filter":[{"key":"playstyle","value":"mammal"}]},"params":{"times":1}}]"#;
    assert!(serde_json::from_str::<Vec<Effect>>(raw).is_err());
}

#[test]
fn rejects_discover_without_choose() {
    let raw = r#"[{"trigger":"on_activate","op":"discover","selector":{"kind":"self"},"params":{"pool":"playstyle"}}]"#;
    // 缺失的必填字段在解码期即被拒绝，不再等到语义校验。
    assert!(serde_json::from_str::<Vec<Effect>>(raw).is_err());
}

#[test]
fn rejects_explicit_duration_on_growth() {
    let raw = r#"[{"trigger":"on_play","op":"growth","selector":{"kind":"self"},"params":{"times":1,"duration":"instant"}}]"#;
    // 不属于 Growth 的字段由 `deny_unknown_fields` 在解码期拒绝。
    assert!(serde_json::from_str::<Vec<Effect>>(raw).is_err());
}
