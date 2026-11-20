#![allow(dead_code)]

use std::collections::BTreeMap;

use rule_engine::combat::{CombatAction, CombatLog};
use rule_engine::data::{CardType, Catalog, Category, CategoryKind, Playstyle};
use rule_engine::effects::{Effect, parse_effects};
use rule_engine::engine::Engine;
use rule_engine::state::{GameState, InstanceId, MinionInstance, Phase, Seat, ShopSlot};

pub const CARDS_JSON: &str = include_str!("../../../../data/v0.1/cards.json");
pub const HEROES_JSON: &str = include_str!("../../../../data/v0.1/heroes.json");
pub const SKILLS_JSON: &str = include_str!("../../../../data/v0.1/skills.json");
pub const CONFIG_JSON: &str = include_str!("../../../../data/v0.1/config.json");
pub const TAXONOMY_JSON: &str = include_str!("../../../../data/v0.1/taxonomy.json");

/// 加载仓库中已提交的 v0.1 数据，供各集成测试复用。
pub fn test_catalog() -> Catalog {
    Catalog::from_json(
        CARDS_JSON,
        HEROES_JSON,
        SKILLS_JSON,
        CONFIG_JSON,
        TAXONOMY_JSON,
    )
    .expect("已提交的 v0.1 数据应能完整加载")
}

/// 测试用固定玩法：由种子 7 抽出，保证各测试开局一致。
pub fn test_playstyles() -> [Playstyle; 5] {
    rule_engine::playstyles::draw(7)
}

/// 按单值玩法构造一条 `play:` 关联；`Neutral` 返回空列表（新数据不再允许）。
pub fn categories_for(playstyle: Playstyle) -> Vec<Category> {
    if playstyle == Playstyle::Neutral {
        Vec::new()
    } else {
        vec![Category {
            kind: CategoryKind::Play,
            value: playstyle.id().to_string(),
        }]
    }
}

/// 卡牌是否带指定玩法归属；供测试断言多玩法。
pub fn card_has_play(card: &rule_engine::data::Card, playstyle: &str) -> bool {
    card.categories
        .iter()
        .any(|category| category.kind == CategoryKind::Play && category.value == playstyle)
}

/// 按卡牌 id 顺序取前 8 张英雄卡，映射到座位 0..8，供开局测试复用。
pub fn heroes() -> BTreeMap<Seat, String> {
    test_catalog()
        .heroes
        .values()
        .take(8)
        .enumerate()
        .map(|(index, hero)| (index as Seat, hero.id.clone()))
        .collect()
}

/// 进入招募阶段的空局：每个座位 30 点生命。
pub fn setup(seed: u8) -> (Engine, GameState) {
    let (engine, mut state) = Engine::new(test_catalog(), [seed; 32], test_playstyles());
    state.phase = Phase::Recruit;
    state.turn = 1;
    for player in &mut state.players {
        player.hp = 30;
    }
    (engine, state)
}

/// 进入招募阶段的空局，并给每个座位 10 点金币。
pub fn setup_with_gold(seed: u8) -> (Engine, GameState) {
    let (engine, mut state) = Engine::new(test_catalog(), [seed; 32], test_playstyles());
    state.phase = Phase::Recruit;
    state.turn = 1;
    for player in &mut state.players {
        player.hp = 30;
        player.gold = 10;
    }
    (engine, state)
}

/// 用前 8 张英雄卡开局后的状态。
pub fn started(seed: u8) -> (Engine, GameState) {
    let (mut engine, mut state) = Engine::new(test_catalog(), [seed; 32], test_playstyles());
    engine
        .start_game(&mut state, &heroes())
        .expect("开局应成功");
    (engine, state)
}

/// 开局并进入第一回合招募。
pub fn turn_started(seed: u8) -> (Engine, GameState) {
    let (mut engine, mut state) = started(seed);
    engine.start_turn(&mut state).expect("开始回合应成功");
    (engine, state)
}

/// 按卡牌 id 构造一个非金色随从实例。
pub fn minion(catalog: &Catalog, instance_id: InstanceId, card_id: &str) -> MinionInstance {
    MinionInstance::new(catalog, instance_id, card_id, false)
}

/// 把随从放到指定座位的棋盘末尾。
pub fn place(catalog: &Catalog, state: &mut GameState, seat: Seat, id: InstanceId, card_id: &str) {
    state.players[usize::from(seat)]
        .board
        .push(minion(catalog, id, card_id));
}

/// 用给定卡牌 id 覆盖某座位的商店槽位；`None` 保留空槽。
pub fn set_shop(state: &mut GameState, seat: Seat, ids: &[Option<String>]) {
    let len = state.players[usize::from(seat)].shop.len().max(ids.len());
    let mut shop: Vec<Option<ShopSlot>> = ids
        .iter()
        .cloned()
        .map(|card_id| {
            card_id.map(|card_id| {
                let instance_id = state.next_instance_id;
                state.next_instance_id = instance_id.saturating_add(1);
                ShopSlot {
                    instance_id,
                    card_id,
                    frozen: false,
                }
            })
        })
        .collect();
    while shop.len() < len {
        shop.push(None);
    }
    state.players[usize::from(seat)].shop = shop;
}

/// 取某星级的第一张随从卡 id。
pub fn minion_id(level: u8) -> String {
    test_catalog()
        .cards
        .values()
        .find(|card| card.card_type == CardType::Minion && card.in_shop_pool && card.level == level)
        .expect("数据中应存在该星级随从")
        .id
        .clone()
}

/// 测试夹具用 DSL 构造结构化效果；加载链路本身不再解析 DSL。
pub fn effects_of(raw: &str) -> Vec<Effect> {
    parse_effects(raw).expect("测试效果应可解析")
}

/// 战斗日志中的复生次数。
pub fn reborn_count(log: &CombatLog) -> usize {
    log.actions
        .iter()
        .filter(|action| matches!(action, CombatAction::Reborn { .. }))
        .count()
}

/// 构造一张非池随从卡（衍生物/生成卡），供测试登记到目录。
pub fn token_card(id: &str, attack: i32, health: i32) -> rule_engine::data::Card {
    rule_engine::data::Card {
        id: id.to_string(),
        card_type: CardType::Minion,
        in_shop_pool: false,
        taxon: "biology".to_string(),
        categories: Vec::new(),
        subtype: None,
        level: 1,
        era: None,
        attack: Some(attack),
        health: Some(health),
        skills: Vec::new(),
        is_scholar: false,
        cost: None,
    }
}
