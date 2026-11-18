//! 战斗行动循环与关键词判定。
//!
//! 本模块负责纯数值结算、关键词判定，以及战斗内效果（战斗开始、死亡结算、
//! 召唤、复制）的接入点。所有输入都是只读快照，`simulate` 在克隆上结算，
//! 调用方的棋盘不会被修改；随机性只来自 [`Stream::Combat`]。
//!
//! 规则口径见 `docs/versions/v0.1/product/rules/core-rules.md` §5.1–§5.4
//! 与 `keywords-and-mechanics.md` §1、§2.3–§2.7、§10。

use std::collections::{BTreeMap, BTreeSet};

use crate::data::{Catalog, Playstyle};
use crate::effects::{Basis, Duration, EffectCtx, Trigger};
use crate::engine::Engine;
use crate::recruit::new_minion;
use crate::rng::{RngStreams, Stream};
use crate::state::{InstanceId, MinionInstance, Modifier, Seat, base_stats};

/// 每场战斗的行动上限，防止死亡循环。
const MAX_ACTIONS: u16 = 200;
/// 单场战斗的效果触发总数上限（关键词 §8）。
pub(crate) const MAX_TRIGGERS: u16 = 200;

/// 关键词：连击。
const WINDFURY: &str = "windfury";
/// 关键词：嘲讽。
const TAUNT: &str = "taunt";
/// 关键词：护盾。
const DIVINE_SHIELD: &str = "divine_shield";
/// 关键词：复生。
const REBORN: &str = "reborn";
/// 关键词：传染。
const CONTAGION: &str = "contagion";
/// 关键词：最优攻击。
const OPTIMAL_ATTACK: &str = "optimal_attack";
/// 关键词：播种。
const SOW: &str = "sow";
/// 关键词：剧毒。
pub(crate) const POISON: &str = "poison";
/// 关键词：减伤；实际存储为「armor N」。
pub(crate) const ARMOR: &str = "armor";
/// 关键词：潜行。
pub(crate) const STEALTH: &str = "stealth";
/// 播种召唤的 token。
pub(crate) const SEEDLING_TOKEN: &str = "seedling";

/// 某条动作执行后，一个受影响随从的现场状态。
///
/// 只读派生自战斗现场，不参与 `state_hash`；客户端按它渲染，不做任何算术。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MinionSnapshot {
    pub instance_id: InstanceId,
    pub attack: i32,
    pub health: i32,
    pub alive: bool,
}

/// 战斗中的一条动作记录。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CombatAction {
    /// 一次主动攻击：主伤害记录在本条动作，目标存活并造成反击伤害时，
    /// 反击结果已并入本条动作的快照，不再单独记录一条 `Attack`。
    Attack {
        attacker: InstanceId,
        target: InstanceId,
        damage: i32,
        blocked: bool,
    },
    /// 随从死亡（复生前）。
    Death { minion: InstanceId },
    /// 随从以 1 点生命复生。
    Reborn { minion: InstanceId },
    /// 目标被叠加一层传染。
    ContagionApplied { target: InstanceId },
    /// 目标被剧毒致死。
    Poisoned { target: InstanceId },
    /// 来源复制了目标的当前攻防与关键词。
    Mimicked {
        source: InstanceId,
        target: InstanceId,
    },
    /// 来源吞噬了目标并永久获得其卡面基础属性。
    Devoured {
        source: InstanceId,
        target: InstanceId,
    },
    /// 引爆：`layers` 为被移除的传染层数，`damage` 为按层伤害实际造成的伤害
    /// （护盾或减伤后；阈值致死不计入）。
    Detonated {
        target: InstanceId,
        layers: i32,
        damage: i32,
    },
    /// 转化：目标达到传染阈值后被消灭，来源方召唤 `token`。
    Converted { target: InstanceId, token: String },
    /// 随从按复苏声明值复活。
    Revived { minion: InstanceId },
    /// 随从死亡后回手（已写入 `CombatLog::hand_minions`）。
    Recalled { minion: InstanceId },
    /// 零件回收：死亡时从本局启用的机械随从中随机获得一张牌。
    Salvaged { seat: Seat, card_id: String },
    /// 战斗中召唤：`seat` 为召唤方座位，`position` 为插入后该随从在
    /// **存活随从序列**中的下标；`card_id` / `golden` 供房间服务补全展示数据
    /// （召唤物不在战前快照里）。
    Summon {
        minion: InstanceId,
        card_id: String,
        golden: bool,
        seat: Seat,
        position: u8,
    },
}

/// 战斗中复制进手牌的记录；由结算层在战斗结束后写入持久手牌。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatHandCopy {
    pub seat: Seat,
    /// 产生该副作用的阵营（0 = 攻击方，1 = 防守方）。
    ///
    /// 幽灵战两侧使用同一座位，结算层据此丢弃幽灵侧的副作用。
    pub side: usize,
    pub card_id: String,
    pub golden: bool,
}

/// 战斗中死亡回手的随从；由结算层在战斗结束后原样写入持久手牌。
///
/// 与 [`CombatHandCopy`] 分开存放：这里保留完整的实例状态（成长、繁衍、
/// 传染、授予关键词等），复制牌则只记录卡面身份。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatHandMinion {
    pub seat: Seat,
    /// 产生该回手的阵营（0 = 攻击方，1 = 防守方），用于过滤幽灵侧副作用。
    pub side: usize,
    pub minion: MinionInstance,
}

/// 伤害来源。`None` 表示无来源伤害（传染、回合开始掉血等），不产生击杀。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageSource {
    Minion(InstanceId),
    None,
}

/// 一条击杀记录：只在本段伤害把目标降到 0 或更低、且伤害有来源时产生。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KillRecord {
    pub killer: InstanceId,
    pub victim: InstanceId,
    /// 击杀者所在阵营。
    pub killer_side: usize,
    /// 攻击者的主伤害击杀目标、目标的这次反击击杀攻击者；为真时跳过击杀类钩子。
    pub mutual: bool,
}

/// 一场战斗的结算结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatLog {
    pub actions: Vec<CombatAction>,
    /// 与 `actions` 同序同长：每条动作写入时，受影响随从动作后的状态。
    pub states: Vec<Vec<MinionSnapshot>>,
    pub attacker_survivors: Vec<InstanceId>,
    pub defender_survivors: Vec<InstanceId>,
    pub winner: Option<Seat>,
    pub action_count: u16,
    /// 本场战斗累计触发的效果次数。
    pub trigger_count: u16,
    /// 本场战斗召唤出的随从数量。
    pub summon_count: u16,
    /// 召唤 card 时从共享池扣除的份数，战斗结束后由结算层写回。
    pub pool_spent: BTreeMap<String, u16>,
    /// 战斗中复制进手牌的卡牌，战斗结束后由结算层写回。
    pub hand_copies: Vec<CombatHandCopy>,
    /// 战斗中死亡回手的随从，战斗结束后由结算层写入持久手牌。
    pub hand_minions: Vec<CombatHandMinion>,
    /// 战斗使用的下一个实例编号；调用方可据此推进 `GameState::next_instance_id`。
    pub next_instance_id: InstanceId,
    /// 战斗结束时的攻击方随从克隆；供结算层回写持久字段（传染、成长）。
    pub attacker_final: Vec<MinionInstance>,
    /// 战斗结束时的防守方随从克隆；供结算层回写持久字段（传染、成长）。
    pub defender_final: Vec<MinionInstance>,
}

