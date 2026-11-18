//! 卡池判定：玩法过滤、中立与学者口径、分类祖先匹配、Token 回退。

mod common;

use std::collections::BTreeSet;

use common::{effects_of, place, setup, test_catalog};
use rule_engine::data::{
    Card, CardType, Catalog, Era, Playstyle, Specialty, SpecialtyKind, Token, card_in_pool,
};
use rule_engine::effects::{Effect, EffectCtx};
use rule_engine::engine::Engine;
use rule_engine::state::MinionInstance;
use rule_engine::state::{GameState, PendingChoice, Phase, ShopSlot};

const ENABLED: [Playstyle; 5] = [
    Playstyle::Mammal,
    Playstyle::Plant,
    Playstyle::Fish,
    Playstyle::Insect,
    Playstyle::Virus,
];

fn enabled_set(playstyles: [Playstyle; 5]) -> BTreeSet<Playstyle> {
    playstyles.into_iter().collect()
}

fn card(id: &str, playstyle: Playstyle, specialty: Vec<Specialty>) -> Card {
    Card {
        id: id.to_string(),
        card_type: CardType::Minion,
        playstyle,
        taxon: "hominidae".to_string(),
        specialty,
        subtype: None,
        level: 1,
        era: None,
        attack: Some(1),
        health: Some(1),
        skills: Vec::new(),
        is_scholar: false,
        is_hero: false,
        cost: Some(1),
    }
}

fn play(value: &str) -> Specialty {
    Specialty {
        kind: SpecialtyKind::Play,
        value: value.to_string(),
    }
}

fn add_card(catalog: &mut Catalog, id: &str, playstyle: Playstyle, specialty: Vec<Specialty>) {
    catalog
        .cards
        .insert(id.to_string(), card(id, playstyle, specialty));
}

/// 只保留 1 星权重，保证商店只抽 1 星测试夹具。
fn only_first_star(engine: &mut Engine) {
    for row in &mut engine.catalog.config.shop_weights {
        *row = [1, 0, 0, 0, 0, 0, 0];
    }
}

fn setup_with(catalog: Catalog, playstyles: [Playstyle; 5]) -> (Engine, GameState) {
    let (engine, mut state) = Engine::new(catalog, [3; 32], playstyles);
    state.phase = Phase::Recruit;
    state.turn = 1;
    for player in &mut state.players {
        player.hp = 30;
        player.gold = 10;
        player.era = Era::Prehistoric;
    }
    (engine, state)
}

fn ctx(seat: u8) -> EffectCtx {
    EffectCtx {
        seat,
        source: None,
        target: None,
        event_target: None,
        golden: false,
        discover_level: None,
    }
}

fn effect(raw: &str) -> Effect {
    effects_of(raw).remove(0)
}

