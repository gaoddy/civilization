mod common;

use common::{CONFIG_JSON, test_catalog};
use rule_engine::data::{CardType, Catalog, Tribe};
use rule_engine::effects::{
    Compare, Duration, Filter, KeywordSpec, Op, Pick, Selector, Side, TimesIf, Trigger, Zone,
    parse_effects, validate_effects,
};
use rule_engine::error::CatalogError;

#[test]
fn parses_a_growth_effect() {
    let e = parse_effects("OnPlay|Growth|friendly:single|times=2").unwrap();
    assert_eq!(e[0].trigger, Trigger::OnPlay);
    assert_eq!(e[0].op, Op::Growth);
    assert!(matches!(
        e[0].selector,
        Selector::Side {
            side: Side::Friendly,
            pick: Pick::Single,
            ..
        }
    ));
    assert_eq!(e[0].params.times, Some(2));
    assert_eq!(e[0].params.duration, Duration::Instant);
}

#[test]
fn parses_aura_and_keyword_value() {
    let e = parse_effects(
        "Aura|ModifyStats|friendly:all:tribe=机械|attack=2,health=2,duration=permanent",
    )
    .unwrap();
    assert_eq!(e[0].params.duration, Duration::Permanent);
    assert_eq!(e[0].params.attack, Some(2));
    assert_eq!(e[0].params.health, Some(2));
    assert_eq!(
        e[0].selector,
        Selector::Side {
            side: Side::Friendly,
            pick: Pick::All,
            filters: vec![Filter {
                key: "tribe".into(),
                value: "机械".into(),
            }],
        }
    );

    let k = parse_effects("OnPlay|GrantKeyword|friendly:single|keyword=繁衍 2,duration=permanent")
        .unwrap();
    assert!(
        matches!(k[0].params.keyword, Some(KeywordSpec { keyword: ref s, value: Some(2) }) if s == "繁衍")
    );
}

#[test]
fn parses_selector_shorthands_and_multiple_filters() {
    let self_card = parse_effects("OnPlay|Growth|self|times=1").unwrap();
    assert_eq!(self_card[0].selector, Selector::SelfCard);

    let target = parse_effects("OnAllyGrowth|ModifyStats|target|health=1").unwrap();
    assert_eq!(target[0].selector, Selector::Target);

    let filters =
        parse_effects("OnPlay|Growth|friendly:all:tribe=植物:other=true:type=随从|times=1")
            .unwrap();
    match &filters[0].selector {
        Selector::Side { filters, .. } => {
            assert_eq!(filters.len(), 3);
            assert_eq!(filters[0].key, "tribe");
            assert_eq!(filters[0].value, "植物");
            assert_eq!(filters[1].key, "other");
            assert_eq!(filters[1].value, "true");
            assert_eq!(filters[2].key, "type");
            assert_eq!(filters[2].value, "随从");
        }
        other => panic!("expected side selector, got {other:?}"),
    }
}

#[test]
fn parses_structured_params() {
    let e = parse_effects(
        "ConditionalAura|ModifyStats|self|attack=1,duration=permanent,condition=count:friendly:tribe=人类:>=2",
    )
    .unwrap();
    let condition = e[0].params.condition.as_ref().unwrap();
    assert_eq!(condition.side, Side::Friendly);
    assert_eq!(
        condition.filter,
        Filter {
            key: "tribe".into(),
            value: "人类".into(),
        }
    );
    assert_eq!(condition.compare, Compare::Ge);
    assert_eq!(condition.count, 2);

    let e = parse_effects("OnPlay|Growth|friendly:single|times=1,timesIf=人类:2").unwrap();
    assert_eq!(
        e[0].params.times_if,
        Some(TimesIf {
            tribe: Tribe::Human,
            times: 2
        })
    );

    let e = parse_effects("OnAllySummon|ModifyStats|self|attack=1,when=tribe=人类").unwrap();
    assert_eq!(
        e[0].params.when,
        Some(Filter {
            key: "tribe".into(),
            value: "人类".into(),
        })
    );

    let e = parse_effects("OnActivate|Copy|self|from=hand,type=随从,to=hand").unwrap();
    assert_eq!(e[0].params.from, Some(Zone::Hand));
    assert_eq!(e[0].params.to, Some(Zone::Hand));
    assert_eq!(e[0].params.card_type, Some(CardType::Minion));

    let e = parse_effects("OnActivate|Discover|self|pool=tribe,choose=1").unwrap();
    assert_eq!(e[0].params.pool.as_deref(), Some("tribe"));
    assert_eq!(e[0].params.choose, Some(1));

    let e = parse_effects("OnFirstRefresh|AddShopSlot|self|era=current,count=1").unwrap();
    assert_eq!(e[0].params.era.as_deref(), Some("current"));
    assert_eq!(e[0].params.count, Some(1));

    let e = parse_effects("OnCombatStart|GrantKeyword|enemy:all|keyword=传染,value=1").unwrap();
    assert!(matches!(
        e[0].params.keyword,
        Some(KeywordSpec { keyword: ref name, value: Some(1) }) if name == "传染"
    ));
    assert_eq!(e[0].params.value, Some(1));

    let e =
        parse_effects("OnPlayEffect|GrantKeyword|friendly:single|keyword=繁衍,value=2").unwrap();
    assert!(matches!(
        e[0].params.keyword,
        Some(KeywordSpec { keyword: ref name, value: Some(2) }) if name == "繁衍"
    ));
    assert_eq!(e[0].params.value, Some(2));
}