/// 战斗开始前由调用方补充的上下文。
#[derive(Debug, Clone, Copy)]
pub struct CombatSetup<'a> {
    /// 攻击方英雄卡 id；用于结算英雄被动。
    pub attacker_hero: Option<&'a str>,
    /// 防守方英雄卡 id；用于结算英雄被动。
    pub defender_hero: Option<&'a str>,
    /// 战斗局部的实例编号起点；缺省时取双方最大编号 + 1。
    pub next_instance_id: Option<InstanceId>,
    /// 共享牌池快照；仅用于判断召唤 card 时是否扣池。
    pub pool: Option<&'a BTreeMap<String, u16>>,
    /// 攻击方手牌数量，用于判断复制进手牌是否有空位。
    pub attacker_hand: usize,
    /// 防守方手牌数量。
    pub defender_hand: usize,
    /// 手牌上限。
    pub hand_slots: usize,
    /// 本局启用的玩法；`None` 表示未提供，随机池不按玩法筛选。
    ///
    /// 仅用于战斗内需要从卡池取牌的效果（零件回收）。
    pub playstyles: Option<&'a BTreeSet<Playstyle>>,
}

impl Default for CombatSetup<'_> {
    fn default() -> Self {
        Self {
            attacker_hero: None,
            defender_hero: None,
            next_instance_id: None,
            pool: None,
            attacker_hand: 0,
            defender_hand: 0,
            hand_slots: usize::MAX,
            playstyles: None,
        }
    }
}

/// 当前攻击力；不包含光环。
pub fn effective_attack(_catalog: &Catalog, minion: &MinionInstance) -> i32 {
    minion.attack()
}

/// 当前攻击力，含外部传入的光环修正。
///
/// 学者在最后一步被强制为 0，下限为 0。
pub fn effective_attack_with(
    _catalog: &Catalog,
    minion: &MinionInstance,
    auras: &[Modifier],
) -> i32 {
    let mut value = minion.attack();
    for aura in auras {
        value = value.saturating_add(aura.attack);
    }
    if minion.is_scholar {
        value = 0;
    }
    value.max(0)
}

/// 当前生命值；不包含光环。
pub fn effective_health(_catalog: &Catalog, minion: &MinionInstance) -> i32 {
    minion.health()
}

/// 当前生命值，含外部传入的光环修正；下限为 0。
pub fn effective_health_with(
    _catalog: &Catalog,
    minion: &MinionInstance,
    auras: &[Modifier],
) -> i32 {
    let mut value = minion.health();
    for aura in auras {
        value = value.saturating_add(aura.health);
    }
    value.max(0)
}

/// 随从当前生效的关键词集合。
///
/// 由所引用技能的 `keyword` 字段与 token 注册表派生，并并入尚未失效的授予关键词；
/// `Instant` 授予在战斗前就已结算完毕，视为不存在。未知关键词原样保留。
pub fn keywords(catalog: &Catalog, minion: &MinionInstance) -> BTreeSet<String> {
    let mut result = base_keywords(catalog, minion);
    for (keyword, duration) in &minion.granted {
        if *duration != Duration::Instant {
            result.insert(keyword.clone());
        }
    }
    result
}

/// 卡面与 Token 自带的关键词，不含战斗内授予与光环派生。
fn base_keywords(catalog: &Catalog, minion: &MinionInstance) -> BTreeSet<String> {
    base_keyword_occurrences(catalog, minion)
        .into_iter()
        .collect()
}

/// 卡面与 Token 自带关键词的逐次出现列表，保留重复（供同层减伤累加）。
fn base_keyword_occurrences(catalog: &Catalog, minion: &MinionInstance) -> Vec<String> {
    let mut result = Vec::new();
    if let Some(token) = catalog.tokens.get(&minion.card_id) {
        result.extend(token.keywords.iter().cloned());
    }
    if let Some(card) = catalog.cards.get(&minion.card_id) {
        for skill_id in &card.skills {
            if let Some(keyword) = catalog
                .skills
                .get(skill_id)
                .and_then(|skill| skill.keyword.as_ref())
            {
                result.push(keyword.clone());
            }
        }
    }
    result
}

/// 排除光环派生关键词后的关键词集合，供「不复制光环」的效果（拟态）使用。
///
/// 卡面本身就有的关键词即使同时被光环授予也保留。
pub fn keywords_without_aura(catalog: &Catalog, minion: &MinionInstance) -> BTreeSet<String> {
    let base = base_keywords(catalog, minion);
    let mut result = keywords(catalog, minion);
    for (keyword, duration) in &minion.granted {
        if *duration == Duration::Aura && !base.contains(keyword) {
            result.remove(keyword);
        }
    }
    result
}

/// 是否携带剧毒关键词。
pub fn has_poison(catalog: &Catalog, minion: &MinionInstance) -> bool {
    keywords(catalog, minion).contains(POISON)
}

/// 当前是否仍处于潜行：携带 `stealth` 关键词且本次战斗尚未将其打破。
pub fn has_stealth(catalog: &Catalog, minion: &MinionInstance) -> bool {
    !minion.stealth_broken && keywords(catalog, minion).contains(STEALTH)
}

/// 当前生效的减伤层数之和；多个「armor N」叠加，同层重复也累加。
///
/// 层数保存在关键词字符串 `armor N` 中（与 `breed N` 同形）；没有数字时按 1。
/// 卡面自带与授予分别逐条统计，因此卡面重复的 `armor 1`、多次授予、光环派生
/// 与卡面层的组合都会相加；`Instant` 授予与关键词集合口径一致，视为不存在。
pub fn armor_value(catalog: &Catalog, minion: &MinionInstance) -> i32 {
    let base = base_keyword_occurrences(catalog, minion)
        .iter()
        .filter_map(|keyword| armor_layer(keyword))
        .fold(0i32, i32::saturating_add);
    minion
        .granted
        .iter()
        .filter(|(_, duration)| *duration != Duration::Instant)
        .filter_map(|(keyword, _)| armor_layer(keyword))
        .fold(base, i32::saturating_add)
}

/// 解析 `armor N` 关键词的层数；不是减伤关键词时返回 `None`。
pub(crate) fn armor_layer(keyword: &str) -> Option<i32> {
    let rest = keyword.strip_prefix(ARMOR)?;
    if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let value = rest.trim();
    Some(if value.is_empty() {
        1
    } else {
        value.parse().unwrap_or(0)
    })
}

/// 每轮该随从拥有的行动次数。
///
/// 「连击」对学者同样生效，学者每轮也行动两次（攻击力仍为 0）。
fn starting_tokens(catalog: &Catalog, minion: &MinionInstance) -> u8 {
    if minion.cannot_attack {
        0
    } else if keywords(catalog, minion).contains(WINDFURY) {
        2
    } else {
        1
    }
}

