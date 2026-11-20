//! 卡池判定：玩法过滤、中立与多玩法归属口径、分类祖先匹配、非池卡事实。

mod common;

use std::collections::BTreeSet;

use common::{effects_of, place, setup, test_catalog};
use rule_engine::data::{
    Card, CardType, Catalog, Category, CategoryKind, Era, Playstyle, card_in_pool, hero_in_pool,
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

fn card(id: &str, categories: Vec<Category>) -> Card {
    Card {
        id: id.to_string(),
        card_type: CardType::Minion,
        in_shop_pool: true,
        taxon: "hominidae".to_string(),
        categories,
        subtype: None,
        level: 1,
        era: None,
        attack: Some(1),
        health: Some(1),
        skills: Vec::new(),
        cost: Some(1),
    }
}

fn play(value: &str) -> Category {
    Category {
        kind: CategoryKind::Play,
        value: value.to_string(),
    }
}

fn has_play(card: &Card, value: &str) -> bool {
    card.categories
        .iter()
        .any(|category| category.kind == CategoryKind::Play && category.value == value)
}

fn add_card(catalog: &mut Catalog, id: &str, categories: Vec<Category>) {
    catalog.cards.insert(id.to_string(), card(id, categories));
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
        level_override: None,
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
fn card_in_pool_requires_any_play_match() {
    let enabled = enabled_set(ENABLED);
    assert!(card_in_pool(
        &card("mammal", vec![play("mammal")]),
        &enabled
    ));
    assert!(!card_in_pool(&card("bird", vec![play("bird")]), &enabled));
}

#[test]
fn card_in_pool_requires_play_categories() {
    let enabled = enabled_set(ENABLED);
    // 没有 `play:`：加载期已拒绝，构造出的卡也不入池。
    assert!(!card_in_pool(&card("no-play", vec![]), &enabled));
    // 有 `play:` 但一个都不命中：不进池。
    assert!(!card_in_pool(
        &card("bird-tool", vec![play("bird")]),
        &enabled
    ));
    // 任一命中即进池。
    assert!(card_in_pool(
        &card("multi-tool", vec![play("bird"), play("mammal")]),
        &enabled
    ));
}

#[test]
fn zero_attack_minion_enters_pool_by_play_association() {
    let catalog = test_catalog();
    let agi = catalog.cards.get("agi").expect("多玩法卡 agi 存在");
    assert!(has_play(agi, "mammal"), "agi 带 play:mammal");
    assert!(has_play(agi, "machine"), "agi 带 play:machine");

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
    assert!(card_in_pool(agi, &with_machine), "按 play: 进池");
    assert!(!card_in_pool(agi, &without), "未命中 play: 时不进池");
}

#[test]
fn hero_pool_documents_playstyles_without_direct_heroes() {
    // 真实英雄数据当前只覆盖 10 个玩法：真菌、细菌、原生生物没有直接归属英雄。
    // 房间层依赖 `assign_candidates` 在过滤后池为空时回退全部英雄（32 张，见
    // room-service 的 `empty_filtered_pool_falls_back_to_all_heroes`）。本用例
    // 固定这一数据事实；后续若为这三个玩法补英雄，应随数据一并更新。
    let catalog = test_catalog();
    let direct = |playstyle: Playstyle| {
        let enabled = BTreeSet::from([playstyle]);
        catalog
            .heroes
            .values()
            .filter(|hero| hero_in_pool(hero, &enabled))
            .count()
    };

    for playstyle in [Playstyle::Fungus, Playstyle::Bacteria, Playstyle::Protozoa] {
        assert_eq!(
            direct(playstyle),
            0,
            "{} 目前没有直接归属英雄，房间层依赖回退",
            playstyle.id()
        );
    }
    for playstyle in Playstyle::ALL {
        if matches!(
            playstyle,
            Playstyle::Fungus | Playstyle::Bacteria | Playstyle::Protozoa
        ) {
            continue;
        }
        assert!(
            direct(playstyle) > 0,
            "{} 应至少有一张直接归属英雄",
            playstyle.id()
        );
    }
    assert_eq!(catalog.heroes.len(), 32, "回退池为全部英雄卡");
}

// ---------------------------------------------------------------------------
// 商店 / 发现 / 商店效果
// ---------------------------------------------------------------------------

#[test]
fn shop_roll_excludes_unenabled_playstyle() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "on-mammal", vec![play("mammal")]);
    add_card(&mut catalog, "off-bird", vec![play("bird")]);
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
    add_card(&mut catalog, "on-mammal", vec![play("mammal")]);
    add_card(&mut catalog, "off-bird", vec![play("bird")]);
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
    add_card(&mut catalog, "off-bird", vec![play("bird")]);
    add_card(&mut catalog, "on-mammal", vec![play("mammal")]);
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
fn card_without_play_never_enters_shop() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "generic-tool", vec![]);
    let (mut engine, mut state) = setup_with(catalog, ENABLED);
    only_first_star(&mut engine);
    state.pool.clear();
    state.pool.insert("generic-tool".to_string(), 20);

    for _ in 0..30 {
        engine.roll_shop(&mut state, 0);
        for slot in state.players[0].shop.iter().flatten() {
            assert_ne!(slot.card_id, "generic-tool", "没有 play: 的卡不得进商店");
        }
    }
}

