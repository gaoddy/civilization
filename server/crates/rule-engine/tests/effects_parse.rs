mod common;

use common::{CONFIG_JSON, TAXONOMY_JSON, effects_of};
use rule_engine::data::{CardType, Catalog};
use rule_engine::effects::{
    Basis, Compare, Duration, Filter, FilterKey, KeywordSpec, Op, Params, Pick, Selector, Side,
    TimesIf, Trigger, Zone, parse_effects,
};

/// 取出与 `op` 对应的参数变体；类型不一致说明解析结果有误。
macro_rules! params {
    ($effect:expr, $variant:ident) => {
        match &$effect.params {
            Params::$variant(params) => params,
            other => panic!("期望 Params::{}，实为 {other:?}", stringify!($variant)),
        }
    };
}
use rule_engine::error::CatalogError;

#[test]
fn parses_new_ops_and_params() {
    let effects = effects_of(
        "Aura|ModifyStats|friendly:all:playstyle=fish|attack=1,health=1,perAttack=1,perHealth=0,maxCount=6",
    );
    let stats = params!(effects[0], ModifyStats);
    assert_eq!(stats.per_attack, Some(1));
    assert_eq!(stats.max_count, Some(6));
    let effects = effects_of("OnCombatStart|Detonate|enemy:single|amount=2,threshold=3");
    assert_eq!(effects[0].op, Op::Detonate);
    assert_eq!(params!(effects[0], Detonate).threshold, Some(3));
}

#[test]
fn parses_all_new_ops() {
    for (raw, op) in [
        (
            "OnCombatStart|Mimic|friendly:single|duration=combat",
            Op::Mimic,
        ),
        ("OnCombatStart|Devour|enemy:single|value=2", Op::Devour),
        (
            "OnCombatStart|Convert|enemy:single|threshold=2,card=fish",
            Op::Convert,
        ),
        (
            "OnCombatStart|Revive|friendly:single|attack=1,health=1",
            Op::Revive,
        ),
        ("OnCombatStart|Recall|friendly:single|to=hand", Op::Recall),
        (
            "OnActivate|Salvage|self|pool=playstyle,count=1",
            Op::Salvage,
        ),
        (
            "OnCombatStart|Latent|enemy:single|amount=2,count=1,threshold=3",
            Op::Latent,
        ),
    ] {
        let effects = effects_of(raw);
        assert_eq!(effects[0].op, op, "{raw}");
    }
}

#[test]
fn parses_mimic_stats_only() {
    let effects = effects_of("OnCombatStart|Mimic|friendly:single|duration=combat,statsOnly=true");
    assert_eq!(effects[0].op, Op::Mimic);
    assert_eq!(params!(effects[0], Mimic).stats_only, Some(true));

    // statsOnly 只属于拟态。
    assert!(
        parse_effects("OnCombatStart|Devour|enemy:single|value=2,statsOnly=true").is_err(),
        "statsOnly 不得用于 Devour"
    );
}

#[test]
fn parses_new_picks() {
    let effects = effects_of("Aura|ModifyStats|friendly:adjacent|attack=1");
    assert!(matches!(
        effects[0].selector,
        Selector::Side {
            pick: Pick::Adjacent,
            ..
        }
    ));

    let effects = effects_of("OnCombatStart|DealDamage|enemy:lowestAttack|amount=1");
    assert!(matches!(
        effects[0].selector,
        Selector::Side {
            pick: Pick::LowestAttack,
            ..
        }
    ));

    // 既有的 leftmost+adjacent 保持原义。
    let effects = effects_of("OnCombatStart|DealDamage|enemy:leftmost+adjacent|amount=1");
    assert!(matches!(
        effects[0].selector,
        Selector::Side {
            pick: Pick::LeftmostAdjacent,
            ..
        }
    ));
}

#[test]
fn parses_new_keywords() {
    let effects = effects_of("OnCombatStart|GrantKeyword|friendly:single|keyword=poison");
    assert_eq!(
        params!(effects[0], GrantKeyword).keyword,
        KeywordSpec {
            keyword: "poison".into(),
            value: None,
        }
    );

    let effects = effects_of("OnCombatStart|GrantKeyword|friendly:single|keyword=armor 2");
    assert_eq!(
        params!(effects[0], GrantKeyword).keyword,
        KeywordSpec {
            keyword: "armor".into(),
            value: Some(2),
        }
    );

    let effects = effects_of("OnCombatStart|GrantKeyword|friendly:single|keyword=stealth");
    assert_eq!(
        params!(effects[0], GrantKeyword).keyword,
        KeywordSpec {
            keyword: "stealth".into(),
            value: None,
        }
    );

    // armor 必须带层数，poison / stealth 禁止层数。
    assert!(parse_effects("OnCombatStart|GrantKeyword|friendly:single|keyword=armor").is_err());
    assert!(
        parse_effects(&format!(
            "OnCombatStart|GrantKeyword|friendly:single|keyword={} 2",
            "poison"
        ))
        .is_err()
    );
    assert!(
        parse_effects("OnCombatStart|GrantKeyword|friendly:single|keyword=stealth,value=1")
            .is_err()
    );
}

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
    assert_eq!(params!(e[0], Growth).times, 2);
}