/// 一方的战斗内状态。输入随从被克隆后在这里结算。
#[derive(Debug, Clone)]
pub(crate) struct SideBoard {
    pub(crate) minions: Vec<MinionInstance>,
    pub(crate) alive: Vec<bool>,
    pub(crate) starting_tokens: Vec<u8>,
    pointer: usize,
}

impl SideBoard {
    fn new(catalog: &Catalog, board: &[MinionInstance]) -> Self {
        let mut minions = board.to_vec();
        let mut alive = Vec::with_capacity(minions.len());
        let mut token_counts = Vec::with_capacity(minions.len());
        for minion in &mut minions {
            let tokens = starting_tokens(catalog, minion);
            minion.attacks_left = tokens;
            // 战斗开始前生命已 ≤0 的随从先视为在场，由开场死亡结算统一处理。
            alive.push(true);
            token_counts.push(tokens);
        }
        Self {
            minions,
            alive,
            starting_tokens: token_counts,
            pointer: 0,
        }
    }

    fn alive_count(&self) -> usize {
        self.alive.iter().filter(|alive| **alive).count()
    }

    fn total_health(&self) -> i32 {
        self.minions
            .iter()
            .zip(&self.alive)
            .filter(|(_, alive)| **alive)
            .fold(0i32, |sum, (minion, _)| {
                sum.saturating_add(minion.stats.health.current)
            })
    }

    fn survivors(&self) -> Vec<InstanceId> {
        self.minions
            .iter()
            .zip(&self.alive)
            .filter(|(_, alive)| **alive)
            .map(|(minion, _)| minion.instance_id)
            .collect()
    }

    fn alive_ids(&self) -> Vec<InstanceId> {
        self.minions
            .iter()
            .zip(&self.alive)
            .filter(|(_, alive)| **alive)
            .map(|(minion, _)| minion.instance_id)
            .collect()
    }

    /// 该侧进入新一轮：指针回到最左，存活随从恢复本轮行动次数。
    fn reset_round(&mut self) {
        self.pointer = 0;
        for index in 0..self.minions.len() {
            if self.alive[index] {
                self.minions[index].attacks_left = self.starting_tokens[index];
            }
        }
    }

    /// 返回当前应出手的随从下标；跳过已死亡、生命 ≤0 或本轮已行动者。
    ///
    /// 本轮扫描到末尾后立即进入新一轮再找一次；全部不能攻击时返回 `None`。
    fn find_attacker(&mut self) -> Option<usize> {
        if self.pointer >= self.minions.len() {
            self.reset_round();
        }
        if let Some(index) = self.scan_attacker() {
            return Some(index);
        }
        // 本轮已走完：该侧下一次轮到从最左开始。
        self.reset_round();
        self.scan_attacker()
    }

    /// 从当前指针向右扫描第一个可出手的随从；指针停在返回值或末尾。
    fn scan_attacker(&mut self) -> Option<usize> {
        while self.pointer < self.minions.len() {
            if self.alive[self.pointer]
                && self.minions[self.pointer].stats.health.current > 0
                && self.minions[self.pointer].attacks_left > 0
            {
                return Some(self.pointer);
            }
            self.pointer += 1;
        }
        None
    }
}

/// 战斗局部的棋盘：双方克隆、座位、实例编号计数器与触发预算。
#[derive(Debug, Clone)]
pub struct BattleField {
    pub(crate) seats: [Seat; 2],
    pub(crate) sides: [SideBoard; 2],
    pub(crate) next_instance_id: InstanceId,
    pub(crate) triggers: u16,
    pub(crate) summon_count: u16,
    pub(crate) actions: Vec<CombatAction>,
    pub(crate) states: Vec<Vec<MinionSnapshot>>,
    pub(crate) hand_copies: Vec<CombatHandCopy>,
    pub(crate) hand_minions: Vec<CombatHandMinion>,
    pub(crate) pool: BTreeMap<String, u16>,
    pub(crate) pool_spent: BTreeMap<String, u16>,
    pub(crate) hand_len: [usize; 2],
    pub(crate) hand_slots: usize,
    /// 本局启用玩法；仅零件回收等取牌效果使用。
    pub(crate) playstyles: Option<BTreeSet<Playstyle>>,
    /// 正在结算死亡；嵌套触发产生的死亡留给外层循环处理。
    pub(crate) resolving: bool,
    pub(crate) first_side: usize,
    /// 进入战斗快照时的当前生命值；召唤时写入新随从。
    pub(crate) combat_start_health: BTreeMap<InstanceId, i32>,
    /// 承受致命伤害前的当前生命值；只在伤害足以致死时写入。
    pub(crate) pre_kill_health: BTreeMap<InstanceId, i32>,
    /// 待触发的击杀记录；死亡队列处理完后按记录顺序触发钩子。
    pub(crate) pending_kills: Vec<KillRecord>,
}

impl BattleField {
    fn new(
        catalog: &Catalog,
        attacker: &[MinionInstance],
        defender: &[MinionInstance],
        attacker_seat: Seat,
        defender_seat: Seat,
        first: Seat,
        setup: CombatSetup<'_>,
    ) -> Self {
        let max_id = attacker
            .iter()
            .chain(defender.iter())
            .map(|minion| minion.instance_id)
            .max()
            .unwrap_or(0);
        let next_instance_id = setup
            .next_instance_id
            .unwrap_or(1)
            // 兼容调用方未推进分配器的情况：至少避开双方棋盘上已有的编号。
            .max(max_id.saturating_add(1));
        let sides = [
            SideBoard::new(catalog, attacker),
            SideBoard::new(catalog, defender),
        ];
        // 战斗开始快照：双方全部随从入场时的当前生命。
        let mut combat_start_health = BTreeMap::new();
        for side in &sides {
            for minion in &side.minions {
                combat_start_health.insert(minion.instance_id, minion.stats.health.current);
            }
        }
        Self {
            seats: [attacker_seat, defender_seat],
            sides,
            next_instance_id,
            triggers: 0,
            summon_count: 0,
            actions: Vec::new(),
            states: Vec::new(),
            hand_copies: Vec::new(),
            hand_minions: Vec::new(),
            pool: setup.pool.cloned().unwrap_or_default(),
            pool_spent: BTreeMap::new(),
            hand_len: [setup.attacker_hand, setup.defender_hand],
            hand_slots: setup.hand_slots,
            playstyles: setup.playstyles.cloned(),
            resolving: false,
            first_side: if first == attacker_seat { 0 } else { 1 },
            combat_start_health,
            pre_kill_health: BTreeMap::new(),
            pending_kills: Vec::new(),
        }
    }

    pub(crate) fn seat(&self, side: usize) -> Seat {
        self.seats[side]
    }

    pub(crate) fn side_of(&self, seat: Seat) -> Option<usize> {
        if self.seats[0] == seat {
            Some(0)
        } else if self.seats[1] == seat {
            Some(1)
        } else {
            None
        }
    }

    pub(crate) fn locate(&self, id: InstanceId) -> Option<(usize, usize)> {
        for side in 0..2 {
            if let Some(index) = self.sides[side]
                .minions
                .iter()
                .position(|minion| minion.instance_id == id)
            {
                return Some((side, index));
            }
        }
        None
    }

