//! 效果 DSL 的类型化 AST。
//!
//! 语法以 `scripts/validate_docs.py` 与 `scripts/audit_effects.py` 为准：
//! 一段效果写作 `<触发>|<原语>|<选择器>|<参数,...>`，多段用 `;` 分隔。
//! 解析在加载期完成，未知触发、原语、选择器、筛选或参数都会直接报错。

mod parse;

pub use parse::parse_effects;

use crate::data::CardType;

/// 效果触发时机。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    Friendly,
    Enemy,
    Both,
}

/// 阵营内的选取方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pick {
    Single,
    All,
    Random,
    Leftmost,
    LeftmostAdjacent,
    TopAttack,
}

/// 选择器筛选条件，写作 `key=value`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter {
    pub key: String,
    pub value: String,
}

/// 效果作用对象。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selector {
    /// `self`
    SelfCard,
    /// `target`
    Target,
    /// `<friendly|enemy|both>:<pick>[:<filter>...]`
    Side {
        side: Side,
        pick: Pick,
        filters: Vec<Filter>,
    },
}

/// 效果持续时间；省略时按 `Instant` 处理。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Duration {
    #[default]
    Instant,
    Permanent,
    Combat,
}

/// 关键词及其层数；`繁衍` / `装配` 可携带 `Some(N)`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeywordSpec {
    Keyword(String, Option<i32>),
}

/// 区域。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Zone {
    Hand,
    Board,
    Shop,
}

/// 数量条件的比较方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Compare {
    Ge,
    Le,
    Eq,
    Gt,
    Lt,
}

/// `condition=count:<side>:<filter>:<compare><n>` 的结构化表示。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Condition {
    pub side: Side,
    pub filter: Filter,
    pub compare: Compare,
    pub count: i32,
}

/// 单段效果的参数集合。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
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
    pub times_if: Option<(String, i32)>,
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

/// 一段解析完成的效果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Effect {
    pub trigger: Trigger,
    pub op: Op,
    pub selector: Selector,
    pub params: Params,
}
