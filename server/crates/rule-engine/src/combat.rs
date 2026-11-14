//! 战斗行动循环与关键词判定。
//!
//! 本模块只做纯数值与关键词结算，不触发战吼、亡语、光环等效果
//! （由后续任务接入）。所有输入都是只读快照，`simulate` 在克隆上结算，
//! 调用方的棋盘不会被修改；随机性只来自 [`Stream::Combat`]。
//!
//! 规则口径见 `docs/versions/v0.1/product/rules/core-rules.md` §5.1–§5.4
//! 与 `keywords-and-mechanics.md` §1、§2.3–§2.4、§10。

use std::collections::BTreeSet;

use crate::data::{Catalog, Tribe};
use crate::effects::Duration;
use crate::rng::{RngStreams, Stream};
use crate::state::{InstanceId, MinionInstance, Modifier, Seat};

/// 每场战斗的行动上限，防止死亡循环。
const MAX_ACTIONS: u16 = 200;

/// 关键词：连击。
const WINDFURY: &str = "连击";
/// 关键词：嘲讽。
const TAUNT: &str = "嘲讽";
/// 关键词：护盾。
const DIVINE_SHIELD: &str = "护盾";
/// 关键词：复生。
const REBORN: &str = "复生";
/// 关键词：传染。
const CONTAGION: &str = "传染";
/// 关键词：最优攻击。
const OPTIMAL_ATTACK: &str = "最优攻击";

/// 战斗中的一条动作记录。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatAction {
    /// 一次主伤害结算；反击不单独记录。
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
}

/// 一场战斗的结算结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatLog {
    pub actions: Vec<CombatAction>,
    pub attacker_survivors: Vec<InstanceId>,
    pub defender_survivors: Vec<InstanceId>,
    pub winner: Option<Seat>,
    pub action_count: u16,
}

/// 卡面基础攻击力；金色翻倍。
fn base_attack(catalog: &Catalog, minion: &MinionInstance) -> i32 {
    let base = catalog
        .cards
        .get(&minion.card_id)
        .and_then(|card| card.attack)
        .unwrap_or(0);
    if minion.golden {
        base.saturating_mul(2)
    } else {
        base
    }
}

/// 卡面基础生命值；金色翻倍。
fn base_health(catalog: &Catalog, minion: &MinionInstance) -> i32 {
    let base = catalog
        .cards
        .get(&minion.card_id)
        .and_then(|card| card.health)
        .unwrap_or(0);
    if minion.golden {
        base.saturating_mul(2)
    } else {
        base
    }
}

fn is_scholar(catalog: &Catalog, minion: &MinionInstance) -> bool {
    catalog
        .cards
        .get(&minion.card_id)
        .map(|card| card.tribe == Tribe::Scholar)
        .unwrap_or(false)
}

/// 当前攻击力；不包含光环。
pub fn effective_attack(catalog: &Catalog, minion: &MinionInstance) -> i32 {
    effective_attack_with(catalog, minion, &[])
}

/// 当前攻击力，含外部传入的光环修正。
///
/// 学者在最后一步被强制为 0，下限为 0。
pub fn effective_attack_with(
    catalog: &Catalog,
    minion: &MinionInstance,
    auras: &[Modifier],
) -> i32 {
    let mut value = base_attack(catalog, minion).saturating_add(minion.growth);
    for modifier in &minion.mods {
        value = value.saturating_add(modifier.attack);
    }
    for aura in auras {
        value = value.saturating_add(aura.attack);
    }
    if is_scholar(catalog, minion) {
        value = 0;
    }
    value.max(0)
}

/// 当前生命值；不包含光环。
pub fn effective_health(catalog: &Catalog, minion: &MinionInstance) -> i32 {
    effective_health_with(catalog, minion, &[])
}

/// 当前生命值，含外部传入的光环修正；下限为 0。
pub fn effective_health_with(
    catalog: &Catalog,
    minion: &MinionInstance,
    auras: &[Modifier],
) -> i32 {
    let mut value = base_health(catalog, minion)
        .saturating_add(minion.growth)
        .saturating_add(minion.growth_health_only);
    for modifier in &minion.mods {
        value = value.saturating_add(modifier.health);
    }
    for aura in auras {
        value = value.saturating_add(aura.health);
    }
    value.max(0)
}

/// 随从当前生效的关键词集合。
///
/// 由所引用技能的 `keyword` 字段派生，并并入尚未失效的授予关键词；
/// `Instant` 授予在战斗前就已结算完毕，视为不存在。未知关键词原样保留。
pub fn keywords(catalog: &Catalog, minion: &MinionInstance) -> BTreeSet<String> {
    let mut result = BTreeSet::new();
    if let Some(card) = catalog.cards.get(&minion.card_id) {
        for skill_id in &card.skills {
            if let Some(keyword) = catalog
                .skills
                .get(skill_id)
                .and_then(|skill| skill.keyword.as_ref())
            {
                result.insert(keyword.clone());
            }
        }
    }
    for (keyword, duration) in &minion.granted {
        if *duration != Duration::Instant {
            result.insert(keyword.clone());
        }
    }
    result
}

