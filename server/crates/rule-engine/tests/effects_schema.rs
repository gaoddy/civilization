mod common;

use common::{CONFIG_JSON, test_catalog};
use rule_engine::data::Catalog;
use rule_engine::effects::{Compare, Effect, Op, Pick, Selector, Side, Trigger, validate_effects};

#[test]
fn deserializes_a_structured_effect() {
    let raw = r#"[{"trigger":"on_activate","op":"growth","selector":{"kind":"side","side":"friendly","pick":"single","filters":[{"key":"tribe","value":"人类"}]},"params":{"times":1,"times_if":{"tribe":"人类","times":2},"condition":{"side":"friendly","filter":{"key":"tribe","value":"人类"},"compare":"ge","count":3}}}]"#;
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

    let discover = r#"[{"trigger":"on_activate","op":"discover","selector":{"kind":"self"},"params":{"pool":"tribe","choose":2}}]"#;
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
    let bad = r#"[{"id":"x","text":"x","effects":"OnPlay|Growth|self|times=1"}]"#;
    assert!(Catalog::from_json("[]", bad, "[]", CONFIG_JSON).is_err());
}

#[test]
fn rejects_unknown_selector_field() {
    let raw = r#"[{"trigger":"on_activate","op":"growth","selector":{"kind":"side","side":"friendly","pick":"all","filter":[{"key":"tribe","value":"人类"}]},"params":{"times":1}}]"#;
    assert!(serde_json::from_str::<Vec<Effect>>(raw).is_err());
}

#[test]
fn rejects_discover_without_choose() {
    let raw = r#"[{"trigger":"on_activate","op":"discover","selector":{"kind":"self"},"params":{"pool":"tribe"}}]"#;
    let effects: Vec<Effect> = serde_json::from_str(raw).unwrap();
    assert!(validate_effects(&effects).is_err());
}

#[test]
fn rejects_explicit_duration_on_growth() {
    let raw = r#"[{"trigger":"on_play","op":"growth","selector":{"kind":"self"},"params":{"times":1,"duration":"instant"}}]"#;
    let effects: Vec<Effect> = serde_json::from_str(raw).unwrap();
    assert!(validate_effects(&effects).is_err());
}