    pub(crate) fn minion(&self, id: InstanceId) -> Option<&MinionInstance> {
        self.locate(id)
            .map(|(side, index)| &self.sides[side].minions[index])
    }

    pub(crate) fn minion_mut(&mut self, id: InstanceId) -> Option<&mut MinionInstance> {
        let (side, index) = self.locate(id)?;
        Some(&mut self.sides[side].minions[index])
    }

    /// 记录一条动作及其受影响随从的动作后状态。
    ///
    /// `actions` 与 `states` 严格同序同长，调用方必须成对传入。
    pub(crate) fn push_action(&mut self, action: CombatAction, states: Vec<MinionSnapshot>) {
        self.actions.push(action);
        self.states.push(states);
    }

    /// 取某个随从的现场状态（含光环折叠后的数值）；不在场上时为 `None`。
    fn snapshot_of(&self, catalog: &Catalog, id: InstanceId) -> Option<MinionSnapshot> {
        let (side, index) = self.locate(id)?;
        let minion = &self.sides[side].minions[index];
        Some(MinionSnapshot {
            instance_id: id,
            attack: effective_attack(catalog, minion),
            health: effective_health(catalog, minion),
            alive: self.sides[side].alive[index],
        })
    }

    /// 收集若干随从的现场状态，跳过已不在场上的编号。
    pub(crate) fn snapshots(&self, catalog: &Catalog, ids: &[InstanceId]) -> Vec<MinionSnapshot> {
        ids.iter()
            .filter_map(|id| self.snapshot_of(catalog, *id))
            .collect()
    }

    pub(crate) fn is_alive(&self, id: InstanceId) -> bool {
        self.locate(id)
            .map(|(side, index)| self.sides[side].alive[index])
            .unwrap_or(false)
    }

    pub(crate) fn alive_ids(&self, side: usize) -> Vec<InstanceId> {
        self.sides[side].alive_ids()
    }

    /// 某一侧的存活随从数量；阵亡条目仍留在向量里，但不计入容量。
    pub(crate) fn alive_count(&self, side: usize) -> usize {
        self.sides[side]
            .alive
            .iter()
            .filter(|alive| **alive)
            .count()
    }

    /// 向量下标之前（不含该下标）的存活随从数量，用于对外发存活序列下标。
    fn alive_rank(&self, side: usize, index: usize) -> usize {
        self.sides[side].alive[..index]
            .iter()
            .filter(|alive| **alive)
            .count()
    }

    /// 该侧是否还有空位；容量取目录配置的 `board_slots`。
    pub(crate) fn has_room(&self, catalog: &Catalog, side: usize) -> bool {
        self.alive_count(side) < catalog.config.board_slots
    }

    pub(crate) fn new_instance_id(&mut self) -> InstanceId {
        let id = self.next_instance_id;
        self.next_instance_id = self.next_instance_id.saturating_add(1);
        id
    }

    /// 取得一次效果触发额度；达到 200 次后返回 `false`。
    pub(crate) fn take_trigger(&mut self) -> bool {
        if self.triggers >= MAX_TRIGGERS {
            return false;
        }
        self.triggers = self.triggers.saturating_add(1);
        true
    }

    /// 在棋盘右侧追加一个随从；容量不足时返回 `None`，不改变棋盘。
    ///
    /// 同时记录一条 `Summon` 动作（战斗中出现的随从都不在战前快照里，
    /// 客户端只能靠这条动作渲染出来）；`position` 是存活序列下标。
    pub(crate) fn append(
        &mut self,
        catalog: &Catalog,
        side: usize,
        mut minion: MinionInstance,
    ) -> Option<InstanceId> {
        if !self.has_room(catalog, side) {
            return None;
        }
        let id = minion.instance_id;
        let card_id = minion.card_id.clone();
        let golden = minion.golden;
        let tokens = starting_tokens(catalog, &minion);
        minion.attacks_left = tokens;
        let health = minion.stats.health.current;
        self.sides[side].minions.push(minion);
        self.sides[side].alive.push(true);
        self.sides[side].starting_tokens.push(tokens);
        self.combat_start_health.insert(id, health);
        let index = self.sides[side].minions.len() - 1;
        let position = self.alive_rank(side, index);
        let states = self.snapshots(catalog, &[id]);
        self.push_action(
            CombatAction::Summon {
                minion: id,
                card_id,
                golden,
                seat: self.seats[side],
                position: u8::try_from(position).unwrap_or(u8::MAX),
            },
            states,
        );
        Some(id)
    }

    /// 在指定站位插入一个随从；容量不足时返回 `None`，不改变棋盘。
    pub(crate) fn insert_at(
        &mut self,
        catalog: &Catalog,
        side: usize,
        position: usize,
        mut minion: MinionInstance,
    ) -> Option<InstanceId> {
        if !self.has_room(catalog, side) {
            return None;
        }
        let id = minion.instance_id;
        let card_id = minion.card_id.clone();
        let golden = minion.golden;
        let tokens = starting_tokens(catalog, &minion);
        minion.attacks_left = tokens;
        let health = minion.stats.health.current;
        let board = &mut self.sides[side];
        let position = position.min(board.minions.len());
        if position <= board.pointer {
            board.pointer += 1;
        }
        board.minions.insert(position, minion);
        board.alive.insert(position, true);
        board.starting_tokens.insert(position, tokens);
        self.combat_start_health.insert(id, health);
        let rank = self.alive_rank(side, position);
        let states = self.snapshots(catalog, &[id]);
        self.push_action(
            CombatAction::Summon {
                minion: id,
                card_id,
                golden,
                seat: self.seats[side],
                position: u8::try_from(rank).unwrap_or(u8::MAX),
            },
            states,
        );
        Some(id)
    }

    pub(crate) fn modify_stats(&mut self, id: InstanceId, modifier: Modifier) {
        let Some((side, index)) = self.locate(id) else {
            return;
        };
        let minion = &mut self.sides[side].minions[index];
        // 分层写入自身会同步 `current`，不再维护平行生命数组。
        match modifier.duration {
            Duration::Combat => {
                minion.stats.attack.add_turn(modifier.attack);
                minion.stats.health.add_turn(modifier.health);
            }
            // `Instant` 在数据中未使用；保持现有「永不清除」语义。
            Duration::Permanent | Duration::Instant | Duration::Aura => {
                minion.stats.attack.add_permanent(modifier.attack);
                minion.stats.health.add_permanent(modifier.health);
            }
        }
    }

    /// 获得成长；`health_only` 时只增加生命。
    pub(crate) fn add_growth(&mut self, id: InstanceId, times: i32, health_only: bool) {
        let Some((side, index)) = self.locate(id) else {
            return;
        };
        let minion = &mut self.sides[side].minions[index];
        if health_only {
            minion.stats.health.add_growth(times);
        } else {
            minion.stats.attack.add_growth(times);
            minion.stats.health.add_growth(times);
        }
    }

