use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use crate::effects::{Effect, validate_effects};
use crate::error::CatalogError;
use crate::taxonomy::Taxonomy;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum CardType {
    #[serde(rename = "minion")]
    Minion,
    #[serde(rename = "effect")]
    Effect,
}

/// 玩法：本局启用的 13 种物种/造物路线，外加中立的通用卡。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Playstyle {
    Mammal,
    Bird,
    Reptile,
    Fish,
    Insect,
    Cephalopod,
    Cnidarian,
    Plant,
    Fungus,
    Bacteria,
    Protozoa,
    Virus,
    Machine,
    Neutral,
}

impl Playstyle {
    /// 可被抽取的 13 个玩法，按枚举顺序排列；不含 `Neutral`。
    pub const ALL: [Playstyle; 13] = [
        Playstyle::Mammal,
        Playstyle::Bird,
        Playstyle::Reptile,
        Playstyle::Fish,
        Playstyle::Insect,
        Playstyle::Cephalopod,
        Playstyle::Cnidarian,
        Playstyle::Plant,
        Playstyle::Fungus,
        Playstyle::Bacteria,
        Playstyle::Protozoa,
        Playstyle::Virus,
        Playstyle::Machine,
    ];

    /// 数据文件与协议中的玩法 id；`Neutral` 为 `neutral`。
    pub fn id(self) -> &'static str {
        match self {
            Playstyle::Mammal => "mammal",
            Playstyle::Bird => "bird",
            Playstyle::Reptile => "reptile",
            Playstyle::Fish => "fish",
            Playstyle::Insect => "insect",
            Playstyle::Cephalopod => "cephalopod",
            Playstyle::Cnidarian => "cnidarian",
            Playstyle::Plant => "plant",
            Playstyle::Fungus => "fungus",
            Playstyle::Bacteria => "bacteria",
            Playstyle::Protozoa => "protozoa",
            Playstyle::Virus => "virus",
            Playstyle::Machine => "machine",
            Playstyle::Neutral => "neutral",
        }
    }

    /// 由玩法 id 解析；未知 id（含 `neutral`）返回 `None`。
    pub fn from_id(id: &str) -> Option<Playstyle> {
        Playstyle::ALL
            .iter()
            .copied()
            .find(|playstyle| playstyle.id() == id)
    }

    /// 状态哈希中的序号（0..13）；`Neutral` 为 13。
    pub fn ordinal(self) -> u8 {
        match self {
            Playstyle::Mammal => 0,
            Playstyle::Bird => 1,
            Playstyle::Reptile => 2,
            Playstyle::Fish => 3,
            Playstyle::Insect => 4,
            Playstyle::Cephalopod => 5,
            Playstyle::Cnidarian => 6,
            Playstyle::Plant => 7,
            Playstyle::Fungus => 8,
            Playstyle::Bacteria => 9,
            Playstyle::Protozoa => 10,
            Playstyle::Virus => 11,
            Playstyle::Machine => 12,
            Playstyle::Neutral => 13,
        }
    }
}

/// 卡牌 `specialty` 条目的类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecialtyKind {
    /// `play:<玩法 id>`：学者与中立非学者卡的本局玩法关联。
    Play,
    /// `prey:<分类节点 id>`：捕食物种。
    Prey,
    /// `host:<分类节点 id>`：寄生物种。
    Host,
}

/// 卡牌与玩法/物种的关联条目。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Specialty {
    pub kind: SpecialtyKind,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[repr(u8)]