/// 战斗开始时该随从拥有的行动令牌数。
///
/// 「连击」对学者不生效（学者攻击力为 0），学者固定只有 1 个令牌。
fn starting_tokens(catalog: &Catalog, minion: &MinionInstance) -> u8 {
    if minion.cannot_attack {
        0
    } else if is_scholar(catalog, minion) {
        1
    } else if keywords(catalog, minion).contains(WINDFURY) {
        2
    } else {
        1
    }
}

/// 一方的战斗内状态。输入随从被克隆后在这里结算。
struct SideBoard {
    minions: Vec<MinionInstance>,
    health: Vec<i32>,
    alive: Vec<bool>,
    starting_tokens: Vec<u8>,
    pointer: usize,
}

impl SideBoard {
    fn new(catalog: &Catalog, board: &[MinionInstance]) -> Self {
        let mut minions = board.to_vec();
        let mut health = Vec::with_capacity(minions.len());
        let mut alive = Vec::with_capacity(minions.len());
        let mut token_counts = Vec::with_capacity(minions.len());
        for minion in &mut minions {
            let current = effective_health(catalog, minion);
            let tokens = starting_tokens(catalog, minion);
            minion.attacks_left = tokens;
            health.push(current);
            // 战斗开始前生命已 ≤0 的随从先视为在场，由开场死亡结算统一处理。
            alive.push(true);
            token_counts.push(tokens);
        }
        Self {
            minions,
            health,
            alive,
            starting_tokens: token_counts,
            pointer: 0,
        }
    }

    fn alive_count(&self) -> usize {
        self.alive.iter().filter(|alive| **alive).count()
    }

    fn total_health(&self) -> i32 {
        self.health
            .iter()
            .zip(&self.alive)
            .filter(|(_, alive)| **alive)
            .fold(0i32, |sum, (health, _)| sum.saturating_add(*health))
    }

    fn survivors(&self) -> Vec<InstanceId> {
        self.minions
            .iter()
            .zip(&self.alive)
            .filter(|(_, alive)| **alive)
            .map(|(minion, _)| minion.instance_id)
            .collect()
    }

    /// 返回当前应出手的随从下标；跳过已死亡或无令牌者。
    fn find_attacker(&mut self) -> Option<usize> {
        while self.pointer < self.minions.len() {
            if self.alive[self.pointer] && self.minions[self.pointer].attacks_left > 0 {
                return Some(self.pointer);
            }
            self.pointer += 1;
        }
        None
    }
}

