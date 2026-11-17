use std::collections::BTreeMap;

use serde::Deserialize;

use crate::effects::{Effect, validate_effects};
use crate::error::CatalogError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum CardType {
    #[serde(rename = "随从")]
    Minion,
    #[serde(rename = "效果")]
    Effect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub enum Tribe {
    #[serde(rename = "人类")]
    Human,
    #[serde(rename = "机械")]
    Machine,
    #[serde(rename = "生物")]
    Biology,
    #[serde(rename = "植物")]
    Plant,
    #[serde(rename = "病毒")]
    Virus,
    #[serde(rename = "学者")]
    Scholar,
    #[serde(rename = "通用")]
    General,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[repr(u8)]
pub enum Era {
    #[serde(rename = "史前")]
    Prehistoric = 1,
    #[serde(rename = "原始")]
    Primitive = 2,
    #[serde(rename = "古典")]
    Classical = 3,
    #[serde(rename = "封建")]
    Feudal = 4,
    #[serde(rename = "工业")]
    Industrial = 5,
    #[serde(rename = "现代")]
    Modern = 6,
    #[serde(rename = "未来")]
    Future = 7,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Card {
    pub id: String,
    pub card_type: CardType,
    pub name: String,
    pub tribe: Tribe,
    pub subtype: Option<String>,
    pub level: u8,
    pub era: Option<Era>,
    pub attack: Option<i32>,
    pub health: Option<i32>,
    pub skills: Vec<String>,
    pub is_scholar: bool,
    pub is_hero: bool,
    pub cost: Option<u8>,
    pub match_text: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Skill {
    pub id: String,
    pub name: Option<String>,
    pub keyword: Option<String>,
    pub text: String,
    pub effects: Vec<Effect>,
    pub cost: Option<u8>,
    pub uses_per_turn: Option<u8>,
    pub match_text: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Token {
    pub id: String,
    pub name: String,
    pub attack: i32,
    pub health: i32,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameConfig {
    pub initial_hp: i32,
    pub board_slots: usize,
    pub hand_slots: usize,
    pub shop_slots: usize,
    pub turn_limit: u8,
    pub damage_cap: i32,
    pub damage_floor: i32,
    pub era_costs: [u16; 6],
    pub pool_counts: [u16; 7],
    pub shop_weights: [[u16; 7]; 7],
    pub gold_curve: [u8; 20],
}

#[derive(Debug, Clone)]
pub struct Catalog {
    pub cards: BTreeMap<String, Card>,
    pub skills: BTreeMap<String, Skill>,
    pub tokens: BTreeMap<String, Token>,
    pub config: GameConfig,
}

impl Catalog {
    pub fn from_json(
        cards: &str,
        skills: &str,
        tokens: &str,
        config: &str,
    ) -> Result<Catalog, CatalogError> {
        let cards: Vec<Card> =
            serde_json::from_str(cards).map_err(|err| CatalogError::File("cards.json", err))?;
        let skills: Vec<Skill> =
            serde_json::from_str(skills).map_err(|err| CatalogError::File("skills.json", err))?;
        let tokens: Vec<Token> =
            serde_json::from_str(tokens).map_err(|err| CatalogError::File("tokens.json", err))?;
        let config: GameConfig =
            serde_json::from_str(config).map_err(|err| CatalogError::File("config.json", err))?;

        let card_map = collect_unique(cards, "cards.json", |card| &card.id)?;
        let skill_map = collect_unique(skills, "skills.json", |skill| &skill.id)?;
        let token_map = collect_unique(tokens, "tokens.json", |token| &token.id)?;

        for card in card_map.values() {
            for skill in &card.skills {
                if !skill_map.contains_key(skill) {
                    return Err(CatalogError::Rule(format!(
                        "cards.json: 卡牌 {} 引用了不存在的技能 {}",
                        card.id, skill
                    )));
                }
            }
        }

        let mut effect_errors = Vec::new();
        for skill in skill_map.values() {
            if let Err(errors) = validate_effects(&skill.effects) {
                for error in errors {
                    effect_errors.push(format!(
                        "skills.json: 技能 {} 的 effects 非法: {error}",
                        skill.id
                    ));
                }
            }
        }
        if !effect_errors.is_empty() {
            return Err(CatalogError::Rule(effect_errors.join("\n")));
        }

        Ok(Catalog {
            cards: card_map,
            skills: skill_map,
            tokens: token_map,
            config,
        })
    }
}

fn collect_unique<T>(
    items: Vec<T>,
    file: &'static str,
    id: impl Fn(&T) -> &String,
) -> Result<BTreeMap<String, T>, CatalogError> {
    let mut map = BTreeMap::new();
    for item in items {
        let key = id(&item).clone();
        if map.contains_key(&key) {
            return Err(CatalogError::Rule(format!("{file}: 重复的 id: {key}")));
        }
        map.insert(key, item);
    }
    Ok(map)
}