pub enum Era {
    #[serde(rename = "prehistoric")]
    Prehistoric = 1,
    #[serde(rename = "primitive")]
    Primitive = 2,
    #[serde(rename = "classical")]
    Classical = 3,
    #[serde(rename = "feudal")]
    Feudal = 4,
    #[serde(rename = "industrial")]
    Industrial = 5,
    #[serde(rename = "modern")]
    Modern = 6,
    #[serde(rename = "future")]
    Future = 7,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Card {
    pub id: String,
    pub card_type: CardType,
    pub playstyle: Playstyle,
    pub taxon: String,
    #[serde(default)]
    pub specialty: Vec<Specialty>,
    pub subtype: Option<String>,
    pub level: u8,
    pub era: Option<Era>,
    pub attack: Option<i32>,
    pub health: Option<i32>,
    pub skills: Vec<String>,
    pub is_scholar: bool,
    pub is_hero: bool,
    pub cost: Option<u8>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Skill {
    pub id: String,
    pub keyword: Option<String>,
    pub effects: Vec<Effect>,
    pub cost: Option<u8>,
    pub uses_per_turn: Option<u8>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Token {
    pub id: String,
    pub attack: i32,
    pub health: i32,
    pub keywords: Vec<String>,
    /// 缺省按 `Neutral` 处理。
    pub playstyle: Option<Playstyle>,
    /// 参与筛选的分类节点；必填。
    pub taxon: String,
}

impl Token {
    /// 参与筛选的玩法；缺字段按 `Neutral`。
    pub fn playstyle(&self) -> Playstyle {
        self.playstyle.unwrap_or(Playstyle::Neutral)
    }
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
    pub taxonomy: Taxonomy,
}

/// 卡牌是否在本局卡池内。
///
/// - 非 `Neutral` 玩法：玩法被启用才在池中。
/// - `Neutral`：`specialty` 中没有 `play:` 时视为纯通用，始终在池中；
///   有 `play:` 时至少一个命中本局玩法才在池中。学者与中立工具卡同一口径。
pub fn card_in_pool(card: &Card, enabled: &BTreeSet<Playstyle>) -> bool {
    if card.playstyle != Playstyle::Neutral {
        return enabled.contains(&card.playstyle);
    }
    let associations = card
        .specialty
        .iter()
        .filter(|specialty| specialty.kind == SpecialtyKind::Play);
    let mut any = false;
    for specialty in associations {
        any = true;
        if Playstyle::from_id(&specialty.value)
            .is_some_and(|playstyle| enabled.contains(&playstyle))
        {
            return true;
        }
    }
    !any
}

impl Catalog {
    pub fn from_json(
        cards: &str,
        skills: &str,
        tokens: &str,
        config: &str,
        taxonomy: &str,
    ) -> Result<Catalog, CatalogError> {
        let cards: Vec<Card> =
            serde_json::from_str(cards).map_err(|err| CatalogError::File("cards.json", err))?;
        let skills: Vec<Skill> =
            serde_json::from_str(skills).map_err(|err| CatalogError::File("skills.json", err))?;
        let tokens: Vec<Token> =
            serde_json::from_str(tokens).map_err(|err| CatalogError::File("tokens.json", err))?;
        let config: GameConfig =
            serde_json::from_str(config).map_err(|err| CatalogError::File("config.json", err))?;
        let taxonomy: Taxonomy = Taxonomy::from_json(taxonomy)
            .map_err(|err| CatalogError::File("taxonomy.json", err))?;

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

        let taxonomy_errors = validate_taxonomy(&taxonomy, &card_map, &token_map);
        if !taxonomy_errors.is_empty() {
            return Err(CatalogError::Rule(taxonomy_errors.join("\n")));
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
            taxonomy,
        })
    }
}

/// 加载期校验分类关联：所有 `taxon`、`prey:`、`host:` 节点存在，且 `taxon` 是叶子。
fn validate_taxonomy(
    taxonomy: &Taxonomy,
    cards: &BTreeMap<String, Card>,
    tokens: &BTreeMap<String, Token>,
) -> Vec<String> {
    let mut errors = Vec::new();
    for card in cards.values() {
        let owner = format!("cards.json: 卡牌 {}", card.id);
        check_taxon(taxonomy, &owner, &card.taxon, &mut errors);
        for specialty in &card.specialty {
            match specialty.kind {
                SpecialtyKind::Play => {
                    if Playstyle::from_id(&specialty.value).is_none() {
                        errors.push(format!(
                            "{owner}: specialty play:{} 不是可启用的玩法",
                            specialty.value
                        ));
                    }
                }
                SpecialtyKind::Prey | SpecialtyKind::Host => {
                    if !taxonomy.contains(&specialty.value) {
                        errors.push(format!(
                            "{owner}: specialty 节点 {} 不在分类树中",
                            specialty.value
                        ));
                    }
                }
            }
        }
    }
    for token in tokens.values() {
        let owner = format!("tokens.json: Token {}", token.id);
        check_taxon(taxonomy, &owner, &token.taxon, &mut errors);
    }
    errors
}

/// 校验一个 `taxon` 存在且为叶子。
fn check_taxon(taxonomy: &Taxonomy, owner: &str, taxon: &str, errors: &mut Vec<String>) {
    if !taxonomy.contains(taxon) {
        errors.push(format!("{owner}: taxon {taxon} 不在分类树中"));
    } else if !taxonomy.is_leaf(taxon) {
        errors.push(format!("{owner}: taxon {taxon} 不是叶子节点"));
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
