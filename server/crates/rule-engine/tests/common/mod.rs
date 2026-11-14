#![allow(dead_code)]

use std::collections::BTreeMap;

use rule_engine::data::Catalog;
use rule_engine::state::Seat;

pub const CARDS_JSON: &str = include_str!("../../../../data/v0.1/cards.json");
pub const SKILLS_JSON: &str = include_str!("../../../../data/v0.1/skills.json");
pub const TOKENS_JSON: &str = include_str!("../../../../data/v0.1/tokens.json");
pub const CONFIG_JSON: &str = include_str!("../../../../data/v0.1/config.json");

/// 加载仓库中已提交的 v0.1 数据，供各集成测试复用。
pub fn test_catalog() -> Catalog {
    Catalog::from_json(CARDS_JSON, SKILLS_JSON, TOKENS_JSON, CONFIG_JSON)
        .expect("已提交的 v0.1 数据应能完整加载")
}

/// 按卡牌 id 顺序取前 8 张英雄卡，映射到座位 0..8，供开局测试复用。
pub fn heroes() -> BTreeMap<Seat, String> {
    test_catalog()
        .cards
        .values()
        .filter(|card| card.is_hero)
        .take(8)
        .enumerate()
        .map(|(index, card)| (index as Seat, card.id.clone()))
        .collect()
}