#[test]
fn parses_multiple_segments_and_empty_effects() {
    assert!(parse_effects("[]").unwrap().is_empty());

    let e = parse_effects(
        "OnActivate|RefreshShop|self|free=true;OnActivate|GainMinion|self|from=shop,level=current,to=hand",
    )
    .unwrap();
    assert_eq!(e.len(), 2);
    assert_eq!(e[0].params.free, Some(true));
    assert_eq!(e[1].params.level.as_deref(), Some("current"));
}

#[test]
fn unknown_trigger_or_op_fails() {
    assert!(parse_effects("OnNope|Growth|self|times=1").is_err());
    assert!(parse_effects("OnPlay|Explode|self").is_err());
}

#[test]
fn unknown_selector_filter_or_param_fails() {
    assert!(parse_effects("OnPlay|Growth|sideways:single|times=1").is_err());
    assert!(parse_effects("OnPlay|Growth|friendly:nope|times=1").is_err());
    assert!(parse_effects("OnPlay|Growth|friendly:all:oops=1|times=1").is_err());
    assert!(parse_effects("OnPlay|Growth|friendly:all:tribe=精灵|times=1").is_err());
    assert!(parse_effects("OnPlay|Growth|friendly:all:type=法术|times=1").is_err());
    assert!(parse_effects("OnPlay|Growth|self|times=1,oops=2").is_err());
    assert!(parse_effects("OnPlay|Growth|self|times=two").is_err());
    assert!(parse_effects("OnPlay|ModifyStats|self|attack=1,duration=forever").is_err());
}

#[test]
fn missing_required_params_and_illegal_combinations_fail() {
    assert!(parse_effects("OnPlay|Growth|self").is_err());
    assert!(parse_effects("OnPlay|ModifyStats|self|duration=permanent").is_err());
    assert!(parse_effects("Aura|DealDamage|friendly:all|amount=1").is_err());
    assert!(parse_effects("OnPlay|Growth").is_err());
    assert!(parse_effects("OnPlay|Growth|self|times=1;").is_err());
}

#[test]
fn errors_include_the_raw_segment() {
    let errors = parse_effects("OnPlay|Explode|self").unwrap_err();
    assert!(
        errors
            .iter()
            .any(|message| message.contains("OnPlay|Explode|self"))
    );
}

#[test]
fn every_skill_in_catalog_validates() {
    let catalog = test_catalog();
    for skill in catalog.skills.values() {
        assert!(validate_effects(&skill.effects).is_ok(), "{}", skill.id);
    }
}

#[test]
fn catalog_rejects_unknown_effect_enum_at_load() {
    let bad_skills = r#"[{"id":"broken-skill","text":"战吼：爆炸","effects":[{"trigger":"on_play","op":"explode","selector":{"kind":"self"},"params":{}}]}]"#;
    let err = Catalog::from_json("[]", bad_skills, "[]", CONFIG_JSON).unwrap_err();
    assert!(
        matches!(err, CatalogError::File("skills.json", _)),
        "实为 {err:?}"
    );
}

#[test]
fn catalog_rejects_semantically_invalid_effect_at_load() {
    let bad_skills = r#"[{"id":"broken-skill","text":"光环：灼烧","effects":[{"trigger":"aura","op":"deal_damage","selector":{"kind":"side","side":"friendly","pick":"all","filters":[]},"params":{"amount":1}}]}]"#;
    let err = Catalog::from_json("[]", bad_skills, "[]", CONFIG_JSON).unwrap_err();
    match err {
        CatalogError::Rule(message) => {
            assert!(message.contains("broken-skill"), "{message}");
            assert!(message.contains("光环"), "{message}");
        }
        other => panic!("expected rule error, got {other:?}"),
    }
}

#[test]
fn parses_contagion_layer_from_value() {
    let e = parse_effects("OnCombatStart|GrantKeyword|enemy:all|keyword=传染,value=2").unwrap();
    assert!(matches!(
        e[0].params.keyword,
        Some(KeywordSpec { keyword: ref name, value: Some(2) }) if name == "传染"
    ));
}

#[test]
fn parses_deal_damage_can_kill() {
    let e = parse_effects("OnCombatStart|DealDamage|enemy:random|amount=3,canKill=false").unwrap();
    assert_eq!(e[0].op, Op::DealDamage);
    assert_eq!(e[0].params.amount, Some(3));
    assert_eq!(e[0].params.can_kill, Some(false));
}
