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

/// 卡牌 `categories` 条目的类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CategoryKind {
    /// `play:<玩法 id>`：本局玩法归属；一张卡可有多条。
    Play,
    /// `prey:<分类节点 id>`：捕食物种。
    Prey,
    /// `host:<分类节点 id>`：寄生物种。
    Host,
}

/// 卡牌与玩法/物种的关联条目。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Category {
    pub kind: CategoryKind,
    pub value: String,
}

/// 迭代全部 `play:` 条目的取值。
pub(crate) fn play_categories(categories: &[Category]) -> impl Iterator<Item = &str> {
    categories
        .iter()
        .filter(|category| category.kind == CategoryKind::Play)
        .map(|category| category.value.as_str())
}

/// 卡牌是否带指定玩法归属。
pub(crate) fn has_play(categories: &[Category], playstyle: &str) -> bool {
    play_categories(categories).any(|value| value == playstyle)
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

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Card {
    pub id: String,
    pub card_type: CardType,
    /// 是否进入商店卡池、共享牌池、三连与发现；衍生物与生成卡为 `false`。
    #[serde(default = "default_true")]
    pub in_shop_pool: bool,
    pub taxon: String,
    pub categories: Vec<Category>,
    pub subtype: Option<String>,
    pub level: u8,
    pub era: Option<Era>,
    pub attack: Option<i32>,
    pub health: Option<i32>,
    pub skills: Vec<String>,
    pub cost: Option<u8>,
}

/// 英雄卡：只保留身份与技能字段，与随从卡分属不同集合。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeroCard {
    pub id: String,
    pub categories: Vec<Category>,
    pub skills: Vec<String>,
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
    pub heroes: BTreeMap<String, HeroCard>,
    pub skills: BTreeMap<String, Skill>,
    pub config: GameConfig,
    pub taxonomy: Taxonomy,
}

/// 卡牌是否在本局卡池内。
///
/// 任一条 `play:` 命中启用玩法即入池；没有 `play:` 时不入池
/// （加载期已拒绝无玩法归属的卡）。
pub fn card_in_pool(card: &Card, enabled: &BTreeSet<Playstyle>) -> bool {
    in_pool(&card.categories, enabled)
}

/// 英雄卡是否在本局英雄候选池内；口径与 [`card_in_pool`] 一致。
pub fn hero_in_pool(hero: &HeroCard, enabled: &BTreeSet<Playstyle>) -> bool {
    in_pool(&hero.categories, enabled)
}

fn in_pool(categories: &[Category], enabled: &BTreeSet<Playstyle>) -> bool {
    play_categories(categories)
        .filter_map(Playstyle::from_id)
        .any(|playstyle| enabled.contains(&playstyle))
}

impl Catalog {
    pub fn from_json(
        cards: &str,
        heroes: &str,
        skills: &str,
        config: &str,
        taxonomy: &str,
    ) -> Result<Catalog, CatalogError> {
        let cards: Vec<Card> =
            serde_json::from_str(cards).map_err(|err| CatalogError::File("cards.json", err))?;
        let heroes: Vec<HeroCard> =
            serde_json::from_str(heroes).map_err(|err| CatalogError::File("heroes.json", err))?;
        let skills: Vec<Skill> =
            serde_json::from_str(skills).map_err(|err| CatalogError::File("skills.json", err))?;
        let config: GameConfig =
            serde_json::from_str(config).map_err(|err| CatalogError::File("config.json", err))?;
        let taxonomy: Taxonomy = Taxonomy::from_json(taxonomy)
            .map_err(|err| CatalogError::File("taxonomy.json", err))?;

        let card_map = collect_unique(cards, "cards.json", |card| &card.id)?;
        let hero_map = collect_unique(heroes, "heroes.json", |hero| &hero.id)?;
        let skill_map = collect_unique(skills, "skills.json", |skill| &skill.id)?;

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

        for hero in hero_map.values() {
            for skill in &hero.skills {
                if !skill_map.contains_key(skill) {
                    return Err(CatalogError::Rule(format!(
                        "heroes.json: 英雄卡 {} 引用了不存在的技能 {}",
                        hero.id, skill
                    )));
                }
            }
            // 两池同名同 id，但必须各自从自己的池取数；关联分类不一致会让
            // 仅按用途选池的查询产生分叉，因此在加载期直接拒绝。技能各自维护，
            // 英雄技能与同名随从卡的技能不要求一致。
            let Some(card) = card_map.get(&hero.id) else {
                return Err(CatalogError::Rule(format!(
                    "heroes.json: 英雄卡 {} 缺少同 id 的随从卡",
                    hero.id
                )));
            };
            if card.card_type != CardType::Minion {
                return Err(CatalogError::Rule(format!(
                    "heroes.json: 英雄卡 {} 的同 id 卡不是随从卡",
                    hero.id
                )));
            }
            if card.categories != hero.categories {
                return Err(CatalogError::Rule(format!(
                    "heroes.json: 英雄卡 {} 与同名随从卡的关联分类不一致",
                    hero.id
                )));
            }
        }

        let taxonomy_errors = validate_taxonomy(&taxonomy, &card_map);
        if !taxonomy_errors.is_empty() {
            return Err(CatalogError::Rule(taxonomy_errors.join("\n")));
        }
        let hero_errors = validate_hero_categories(&taxonomy, &hero_map);
        if !hero_errors.is_empty() {
            return Err(CatalogError::Rule(hero_errors.join("\n")));
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
            heroes: hero_map,
            skills: skill_map,
            config,
            taxonomy,
        })
    }
}

