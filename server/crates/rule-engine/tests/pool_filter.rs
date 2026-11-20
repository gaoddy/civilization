use std::collections::BTreeSet;

use rule_engine::data::{Card, CardType, Category, CategoryKind, Playstyle, card_in_pool};

fn card(categories: Vec<(&str, &str)>) -> Card {
    Card {
        id: "test-card".into(),
        card_type: CardType::Effect,
        in_shop_pool: true,
        taxon: "biology".into(),
        categories: categories
            .into_iter()
            .map(|(kind, value)| Category {
                kind: match kind {
                    "play" => CategoryKind::Play,
                    "prey" => CategoryKind::Prey,
                    _ => CategoryKind::Host,
                },
                value: value.into(),
            })
            .collect(),
        subtype: None,
        level: 3,
        era: None,
        attack: None,
        health: None,
        skills: vec![],
        is_scholar: false,
        cost: Some(2),
    }
}

fn enabled(list: &[Playstyle]) -> BTreeSet<Playstyle> {
    list.iter().copied().collect()
}

#[test]
fn effect_with_play_category_follows_enabled_playstyle() {
    let c = card(vec![("play", "fish")]);
    assert!(!card_in_pool(&c, &enabled(&[Playstyle::Bird])));
    assert!(card_in_pool(&c, &enabled(&[Playstyle::Fish])));
}

#[test]
fn card_without_play_is_never_in_pool() {
    // 加载期会拒绝没有 `play:` 的卡；绕过加载构造时也不入池。
    let c = card(vec![]);
    assert!(!card_in_pool(&c, &enabled(&[Playstyle::Bird])));
    assert!(!card_in_pool(&c, &enabled(&[])));
}

#[test]
fn multi_play_card_enters_pool_on_any_match() {
    let c = card(vec![("play", "bird"), ("play", "mammal")]);
    assert!(card_in_pool(&c, &enabled(&[Playstyle::Bird])));
    assert!(card_in_pool(&c, &enabled(&[Playstyle::Mammal])));
    assert!(!card_in_pool(&c, &enabled(&[Playstyle::Fish])));
}

#[test]
fn playstyle_minion_requires_any_play_match() {
    let mut c = card(vec![("play", "bird")]);
    c.card_type = CardType::Minion;
    assert!(!card_in_pool(&c, &enabled(&[Playstyle::Fish])));
    assert!(card_in_pool(&c, &enabled(&[Playstyle::Bird])));
}
