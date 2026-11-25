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
pub(crate) use exec::{own_target_refs, target_in_other_zone, target_in_own_zone};

use serde::{Deserialize, Serialize};

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
    SevenVoyages,
}

/// 选择器的阵营。
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
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

/// 选择器筛选键；数据与协议中的字符串在加载期解析成该枚举。
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterKey {
    Taxon,
    Playstyle,
    Play,
    Prey,
    Host,
    Subtype,
    Id,
    #[serde(rename = "type")]
    Type,
    Keyword,
    Other,
}

impl FilterKey {
    /// 数据与协议中的字符串形式。
    pub fn id(self) -> &'static str {
        match self {
            FilterKey::Taxon => "taxon",
            FilterKey::Playstyle => "playstyle",
            FilterKey::Play => "play",
            FilterKey::Prey => "prey",
            FilterKey::Host => "host",
            FilterKey::Subtype => "subtype",
            FilterKey::Id => "id",
            FilterKey::Type => "type",
            FilterKey::Keyword => "keyword",
            FilterKey::Other => "other",
        }
    }

    /// 由字符串解析；未知筛选键返回 `None`。
    pub fn from_id(id: &str) -> Option<FilterKey> {
        [
            FilterKey::Taxon,
            FilterKey::Playstyle,
            FilterKey::Play,
            FilterKey::Prey,
            FilterKey::Host,
            FilterKey::Subtype,
            FilterKey::Id,
            FilterKey::Type,
            FilterKey::Keyword,
            FilterKey::Other,
        ]
        .into_iter()
        .find(|key| key.id() == id)
    }
}

/// 选择器筛选条件，写作 `key=value`。
#[derive(Serialize, Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Filter {
    pub key: FilterKey,
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
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
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
#[derive(Serialize, Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeywordSpec {
    pub keyword: String,
    #[serde(default)]
    pub value: Option<i32>,
}

/// `Copy` 的复制口径。
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CopyState {
    /// 按卡面基础值生成一张新牌，不继承来源状态。
    Base,
    /// 完整继承来源实例的当前状态。
    Current,
}

/// `Decompose` 取生命值的口径。
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
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
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
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
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Compare {
    Ge,
    Le,
    Eq,
    Gt,
    Lt,
}

/// `condition` 的结构化表示。
#[derive(Serialize, Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Condition {
    pub side: Side,
    pub filter: Filter,
    pub compare: Compare,
    pub count: i32,
}

/// `timesIf` 的结构化表示：`<键>:<值>:<次数>`。
#[derive(Serialize, Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimesIf {
    /// 筛选键，取值集合与 [`Filter`] 一致。
    pub key: FilterKey,
    /// 筛选值；语法要求必须带值。
    pub value: String,
    pub times: i32,
}

/// `Growth` 的参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrowthParams {
    pub times: i32,
    pub health_only: Option<bool>,
    pub times_if: Option<TimesIf>,
    pub condition: Option<Condition>,
    pub when: Option<Filter>,
}

/// `ModifyStats` 的参数；群聚字段只用于光环。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModifyStatsParams {
    pub attack: Option<i32>,
    pub health: Option<i32>,
    /// 群聚：每个其他同玩法随从额外攻击力。
    pub per_attack: Option<i32>,
    /// 群聚：每个其他同玩法随从额外生命值。
    pub per_health: Option<i32>,
    /// 群聚：最多计算的随从数。
    pub max_count: Option<i32>,
    pub duration: Option<Duration>,
    pub can_kill: Option<bool>,
    pub condition: Option<Condition>,
    pub when: Option<Filter>,
}

/// `GrantKeyword` 的参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantKeywordParams {
    pub keyword: KeywordSpec,
    /// 层数的旧写法；与 `keyword.value` 二选一，执行器优先读 `keyword.value`。
    pub value: Option<i32>,
    pub duration: Option<Duration>,
    pub condition: Option<Condition>,
    pub when: Option<Filter>,
}

/// `RemoveGrowth` 的参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoveGrowthParams {
    pub condition: Option<Condition>,
    pub when: Option<Filter>,
}

/// `CannotAttack` 的参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CannotAttackParams {
    pub duration: Option<Duration>,
}

/// `DealDamage` 的参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DealDamageParams {
    pub amount: i32,
    pub can_kill: Option<bool>,
    pub when: Option<Filter>,
}

/// `Summon` 的参数；`token` 与 `card` 只能二选一。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SummonParams {
    pub count: i32,
    pub token: Option<String>,
    pub card: Option<String>,
    pub when: Option<Filter>,
}

/// `Copy` 的参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CopyParams {
    pub to: Zone,
    pub from: Option<Zone>,
    pub card_type: Option<CardType>,
    /// 复制口径：缺省按卡面复制原始牌，`current` 为完全复制当前状态。
    pub state: Option<CopyState>,
}

/// `Discover` 的参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoverParams {
    pub pool: String,
    pub choose: u8,
}

/// `RefreshShop` 的参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefreshShopParams {
    pub free: Option<bool>,
}