#[test]
fn parses_aura_and_keyword_value() {
    let e = parse_effects(
        "Aura|ModifyStats|friendly:all:playstyle=machine|attack=2,health=2,duration=permanent",
    )
    .unwrap();
    let stats = params!(e[0], ModifyStats);
    assert_eq!(stats.duration, Some(Duration::Permanent));
    assert_eq!(stats.attack, Some(2));
    assert_eq!(stats.health, Some(2));
    assert_eq!(
        e[0].selector,
        Selector::Side {
            side: Side::Friendly,
            pick: Pick::All,
            filters: vec![Filter {
                key: FilterKey::Playstyle,
                value: "machine".into(),
            }],
        }
    );

    let k = parse_effects("OnPlay|GrantKeyword|friendly:single|keyword=breed 2,duration=permanent")
        .unwrap();
    assert!(
        matches!(&params!(k[0], GrantKeyword).keyword, KeywordSpec { keyword: s, value: Some(2) } if s == "breed")
    );
}

#[test]
fn parses_selector_shorthands_and_multiple_filters() {
    let self_card = parse_effects("OnPlay|Growth|self|times=1").unwrap();
    assert_eq!(self_card[0].selector, Selector::SelfCard);

    let target = parse_effects("OnAllyGrowth|ModifyStats|target|health=1").unwrap();
    assert_eq!(
        target[0].selector,
        Selector::Target {
            zone: Zone::Board,
            filters: vec![]
        },
        "裸 target 默认指向我方棋盘"
    );

    let filters =
        parse_effects("OnPlay|Growth|friendly:all:playstyle=plant:other=true:type=minion|times=1")
            .unwrap();
    match &filters[0].selector {
        Selector::Side { filters, .. } => {
            assert_eq!(filters.len(), 3);
            assert_eq!(filters[0].key, FilterKey::Playstyle);
            assert_eq!(filters[0].value, "plant");
            assert_eq!(filters[1].key, FilterKey::Other);
            assert_eq!(filters[1].value, "true");
            assert_eq!(filters[2].key, FilterKey::Type);
            assert_eq!(filters[2].value, "minion");
        }
        other => panic!("expected side selector, got {other:?}"),
    }
}

#[test]
fn parses_structured_params() {
    let e = parse_effects(
        "ConditionalAura|ModifyStats|self|attack=1,duration=permanent,condition=count:friendly:playstyle=mammal:>=2",
    )
    .unwrap();
    let condition = e[0].params.condition().unwrap();
    assert_eq!(condition.side, Side::Friendly);
    assert_eq!(
        condition.filter,
        Filter {
            key: FilterKey::Playstyle,
            value: "mammal".into(),
        }
    );
    assert_eq!(condition.compare, Compare::Ge);
    assert_eq!(condition.count, 2);

    let e =
        parse_effects("OnPlay|Growth|friendly:single|times=1,timesIf=playstyle:mammal:2").unwrap();
    assert_eq!(
        params!(e[0], Growth).times_if,
        Some(TimesIf {
            key: FilterKey::Playstyle,
            value: "mammal".into(),
            times: 2
        })
    );

    let e = parse_effects("OnAllySummon|ModifyStats|self|attack=1,when=playstyle=mammal").unwrap();
    assert_eq!(
        params!(e[0], ModifyStats).when,
        Some(Filter {
            key: FilterKey::Playstyle,
            value: "mammal".into(),
        })
    );

    let e = parse_effects("OnActivate|Copy|target:hand:type=minion|from=hand,type=minion,to=hand")
        .unwrap();
    let copy = params!(e[0], Copy);
    assert_eq!(copy.from, Some(Zone::Hand));
    assert_eq!(copy.to, Zone::Hand);
    assert_eq!(copy.card_type, Some(CardType::Minion));

    let e = parse_effects("OnActivate|Discover|self|pool=playstyle,choose=1").unwrap();
    let discover = params!(e[0], Discover);
    assert_eq!(discover.pool, "playstyle");
    assert_eq!(discover.choose, 1);

    let e = parse_effects("OnFirstRefresh|AddShopSlot|self|era=current,count=1").unwrap();
    let slots = params!(e[0], AddShopSlot);
    assert_eq!(slots.era, "current");
    assert_eq!(slots.count, 1);

    let e =
        parse_effects("OnCombatStart|GrantKeyword|enemy:all|keyword=contagion,value=1").unwrap();
    assert!(matches!(
        &params!(e[0], GrantKeyword).keyword,
        KeywordSpec { keyword: name, value: Some(1) } if name == "contagion"
    ));
    assert_eq!(params!(e[0], GrantKeyword).value, Some(1));

    let e =
        parse_effects("OnPlayEffect|GrantKeyword|friendly:single|keyword=breed,value=2").unwrap();
    assert!(matches!(
        &params!(e[0], GrantKeyword).keyword,
        KeywordSpec { keyword: name, value: Some(2) } if name == "breed"
    ));
    assert_eq!(params!(e[0], GrantKeyword).value, Some(2));
}