/// 选择目标：最优攻击 → 嘲讽 → 全体。
fn pick_target(
    catalog: &Catalog,
    attacker: &MinionInstance,
    defender: &SideBoard,
    rng: &mut RngStreams,
) -> Option<usize> {
    let candidates: Vec<usize> = (0..defender.minions.len())
        .filter(|index| defender.alive[*index])
        .collect();
    if candidates.is_empty() {
        return None;
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
                !shielded && attack >= defender.health[*index]
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

/// 结算一次攻击，并处理双方死亡。
fn resolve_attack(
    catalog: &Catalog,
    attacking: &mut SideBoard,
    defending: &mut SideBoard,
    attacker_index: usize,
    rng: &mut RngStreams,
    actions: &mut Vec<CombatAction>,
) {
    let Some(target_index) =
        pick_target(catalog, &attacking.minions[attacker_index], defending, rng)
    else {
        return;
    };

    let attacker_id = attacking.minions[attacker_index].instance_id;
    let target_id = defending.minions[target_index].instance_id;
    let attack = effective_attack(catalog, &attacking.minions[attacker_index]);
    let attacker_keywords = keywords(catalog, &attacking.minions[attacker_index]);

    // (a) 护盾或掉血。
    let mut damage = 0;
    let mut blocked = false;
    {
        let target = &mut defending.minions[target_index];
        let shielded = keywords(catalog, target).contains(DIVINE_SHIELD) && !target.shield_used;
        if shielded {
            target.shield_used = true;
            blocked = true;
        } else {
            damage = attack;
            defending.health[target_index] = defending.health[target_index].saturating_sub(damage);
        }
    }

    // (b) 传染只在造成实际伤害时叠加。
    if attacker_keywords.contains(CONTAGION) && damage > 0 {
        let target = &mut defending.minions[target_index];
        target.contagion = target.contagion.saturating_add(1);
        actions.push(CombatAction::ContagionApplied { target: target_id });
    }

    // (c) 反击：目标在主伤害后仍存活（当前生命 > 0）才反击。
    // 反击造成的伤害用攻击者的护盾规则结算；被护盾抵消时不额外记录事件。
    let counter = if defending.health[target_index] > 0 {
        effective_attack(catalog, &defending.minions[target_index])
    } else {
        0
    };
    let mut counter_damage = 0;
    if counter > 0 {
        let attacker = &mut attacking.minions[attacker_index];
        let shielded = keywords(catalog, attacker).contains(DIVINE_SHIELD) && !attacker.shield_used;
        if shielded {
            attacker.shield_used = true;
        } else {
            counter_damage = counter;
            attacking.health[attacker_index] =
                attacking.health[attacker_index].saturating_sub(counter);
        }
    }

    // (d) 主伤害事件先于反击事件与死亡处理记录。
    actions.push(CombatAction::Attack {
        attacker: attacker_id,
        target: target_id,
        damage,
        blocked,
    });
    if counter_damage > 0 {
        actions.push(CombatAction::Attack {
            attacker: target_id,
            target: attacker_id,
            damage: counter_damage,
            blocked: false,
        });
    }

    // 死亡队列：攻击方在前，防御方在后，各自按站位从左到右。
    process_deaths(catalog, attacking, defending, actions);

    // 推进攻击令牌。
    let attacker = &mut attacking.minions[attacker_index];
    attacker.attacks_left = attacker.attacks_left.saturating_sub(1);
}

/// 处理当前生命 ≤ 0 的随从：攻击方在前、防御方在后，各侧从左到右。
///
/// 带「复生」且本场未触发过的随从以 1 点生命复活并重置攻击令牌。
fn process_deaths(
    catalog: &Catalog,
    attacking: &mut SideBoard,
    defending: &mut SideBoard,
    actions: &mut Vec<CombatAction>,
) {
    let mut dead = Vec::new();
    for index in 0..attacking.minions.len() {
        if attacking.alive[index] && attacking.health[index] <= 0 {
            dead.push((true, index));
        }
    }
    for index in 0..defending.minions.len() {
        if defending.alive[index] && defending.health[index] <= 0 {
            dead.push((false, index));
        }
    }
    for (is_attacker_side, index) in dead {
        let board = if is_attacker_side {
            &mut *attacking
        } else {
            &mut *defending
        };
        let id = board.minions[index].instance_id;
        actions.push(CombatAction::Death { minion: id });
        let can_reborn = keywords(catalog, &board.minions[index]).contains(REBORN)
            && !board.minions[index].reborn_used;
        if can_reborn {
            board.minions[index].reborn_used = true;
            board.health[index] = 1;
            board.alive[index] = true;
            board.minions[index].attacks_left = board.starting_tokens[index];
            actions.push(CombatAction::Reborn { minion: id });
        } else {
            board.alive[index] = false;
        }
    }
}

/// 模拟一场战斗。
///
/// 输入不会被修改；`first` 为战先手方座位，用于决定行动顺序。
pub fn simulate(
    catalog: &Catalog,
    attacker: &[MinionInstance],
    defender: &[MinionInstance],
    attacker_seat: Seat,
    defender_seat: Seat,
    first: Seat,
    rng: &mut RngStreams,
) -> CombatLog {
    let mut attacker_side = SideBoard::new(catalog, attacker);
    let mut defender_side = SideBoard::new(catalog, defender);
    let mut actions = Vec::new();
    // 开场先结算生命已 ≤0 的随从（攻击方在前），复生者可重新入场。
    process_deaths(
        catalog,
        &mut attacker_side,
        &mut defender_side,
        &mut actions,
    );
    let mut action_count: u16 = 0;
    let mut attacker_turn = first == attacker_seat;

    while action_count < MAX_ACTIONS {
        if attacker_side.alive_count() == 0 || defender_side.alive_count() == 0 {
            break;
        }

        let current_index = if attacker_turn {
            attacker_side.find_attacker()
        } else {
            defender_side.find_attacker()
        };
        let (acting_is_attacker, attacker_index) = match current_index {
            Some(index) => (attacker_turn, index),
            None => {
                let other_index = if attacker_turn {
                    defender_side.find_attacker()
                } else {
                    attacker_side.find_attacker()
                };
                match other_index {
                    Some(index) => (!attacker_turn, index),
                    None => break,
                }
            }
        };

        if acting_is_attacker {
            resolve_attack(
                catalog,
                &mut attacker_side,
                &mut defender_side,
                attacker_index,
                rng,
                &mut actions,
            );
        } else {
            resolve_attack(
                catalog,
                &mut defender_side,
                &mut attacker_side,
                attacker_index,
                rng,
                &mut actions,
            );
        }
        action_count += 1;
        attacker_turn = !acting_is_attacker;
    }

    let attacker_alive = attacker_side.alive_count();
    let defender_alive = defender_side.alive_count();
    let winner = match (attacker_alive, defender_alive) {
        (0, 0) => None,
        (0, _) => Some(defender_seat),
        (_, 0) => Some(attacker_seat),
        // 行动数到达上限而双方仍有存活者：按规则 §8.3 视为平局，
        // 由结算层按 §5.3（生命 → 累计伤害 → 座位）判定。
        _ if action_count >= MAX_ACTIONS => None,
        _ => {
            let attacker_health = attacker_side.total_health();
            let defender_health = defender_side.total_health();
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
        actions,
        attacker_survivors: attacker_side.survivors(),
        defender_survivors: defender_side.survivors(),
        winner,
        action_count,
    }
}