/// `AddShopSlot` 的参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddShopSlotParams {
    pub era: String,
    pub count: i32,
}

/// `GainMinion` 的参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GainMinionParams {
    pub level: String,
    pub to: Zone,
    pub from: Option<Zone>,
}

/// `GrantMinion` 的参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantMinionParams {
    pub level: String,
    pub to: Zone,
}

/// `Decompose` 的参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecomposeParams {
    pub basis: Basis,
    pub duration: Duration,
    pub condition: Option<Condition>,
    pub when: Option<Filter>,
}

/// `Mimic` 的参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MimicParams {
    pub duration: Duration,
    /// 拟态只复制攻防，不复制关键词。
    pub stats_only: Option<bool>,
}

/// `Devour` 的参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DevourParams {
    pub value: i32,
    pub duration: Option<Duration>,
    pub when: Option<Filter>,
}

/// `Detonate` 的参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetonateParams {
    pub amount: i32,
    pub threshold: Option<i32>,
    pub when: Option<Filter>,
}

/// `Convert` 的参数；`token` 与 `card` 只能二选一。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConvertParams {
    pub threshold: i32,
    pub token: Option<String>,
    pub card: Option<String>,
    pub count: Option<i32>,
    pub when: Option<Filter>,
}

/// `Revive` 的参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviveParams {
    pub attack: i32,
    pub health: i32,
}

/// `Recall` 的参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecallParams {
    pub to: Zone,
}

/// `Salvage` 的参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SalvageParams {
    pub pool: String,
    pub count: i32,
}

/// `Latent` 的参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LatentParams {
    pub amount: i32,
    pub count: i32,
    pub threshold: Option<i32>,
    pub when: Option<Filter>,
}

/// `SevenVoyages` 的参数。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SevenVoyagesParams {
    /// 航程回合数上限；实际回合数在 1..=turns_max 内随机。
    pub turns_max: u8,
}

/// 单段效果的参数；每个原语只携带自己需要的字段。
///
/// 数据文件仍用 `"params": { ... }` 的扁平对象，加载时按 `op` 解码为对应变体，
/// 未知字段与缺失的必填字段在加载期即被拒绝。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Params {
    Growth(GrowthParams),
    ModifyStats(ModifyStatsParams),
    GrantKeyword(GrantKeywordParams),
    RemoveGrowth(RemoveGrowthParams),
    CannotAttack(CannotAttackParams),
    DealDamage(DealDamageParams),
    Summon(SummonParams),
    Copy(CopyParams),
    Discover(DiscoverParams),
    RefreshShop(RefreshShopParams),
    AddShopSlot(AddShopSlotParams),
    GainMinion(GainMinionParams),
    GrantMinion(GrantMinionParams),
    Decompose(DecomposeParams),
    Mimic(MimicParams),
    Devour(DevourParams),
    Detonate(DetonateParams),
    Convert(ConvertParams),
    Revive(ReviveParams),
    Recall(RecallParams),
    Salvage(SalvageParams),
    Latent(LatentParams),
    SevenVoyages(SevenVoyagesParams),
}

impl Params {
    /// 金色效果的数值产出翻倍：只翻倍实际产出的数值，条件与阈值原样保留。
    ///
    /// 金色按「单次结算 + 产出 ×2」处理，不再连续执行两遍。字段选择：
    /// 成长次数、攻防加成、伤害、召唤/获得数量、关键词层数与复活攻防 ×2；
    /// `condition`、`threshold`、`value`、`max_count`、`when` 等条件/上限不变。
    pub fn doubled_outputs(&self) -> Params {
        fn double(value: i32) -> i32 {
            value.saturating_mul(2)
        }
        match self {
            Params::Growth(p) => Params::Growth(GrowthParams {
                times: double(p.times),
                times_if: p.times_if.as_ref().map(|times_if| TimesIf {
                    times: double(times_if.times),
                    ..times_if.clone()
                }),
                ..p.clone()
            }),
            Params::ModifyStats(p) => Params::ModifyStats(ModifyStatsParams {
                attack: p.attack.map(double),
                health: p.health.map(double),
                per_attack: p.per_attack.map(double),
                per_health: p.per_health.map(double),
                ..p.clone()
            }),
            Params::GrantKeyword(p) => Params::GrantKeyword(GrantKeywordParams {
                keyword: KeywordSpec {
                    keyword: p.keyword.keyword.clone(),
                    value: p.keyword.value.map(double),
                },
                value: p.value.map(double),
                ..p.clone()
            }),
            Params::DealDamage(p) => Params::DealDamage(DealDamageParams {
                amount: double(p.amount),
                ..p.clone()
            }),
            Params::Summon(p) => Params::Summon(SummonParams {
                count: double(p.count),
                ..p.clone()
            }),
            Params::AddShopSlot(p) => Params::AddShopSlot(AddShopSlotParams {
                count: double(p.count),
                ..p.clone()
            }),
            Params::Salvage(p) => Params::Salvage(SalvageParams {
                count: double(p.count),
                ..p.clone()
            }),
            Params::Revive(p) => Params::Revive(ReviveParams {
                attack: double(p.attack),
                health: double(p.health),
            }),
            Params::Detonate(p) => Params::Detonate(DetonateParams {
                amount: double(p.amount),
                ..p.clone()
            }),
            Params::Convert(p) => Params::Convert(ConvertParams {
                count: p.count.map(double),
                ..p.clone()
            }),
            Params::Latent(p) => Params::Latent(LatentParams {
                amount: double(p.amount),
                count: double(p.count),
                ..p.clone()
            }),
            // 条件、阈值、上限与无数字产出的原语保持原样。
            other => other.clone(),
        }
    }

