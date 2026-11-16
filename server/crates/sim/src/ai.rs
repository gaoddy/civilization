//! 确定性 AI：只读取 `GameState` 与 `Catalog`，按固定优先级给出下一条意图。
//!
//! 所有比较都使用稳定键：商店槽位下标、实例编号、目录顺序（`BTreeMap` 的卡牌 id 顺序）。
//! 输出只依赖输入状态，不读时钟与环境变量，不使用浮点。

use std::collections::BTreeMap;

use rule_engine::combat::{effective_attack, effective_health};
use rule_engine::data::{Card, CardType, Catalog, Skill, Tribe};
use rule_engine::effects::{
    Duration, KeywordSpec, Op, Pick, Selector, Side, Trigger, parse_effects,
};
use rule_engine::engine::Engine;
use rule_engine::intent::{Intent, Reject, RejectCode};
use rule_engine::state::{GameState, MinionInstance, PendingChoice, PlayerState, Seat};

/// 手动刷新商店的金币价格：本回合首次刷新免费，之后每次 1 金币（与引擎一致）。
const REROLL_PRICE: u8 = 1;

/// 选择下一条意图；座位已出局时返回 `None`。
///
/// 优先级：待选 → 英雄技能 → 打出手牌随从（腾出手牌空位） → 购买 → 升时代 → 刷新 → `EndRecruit`。
/// 英雄技能在可支付且次数未用尽时使用，任何待选先消费；买入一律入手牌，
/// 因此先把手牌里的随从打上棋盘，避免手牌积压后买不了牌。
/// 1. 消费待选（发现 / 进化），否则无法结束招募；
/// 2. 英雄技能：可支付、本回合次数未用尽且有可收益目标时使用（见 [`hero_power_use`]）；
/// 3. 填满棋盘：手牌有随从且棋盘有空位时打出；棋盘与手牌都满时卖出本回合最弱的随从，
///    腾出棋盘空位让手牌能继续打出；
/// 4. 购买：买得起的最贵卡（金币 → 卡牌等级 → 槽位下标小的优先），手牌满则不买；
/// 5. 升时代：金币够付当前时代的 `era_costs` 时升级；
/// 6. 刷新：商店没有买得起的随从时刷新一次（每回合最多触发一次：刷新后商店仍买不起时，
///    剩余金币已不足以再付一次刷新费，AI 转向升时代或结束）；
/// 7. `EndRecruit`。
pub fn choose(state: &GameState, seat: Seat, catalog: &Catalog) -> Option<Intent> {
    plan(state, seat, catalog, &[])
}

/// 与 [`choose`] 相同的优先级；`blocked` 中的意图被跳过（本轮曾被引擎拒绝、状态未变）。
///
/// 跳过只会改变选中哪一步，不会改变比较规则，因此结果仍是确定的。
pub fn plan(
    state: &GameState,
    seat: Seat,
    catalog: &Catalog,
    blocked: &[Intent],
) -> Option<Intent> {
    let player = state.players.get(usize::from(seat))?;
    if player.eliminated {
        return None;
    }
    let candidates = [
        choose_pending(state, player, catalog),
        choose_hero_power(player, catalog),
        choose_fill_board(state, player, catalog),
        choose_buy(state, player, catalog),
        can_upgrade(player, catalog).then_some(Intent::UpgradeEra),
        choose_reroll(player, catalog),
    ];
    for intent in candidates.into_iter().flatten() {
        if !blocked.contains(&intent) {
            return Some(intent);
        }
    }
    Some(Intent::EndRecruit)
}

/// 执行 AI 的一条意图；返回 `Ok(None)` 表示无可执行意图，`Err` 为拒绝（状态不变）。
pub fn play_step(
    engine: &mut Engine,
    state: &mut GameState,
    seat: Seat,
) -> Result<Option<Intent>, Reject> {
    let Some(intent) = choose(state, seat, &engine.catalog) else {
        return Ok(None);
    };
    engine.apply(state, seat, intent.clone())?;
    Ok(Some(intent))
}

/// 购买价格：随从固定 3 金币，效果牌取 `cost`。
pub fn buy_cost(card: &Card) -> u8 {
    match card.card_type {
        CardType::Minion => 3,
        CardType::Effect => card.cost.unwrap_or(3),
    }
}

