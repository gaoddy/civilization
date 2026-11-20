use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::Deserialize;

use crate::effects::{Effect, validate_effects};
use crate::error::CatalogError;
use crate::keyword::KeywordSet;
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

    /// 卡牌 `play:` 位掩码中的二进制位。
    pub(crate) fn bit(self) -> u16 {
        1u16 << self.ordinal()
    }
}

/// 筛选键取值的玩法位；`neutral` 也是可被筛中的实体取值。
pub(crate) fn play_bit(value: &str) -> Option<u16> {
    if value == Playstyle::Neutral.id() {
        return Some(Playstyle::Neutral.bit());
    }
    Playstyle::from_id(value).map(Playstyle::bit)
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

/// 技能在目录内的整数标识；下标即 [`Catalog::skills`] 的位置。
///
/// 卡牌与英雄的 `skills` 字段保存该标识，触发与效果收集按整数下标取技能，
/// 不再每次按字符串查表；协议、视图与 `hero_uses` 仍以字符串技能 id 为标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SkillId(u16);

impl SkillId {
    pub(crate) fn from_index(index: usize) -> Self {
        Self(u16::try_from(index).expect("技能数超出 u16 上限"))
    }

    pub(crate) fn index(self) -> usize {
        usize::from(self.0)
    }
}

/// 筛选键在加载期预计算的卡牌事实。
///
/// `play_mask` 把 `play:` 归属压成位掩码；`prey_ancestors` / `host_ancestors`
/// 把捕食/寄生分类节点连同祖先压成整数下标，筛选时不再逐条比较字符串。
#[derive(Debug, Clone, Default)]
pub(crate) struct CardFilterFacts {
    pub play_mask: u16,
    /// 卡牌 `taxon` 的分类节点下标（加载期已校验为叶子）。
    pub taxon: Option<u16>,
    /// 捕食分类节点及其祖先的整数下标（升序、去重）。
    pub prey_ancestors: Vec<u16>,
    /// 寄生分类节点及其祖先的整数下标（升序、去重）。
    pub host_ancestors: Vec<u16>,
}

/// 由卡牌计算筛选事实；分类树缺失的节点按不命中处理。
fn compute_filter_facts(taxonomy: &Taxonomy, card: &Card) -> CardFilterFacts {
    let mut facts = CardFilterFacts {
        taxon: taxonomy.node_id(&card.taxon),
        ..CardFilterFacts::default()
    };
    for category in &card.categories {
        match category.kind {
            CategoryKind::Play => {
                facts.play_mask |= play_bit(&category.value).unwrap_or(0);
            }
            CategoryKind::Prey | CategoryKind::Host => {
                let Some(node) = taxonomy.node_id(&category.value) else {
                    continue;
                };
                let target = if category.kind == CategoryKind::Prey {
                    &mut facts.prey_ancestors
                } else {
                    &mut facts.host_ancestors
                };
                target.extend(taxonomy.ancestor_ids(node));
                target.sort_unstable();
                target.dedup();
            }
        }
    }
    facts
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

#[derive(Debug, Clone)]
pub struct Card {
    pub id: String,
    pub card_type: CardType,
    /// 是否进入商店卡池、共享牌池、三连与发现；衍生物与生成卡为 `false`。
    pub in_shop_pool: bool,
    pub taxon: String,
    pub categories: Vec<Category>,
    pub subtype: Option<String>,
    pub level: u8,
    pub era: Option<Era>,
    pub attack: Option<i32>,
    pub health: Option<i32>,
    pub skills: Vec<SkillId>,
    pub cost: Option<u8>,
}

/// 英雄卡：只保留身份与技能字段，与随从卡分属不同集合。
#[derive(Debug, Clone)]
pub struct HeroCard {
    pub id: String,
    pub categories: Vec<Category>,
    pub skills: Vec<SkillId>,
}

/// `cards.json` 的原始条目；`skills` 在技能表建立后再解析为整数标识。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCard {
    id: String,
    card_type: CardType,
    #[serde(default = "default_true")]
    in_shop_pool: bool,
    taxon: String,
    categories: Vec<Category>,
    subtype: Option<String>,
    level: u8,
    era: Option<Era>,
    attack: Option<i32>,
    health: Option<i32>,
    skills: Vec<String>,
    cost: Option<u8>,
}

/// `heroes.json` 的原始条目。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHeroCard {
    id: String,
    categories: Vec<Category>,
    skills: Vec<String>,
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

/// 加载期构建的只读目录。
///
/// `cards` / `heroes` 公开，但 `card_keywords` 位图侧表只在 [`Catalog::from_json`]
/// 时按当时的内容构建一次。加载完成后不得原地修改 `cards` 中**已存在**条目的
/// 技能或关键词；需要改动时重建 `Catalog`。加载后**新增**的卡 id 不在侧表内，
/// `base_keywords` 会回退到字符串路径，结果与加载期口径一致。
///
/// 卡牌与英雄引用技能时保存 [`SkillId`]，测试新增技能走 [`Catalog::insert_skill`]。
#[derive(Debug, Clone)]
pub struct Catalog {
    pub cards: BTreeMap<String, Card>,
    pub heroes: BTreeMap<String, HeroCard>,
    /// 技能表，下标即 [`SkillId`]。
    skills: Vec<Skill>,
    /// 技能 id 到整数标识的映射；与 `skills` 下标一一对应。
    skill_ids: HashMap<String, SkillId>,
    pub config: GameConfig,
    pub taxonomy: Taxonomy,
    /// 卡面自带关键词的位图侧表；由 `from_json` 预计算。
    ///
    /// 测试可能直接往 `cards` 插卡，因此 `base_keywords` 在侧表
    /// 缺失时回退到即时计算。
    card_keywords: BTreeMap<String, KeywordSet>,
    /// 筛选键的预计算事实；由 `from_json` 生成，测试直接插卡时回退即时计算。
    card_filter_facts: BTreeMap<String, CardFilterFacts>,
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
        let raw_cards: Vec<RawCard> =
            serde_json::from_str(cards).map_err(|err| CatalogError::File("cards.json", err))?;
        let raw_heroes: Vec<RawHeroCard> =
            serde_json::from_str(heroes).map_err(|err| CatalogError::File("heroes.json", err))?;
        let skills: Vec<Skill> =
            serde_json::from_str(skills).map_err(|err| CatalogError::File("skills.json", err))?;
        let config: GameConfig =
            serde_json::from_str(config).map_err(|err| CatalogError::File("config.json", err))?;
        let taxonomy: Taxonomy = Taxonomy::from_json(taxonomy)
            .map_err(|err| CatalogError::File("taxonomy.json", err))?;

        let raw_card_map = collect_unique(raw_cards, "cards.json", |card| &card.id)?;
        let raw_hero_map = collect_unique(raw_heroes, "heroes.json", |hero| &hero.id)?;
        let skill_map = collect_unique(skills, "skills.json", |skill| &skill.id)?;

        // 技能按下标编号；`BTreeMap` 的升序遍历让编号只由 id 排序决定。
        let skill_list: Vec<Skill> = skill_map.values().cloned().collect();
        let skill_ids: HashMap<String, SkillId> = skill_map
            .keys()
            .enumerate()
            .map(|(index, id)| (id.clone(), SkillId::from_index(index)))
            .collect();

        let card_map = raw_card_map
            .into_iter()
            .map(|(id, raw)| {
                let skills =
                    resolve_skills(&skill_ids, &raw.skills, "cards.json", &format!("卡牌 {id}"))?;
                Ok((
                    id,
                    Card {
                        id: raw.id,
                        card_type: raw.card_type,
                        in_shop_pool: raw.in_shop_pool,
                        taxon: raw.taxon,
                        categories: raw.categories,
                        subtype: raw.subtype,
                        level: raw.level,
                        era: raw.era,
                        attack: raw.attack,
                        health: raw.health,
                        skills,
                        cost: raw.cost,
                    },
                ))
            })
            .collect::<Result<BTreeMap<String, Card>, CatalogError>>()?;

        let hero_map = raw_hero_map
            .into_iter()
            .map(|(id, raw)| {
                let skills = resolve_skills(
                    &skill_ids,
                    &raw.skills,
                    "heroes.json",
                    &format!("英雄卡 {id}"),
                )?;
                Ok((
                    id,
                    HeroCard {
                        id: raw.id,
                        categories: raw.categories,
                        skills,
                    },
                ))
            })
            .collect::<Result<BTreeMap<String, HeroCard>, CatalogError>>()?;

        for hero in hero_map.values() {
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

        let card_keywords = card_map
            .iter()
            .map(|(id, card)| {
                let mut set = KeywordSet::default();
                for skill_id in &card.skills {
                    if let Some(keyword) = skill_list[skill_id.index()].keyword.as_deref() {
                        set.insert(keyword);
                    }
                }
                (id.clone(), set)
            })
            .collect();

        let card_filter_facts = card_map
            .iter()
            .map(|(id, card)| (id.clone(), compute_filter_facts(&taxonomy, card)))
            .collect();

        Ok(Catalog {
            cards: card_map,
            heroes: hero_map,
            skills: skill_list,
            skill_ids,
            config,
            taxonomy,
            card_keywords,
            card_filter_facts,
        })
    }

    /// 技能表。
    pub fn skills(&self) -> &[Skill] {
        &self.skills
    }

    /// 按整数标识取技能；标识只由本目录产生，越界属调用方缺陷。
    pub fn skill(&self, id: SkillId) -> &Skill {
        &self.skills[id.index()]
    }

    /// 整数标识对应的技能 id；与 [`Catalog::skill`] 同源。
    pub fn skill_name(&self, id: SkillId) -> &str {
        &self.skill(id).id
    }

    /// 技能 id 的整数标识；未知 id 返回 `None`。
    pub fn skill_id(&self, id: &str) -> Option<SkillId> {
        self.skill_ids.get(id).copied()
    }

    /// 按字符串技能 id 取技能；视图与意图入口使用。
    pub fn skill_by_name(&self, id: &str) -> Option<&Skill> {
        self.skill_id(id).map(|skill| self.skill(skill))
    }

    /// 新增或替换一个技能，返回其整数标识；测试构造目录时使用。
    ///
    /// 替换同名技能不改变已有 [`SkillId`]，因此已经解析过该技能的卡牌条目
    /// 会读到最后写入的内容。
    pub fn insert_skill(&mut self, skill: Skill) -> SkillId {
        if let Some(existing) = self.skill_id(&skill.id) {
            self.skills[existing.index()] = skill;
            return existing;
        }
        let id = SkillId::from_index(self.skills.len());
        self.skill_ids.insert(skill.id.clone(), id);
        self.skills.push(skill);
        id
    }

    /// 筛选键使用的卡牌事实；侧表缺失时按当前分类即时计算。
    pub(crate) fn filter_facts(
        &self,
        card_id: &str,
    ) -> Option<std::borrow::Cow<'_, CardFilterFacts>> {
        if let Some(facts) = self.card_filter_facts.get(card_id) {
            return Some(std::borrow::Cow::Borrowed(facts));
        }
        self.cards
            .get(card_id)
            .map(|card| std::borrow::Cow::Owned(compute_filter_facts(&self.taxonomy, card)))
    }

    /// 卡面自带关键词的位图集合，不含战斗内授予与光环派生。
    ///
    /// 侧表命中的卡直接克隆预计算值；侧表缺失（测试直接插卡，或加载后新增的
    /// 卡 id）时按当前 `cards` / `skills` 即时计算，保证与旧口径一致。已存在
    /// 条目在加载后被原地改动时侧表会过期，调用方不得依赖该行为。
    pub fn base_keywords(&self, card_id: &str) -> KeywordSet {
        if let Some(set) = self.card_keywords.get(card_id) {
            return set.clone();
        }
        let mut set = KeywordSet::default();
        if let Some(card) = self.cards.get(card_id) {
            for skill_id in &card.skills {
                if let Some(keyword) = self.skill(*skill_id).keyword.as_deref() {
                    set.insert(keyword);
                }
            }
        }
        set
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

/// 把技能 id 列表解析为整数标识；未知技能带上文件与归属信息报错。
fn resolve_skills(
    skill_ids: &HashMap<String, SkillId>,
    names: &[String],
    file: &'static str,
    owner: &str,
) -> Result<Vec<SkillId>, CatalogError> {
    names
        .iter()
        .map(|name| {
            skill_ids.get(name).copied().ok_or_else(|| {
                CatalogError::Rule(format!("{file}: {owner} 引用了不存在的技能 {name}"))
            })
        })
        .collect()
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
