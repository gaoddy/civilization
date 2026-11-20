//! 对局状态模型。
//!
//! 所有集合使用 `BTreeMap` / `BTreeSet` / `Vec`，保证遍历顺序确定；
//! 状态中只含整数，不含浮点、时间或地址。

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::data::{Catalog, Era, Playstyle};
use crate::effects::Duration;
use crate::rng::RngStreams;

pub use crate::settle::Pairing;

/// 座位号，0..8。
pub type Seat = u8;

/// 随从实例编号，由 `GameState::next_instance_id` 单调分配。
pub type InstanceId = u64;

/// 对局阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Lobby,
    Recruit,
    Combat,
    Settle,
    GameOver,
}

/// 临时属性修正；`Combat` 结束后清除。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Modifier {
    pub attack: i32,
    pub health: i32,
    pub duration: Duration,
}

/// 随从单项属性（攻击或生命）的分层数值。
///
/// 各层语义：`initial` 为基础值，`growth` 为可移除的成长增量，
/// `permanent` 为不可移除的永久增量，`turn` 为本回合临时增量，
/// `aura` 为光环派生增量。`current` 为当前实际值，
/// 与各层之和（[`StatComponent::layers`]）的差即已受伤害。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StatComponent {
    pub initial: i32,
    pub growth: i32,
    pub permanent: i32,
    pub turn: i32,
    pub aura: i32,
    pub current: i32,
}

impl StatComponent {
    /// 以基础值创建；各层增量为 0，当前值等于基础值。
    pub fn new(initial: i32) -> Self {
        Self {
            initial,
            growth: 0,
            permanent: 0,
            turn: 0,
            aura: 0,
            current: initial,
        }
    }

    /// 各层增量之和，即未受伤时的应有值。
    pub fn layers(&self) -> i32 {
        self.initial
            .saturating_add(self.growth)
            .saturating_add(self.permanent)
            .saturating_add(self.turn)
            .saturating_add(self.aura)
    }

    /// 已受伤害；复生等情况下可为负。
    pub fn damage(&self) -> i32 {
        self.layers().saturating_sub(self.current)
    }

    /// 成长增量与当前值同时增加。
    pub fn add_growth(&mut self, delta: i32) {
        self.growth = self.growth.saturating_add(delta);
        self.current = self.current.saturating_add(delta);
    }

    /// 永久增量与当前值同时增加。
    pub fn add_permanent(&mut self, delta: i32) {
        self.permanent = self.permanent.saturating_add(delta);
        self.current = self.current.saturating_add(delta);
    }

    /// 本回合临时增量与当前值同时增加。
    pub fn add_turn(&mut self, delta: i32) {
        self.turn = self.turn.saturating_add(delta);
        self.current = self.current.saturating_add(delta);
    }

    /// 按差值把光环层更新到 `value`，当前值同步变化。
    pub fn set_aura(&mut self, value: i32) {
        let delta = value.saturating_sub(self.aura);
        self.aura = value;
        self.current = self.current.saturating_add(delta);
    }

    /// 清除本回合临时增量。
    pub fn clear_turn(&mut self) {
        self.add_turn(self.turn.saturating_neg());
    }

    /// 移除成长增量；永久增量不受影响。
    pub fn remove_growth(&mut self) {
        self.add_growth(self.growth.saturating_neg());
    }

    /// 按差值把基础值更新到 `value`，当前值同步变化。
    pub fn set_initial(&mut self, value: i32) {
        let delta = value.saturating_sub(self.initial);
        self.initial = value;
        self.current = self.current.saturating_add(delta);
    }

    /// 只扣减当前值；负数伤害按 0 处理。
    pub fn apply_damage(&mut self, amount: i32) {
        self.current = self.current.saturating_sub(amount.max(0));
    }

    /// 只设置当前值，允许偏离层和（复生）。
    pub fn revive_to(&mut self, value: i32) {
        self.current = value;
    }

    /// 当前值的非负形式，用于展示。
    pub fn current_non_negative(&self) -> i32 {
        self.current.max(0)
    }
}

/// 随从的攻击与生命分层数值。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MinionStats {
    pub attack: StatComponent,
    pub health: StatComponent,
}

impl MinionStats {
    /// 以攻击、生命的基础值创建，两层增量均为 0。
    pub fn new(attack: i32, health: i32) -> Self {
        Self {
            attack: StatComponent::new(attack),
            health: StatComponent::new(health),
        }
    }
}