fn choose_pending(state: &GameState, player: &PlayerState, catalog: &Catalog) -> Option<Intent> {
    let pending = state
        .pending
        .iter()
        .find(|pending| pending_seat(pending) == player.seat)?;
    match pending {
        PendingChoice::Discover { candidates, .. } => Some(Intent::ChooseDiscover {
            choice: discover_choice(player, candidates, catalog),
        }),
        // 进化选项：0 = +2/+2，确定性取首项。
        PendingChoice::Evolve { .. } => Some(Intent::ChooseDiscover { choice: 0 }),
    }
}

/// 发现选项：优先能凑成三连的候选（手牌 + 棋盘已有 2 张）；其次卡牌等级高者；
/// 再按候选顺序靠前者。
pub fn discover_choice(player: &PlayerState, candidates: &[String], catalog: &Catalog) -> u8 {
    let copies = |card_id: &str| {
        player
            .hand
            .iter()
            .filter(|card| card.card_id == card_id)
            .count()
            + player
                .board
                .iter()
                .filter(|minion| minion.card_id == card_id)
                .count()
    };
    let key = |card_id: &str| {
        let level = catalog.cards.get(card_id).map_or(0, |card| card.level);
        (copies(card_id) >= 2, level)
    };
    let mut best: Option<(usize, &str)> = None;
    for (index, card_id) in candidates.iter().enumerate() {
        let better = match best {
            None => true,
            Some((_, best_id)) => key(card_id) > key(best_id),
        };
        if better {
            best = Some((index, card_id.as_str()));
        }
    }
    best.map_or(0, |(index, _)| u8::try_from(index).unwrap_or(0))
}

fn pending_seat(pending: &PendingChoice) -> Seat {
    match pending {
        PendingChoice::Discover { seat, .. } | PendingChoice::Evolve { seat, .. } => *seat,
    }
}

fn choose_hero_power(player: &PlayerState, catalog: &Catalog) -> Option<Intent> {
    let hero_id = player.hero.as_deref()?;
    let hero = catalog.cards.get(hero_id)?;
    for skill_id in &hero.skills {
        let Some(skill) = catalog.skills.get(skill_id) else {
            continue;
        };
        let Some(cost) = skill.cost else {
            continue;
        };
        let limit = skill.uses_per_turn.unwrap_or(1);
        let used = player.hero_uses.get(skill_id).copied().unwrap_or(0);
        if used >= limit || player.gold < cost {
            continue;
        }
        let Some(target) = hero_power_use(player, catalog, skill) else {
            continue;
        };
        return Some(Intent::UseHeroPower {
            skill_id: skill_id.clone(),
            target,
        });
    }
    None
}

/// 英雄技能的使用与目标；返回 `None` 表示本回合不应使用该技能（没有能收益的目标）。
///
/// 按声明顺序检查技能的 `OnActivate` 效果，取第一个可识别的目标来源：
///
/// - `Discover` 且选择器为 `self`（如 `pool=tribe`）：目标只表示流派偏好，取棋盘上最强的
///   随从；棋盘为空时为 `None`。
/// - 选择器为 `target`：取棋盘上最强的随从。
/// - 友方单体（`friendly:single`，无筛选）、永久生效的关键词授予：取棋盘上（实例编号
///   升序）第一个尚未拥有该关键词的随从；全部已有时为 `None`。
/// - 友方单体（`friendly:single`，无筛选）的成长：取成长层数最少（同层取实例编号小者）
///   的随从；棋盘为空时为 `None`。
///
/// `duration=combat` 的授予不会写入 `MinionInstance::granted`，不能作为「何时使用」的
/// 判据，因此不作为目标来源；没有可识别目标的技能不提议使用。
fn hero_power_use(player: &PlayerState, catalog: &Catalog, skill: &Skill) -> Option<Option<u64>> {
    let effects = parse_effects(&skill.effects).ok()?;
    for effect in &effects {
        if effect.trigger != Trigger::OnActivate {
            continue;
        }
        if effect.op == Op::Discover && effect.selector == Selector::SelfCard {
            if let Some(target) = strongest_target(player, catalog) {
                return Some(Some(target));
            }
            return None;
        }
        if effect.selector == Selector::Target {
            if let Some(target) = strongest_target(player, catalog) {
                return Some(Some(target));
            }
            return None;
        }
        let Selector::Side {
            side,
            pick,
            filters,
        } = &effect.selector
        else {
            continue;
        };
        if *side != Side::Friendly
            || *pick != Pick::Single
            || !filters.is_empty()
            || effect.params.duration != Duration::Permanent
        {
            continue;
        }
        if effect.op == Op::GrantKeyword
            && let Some(KeywordSpec::Keyword(keyword, _)) = &effect.params.keyword
        {
            let target = player
                .board
                .iter()
                .filter(|minion| !rule_engine::combat::keywords(catalog, minion).contains(keyword))
                .min_by_key(|minion| minion.instance_id);
            if let Some(minion) = target {
                return Some(Some(minion.instance_id));
            }
            return None;
        }
        if effect.op == Op::Growth {
            let target = player
                .board
                .iter()
                .min_by_key(|minion| (minion.stats.attack.growth, minion.instance_id));
            if let Some(minion) = target {
                return Some(Some(minion.instance_id));
            }
            return None;
        }
    }
    None
}