#[test]
fn targeted_effect_follows_enabled_playstyle_through_shop_roll() {
    // 真实卡表：yangliu 是鱼纲效果牌，dishengxiyu 是哺乳纲效果牌。
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
    let mut saw_mammal = false;
    for _ in 0..30 {
        engine.roll_shop(&mut state, 0);
        for slot in &state.players[0].shop {
            let slot = slot.as_ref().expect("1 星效果牌非空");
            assert_ne!(slot.card_id, "yangliu", "未启用鱼纲时定向牌不得进商店");
            saw_mammal |= slot.card_id == "dishengxiyu";
        }
    }
    assert!(saw_mammal, "启用哺乳纲后对应效果牌可抽");

    let (mut engine, mut state) = setup_with(test_catalog(), with_fish);
    only_first_star(&mut engine);
    state.pool.clear();
    let (mut saw_targeted, mut saw_mammal) = (false, false);
    for _ in 0..30 {
        engine.roll_shop(&mut state, 0);
        for slot in &state.players[0].shop {
            let slot = slot.as_ref().expect("1 星效果牌非空");
            saw_targeted |= slot.card_id == "yangliu";
            saw_mammal |= slot.card_id == "dishengxiyu";
        }
    }
    assert!(saw_targeted, "启用鱼纲后定向牌可进商店");
    assert!(saw_mammal, "哺乳纲效果牌仍可抽");
}

#[test]
fn multi_play_minion_discover_uses_play_association() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "on-mammal", vec![play("mammal")]);
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
    // 真实多玩法卡 agi：play:mammal / play:machine。
    state.pool.clear();
    state.pool.insert("agi".to_string(), 20);
    state.pool.insert("on-mammal".to_string(), 20);

    let mut discover_ctx = ctx(0);
    discover_ctx.level_override = Some(6);
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
    let mut hunting_card = card("hunting-card", vec![play("mammal")]);
    hunting_card.categories.extend([
        Category {
            kind: CategoryKind::Prey,
            value: "artiodactyla".to_string(),
        },
        Category {
            kind: CategoryKind::Host,
            value: "hominidae".to_string(),
        },
    ]);
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
fn play_and_playstyle_filters_match_any_play_category() {
    let (mut engine, mut state) = setup(1);
    place(&engine.catalog, &mut state, 0, 1, "nanobot"); // play:machine, automation

    engine.apply_op(
        &mut state,
        &effect("OnActivate|Growth|friendly:all:playstyle=machine|times=1"),
        &ctx(0),
        None,
    );
    // play 与 playstyle 同口径：nanobot 带 play:machine，两键都命中。
    engine.apply_op(
        &mut state,
        &effect("OnActivate|Growth|friendly:all:play=machine|times=1"),
        &ctx(0),
        None,
    );
    assert_eq!(state.players[0].board[0].stats.attack.growth, 2);
}

#[test]
fn multi_play_minion_with_play_machine_matches_any_machine_filter() {
    // agi 同时带 play:mammal 与 play:machine；两条玩法筛选都应命中它。
    let (mut engine, mut state) = setup(1);
    place(&engine.catalog, &mut state, 0, 1, "agi");

    engine.apply_op(
        &mut state,
        &effect("OnActivate|Growth|friendly:all:playstyle=machine|times=1"),
        &ctx(0),
        None,
    );
    engine.apply_op(
        &mut state,
        &effect("OnActivate|Growth|friendly:all:playstyle=mammal|times=1"),
        &ctx(0),
        None,
    );
    // 两次成长按普通随从各 +1/+1。
    assert_eq!(state.players[0].board[0].stats.attack.growth, 2);
    assert_eq!(
        state.players[0].board[0].stats.health.growth, 2,
        "机械与哺乳纲两条筛选都应命中多玩法随从"
    );
}

#[test]
fn multi_play_minion_matches_each_play_filter_independently() {
    let (mut engine, mut state) = setup(1);
    // aolinpike-jidian 是效果牌，不能上棋盘；agi 同时带 mammal/machine。
    place(&engine.catalog, &mut state, 0, 1, "agi");
    place(&engine.catalog, &mut state, 0, 2, "clownfish"); // play:fish

    engine.apply_op(
        &mut state,
        &effect("OnActivate|Growth|friendly:all:play=machine|times=1"),
        &ctx(0),
        None,
    );
    assert_eq!(state.players[0].board[0].stats.health.growth, 1);
    assert_eq!(
        state.players[0].board[1].stats.health.growth, 0,
        "只命中带 play:machine 的随从"
    );

    engine.apply_op(
        &mut state,
        &effect("OnActivate|Growth|friendly:all:play=fish|times=1"),
        &ctx(0),
        None,
    );
    assert_eq!(state.players[0].board[1].stats.health.growth, 1);
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

    // 缺玩法的非池卡：没有 play 归属，分类直接取自身 taxon。
    engine.catalog.cards.insert(
        "plain-token".to_string(),
        Card {
            id: "plain-token".to_string(),
            card_type: CardType::Minion,
            in_shop_pool: false,
            taxon: "biology".to_string(),
            categories: Vec::new(),
            subtype: None,
            level: 1,
            era: None,
            attack: Some(1),
            health: Some(1),
            skills: Vec::new(),
            cost: None,
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

    // 缺省玩法：没有 play 归属，taxon 仍可参与筛选。
    let plain_token = engine
        .catalog
        .cards
        .get("plain-token")
        .expect("刚插入的非池卡");
    assert!(plain_token.categories.is_empty());
    assert_eq!(plain_token.taxon, "biology");
    engine.apply_op(
        &mut state,
        &effect("OnActivate|Growth|friendly:all:taxon=biology|times=1"),
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
