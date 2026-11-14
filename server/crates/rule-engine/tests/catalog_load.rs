mod common;

use common::{CONFIG_JSON, test_catalog};
use rule_engine::data::Catalog;

#[test]
fn loads_all_committed_data() {
    let catalog = test_catalog();
    assert_eq!(catalog.cards.len(), 220);
    assert_eq!(catalog.skills.len(), 118);
    assert_eq!(catalog.tokens.len(), 3);
    assert_eq!(catalog.cards.values().filter(|c| c.is_hero).count(), 13);
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
    let bad = r#"[{"id":"x","card_type":"随从","name":"x","tribe":"人类","level":1,"era":"史前","attack":1,"health":2,"is_scholar":false,"is_hero":false,"oops":1}]"#;
    assert!(Catalog::from_json(bad, "[]", "[]", CONFIG_JSON).is_err());
}