    /// 移除全部成长，同时扣减当前生命。
    pub(crate) fn remove_growth(&mut self, id: InstanceId) {
        let Some(minion) = self.minion_mut(id) else {
            return;
        };
        // 成长对攻防各记一层；生命侧同时包含「成长」与「只加生命」两种来源，
        // `remove_growth` 会按生命层增量同步扣减 `current`。
        minion.stats.attack.remove_growth();
        minion.stats.health.remove_growth();
    }

    /// 唯一的战斗掉血入口：只写 `stats.health.current`。
    ///
    /// 写入前先减去目标当前 `armor N` 之和，再按 `can_kill` 夹取；
    /// `can_kill = false` 时最多把当前生命降到 1。`can_kill = true` 时允许降到
    /// 0 或更低，由死亡判定统一处理。伤害足以致死时在扣血前写入 `pre_kill_health`；
    /// `source` 为随从且本段伤害把生命降到 ≤0 时，追加一条击杀记录。
    ///
    /// 返回实际写入的伤害值（已扣减伤，且可能是 0）。
    pub(crate) fn apply_damage(
        &mut self,
        catalog: &Catalog,
        id: InstanceId,
        amount: i32,
        can_kill: bool,
        source: DamageSource,
    ) -> i32 {
        let killer = match source {
            DamageSource::Minion(killer) => Some(killer),
            DamageSource::None => None,
        };
        // 先取阵营，避免在可变借用随从后再借用棋盘。
        let killer_side = killer.and_then(|killer| self.locate(killer).map(|(side, _)| side));
        let stealth_source = killer
            .and_then(|killer| self.minion(killer))
            .is_some_and(|minion| has_stealth(catalog, minion));
        let armor = self
            .locate(id)
            .map(|(side, index)| armor_value(catalog, &self.sides[side].minions[index]))
            .unwrap_or(0);
        let Some((side, index)) = self.locate(id) else {
            return 0;
        };
        let minion = &mut self.sides[side].minions[index];
        // 减伤在写入前结算；`can_kill` 的夹取在减伤之后执行。
        let amount = amount.max(0).saturating_sub(armor).max(0);
        let before = minion.stats.health.current;
        let lethal = can_kill && amount >= before;
        if lethal && before > 0 {
            self.pre_kill_health.insert(id, before);
        }
        let damage = if can_kill {
            amount
        } else {
            amount.min(before.saturating_sub(1).max(0))
        };
        minion.stats.health.apply_damage(damage);
        let dead = self.sides[side].alive[index] && minion.stats.health.current <= 0;
        // 造成实际伤害的潜行随从打破自身潜行；攻击、反击、俯冲与效果伤害都走这里。
        if damage > 0
            && stealth_source
            && let Some(killer) = killer
            && let Some(source) = self.minion_mut(killer)
        {
            source.stealth_broken = true;
        }
        if dead && let (Some(killer), Some(killer_side)) = (killer, killer_side) {
            self.pending_kills.push(KillRecord {
                killer,
                victim: id,
                killer_side,
                mutual: false,
            });
        }
        damage
    }

    /// 效果层伤害入口；与 [`BattleField::apply_damage`] 同义，无伤害来源。
    pub(crate) fn deal_damage(
        &mut self,
        catalog: &Catalog,
        id: InstanceId,
        amount: i32,
        can_kill: bool,
    ) -> i32 {
        self.apply_damage(catalog, id, amount, can_kill, DamageSource::None)
    }

    /// 分解取值：按口径返回 `event_target` 被分解随从的生命值；快照缺失时为 `None`。
    pub(crate) fn decompose_health(
        &self,
        catalog: &Catalog,
        victim: InstanceId,
        basis: Basis,
    ) -> Option<i32> {
        match basis {
            Basis::Original => {
                let minion = self.minion(victim)?;
                base_stats(catalog, &minion.card_id, minion.golden).map(|(_, health)| health)
            }
            Basis::CombatStart => self.combat_start_health.get(&victim).copied(),
            Basis::PreKill => self.pre_kill_health.get(&victim).copied(),
        }
    }

    pub(crate) fn cannot_attack(&mut self, id: InstanceId) {
        if let Some(minion) = self.minion_mut(id) {
            minion.cannot_attack = true;
            minion.attacks_left = 0;
        }
    }

    /// 记录一次复制进手牌；手牌已满时返回 `false`。
    ///
    /// `side` 由调用方按来源随从的真实阵营传入。不能用 `side_of(seat)` 反推：
    /// 幽灵战两侧共用同一座位，反推会恒得到攻击方。
    pub(crate) fn record_hand_copy(&mut self, side: usize, card_id: &str, golden: bool) -> bool {
        let seat = self.seats[side];
        if self.hand_len[side] >= self.hand_slots {
            return false;
        }
        self.hand_len[side] = self.hand_len[side].saturating_add(1);
        self.hand_copies.push(CombatHandCopy {
            seat,
            side,
            card_id: card_id.to_string(),
            golden,
        });
        true
    }

    /// 记录一次召唤 card 的扣池；池中无剩余份数时不扣。
    pub(crate) fn record_pool_spend(&mut self, card_id: &str) {
        let available = self
            .pool
            .get(card_id)
            .copied()
            .unwrap_or(0)
            .saturating_sub(self.pool_spent.get(card_id).copied().unwrap_or(0));
        if available > 0 {
            let entry = self.pool_spent.entry(card_id.to_string()).or_insert(0);
            *entry = entry.saturating_add(1);
        }
    }