/// 棋盘上最强的随从（有效攻防之和降序，同强度取实例编号小者）；棋盘为空时为 `None`。
fn strongest_target(player: &PlayerState, catalog: &Catalog) -> Option<u64> {
    player
        .board
        .iter()
        .max_by_key(|minion| {
            (
                strength(catalog, minion),
                std::cmp::Reverse(minion.instance_id),
            )
        })
        .map(|minion| minion.instance_id)
}

/// 购买候选的排序键：价格 → 卡牌等级 → 槽位下标小者优先。
type CandidateKey = (u8, u8, std::cmp::Reverse<u8>);

/// 商店中买得起且能放入手牌的候选里按降序键排列后的第一个槽位：
/// 金币 → 卡牌等级 → 槽位下标小的优先。买入一律入手牌，因此只看手牌是否有空位。
fn best_affordable(
    state: &GameState,
    player: &PlayerState,
    catalog: &Catalog,
    gold: u8,
) -> Option<u8> {
    if player.hand.len() >= catalog.config.hand_slots {
        return None;
    }
    let mut best: Option<(CandidateKey, u8)> = None;
    for (slot_index, slot) in player.shop.iter().enumerate() {
        let Some(slot) = slot else { continue };
        let Some(card) = catalog.cards.get(&slot.card_id) else {
            continue;
        };
        if card.is_hero {
            continue;
        }
        if card.card_type == CardType::Minion && state.pool.get(&card.id).copied().unwrap_or(0) == 0
        {
            continue;
        }
        let cost = buy_cost(card);
        if gold < cost {
            continue;
        }
        let Ok(slot_u8) = u8::try_from(slot_index) else {
            continue;
        };
        let key = (cost, card.level, std::cmp::Reverse(slot_u8));
        if best.is_none_or(|(current, _)| key > current) {
            best = Some((key, slot_u8));
        }
    }
    best.map(|(_, slot)| slot)
}

fn choose_buy(state: &GameState, player: &PlayerState, catalog: &Catalog) -> Option<Intent> {
    best_affordable(state, player, catalog, player.gold).map(|slot| Intent::BuyMinion { slot })
}

/// 棋盘有空位时把手牌里的随从打上棋盘；手牌已满导致买不了牌时，
/// 卖出本回合最弱的随从腾出棋盘空位，让手牌能继续打出。
fn choose_fill_board(state: &GameState, player: &PlayerState, catalog: &Catalog) -> Option<Intent> {
    let minion_in_hand = player.hand.iter().enumerate().find(|(_, card)| {
        catalog
            .cards
            .get(&card.card_id)
            .is_some_and(|card| card.card_type == CardType::Minion)
    });
    let (index, _) = minion_in_hand?;
    if player.board.len() < catalog.config.board_slots {
        return u8::try_from(index).ok().map(|hand_index| Intent::PlayCard {
            hand_index,
            position: None,
            target: None,
        });
    }
    if player.hand.len() < catalog.config.hand_slots {
        return None;
    }
    let (instance_id, golden) = weakest_minion(player, catalog)?;
    let gold_after = player.gold.saturating_add(if golden { 3 } else { 1 });
    best_affordable(state, player, catalog, gold_after)?;
    Some(Intent::SellMinion {
        minion: instance_id,
    })
}