fn discover_candidates(state: &GameState) -> Vec<String> {
    match state.pending.front() {
        Some(PendingChoice::Discover { candidates, .. }) => candidates.clone(),
        other => panic!("应登记发现待选，实为 {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// card_in_pool 边界
// ---------------------------------------------------------------------------

#[test]
fn card_in_pool_checks_non_neutral_by_exact_playstyle() {
    let enabled = enabled_set(ENABLED);
    assert!(card_in_pool(
        &card("mammal", Playstyle::Mammal, vec![]),
        &enabled
    ));
    assert!(!card_in_pool(
        &card("bird", Playstyle::Bird, vec![]),
        &enabled
    ));
}

#[test]
fn card_in_pool_neutral_uses_play_associations() {
    let enabled = enabled_set(ENABLED);
    // 没有 `play:`：纯通用，始终可用。
    assert!(card_in_pool(
        &card("generic", Playstyle::Neutral, vec![]),
        &enabled
    ));
    // 有 `play:` 但一个都不命中：不进池。
    assert!(!card_in_pool(
        &card("bird-tool", Playstyle::Neutral, vec![play("bird")]),
        &enabled
    ));
    // 任一命中即进池。
    assert!(card_in_pool(
        &card(
            "multi-tool",
            Playstyle::Neutral,
            vec![play("bird"), play("mammal")]
        ),
        &enabled
    ));
}

#[test]
fn scholar_enters_pool_by_play_association() {
    let catalog = test_catalog();
    let agi = catalog.cards.get("agi").expect("学者卡 agi 存在");
    assert!(agi.is_scholar);
    assert_eq!(agi.playstyle, Playstyle::Neutral);

    let with_machine = enabled_set([
        Playstyle::Machine,
        Playstyle::Plant,
        Playstyle::Fish,
        Playstyle::Insect,
        Playstyle::Virus,
    ]);
    let without = enabled_set([
        Playstyle::Bird,
        Playstyle::Plant,
        Playstyle::Fish,
        Playstyle::Insect,
        Playstyle::Virus,
    ]);
    assert!(card_in_pool(agi, &with_machine), "学者按 play: 进池");
    assert!(!card_in_pool(agi, &without), "未命中 play: 时不进池");
}

// ---------------------------------------------------------------------------
// 商店 / 发现 / 商店效果
// ---------------------------------------------------------------------------

#[test]
fn shop_roll_excludes_unenabled_playstyle() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "on-mammal", Playstyle::Mammal, vec![]);
    add_card(&mut catalog, "off-bird", Playstyle::Bird, vec![]);
    let (mut engine, mut state) = setup_with(catalog, ENABLED);
    only_first_star(&mut engine);
    state.pool.clear();
    state.pool.insert("on-mammal".to_string(), 20);
    state.pool.insert("off-bird".to_string(), 20);

    for _ in 0..30 {
        engine.roll_shop(&mut state, 0);
        for slot in &state.players[0].shop {
            let slot = slot.as_ref().expect("1 星池非空，槽位应有卡");
            assert_ne!(slot.card_id, "off-bird", "未启用玩法的卡不得进商店");
            let card = engine
                .catalog
                .cards
                .get(&slot.card_id)
                .expect("商店卡应存在于目录");
            if card.card_type == CardType::Minion {
                assert_eq!(card.id, "on-mammal", "1 星随从只应来自启用玩法");
            }
        }
    }
}

#[test]
fn discover_candidates_exclude_unenabled_playstyle() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "on-mammal", Playstyle::Mammal, vec![]);
    add_card(&mut catalog, "off-bird", Playstyle::Bird, vec![]);
    let (mut engine, mut state) = setup_with(catalog, ENABLED);
    state.pool.clear();
    state.pool.insert("on-mammal".to_string(), 20);
    state.pool.insert("off-bird".to_string(), 20);

    engine.apply_op(
        &mut state,
        &effect("OnActivate|Discover|self|pool=playstyle,choose=1"),
        &ctx(0),
        None,
    );
    assert_eq!(discover_candidates(&state), vec!["on-mammal".to_string()]);
}

#[test]
fn shop_effects_exclude_unenabled_playstyle() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "off-bird", Playstyle::Bird, vec![]);
    add_card(&mut catalog, "on-mammal", Playstyle::Mammal, vec![]);
    let (mut engine, mut state) = setup_with(catalog, ENABLED);
    state.pool.clear();
    state.pool.insert("off-bird".to_string(), 20);
    state.pool.insert("on-mammal".to_string(), 20);

    // AddShopSlot：目录顺序里 off-bird 在 on-mammal 之前，过滤后应选中 on-mammal。
    engine.apply_op(
        &mut state,
        &effect("OnFirstRefresh|AddShopSlot|self|era=current,count=1"),
        &ctx(0),
        None,
    );
    assert_eq!(
        state.players[0]
            .shop
            .last()
            .and_then(Option::as_ref)
            .map(|slot| slot.card_id.as_str()),
        Some("on-mammal")
    );

    // GainMinion：商店只有未启用玩法的卡时不获得。
    state.players[0].hand.clear();
    state.players[0].shop = vec![Some(ShopSlot {
        instance_id: 4_242,
        card_id: "off-bird".to_string(),
        frozen: false,
    })];
    engine.apply_op(
        &mut state,
        &effect("OnActivate|GainMinion|self|from=shop,level=current,to=hand"),
        &ctx(0),
        None,
    );
    assert!(state.players[0].hand.is_empty(), "未启用玩法不可获得");

    state.players[0].shop = vec![Some(ShopSlot {
        instance_id: 4_243,
        card_id: "on-mammal".to_string(),
        frozen: false,
    })];
    engine.apply_op(
        &mut state,
        &effect("OnActivate|GainMinion|self|from=shop,level=current,to=hand"),
        &ctx(0),
        None,
    );
    assert_eq!(state.players[0].hand.len(), 1);
    assert_eq!(state.players[0].hand[0].card_id(), "on-mammal");
}

#[test]
fn neutral_generic_card_always_available() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "generic-tool", Playstyle::Neutral, vec![]);
    let (mut engine, mut state) = setup_with(catalog, ENABLED);
    only_first_star(&mut engine);
    state.pool.clear();
    state.pool.insert("generic-tool".to_string(), 20);

    let mut seen = false;
    for _ in 0..30 {
        engine.roll_shop(&mut state, 0);
        for slot in &state.players[0].shop {
            let slot = slot.as_ref().expect("1 星池非空，槽位应有卡");
            let card = engine
                .catalog
                .cards
                .get(&slot.card_id)
                .expect("商店卡应存在于目录");
            if card.card_type == CardType::Minion {
                assert_eq!(card.id, "generic-tool", "纯通用随从始终在池中");
                seen = true;
            }
        }
    }
    assert!(seen, "纯通用卡应能被抽到");
}