    /// 结算当前生命 ≤0 的随从：死亡 → 亡语 → 播种 → 复生，再触发友方/敌方死亡。
    pub(crate) fn resolve_deaths(
        &mut self,
        engine: Option<&Engine>,
        catalog: &Catalog,
        rng: &mut RngStreams,
    ) {
        if self.resolving {
            return;
        }
        self.resolving = true;
        loop {
            let mut dead = Vec::new();
            for side in 0..2 {
                for index in 0..self.sides[side].minions.len() {
                    let health = self.sides[side].minions[index].stats.health.current;
                    if self.sides[side].alive[index] && health <= 0 {
                        // 死亡效果可能插入随从导致下标移动，队列只记录 (阵营, 实例编号)。
                        dead.push((side, self.sides[side].minions[index].instance_id));
                    }
                }
            }
            // 本轮死亡与友方/敌方死亡钩子都处理完，再按记录顺序触发击杀类钩子；
            // 钩子造成的死亡（连同新产生的击杀记录）回到下一轮统一结算。
            if dead.is_empty() && (self.pending_kills.is_empty() || engine.is_none()) {
                break;
            }

            let mut dead_ids = Vec::new();
            for (side, id) in dead {
                // 重新定位当前仍处于死亡态的实例；插入随从会移动下标。
                let Some(index) = (0..self.sides[side].minions.len()).find(|index| {
                    self.sides[side].minions[*index].instance_id == id
                        && self.sides[side].alive[*index]
                        && self.sides[side].minions[*index].stats.health.current <= 0
                }) else {
                    continue;
                };
                let id = self.sides[side].minions[index].instance_id;
                let golden = self.sides[side].minions[index].golden;
                let card_id = self.sides[side].minions[index].card_id.clone();
                self.sides[side].alive[index] = false;
                let states = self.snapshots(catalog, &[id]);
                self.push_action(CombatAction::Death { minion: id }, states);

                if let Some(engine) = engine {
                    let ctx = EffectCtx {
                        seat: self.seats[side],
                        source: Some(id),
                        target: None,
                        event_target: None,
                        golden,
                        discover_level: None,
                    };
                    engine.fire_combat_death(self, rng, &card_id, ctx);
                    if keywords(catalog, &self.sides[side].minions[index]).contains(SOW)
                        && catalog.tokens.contains_key(SEEDLING_TOKEN)
                    {
                        let count = if golden { 2 } else { 1 };
                        for _ in 0..count {
                            if !self.has_room(catalog, side) {
                                break;
                            }
                            let instance =
                                new_minion(catalog, self.new_instance_id(), SEEDLING_TOKEN, false);
                            let Some(summoned) = self.append(catalog, side, instance) else {
                                break;
                            };
                            self.summon_count = self.summon_count.saturating_add(1);
                            engine.fire_ally_summon(self, rng, side, summoned);
                        }
                    }
                }

                let can_reborn = keywords(catalog, &self.sides[side].minions[index])
                    .contains(REBORN)
                    && !self.sides[side].minions[index].reborn_used
                    // 死亡 → 召唤 → 复生：召唤占满棋盘时复生作废。
                    && self.has_room(catalog, side);
                if can_reborn {
                    let board = &mut self.sides[side];
                    board.minions[index].reborn_used = true;
                    // 直接置当前生命为 1，允许与层和无关联（复生不走伤害入口）。
                    board.minions[index].stats.health.revive_to(1);
                    board.alive[index] = true;
                    // 保留本轮的剩余行动次数：本轮已行动过的随从复生后不会在同一轮
                    // 再次出手，未行动过的随从由指针位置决定本轮是否轮到。
                    let states = self.snapshots(catalog, &[id]);
                    self.push_action(CombatAction::Reborn { minion: id }, states);
                }

                // 复苏与回手：在亡语、播种、复生之后结算，保证同一次死亡事件
                // 只触发一次亡语、只复生或复苏一次。复生已把随从救回时不进入本块。
                if let Some(engine) = engine
                    && let Some((side_now, index_now)) = self.locate(id)
                    && side_now == side
                {
                    let still_dead = !self.sides[side].alive[index_now]
                        && self.sides[side].minions[index_now].stats.health.current <= 0;
                    if still_dead {
                        // 复苏：未使用过时按声明值增加永久攻防并复活，同时置
                        // `reborn_used`，避免同一次死亡再触发复生。
                        if !self.sides[side].minions[index_now].revive_used
                            && let Some((bonus_attack, bonus_health)) =
                                engine.death_revive(&card_id)
                        {
                            let minion = &mut self.sides[side].minions[index_now];
                            minion.stats.attack.add_permanent(bonus_attack);
                            minion.stats.health.add_permanent(bonus_health);
                            // 以分层和在场的成长、永久增量恢复生命，清除战斗伤害。
                            let restored = minion.stats.health.layers().max(1);
                            minion.stats.health.revive_to(restored);
                            minion.revive_used = true;
                            minion.reborn_used = true;
                            self.sides[side].alive[index_now] = true;
                            let states = self.snapshots(catalog, &[id]);
                            self.push_action(CombatAction::Revived { minion: id }, states);
                        }

                        // 回手：仍处于死亡态且携带 `Recall` 时克隆进手牌。
                        if !self.sides[side].alive[index_now]
                            && engine.death_recall(&card_id)
                            && self.hand_len[side] < self.hand_slots
                            && let Some(mut hand_minion) = self.minion(id).cloned()
                        {
                            // 清掉只在战场有效的临时层与一次性战斗标记；
                            // 成长、繁衍、传染、授予关键词、金色原样保留。
                            hand_minion.stats.attack.clear_turn();
                            hand_minion.stats.health.clear_turn();
                            hand_minion.stats.attack.set_aura(0);
                            hand_minion.stats.health.set_aura(0);
                            hand_minion
                                .stats
                                .attack
                                .revive_to(hand_minion.stats.attack.layers());
                            hand_minion
                                .stats
                                .health
                                .revive_to(hand_minion.stats.health.layers());
                            hand_minion.attacks_left = 0;
                            hand_minion.cannot_attack = false;
                            hand_minion.shield_used = false;
                            hand_minion.reborn_used = false;
                            hand_minion.revive_used = false;
                            hand_minion.stealth_broken = false;
                            // 回手后重新打出是一次新的入场，延迟战吼应可再次结算。
                            hand_minion.delayed_battlecry_used = false;
                            self.hand_len[side] = self.hand_len[side].saturating_add(1);
                            self.hand_minions.push(CombatHandMinion {
                                seat: self.seats[side],
                                side,
                                minion: hand_minion,
                            });
                            let states = self.snapshots(catalog, &[id]);
                            self.push_action(CombatAction::Recalled { minion: id }, states);
                        }
                    }
                }
                dead_ids.push((side, id));
            }

            if let Some(engine) = engine {
                for (side, dead_id) in &dead_ids {
                    engine.fire_combat_board_trigger(
                        self,
                        rng,
                        Trigger::OnFriendlyDeath,
                        *side,
                        Some(*dead_id),
                        Some(*dead_id),
                    );
                    engine.fire_combat_board_trigger(
                        self,
                        rng,
                        Trigger::OnEnemyDeath,
                        1 - *side,
                        Some(*dead_id),
                        Some(*dead_id),
                    );
                }
            }
            // 死亡队列处理完后，按记录顺序触发击杀类钩子；钩子造成的死亡回到循环结算。
            if let Some(engine) = engine {
                self.fire_kill_triggers(engine, rng);
            }
        }
        self.resolving = false;
    }
}

impl BattleField {
    /// 按「记录顺序 → `OnKill` → `OnAllyKill`」触发当前待处理的击杀钩子。
    ///
    /// `OnKill`：只对仍存活的击杀者触发，事件目标为被击杀者。
    /// `OnAllyKill`：对击杀方全部存活随从（含击杀者）触发，事件目标为被击杀者；
    /// 被击杀者必须是击杀者的敌方。互相击杀与击杀者已离场都不触发。
    fn fire_kill_triggers(&mut self, engine: &Engine, rng: &mut RngStreams) {
        let kills = std::mem::take(&mut self.pending_kills);
        for kill in kills {
            // 击杀者须仍存活：已离场或带复生但本场尚未复生的击杀者不触发。
            let killer_alive = self.locate(kill.killer).is_some_and(|(side, index)| {
                self.sides[side].alive[index]
                    && self.sides[side].minions[index].stats.health.current > 0
            });
            if kill.mutual || !killer_alive {
                continue;
            }
            let Some(victim_side) = self.locate(kill.victim).map(|(side, _)| side) else {
                continue;
            };

            engine.fire_combat_single(self, rng, kill.killer, Trigger::OnKill, Some(kill.victim));
            if victim_side != kill.killer_side {
                engine.fire_combat_board_trigger(
                    self,
                    rng,
                    Trigger::OnAllyKill,
                    kill.killer_side,
                    None,
                    Some(kill.victim),
                );
            }
        }
    }
}

