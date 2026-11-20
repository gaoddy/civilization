mod common;

use common::{effects_of, minion, test_catalog};
use rule_engine::combat::{keyword_set, keywords};
use rule_engine::data::{Card, CardType, Skill};
use rule_engine::effects::Duration;
use rule_engine::keyword::Keyword;

#[test]
fn keyword_set_agrees_with_string_aggregate() {
    let catalog = test_catalog();
    for card_id in ["nanobot", "ant", "bee"] {
        let subject = minion(&catalog, 1, card_id);
        let set = keyword_set(&catalog, &subject);
        for name in keywords(&catalog, &subject) {
            assert!(
                set.contains_name(&name),
                "{card_id} 的 {name} 应在 KeywordSet 中"
            );
        }
    }
}

#[test]
fn unknown_and_armor_layers_are_preserved() {
    let catalog = test_catalog();
    let mut subject = minion(&catalog, 1, "nanobot");
    subject.granted = vec![
        ("未知关键词".to_string(), Duration::Permanent),
        ("armor 1".to_string(), Duration::Permanent),
        ("armor 2".to_string(), Duration::Permanent),
    ];

    let set = keyword_set(&catalog, &subject);
    assert!(set.contains_name("未知关键词"));
    assert_eq!(set.armor(), 3);
}

/// 已知关键词按枚举变体查询；`nanobot` 自带 `ignore_taunt` 与 `assemble 1`。
#[test]
fn known_keywords_query_by_variant() {
    let catalog = test_catalog();
    let subject = minion(&catalog, 1, "nanobot");

    let set = keyword_set(&catalog, &subject);
    assert!(set.contains(Keyword::IgnoreTaunt));
    assert!(set.contains(Keyword::Assemble));
    assert!(!set.contains(Keyword::Taunt));
    assert!(!set.contains(Keyword::Armor));
    assert_eq!(set.armor(), 0);
}

/// 测试直接往 `catalog.cards` / `catalog.skills` 插卡时，侧表缺失，
/// `base_keywords` 必须回退到即时计算。
#[test]
fn base_keywords_falls_back_for_directly_inserted_card() {
    let mut catalog = test_catalog();
    let skill_id = "fallback-skill".to_string();
    catalog.skills.insert(
        skill_id.clone(),
        Skill {
            id: skill_id.clone(),
            keyword: Some("taunt".to_string()),
            effects: effects_of("[]"),
            cost: None,
            uses_per_turn: None,
        },
    );
    catalog.cards.insert(
        "fallback-card".to_string(),
        Card {
            id: "fallback-card".to_string(),
            card_type: CardType::Minion,
            in_shop_pool: true,
            taxon: "hominidae".to_string(),
            categories: Vec::new(),
            subtype: None,
            level: 1,
            era: None,
            attack: Some(1),
            health: Some(1),
            skills: vec![skill_id],
            is_scholar: false,
            cost: Some(1),
        },
    );

    let base = catalog.base_keywords("fallback-card");
    assert!(base.contains(Keyword::Taunt));
    assert!(base.contains_name("taunt"));
    assert!(!base.contains(Keyword::Poison));
}