#[test]
fn targeted_effect_follows_enabled_playstyle_through_shop_roll() {
    // 真实卡表：yangliu 是鱼纲定向 1 星效果牌，dishengxiyu 是通用 1 星效果牌。
    let no_fish = [
        Playstyle::Mammal,
        Playstyle::Bird,
        Playstyle::Plant,
        Playstyle::Insect,
        Playstyle::Virus,
    ];
    let with_fish = [
        Playstyle::Mammal,
        Playstyle::Fish,
        Playstyle::Plant,
        Playstyle::Insect,
        Playstyle::Virus,
    ];

    let (mut engine, mut state) = setup_with(test_catalog(), no_fish);
    only_first_star(&mut engine);
    state.pool.clear(); // 效果牌不占共享池；清空后随从无份数，候选只剩效果牌。
    let mut saw_generic = false;
    for _ in 0..30 {
        engine.roll_shop(&mut state, 0);
        for slot in &state.players[0].shop {
            let slot = slot.as_ref().expect("1 星效果牌非空");
            assert_ne!(slot.card_id, "yangliu", "未启用鱼纲时定向牌不得进商店");
            saw_generic |= slot.card_id == "dishengxiyu";
        }
    }
    assert!(saw_generic, "通用效果牌始终可抽");

    let (mut engine, mut state) = setup_with(test_catalog(), with_fish);
    only_first_star(&mut engine);
    state.pool.clear();
    let (mut saw_targeted, mut saw_generic) = (false, false);
    for _ in 0..30 {
        engine.roll_shop(&mut state, 0);
        for slot in &state.players[0].shop {
            let slot = slot.as_ref().expect("1 星效果牌非空");
            saw_targeted |= slot.card_id == "yangliu";
            saw_generic |= slot.card_id == "dishengxiyu";
        }
    }
    assert!(saw_targeted, "启用鱼纲后定向牌可进商店");
    assert!(saw_generic, "通用效果牌始终可抽");
}

#[test]
fn scholar_discover_uses_play_association() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "on-mammal", Playstyle::Mammal, vec![]);
    let (mut engine, mut state) = setup_with(
        catalog,
        [
            Playstyle::Machine,
            Playstyle::Plant,
            Playstyle::Fish,
            Playstyle::Insect,
            Playstyle::Virus,
        ],
    );
    // 真实学者卡 agi：neutral + play:machine / play:mammal。
    state.pool.clear();
    state.pool.insert("agi".to_string(), 20);
    state.pool.insert("on-mammal".to_string(), 20);

    let mut discover_ctx = ctx(0);
    discover_ctx.discover_level = Some(6);
    engine.apply_op(
        &mut state,
        &effect("OnActivate|Discover|self|pool=playstyle,choose=1"),
        &discover_ctx,
        None,
    );
    assert_eq!(discover_candidates(&state), vec!["agi".to_string()]);
}

// ---------------------------------------------------------------------------
// 分类祖先匹配与 Token 回退
// ---------------------------------------------------------------------------

#[test]
fn taxon_filter_matches_ancestors() {
    let (mut engine, mut state) = setup(1);
    // adam-smith：hominidae(科) < primates(目) < mammalia(纲)。
    place(&engine.catalog, &mut state, 0, 1, "adam-smith");

    engine.apply_op(
        &mut state,
        &effect("OnActivate|Growth|friendly:all:taxon=primates|times=1"),
        &ctx(0),
        None,
    );
    assert_eq!(state.players[0].board[0].stats.attack.growth, 1);

    // 无关祖先不命中。
    engine.apply_op(
        &mut state,
        &effect("OnActivate|Growth|friendly:all:taxon=felidae|times=1"),
        &ctx(0),
        None,
    );
    assert_eq!(state.players[0].board[0].stats.attack.growth, 1);

    // 祖先链上的高阶节点同样命中。
    engine.apply_op(
        &mut state,
        &effect("OnActivate|Growth|friendly:all:taxon=mammalia|times=1"),
        &ctx(0),
        None,
    );
    assert_eq!(state.players[0].board[0].stats.attack.growth, 2);
}