/// 选择目标：最优攻击 → 嘲讽 → 全体。
///
/// 仍有潜行的防守方随从不在候选池内；若所有存活防守方都在潜行，退回完整
/// 候选池并固定选择最左侧，保证自动战斗必然推进。
fn pick_target(
    catalog: &Catalog,
    attacker: &MinionInstance,
    defender: &SideBoard,
    rng: &mut RngStreams,
) -> Option<usize> {
    let all_candidates: Vec<usize> = (0..defender.minions.len())
        .filter(|index| defender.alive[*index])
        .collect();
    if all_candidates.is_empty() {
        return None;
    }
    let candidates: Vec<usize> = all_candidates
        .iter()
        .copied()
        .filter(|index| !has_stealth(catalog, &defender.minions[*index]))
        .collect();
    if candidates.is_empty() {
        // 全部潜行：攻击最左侧，避免自动战斗卡死。
        return all_candidates.first().copied();
    }

    let attacker_keywords = keywords(catalog, attacker);
    let mut pool: Option<Vec<usize>> = None;

    if attacker_keywords.contains(OPTIMAL_ATTACK) {
        let attack = effective_attack(catalog, attacker);
        let killable: Vec<usize> = candidates
            .iter()
            .copied()
            .filter(|index| {
                let target = &defender.minions[*index];
                let shielded =
                    keywords(catalog, target).contains(DIVINE_SHIELD) && !target.shield_used;
                !shielded && attack >= defender.minions[*index].stats.health.current
            })
            .collect();
        if !killable.is_empty() {
            pool = Some(killable);
        }
    }

    let pool = pool.unwrap_or_else(|| {
        let taunts: Vec<usize> = candidates
            .iter()
            .copied()
            .filter(|index| keywords(catalog, &defender.minions[*index]).contains(TAUNT))
            .collect();
        if taunts.is_empty() {
            candidates
        } else {
            taunts
        }
    });

    let offset = rng.below(Stream::Combat, pool.len() as u64) as usize;
    pool.get(offset).copied()
}

/// 随从仍在场且当前生命 > 0；剧毒只对这样的目标生效。
fn still_standing(field: &BattleField, id: InstanceId) -> bool {
    field.locate(id).is_some_and(|(side, index)| {
        field.sides[side].alive[index] && field.sides[side].minions[index].stats.health.current > 0
    })
}

/// 结算一次攻击，并处理双方死亡。
fn resolve_attack(
    engine: Option<&Engine>,
    catalog: &Catalog,
    field: &mut BattleField,
    attacking_side: usize,
    attacker_index: usize,
    rng: &mut RngStreams,
) {
    let defending_side = 1 - attacking_side;
    let attacker_id = field.sides[attacking_side].minions[attacker_index].instance_id;

    let Some(target_index) = pick_target(
        catalog,
        &field.sides[attacking_side].minions[attacker_index],
        &field.sides[defending_side],
        rng,
    ) else {
        return;
    };

    let target_id = field.sides[defending_side].minions[target_index].instance_id;
    let attack = effective_attack(
        catalog,
        &field.sides[attacking_side].minions[attacker_index],
    );
    let attacker_keywords = keywords(
        catalog,
        &field.sides[attacking_side].minions[attacker_index],
    );

    // (a) 护盾或掉血。
    let mut damage = 0;
    let mut blocked = false;
    // 本次交换开始前已记录的击杀数；互相击杀只标记本次交换的攻守两条记录。
    let pending_before = field.pending_kills.len();
    {
        let target = &field.sides[defending_side].minions[target_index];
        let shielded = keywords(catalog, target).contains(DIVINE_SHIELD) && !target.shield_used;
        if shielded {
            field.sides[defending_side].minions[target_index].shield_used = true;
            blocked = true;
        } else {
            damage = field.apply_damage(
                catalog,
                target_id,
                attack,
                true,
                DamageSource::Minion(attacker_id),
            );
        }
    }

    // (b) 传染只在造成实际伤害时叠加。
    if attacker_keywords.contains(CONTAGION) && damage > 0 {
        let target = &mut field.sides[defending_side].minions[target_index];
        target.contagion = target.contagion.saturating_add(1);
        let states = field.snapshots(catalog, &[target_id]);
        field.push_action(CombatAction::ContagionApplied { target: target_id }, states);
    }

    // (b2) 剧毒：主伤害 > 0 且目标仍存活时补一次致死伤害。
    // 护盾抵消时主伤害为 0，不会进入这里。
    if attacker_keywords.contains(POISON) && damage > 0 && still_standing(field, target_id) {
        field.apply_damage(
            catalog,
            target_id,
            i32::MAX,
            true,
            DamageSource::Minion(attacker_id),
        );
        let states = field.snapshots(catalog, &[target_id]);
        field.push_action(CombatAction::Poisoned { target: target_id }, states);
    }

    // (c) 反击：无论目标是否被主伤害击杀，都按目标当前攻击力结算。
    let counter = effective_attack(catalog, &field.sides[defending_side].minions[target_index]);
    let mut counter_damage = 0;
    if counter > 0 {
        let attacker = &field.sides[attacking_side].minions[attacker_index];
        let shielded = keywords(catalog, attacker).contains(DIVINE_SHIELD) && !attacker.shield_used;
        if shielded {
            field.sides[attacking_side].minions[attacker_index].shield_used = true;
        } else {
            counter_damage = field.apply_damage(
                catalog,
                attacker_id,
                counter,
                true,
                DamageSource::Minion(target_id),
            );
        }
    }

    // (c2) 剧毒：反击伤害 > 0 且攻击者仍存活时补一次致死伤害。
    let target_poisonous = has_poison(catalog, &field.sides[defending_side].minions[target_index]);
    if target_poisonous && counter_damage > 0 && still_standing(field, attacker_id) {
        field.apply_damage(
            catalog,
            attacker_id,
            i32::MAX,
            true,
            DamageSource::Minion(target_id),
        );
        let states = field.snapshots(catalog, &[attacker_id]);
        field.push_action(
            CombatAction::Poisoned {
                target: attacker_id,
            },
            states,
        );
    }

    // (c3) 潜行随从的主动攻击本身即打破潜行，即使伤害被护盾抵消或攻击力为 0。
    if attacker_keywords.contains(STEALTH)
        && let Some(minion) = field.minion_mut(attacker_id)
    {
        minion.stealth_broken = true;
    }

    // (d) 只记录主动攻击：快照在反击结算之后抓取，已含双方最终血量。
    let main_states = field.snapshots(catalog, &[attacker_id, target_id]);
    field.push_action(
        CombatAction::Attack {
            attacker: attacker_id,
            target: target_id,
            damage,
            blocked,
        },
        main_states,
    );

    // (e) 同一次交换中攻守双方都被打死：本次交换的主伤害与反击击杀都记为互相击杀。
    // 必须在 `OnDealDamage` 之前判定：其效果会就地结算死亡并触发击杀钩子。
    let attacker_lethal = field.locate(attacker_id).is_some_and(|(side, index)| {
        field.sides[side].alive[index] && field.sides[side].minions[index].stats.health.current <= 0
    });
    let target_lethal = field.locate(target_id).is_some_and(|(side, index)| {
        field.sides[side].alive[index] && field.sides[side].minions[index].stats.health.current <= 0
    });
    if attacker_lethal && target_lethal {
        let end = field.pending_kills.len();
        for kill in &mut field.pending_kills[pending_before.min(end)..] {
            let attacker_kills_target = kill.killer == attacker_id && kill.victim == target_id;
            let target_kills_attacker = kill.killer == target_id && kill.victim == attacker_id;
            if attacker_kills_target || target_kills_attacker {
                kill.mutual = true;
            }
        }
    }

    // (f) 造成伤害后，在本次交换的死亡判定前触发 `OnDealDamage`。
    // 被主伤害击杀的目标不再触发自己的 `OnDealDamage`。
    if let Some(engine) = engine {
        if damage > 0 {
            engine.fire_combat_single(
                field,
                rng,
                attacker_id,
                Trigger::OnDealDamage,
                Some(target_id),
            );
        }
        if counter_damage > 0 && !target_lethal {
            engine.fire_combat_single(
                field,
                rng,
                target_id,
                Trigger::OnDealDamage,
                Some(attacker_id),
            );
        }
    }

    // 死亡队列：攻击方在前，防御方在后，各自按站位从左到右。
    field.resolve_deaths(engine, catalog, rng);

    // 推进攻击令牌（按实例编号定位，避免召唤导致的下标移动）。
    if let Some((side, index)) = field.locate(attacker_id) {
        field.sides[side].minions[index].attacks_left = field.sides[side].minions[index]
            .attacks_left
            .saturating_sub(1);
    }
}