/// 棋盘上最弱的随从：(实例编号, 是否金色)。先按强度升序，同强度取实例编号小者。
fn weakest_minion(player: &PlayerState, catalog: &Catalog) -> Option<(u64, bool)> {
    player
        .board
        .iter()
        .min_by_key(|minion| (strength(catalog, minion), minion.instance_id))
        .map(|minion| (minion.instance_id, minion.golden))
}

/// 随从综合强度：有效攻击 + 有效生命（金色按 2 倍计入），只用于比较。
pub fn strength(catalog: &Catalog, minion: &MinionInstance) -> i32 {
    let total = effective_attack(catalog, minion).saturating_add(effective_health(catalog, minion));
    if minion.golden {
        total.saturating_mul(2)
    } else {
        total
    }
}

fn can_upgrade(player: &PlayerState, catalog: &Catalog) -> bool {
    let era_index = player.era as usize - 1;
    let Some(cost) = catalog.config.era_costs.get(era_index) else {
        return false;
    };
    u16::from(player.gold) >= *cost
}

/// 商店没有任何买得起的随从时提议刷新；价格为本回合首次刷新免费、之后每次 1 金币。
///
/// 刷新后商店若仍然买不起，商店里的随从价格都高于剩余金币，AI 会转向升时代或结束，
/// 因此每回合最多连续刷新一次；引擎拒绝时驱动方会跳过该意图重新规划。
fn choose_reroll(player: &PlayerState, catalog: &Catalog) -> Option<Intent> {
    let any_affordable = player.shop.iter().flatten().any(|slot| {
        catalog.cards.get(&slot.card_id).is_some_and(|card| {
            !card.is_hero && card.card_type == CardType::Minion && buy_cost(card) <= player.gold
        })
    });
    if any_affordable {
        return None;
    }
    let price = if player.first_refresh_used {
        REROLL_PRICE
    } else {
        0
    };
    (player.gold >= price).then_some(Intent::RerollShop)
}

/// 拒绝是否属于「AI 计划与引擎判定不一致」（价格、空间、次数、目标）：驱动方据此丢弃
/// 当前意图、换下一条候选重新规划，而不是判定模拟失败。阶段错误不属于此类，驱动方会直接
/// 报错退出——出现阶段错误说明驱动顺序与引擎约定不一致。
pub fn planning_reject(code: RejectCode) -> bool {
    matches!(
        code,
        RejectCode::NotEnoughGold
            | RejectCode::ShopSlotEmpty
            | RejectCode::NoSpace
            | RejectCode::NoSuchMinion
            | RejectCode::AlreadyMaxEra
            | RejectCode::CardNotInHand
            | RejectCode::HeroPowerLimit
            | RejectCode::NoValidTarget
            | RejectCode::TargetNotAllowed
            | RejectCode::InvalidChoice
    )
}

/// 流派判定：统计阵容随从的阵营，取数量最多的流派；并列时学者优先，再按流派声明顺序
/// （`Tribe` 的 `Ord`：人类 → 机械 → 生物 → 植物 → 病毒 → 学者 → 通用）取靠前者。
/// 通用随从与未知卡牌不参与统计；没有可统计随从时返回 `通用`。
pub fn dominant_tribe<'a>(board: impl IntoIterator<Item = &'a str>, catalog: &Catalog) -> Tribe {
    let mut counts: BTreeMap<Tribe, usize> = BTreeMap::new();
    for card_id in board {
        let Some(card) = catalog.cards.get(card_id) else {
            continue;
        };
        if card.tribe == Tribe::General {
            continue;
        }
        *counts.entry(card.tribe).or_insert(0) += 1;
    }
    let mut best: Option<(Tribe, usize)> = None;
    for (&tribe, &count) in &counts {
        let better = match best {
            None => true,
            Some((current_tribe, current_count)) => {
                if count != current_count {
                    count > current_count
                } else {
                    match (tribe == Tribe::Scholar, current_tribe == Tribe::Scholar) {
                        (true, false) => true,
                        (false, true) => false,
                        _ => tribe < current_tribe,
                    }
                }
            }
        };
        if better {
            best = Some((tribe, count));
        }
    }
    best.map_or(Tribe::General, |(tribe, _)| tribe)
}