#[test]
fn parses_multiple_segments_and_empty_effects() {
    assert!(parse_effects("[]").unwrap().is_empty());

    let e = parse_effects(
        "OnActivate|RefreshShop|self|free=true;OnActivate|GainMinion|self|from=shop,level=current,to=hand",
    )
    .unwrap();
    assert_eq!(e.len(), 2);
    assert_eq!(params!(e[0], RefreshShop).free, Some(true));
    assert_eq!(params!(e[1], GainMinion).level, "current");
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
    assert!(parse_effects("OnPlay|Growth|friendly:all:playstyle=古菌|times=1").is_err());
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
fn catalog_rejects_unknown_effect_enum_at_load() {
    let bad_skills = r#"[{"id":"broken-skill","effects":[{"trigger":"on_play","op":"explode","selector":{"kind":"self"},"params":{}}]}]"#;
    let err = Catalog::from_json("[]", "[]", bad_skills, CONFIG_JSON, TAXONOMY_JSON).unwrap_err();
    assert!(
        matches!(err, CatalogError::File("skills.json", _)),
        "实为 {err:?}"
    );
}

#[test]
fn catalog_rejects_semantically_invalid_effect_at_load() {
    let bad_skills = r#"[{"id":"broken-skill","effects":[{"trigger":"aura","op":"deal_damage","selector":{"kind":"side","side":"friendly","pick":"all","filters":[]},"params":{"amount":1}}]}]"#;
    let err = Catalog::from_json("[]", "[]", bad_skills, CONFIG_JSON, TAXONOMY_JSON).unwrap_err();
    match err {
        CatalogError::Rule(message) => {
            assert!(message.contains("broken-skill"), "{message}");
            assert!(message.contains("光环"), "{message}");
        }
        other => panic!("expected rule error, got {other:?}"),
    }
}

#[test]
fn parses_deal_damage_can_kill() {
    let e = parse_effects("OnCombatStart|DealDamage|enemy:random|amount=3,canKill=false").unwrap();
    assert_eq!(e[0].op, Op::DealDamage);
    assert_eq!(params!(e[0], DealDamage).amount, 3);
    assert_eq!(params!(e[0], DealDamage).can_kill, Some(false));
}

#[test]
fn semantic_rules_live_in_validate_only() {
    // DSL 解析只负责语法；跨字段规则统一由 parse_chunk 末尾的 validate_effect 给出。
    let semantic = [
        "OnActivate|GainMinion|self|from=board,level=current,to=hand",
        "OnActivate|Discover|self|pool=playstyle,choose=2",
        "OnCombatStart|GrantKeyword|enemy:all|keyword=繁殖",
        "OnCombatStart|GrantKeyword|enemy:all|keyword=taunt 1",
        "Aura|DealDamage|enemy:all|amount=1",
        "OnPlay|Growth|self|times=1,timesIf=unknown:2",
    ];
    for raw in semantic {
        let errors = parse_effects(raw).expect_err(raw);
        assert_eq!(errors.len(), 1, "{raw} 应只报一次：{errors:?}");
    }

    // 解析器只保留取值归一化，不再重复判定。
    let e = parse_effects("OnCombatStart|GrantKeyword|enemy:all|keyword=breed 2")
        .expect("内联层数应可解析");
    assert_eq!(
        params!(e[0], GrantKeyword).keyword,
        KeywordSpec {
            keyword: "breed".into(),
            value: Some(2)
        }
    );
}

#[test]
fn parses_kill_triggers_and_decompose_basis() {
    let e = parse_effects("OnKill|Decompose|self|basis=original,duration=permanent").unwrap();
    assert_eq!(e[0].trigger, Trigger::OnKill);
    assert_eq!(e[0].op, Op::Decompose);
    assert_eq!(params!(e[0], Decompose).basis, Basis::Original);
    assert_eq!(params!(e[0], Decompose).duration, Duration::Permanent);

    let ally =
        parse_effects("OnAllyKill|Decompose|self|basis=combat_start,duration=combat").unwrap();
    assert_eq!(ally[0].trigger, Trigger::OnAllyKill);
    assert_eq!(params!(ally[0], Decompose).basis, Basis::CombatStart);

    let pre_kill =
        parse_effects("OnFriendlyDeath|Decompose|self|basis=pre_kill,duration=permanent").unwrap();
    assert_eq!(params!(pre_kill[0], Decompose).basis, Basis::PreKill);
}

#[test]
fn decompose_requires_known_basis_and_duration() {
    for raw in [
        "OnKill|Decompose|self|duration=permanent",
        "OnKill|Decompose|self|basis=original",
        "OnKill|Decompose|self|basis=unknown,duration=permanent",
        "OnKill|Decompose|self|basis=original,amount=2,duration=permanent",
    ] {
        parse_effects(raw).expect_err(raw);
    }

    // when 可用于击杀类事件触发。
    parse_effects(
        "OnAllyKill|Decompose|self|basis=original,duration=permanent,when=taxon=animalia",
    )
    .expect("when 应允许用于 OnAllyKill");
}