    /// 条件筛选；只有部分原语携带。
    pub fn condition(&self) -> Option<&Condition> {
        match self {
            Params::Growth(p) => p.condition.as_ref(),
            Params::ModifyStats(p) => p.condition.as_ref(),
            Params::GrantKeyword(p) => p.condition.as_ref(),
            Params::RemoveGrowth(p) => p.condition.as_ref(),
            Params::Decompose(p) => p.condition.as_ref(),
            _ => None,
        }
    }

    /// 事件筛选；只有部分原语携带。
    pub fn when(&self) -> Option<&Filter> {
        match self {
            Params::Growth(p) => p.when.as_ref(),
            Params::ModifyStats(p) => p.when.as_ref(),
            Params::GrantKeyword(p) => p.when.as_ref(),
            Params::RemoveGrowth(p) => p.when.as_ref(),
            Params::DealDamage(p) => p.when.as_ref(),
            Params::Summon(p) => p.when.as_ref(),
            Params::Decompose(p) => p.when.as_ref(),
            Params::Devour(p) => p.when.as_ref(),
            Params::Detonate(p) => p.when.as_ref(),
            Params::Convert(p) => p.when.as_ref(),
            Params::Latent(p) => p.when.as_ref(),
            _ => None,
        }
    }
}

/// 一段结构化效果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Effect {
    pub trigger: Trigger,
    pub op: Op,
    pub selector: Selector,
    pub params: Params,
}

impl Effect {
    /// 金色来源的同一段效果：数值产出 ×2，选择器与条件保持不变。
    pub fn doubled_outputs(&self) -> Effect {
        Effect {
            trigger: self.trigger,
            op: self.op,
            selector: self.selector.clone(),
            params: self.params.doubled_outputs(),
        }
    }
}

impl<'de> Deserialize<'de> for Effect {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct EffectWire {
            trigger: Trigger,
            op: Op,
            selector: Selector,
            #[serde(default)]
            params: Option<serde_json::Value>,
        }

        let wire = EffectWire::deserialize(deserializer)?;
        let params = decode_params(wire.op, wire.params).map_err(serde::de::Error::custom)?;
        Ok(Effect {
            trigger: wire.trigger,
            op: wire.op,
            selector: wire.selector,
            params,
        })
    }
}

/// 按原语把扁平参数对象解码为类型化参数；缺省参数按空对象处理。
fn decode_params(op: Op, params: Option<serde_json::Value>) -> Result<Params, String> {
    fn decode<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> Result<T, String> {
        serde_json::from_value(value).map_err(|err| err.to_string())
    }

    let value = match params {
        Some(serde_json::Value::Null) | None => serde_json::Value::Object(serde_json::Map::new()),
        Some(value) => value,
    };
    Ok(match op {
        Op::Growth => Params::Growth(decode(value)?),
        Op::ModifyStats => Params::ModifyStats(decode(value)?),
        Op::GrantKeyword => Params::GrantKeyword(decode(value)?),
        Op::RemoveGrowth => Params::RemoveGrowth(decode(value)?),
        Op::CannotAttack => Params::CannotAttack(decode(value)?),
        Op::DealDamage => Params::DealDamage(decode(value)?),
        Op::Summon => Params::Summon(decode(value)?),
        Op::Copy => Params::Copy(decode(value)?),
        Op::Discover => Params::Discover(decode(value)?),
        Op::RefreshShop => Params::RefreshShop(decode(value)?),
        Op::AddShopSlot => Params::AddShopSlot(decode(value)?),
        Op::GainMinion => Params::GainMinion(decode(value)?),
        Op::GrantMinion => Params::GrantMinion(decode(value)?),
        Op::Decompose => Params::Decompose(decode(value)?),
        Op::Mimic => Params::Mimic(decode(value)?),
        Op::Devour => Params::Devour(decode(value)?),
        Op::Detonate => Params::Detonate(decode(value)?),
        Op::Convert => Params::Convert(decode(value)?),
        Op::Revive => Params::Revive(decode(value)?),
        Op::Recall => Params::Recall(decode(value)?),
        Op::Salvage => Params::Salvage(decode(value)?),
        Op::Latent => Params::Latent(decode(value)?),
        Op::SevenVoyages => Params::SevenVoyages(decode(value)?),
    })
}