/// 某个棋盘实例当前生效的光环派生值。
///
/// 光环只存在于派生缓存中，绝不写入 [`MinionInstance`] 的持久实例或
/// `granted`；战斗快照折叠时才临时并入克隆。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AuraEffect {
    pub attack: i32,
    pub health: i32,
    pub keywords: crate::keyword::KeywordSet,
    /// 受到任一光环作用；用于战斗快照的生命下限 1。
    pub floor_one: bool,
}

/// 解析卡牌的基础攻防；金色按卡面 ×2。
///
/// 衍生物与生成卡同样在 `cards` 中，按 `in_shop_pool` 区分仅影响入池，不影响取值。
/// 「金色 ×2」规则只在这里实现，`MinionInstance::new` 与三连升金共用。
pub(crate) fn base_stats(catalog: &Catalog, card_id: &str, golden: bool) -> Option<(i32, i32)> {
    let card = catalog.cards.get(card_id)?;
    let (attack, health) = (card.attack.unwrap_or(0), card.health.unwrap_or(0));
    Some(if golden {
        (attack.saturating_mul(2), health.saturating_mul(2))
    } else {
        (attack, health)
    })
}

/// 棋盘上的随从实例。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinionInstance {
    pub instance_id: InstanceId,
    pub card_id: String,
    pub golden: bool,
    /// 攻击与生命的分层构成，`current` 为战斗与视图读取的当前量。
    pub stats: MinionStats,
    pub breed: i32,
    pub breed_applied: i32,
    pub contagion: i32,
    pub granted: Vec<(String, Duration)>,
    pub shield_used: bool,
    pub reborn_used: bool,
    /// 潜行是否已被打破；没有潜行关键词时无意义。
    pub stealth_broken: bool,
    /// 复苏是否已使用；与 `reborn_used` 独立。
    pub revive_used: bool,
    /// 潜伏爆发是否已使用；爆发每场最多一次。
    pub latent_used: bool,
    /// 延迟战吼是否已在某场战斗开始时结算；每个实例只结算一次。
    pub delayed_battlecry_used: bool,
    pub attacks_left: u8,
    pub cannot_attack: bool,
}

impl MinionInstance {
    /// 按目录解析卡面攻防；token 取注册表数值，普通卡取卡牌攻防，金色 ×2。
    pub fn new(catalog: &Catalog, instance_id: InstanceId, card_id: &str, golden: bool) -> Self {
        let (attack, health) = base_stats(catalog, card_id, golden).unwrap_or((0, 0));
        Self {
            instance_id,
            card_id: card_id.to_string(),
            golden,
            stats: MinionStats::new(attack, health),
            breed: 0,
            breed_applied: 0,
            contagion: 0,
            granted: Vec::new(),
            shield_used: false,
            reborn_used: false,
            stealth_broken: false,
            revive_used: false,
            latent_used: false,
            delayed_battlecry_used: false,
            attacks_left: 0,
            cannot_attack: false,
        }
    }

    /// 读取攻击力：`current` 的非负值。
    pub fn attack(&self) -> i32 {
        self.stats.attack.current_non_negative()
    }

    /// 读取生命值，下限 0。
    pub fn health(&self) -> i32 {
        self.stats.health.current_non_negative()
    }

    pub fn stats_view(&self) -> &MinionStats {
        &self.stats
    }

    pub fn stats_view_mut(&mut self) -> &mut MinionStats {
        &mut self.stats
    }
}

/// 商店中的一个卡槽。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShopSlot {
    /// 这次出现的卡牌实例编号；买入后由手牌沿用。
    pub instance_id: InstanceId,
    pub card_id: String,
    pub frozen: bool,
}

/// 手牌中的一张卡。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandCard {
    /// 随从牌：编号、卡牌、金色标记与全部状态都在随从实例上。
    Minion(MinionInstance),
    /// 效果牌：只有身份与「三连奖励」锁定的发现星级。
    Effect {
        instance_id: InstanceId,
        card_id: String,
        reward_level: Option<u8>,
    },
}

impl HandCard {
    /// 按卡牌目录构造一张无状态的效果牌。
    pub fn effect(instance_id: InstanceId, card_id: &str) -> Self {
        Self::Effect {
            instance_id,
            card_id: card_id.to_string(),
            reward_level: None,
        }
    }

    pub fn card_id(&self) -> &str {
        match self {
            Self::Minion(minion) => &minion.card_id,
            Self::Effect { card_id, .. } => card_id,
        }
    }