#[test]
fn prey_and_host_filters_match_ancestor_node() {
    let mut catalog = test_catalog();
    let mut hunting_card = card("hunting-card", Playstyle::Mammal, vec![]);
    hunting_card.specialty = vec![
        Specialty {
            kind: SpecialtyKind::Prey,
            value: "artiodactyla".to_string(),
        },
        Specialty {
            kind: SpecialtyKind::Host,
            value: "hominidae".to_string(),
        },
    ];
    catalog
        .cards
        .insert("hunting-card".to_string(), hunting_card);
    let (mut engine, mut state) = setup_with(catalog, ENABLED);
    place(&engine.catalog, &mut state, 0, 1, "hunting-card");

    // artiodactyla(目) < mammalia(纲)。
    engine.apply_op(
        &mut state,
        &effect("OnActivate|Growth|friendly:all:prey=mammalia|times=1"),
        &ctx(0),
        None,
    );
    assert_eq!(state.players[0].board[0].stats.attack.growth, 1);

    engine.apply_op(
        &mut state,
        &effect("OnActivate|Growth|friendly:all:prey=carnivora|times=1"),
        &ctx(0),
        None,
    );
    assert_eq!(state.players[0].board[0].stats.attack.growth, 1);

    // host: hominidae < primates。
    engine.apply_op(
        &mut state,
        &effect("OnActivate|Growth|friendly:all:host=primates|times=1"),
        &ctx(0),
        None,
    );
    assert_eq!(state.players[0].board[0].stats.attack.growth, 2);

    engine.apply_op(
        &mut state,
        &effect("OnActivate|Growth|friendly:all:host=felidae|times=1"),
        &ctx(0),
        None,
    );
    assert_eq!(state.players[0].board[0].stats.attack.growth, 2);
}

#[test]
fn play_and_playstyle_filters_use_card_playstyle() {
    let (mut engine, mut state) = setup(1);
    place(&engine.catalog, &mut state, 0, 1, "nanobot"); // machine, automation

    engine.apply_op(
        &mut state,
        &effect("OnActivate|Growth|friendly:all:playstyle=machine|times=1"),
        &ctx(0),
        None,
    );
    // play：取 specialty 中的 play 关联；nanobot 没有 play 关联，不命中。
    engine.apply_op(
        &mut state,
        &effect("OnActivate|Growth|friendly:all:play=machine|times=1"),
        &ctx(0),
        None,
    );
    assert_eq!(state.players[0].board[0].stats.attack.growth, 1);
}

#[test]
fn token_uses_own_playstyle_and_taxon_defaults() {
    let (mut engine, mut state) = setup(1);
    // 真实 Token：seedling 为植物，taxon plantae-incertae-sedis < plantae。
    let seedling_id = state.next_instance_id;
    state.next_instance_id = seedling_id.saturating_add(1);
    state.players[0].board.push(MinionInstance::new(
        &engine.catalog,
        seedling_id,
        "seedling",
        false,
    ));

    engine.apply_op(
        &mut state,
        &effect("OnActivate|Growth|friendly:all:playstyle=plant|times=1"),
        &ctx(0),
        None,
    );
    assert_eq!(state.players[0].board[0].stats.attack.growth, 1);
    engine.apply_op(
        &mut state,
        &effect("OnActivate|Growth|friendly:all:taxon=plantae|times=1"),
        &ctx(0),
        None,
    );
    assert_eq!(state.players[0].board[0].stats.attack.growth, 2);

    // 缺字段的 Token：按 neutral / phenomenon 处理。
    engine.catalog.tokens.insert(
        "plain-token".to_string(),
        Token {
            id: "plain-token".to_string(),
            attack: 1,
            health: 1,
            keywords: Vec::new(),
            playstyle: None,
            taxon: None,
        },
    );
    let plain_id = state.next_instance_id;
    state.next_instance_id = plain_id.saturating_add(1);
    state.players[0].board.push(MinionInstance::new(
        &engine.catalog,
        plain_id,
        "plain-token",
        false,
    ));

    // 缺省玩法与分类的回退口径。
    let plain_token = engine
        .catalog
        .tokens
        .get("plain-token")
        .expect("刚插入的 Token");
    assert_eq!(plain_token.playstyle(), Playstyle::Neutral);
    assert_eq!(plain_token.taxon(), "phenomenon");
    engine.apply_op(
        &mut state,
        &effect("OnActivate|Growth|friendly:all:taxon=phenomenon|times=1"),
        &ctx(0),
        None,
    );
    assert_eq!(state.players[0].board[1].stats.attack.growth, 1);
}

#[test]
fn missing_card_facts_are_skipped() {
    let (mut engine, mut state) = setup(1);
    place(&engine.catalog, &mut state, 0, 1, "adam-smith");
    engine.apply_op(
        &mut state,
        &effect("OnActivate|Growth|friendly:all:taxon=no-such-node|times=1"),
        &ctx(0),
        None,
    );
    assert_eq!(state.players[0].board[0].stats.attack.growth, 0);
}