/// 模拟一场战斗。
///
/// 输入不会被修改；`first` 为战先手方座位，用于决定行动顺序。
/// 本入口不结算卡牌效果，供纯数值测试使用；需要效果时使用 [`simulate_with`]。
pub fn simulate(
    catalog: &Catalog,
    attacker: &[MinionInstance],
    defender: &[MinionInstance],
    attacker_seat: Seat,
    defender_seat: Seat,
    first: Seat,
    rng: &mut RngStreams,
) -> CombatLog {
    simulate_inner(
        catalog,
        attacker,
        defender,
        attacker_seat,
        defender_seat,
        first,
        rng,
        None,
        CombatSetup::default(),
    )
}

/// 模拟一场战斗，并结算战斗开始、死亡、召唤等卡牌效果。
#[allow(clippy::too_many_arguments)]
pub fn simulate_with(
    engine: &Engine,
    attacker: &[MinionInstance],
    defender: &[MinionInstance],
    attacker_seat: Seat,
    defender_seat: Seat,
    first: Seat,
    rng: &mut RngStreams,
    setup: CombatSetup<'_>,
) -> CombatLog {
    simulate_inner(
        &engine.catalog,
        attacker,
        defender,
        attacker_seat,
        defender_seat,
        first,
        rng,
        Some(engine),
        setup,
    )
}

#[allow(clippy::too_many_arguments)]
fn simulate_inner(
    catalog: &Catalog,
    attacker: &[MinionInstance],
    defender: &[MinionInstance],
    attacker_seat: Seat,
    defender_seat: Seat,
    first: Seat,
    rng: &mut RngStreams,
    engine: Option<&Engine>,
    setup: CombatSetup<'_>,
) -> CombatLog {
    let mut field = BattleField::new(
        catalog,
        attacker,
        defender,
        attacker_seat,
        defender_seat,
        first,
        setup,
    );

    // 开场先结算生命已 ≤0 的随从（攻击方在前），复生者可重新入场。
    field.resolve_deaths(engine, catalog, rng);

    if let Some(engine) = engine {
        engine.fire_combat_start(&mut field, rng, &setup);
        field.resolve_deaths(Some(engine), catalog, rng);
    }

    let mut action_count: u16 = 0;
    let mut attacker_turn = first == attacker_seat;

    while action_count < MAX_ACTIONS {
        if field.sides[0].alive_count() == 0 || field.sides[1].alive_count() == 0 {
            break;
        }

        let current_index = if attacker_turn {
            field.sides[0].find_attacker()
        } else {
            field.sides[1].find_attacker()
        };
        let (acting_side, attacker_index) = match current_index {
            Some(index) => (if attacker_turn { 0 } else { 1 }, index),
            None => {
                let other_index = if attacker_turn {
                    field.sides[1].find_attacker()
                } else {
                    field.sides[0].find_attacker()
                };
                match other_index {
                    Some(index) => (if attacker_turn { 1 } else { 0 }, index),
                    None => break,
                }
            }
        };

        // 同一拍内连续结算本拍的攻击次数（普通 1 次、连击 2 次），
        // 两次攻击之间不切换到对方。
        let attacker_id = field.sides[acting_side].minions[attacker_index].instance_id;
        let attacks = field.sides[acting_side].minions[attacker_index].attacks_left;
        for _ in 0..attacks {
            if action_count >= MAX_ACTIONS {
                break;
            }
            // 召唤或复生可能移动实例下标，攻击前按实例编号重新定位。
            let Some((side, index)) = field.locate(attacker_id) else {
                break;
            };
            if side != acting_side
                || !field.is_alive(attacker_id)
                || field.sides[side].minions[index].attacks_left == 0
            {
                break;
            }
            if field.sides[0].alive_count() == 0 || field.sides[1].alive_count() == 0 {
                break;
            }
            resolve_attack(engine, catalog, &mut field, side, index, rng);
            action_count += 1;
        }
        attacker_turn = acting_side == 1;
    }

    let attacker_alive = field.sides[0].alive_count();
    let defender_alive = field.sides[1].alive_count();
    let winner = match (attacker_alive, defender_alive) {
        (0, 0) => None,
        (0, _) => Some(defender_seat),
        (_, 0) => Some(attacker_seat),
        // 行动数到达上限而双方仍有存活者：这不是同回合清场，由结算层按
        // §5.3（生命 → 累计伤害 → 座位）分出胜负。
        _ if action_count >= MAX_ACTIONS => None,
        _ => {
            let attacker_health = field.sides[0].total_health();
            let defender_health = field.sides[1].total_health();
            if attacker_health != defender_health {
                Some(if attacker_health > defender_health {
                    attacker_seat
                } else {
                    defender_seat
                })
            } else {
                Some(attacker_seat.min(defender_seat))
            }
        }
    };

    CombatLog {
        actions: std::mem::take(&mut field.actions),
        states: std::mem::take(&mut field.states),
        attacker_survivors: field.sides[0].survivors(),
        defender_survivors: field.sides[1].survivors(),
        winner,
        action_count,
        trigger_count: field.triggers,
        summon_count: field.summon_count,
        pool_spent: std::mem::take(&mut field.pool_spent),
        hand_copies: std::mem::take(&mut field.hand_copies),
        hand_minions: std::mem::take(&mut field.hand_minions),
        next_instance_id: field.next_instance_id,
        attacker_final: field.sides[0].minions.clone(),
        defender_final: field.sides[1].minions.clone(),
    }
}
