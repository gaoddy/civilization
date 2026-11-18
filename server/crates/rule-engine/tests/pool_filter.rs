use std::collections::BTreeSet;

use rule_engine::data::{Card, CardType, Playstyle, Specialty, SpecialtyKind, card_in_pool};

fn card(playstyle: Playstyle, specialty: Vec<(&str, &str)>) -> Card {
    Card {
        id: "test-card".into(),
        card_type: CardType::Effect,
        playstyle,
        taxon: "phenomenon".into(),
        specialty: specialty
            .into_iter()
            .map(|(kind, value)| Specialty {
                kind: match kind {
                    "play" => SpecialtyKind::Play,
                    "prey" => SpecialtyKind::Prey,
                    _ => SpecialtyKind::Host,
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
        is_hero: false,
        cost: Some(2),
    }
}

fn enabled(list: &[Playstyle]) -> BTreeSet<Playstyle> {
    list.iter().copied().collect()
}

#[test]
fn effect_with_play_association_follows_enabled_playstyle() {
    let c = card(Playstyle::Neutral, vec![("play", "fish")]);
    assert!(!card_in_pool(&c, &enabled(&[Playstyle::Bird])));
    assert!(card_in_pool(&c, &enabled(&[Playstyle::Fish])));
}

#[test]
fn generic_effect_is_always_available() {
    let c = card(Playstyle::Neutral, vec![]);
    assert!(card_in_pool(&c, &enabled(&[Playstyle::Bird])));
    assert!(card_in_pool(&c, &enabled(&[])));
}

#[test]
fn playstyle_minion_requires_its_playstyle() {
    let mut c = card(Playstyle::Bird, vec![]);
    c.card_type = CardType::Minion;
    assert!(!card_in_pool(&c, &enabled(&[Playstyle::Fish])));
    assert!(card_in_pool(&c, &enabled(&[Playstyle::Bird])));
}