    pub fn instance_id(&self) -> InstanceId {
        match self {
            Self::Minion(minion) => minion.instance_id,
            Self::Effect { instance_id, .. } => *instance_id,
        }
    }

    pub fn golden(&self) -> bool {
        match self {
            Self::Minion(minion) => minion.golden,
            Self::Effect { .. } => false,
        }
    }

    pub fn reward_level(&self) -> Option<u8> {
        match self {
            Self::Minion(_) => None,
            Self::Effect { reward_level, .. } => *reward_level,
        }
    }

    pub fn minion(&self) -> Option<&MinionInstance> {
        match self {
            Self::Minion(minion) => Some(minion),
            Self::Effect { .. } => None,
        }
    }

    pub fn minion_mut(&mut self) -> Option<&mut MinionInstance> {
        match self {
            Self::Minion(minion) => Some(minion),
            Self::Effect { .. } => None,
        }
    }
}

/// 单个玩家的状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerState {
    pub seat: Seat,
    pub hp: i32,
    pub gold: u8,
    pub era: Era,
    pub shop: Vec<Option<ShopSlot>>,
    pub board: Vec<MinionInstance>,
    pub hand: Vec<HandCard>,
    pub hero: Option<String>,
    pub hero_uses: BTreeMap<String, u8>,
    pub damage_dealt: i32,
    pub eliminated: bool,
    pub first_refresh_used: bool,
    /// 本回合的首次刷新是否尚未触发 `OnFirstRefresh`。
    pub first_refresh_pending: bool,
    /// 本回合是否已提交 `EndRecruit`（本玩家结束招募）；全员结束或超时后才进入 `Combat`。
    pub recruit_done: bool,
    /// 本回合是否冻结过商店。解冻只在冻结的当回合可用，进入下一回合后清零；
    /// 冻结标记本身跨回合保留，只由手动刷新或解冻清除。
    pub froze_this_turn: bool,
}

/// 等待玩家做出的选择。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingChoice {
    /// 三连或效果驱动的发现：从 `candidates` 中选一张进入手牌，扣 1 份共享池。
    Discover {
        seat: Seat,
        candidates: Vec<String>,
        source: InstanceId,
    },
    /// 进化：来源随从从 `EVOLVE_CHOICES` 中选一项，`remaining` 为还需选择的次数。
    ///
    /// 进化不算发现：不扣池、不进手牌，结算在来源随从身上完成。
    Evolve {
        seat: Seat,
        source: InstanceId,
        remaining: u8,
    },
    /// 战吼已入场但尚未选定目标。
    ///
    /// `hand_card` 是入场前的原手牌，`hand_index` 是它原来的位置：放弃时原样退回，
    /// 保留成长、护盾与金色标记。`fire_ally_evolve` 表示入场观察者是否推迟到
    /// 目标结算后触发——来源登记了进化待选时为假，由进化结算负责触发。
    Target {
        seat: Seat,
        source: InstanceId,
        hand_card: HandCard,
        hand_index: u8,
        fire_ally_evolve: bool,
    },
}

/// 整局对局状态，`state_hash` 的输入。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameState {
    pub turn: u8,
    pub phase: Phase,
    pub players: Vec<PlayerState>,
    pub pool: BTreeMap<String, u16>,
    pub rng: RngStreams,
    pub first_seat: Seat,
    /// 本局启用的 5 个玩法；不含 `Neutral`。
    pub enabled_playstyles: BTreeSet<Playstyle>,
    pub previous_opponent: BTreeMap<Seat, Option<Seat>>,
    pub last_winner: Option<Seat>,
    /// 本回合配对结果，由招募结束写入、结算结束清空。
    pub pairings: Vec<Pairing>,
    /// 本回合每名败者将受到的伤害，由战斗结算写入、结算清空。
    pub pending_settle: BTreeMap<Seat, i32>,
    /// 本回合赢下自己战斗的座位（幽灵战获胜计入，平局与失败不计）。
    pub last_won: BTreeSet<Seat>,
    pub pending: VecDeque<PendingChoice>,
    /// 已确定的排名（从第 1 名到末名）；出局者先进入尾部，结束时补齐。
    pub ranking: Vec<Seat>,
    pub next_instance_id: InstanceId,
    /// 按实例编号索引的光环派生值；每次区域变化后全量重算。
    pub auras: BTreeMap<InstanceId, AuraEffect>,
}