/// 加载期校验分类关联：所有 `taxon`、`prey:`、`host:` 节点存在，且 `taxon` 是叶子。
fn validate_taxonomy(taxonomy: &Taxonomy, cards: &BTreeMap<String, Card>) -> Vec<String> {
    let mut errors = Vec::new();
    for card in cards.values() {
        let owner = format!("cards.json: 卡牌 {}", card.id);
        check_taxon(taxonomy, &owner, &card.taxon, &mut errors);
        validate_categories(
            taxonomy,
            &owner,
            &card.categories,
            Some(&card.taxon),
            card.in_shop_pool,
            &mut errors,
        );
    }
    errors
}

/// 校验英雄卡的关联分类：至少 1 条 `play:`，`play:` 必须是合法玩法，其余节点必须在分类树中。
///
/// 英雄卡不持有 `taxon`；`prey:` / `host:` 的生物界要求由同 id 随从卡承担。
fn validate_hero_categories(
    taxonomy: &Taxonomy,
    heroes: &BTreeMap<String, HeroCard>,
) -> Vec<String> {
    let mut errors = Vec::new();
    for hero in heroes.values() {
        let owner = format!("heroes.json: 英雄卡 {}", hero.id);
        validate_categories(taxonomy, &owner, &hero.categories, None, true, &mut errors);
    }
    errors
}

/// 生物界的六个根节点；`prey:` / `host:` 只允许落在这些根下的卡牌。
const BIOLOGICAL_KINGDOMS: [&str; 6] = [
    "animalia", "plantae", "fungi", "bacteria", "protozoa", "virus",
];

/// 校验一份关联分类：玩法合法、节点存在、无重复；`require_play` 时至少 1 条 `play:`。
///
/// `taxon` 为 `Some` 时额外要求 `prey:` / `host:` 落在生物界卡牌上。
/// 不进商店卡池的卡不参与玩法筛选，因此不要求 `play:`。
fn validate_categories(
    taxonomy: &Taxonomy,
    owner: &str,
    categories: &[Category],
    taxon: Option<&str>,
    require_play: bool,
    errors: &mut Vec<String>,
) {
    let biological = taxon.is_some_and(|taxon| is_biological(taxonomy, taxon));
    let mut seen = BTreeSet::new();
    let mut plays = 0usize;
    for category in categories {
        let kind = match category.kind {
            CategoryKind::Play => "play",
            CategoryKind::Prey => "prey",
            CategoryKind::Host => "host",
        };
        let item = format!("{kind}:{}", category.value);
        if !seen.insert((category.kind, category.value.as_str())) {
            errors.push(format!("{owner}: 关联分类条目重复 {item}"));
        }
        match category.kind {
            CategoryKind::Play => {
                plays += 1;
                if Playstyle::from_id(&category.value).is_none() {
                    errors.push(format!(
                        "{owner}: play:{} 不是可启用的玩法（不含 neutral）",
                        category.value
                    ));
                }
            }
            CategoryKind::Prey | CategoryKind::Host => {
                if !taxonomy.contains(&category.value) {
                    errors.push(format!(
                        "{owner}: 关联分类节点 {} 不在分类树中",
                        category.value
                    ));
                }
                if taxon.is_some() && !biological {
                    errors.push(format!(
                        "{owner}: {item} 只允许生物卡（taxon 根节点必须是生物界）"
                    ));
                }
            }
        }
    }
    if require_play && plays == 0 {
        errors.push(format!("{owner}: 关联分类至少要有 1 条 play:"));
    }
}

/// 分类节点是否属于生物界。
fn is_biological(taxonomy: &Taxonomy, taxon: &str) -> bool {
    taxonomy
        .path(taxon)
        .last()
        .is_some_and(|root| BIOLOGICAL_KINGDOMS.contains(root))
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
