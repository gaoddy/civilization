//! 结构化效果的类型化 AST。
//!
//! 引擎加载的 `skills.json` 直接使用本章的结构化数组；`parse` 只保留为
//! 源文档 DSL 的作者工具，不参与加载与执行。跨字段语义由 `validate` 校验。

mod aura;
mod evolve;
mod exec;
mod parse;
mod shop_effects;
mod validate;

pub use exec::{EffectCtx, effect_coverage};
pub use parse::parse_effects;
pub use validate::validate_effects;
// 招募意图预检需要校验进化选项下标，但不触发结算。
pub(crate) use evolve::EVOLVE_CHOICES;

use serde::Deserialize;

use crate::data::{CardType, Tribe};

/// 效果触发时机。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    OnPlay,
    OnPlayEffect,
    OnUse,
    OnActivate,
    OnTurnStart,
    OnFirstRefresh,
    OnCombatStart,
    OnDealDamage,
    OnDeath,
    OnFriendlyDeath,
    OnEnemyDeath,
    OnAllySummon,
    OnAllyGrowth,
    OnAllyEvolve,
    OnCombatEnd,
    Aura,
    ConditionalAura,
}

/// 效果原语。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    Growth,
    ModifyStats,
    GrantKeyword,
    RemoveGrowth,
    CannotAttack,
    DealDamage,
    Summon,
    Copy,
    Discover,
    RefreshShop,
    AddShopSlot,
    GainMinion,
}

/// 选择器的阵营。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Friendly,
    Enemy,
    Both,
}

/// 阵营内的选取方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pick {
    Single,
    All,
    Random,
    Leftmost,
    LeftmostAdjacent,
    TopAttack,
}

/// 选择器筛选条件，写作 `key=value`。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Filter {
    pub key: String,
    pub value: String,
}

/// 效果作用对象。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Selector {
    /// `{"kind": "self"}`
    #[serde(rename = "self")]
    SelfCard,
    /// `{"kind": "target"}`
    Target,
    /// `{"kind": "side", "side": ..., "pick": ..., "filters": [...]}`
    Side {
        side: Side,
        pick: Pick,
        #[serde(default)]
        filters: Vec<Filter>,
    },
}

/// 效果持续时间；省略时按 `Instant` 处理。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Duration {
    #[default]
    Instant,
    Permanent,
    Combat,
}

/// 关键词及其层数；`繁衍` / `装配` / `传染` 可携带 `Some(N)`。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeywordSpec {
    pub keyword: String,
    #[serde(default)]
    pub value: Option<i32>,
}

/// 区域。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Zone {
    Hand,
    Board,
    Shop,
}

/// 数量条件的比较方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Compare {
    Ge,
    Le,
    Eq,
    Gt,
    Lt,
}

/// `condition` 的结构化表示。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Condition {
    pub side: Side,
    pub filter: Filter,
    pub compare: Compare,
    pub count: i32,
}

/// `timesIf` 的结构化表示。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimesIf {
    pub tribe: Tribe,
    pub times: i32,
}

/// 单段效果的参数集合。
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Params {
    pub times: Option<i32>,
    pub count: Option<i32>,
    pub amount: Option<i32>,
    pub attack: Option<i32>,
    pub health: Option<i32>,
    pub value: Option<i32>,
    pub extra: Option<i32>,
    pub duration: Duration,
    pub can_kill: Option<bool>,
    pub health_only: Option<bool>,
    pub times_if: Option<TimesIf>,
    pub condition: Option<Condition>,
    pub when: Option<Filter>,
    pub keyword: Option<KeywordSpec>,
    pub token: Option<String>,
    pub card: Option<String>,
    pub to: Option<Zone>,
    pub from: Option<Zone>,
    pub card_type: Option<CardType>,
    pub pool: Option<String>,
    pub choose: Option<u8>,
    pub free: Option<bool>,
    pub era: Option<String>,
    pub level: Option<String>,
}

/// 一段结构化效果。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Effect {
    pub trigger: Trigger,
    pub op: Op,
    pub selector: Selector,
    pub params: Params,
}
