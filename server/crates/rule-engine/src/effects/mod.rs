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
pub(crate) use exec::{target_in_other_zone, target_in_own_zone};

use serde::Deserialize;

use crate::data::CardType;
use crate::state::InstanceId;

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
    OnKill,
    OnAllyKill,
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
    GrantMinion,
    Decompose,
    Mimic,
    Devour,
    Detonate,
    Convert,
    Revive,
    Recall,
    Salvage,
    Latent,
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
    /// 攻击力最低者；仅用于 `DealDamage` 的敌方选择器。
    LowestAttack,
    /// 携带者左右的存活随从；仅用于光环。
    Adjacent,
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
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Selector {
    /// `{"kind": "self"}`
    #[serde(rename = "self")]
    SelfCard,
    /// `{"kind": "target"}` 或 `{"kind": "target", "zone": ..., "filters": [...]}`
    Target {
        /// 目标所在区域；省略时为棋盘，保持旧数据的语义。
        #[serde(default)]
        zone: Zone,
        #[serde(default)]
        filters: Vec<Filter>,
    },
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
    /// 光环折叠写入的派生关键词；DSL 无法指定，只用于区分来源，
    /// 使拟态等「不复制光环」的效果能够剔除它们。
    Aura,
}

/// 关键词及其层数；`繁衍` / `装配` / `传染` 可携带 `Some(N)`。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeywordSpec {
    pub keyword: String,
    #[serde(default)]
    pub value: Option<i32>,
}

/// `Copy` 的复制口径。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CopyState {
    /// 按卡面基础值生成一张新牌，不继承来源状态。
    Base,
    /// 完整继承来源实例的当前状态。
    Current,
}

/// `Decompose` 取生命值的口径。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Basis {
    /// 卡面基础生命值；金色来源 ×2。
    Original,
    /// 进入战斗快照时的当前生命值。
    CombatStart,
    /// 承受致命伤害前的当前生命值。
    PreKill,
}

/// 区域。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Zone {
    Hand,
    #[default]
    Board,
    Shop,
}

/// 目标引用：区域 + 实例编号。
///
/// 手牌、商店与棋盘的实例编号由同一个 `next_instance_id` 分配，全局唯一；
/// 区域用于判定该编号属于哪个区域，以及意图提交时的本人区域校验。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TargetRef {
    pub zone: Zone,
    pub id: InstanceId,
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

/// `timesIf` 的结构化表示：`<键>:<值>:<次数>`，`scholar` 没有值。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimesIf {
    /// 筛选键，取值集合与 [`Filter`] 一致。
    pub key: String,
    /// 筛选值；`scholar` 等无值键为 `None`。
    pub value: Option<String>,
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
    /// 群聚：每个其他同玩法随从额外攻击力。
    pub per_attack: Option<i32>,
    /// 群聚：每个其他同玩法随从额外生命值。
    pub per_health: Option<i32>,
    /// 群聚：最多计算的随从数。
    pub max_count: Option<i32>,
    /// 引爆 / 转化 / 潜伏的层数阈值。
    pub threshold: Option<i32>,
    pub value: Option<i32>,
    pub extra: Option<i32>,
    pub duration: Option<Duration>,
    pub can_kill: Option<bool>,
    pub health_only: Option<bool>,
    /// 拟态只复制攻防，不复制关键词。
    pub stats_only: Option<bool>,
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
    /// `Copy` 的复制口径：缺省按卡面复制原始牌，`current` 为完全复制当前状态。
    pub state: Option<CopyState>,
    /// `Decompose` 的生命值口径。
    pub basis: Option<Basis>,
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
