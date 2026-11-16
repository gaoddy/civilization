//! 只读视图驱动的确定性机器人决策。
//!
//! 输入只有服务端下发的 [`PlayerView`]（含服务端算好的 `affordances`）与静态卡表；
//! 本模块不链接规则引擎的判定函数、不读时钟与环境变量、不使用浮点。
//!
//! 优先级严格固定：
//! 1. 消费 `pendingChoices`；
//! 2. 打出一张可打出的手牌（买入一律入手牌，先腾出手牌空位）；
//! 3. 购买买得起的最贵商店槽位；
//! 4. 升时代；
//! 5. 刷新商店；
//! 6. `EndRecruit`。
//!
//! `targetMinionId` 只在手牌声明 `requiresTarget` 时预填己方第一个随从，
//! 其余情况不填：M1 机器人只做不需要目标的操作。

use protocol::PlayerViewSelf;
use protocol::intents::Intent;
use protocol::view::{PendingChoice, Phase, PlayerView};
use rule_engine::data::{CardType, Catalog};

/// 随从的商店买价：与规则引擎一致，随从固定 3 金。
const MINION_COST: u8 = 3;

/// 给出下一条意图；该座位已结束招募、不在招募阶段或无事可做时返回 `None`。
///
/// `seed` 只用于在三个发现候选间做确定性选择，不引入任何外部随机源。
pub fn next_intent(view: &PlayerView, catalog: &Catalog, seed: u64) -> Option<Intent> {
    if view.phase != Phase::Recruit {
        return None;
    }
    let me = &view.self_;
    if me.recruited {
        return None;
    }

    if let Some(intent) = pending_intent(me, seed) {
        return Some(intent);
    }
    if let Some(hand_index) = first_playable_hand(me) {
        return Some(Intent::PlayCard {
            hand_index,
            position: None,
            target_minion_id: prefill_target(me, hand_index),
        });
    }
    if let Some((slot, _cost)) = best_shop_slot(me, catalog) {
        return Some(Intent::BuyMinion { slot });
    }
    if me.affordances.can_upgrade_era {
        return Some(Intent::UpgradeEra);
    }
    if me.affordances.can_reroll {
        return Some(Intent::RerollShop);
    }
    if me.affordances.can_end_recruit {
        return Some(Intent::EndRecruit);
    }
    None
}

/// 待选队列的第一条。发现按 `seed` 取候选下标，进化固定取 0（与 M0 AI 一致），
/// 目标待选取棋盘最左侧随从。
fn pending_intent(me: &PlayerViewSelf, seed: u64) -> Option<Intent> {
    match me.pending_choices.first()? {
        PendingChoice::Discover { candidates } => {
            let index = if candidates.is_empty() {
                0
            } else {
                (seed % candidates.len() as u64) as usize
            };
            Some(Intent::ChooseDiscover {
                choice: i64::try_from(index).unwrap_or(0),
            })
        }
        PendingChoice::Evolve { .. } => Some(Intent::ChooseDiscover { choice: 0 }),
        PendingChoice::Target { .. } => Some(Intent::ChooseTarget {
            minion_id: me.board.first()?.minion_id.clone(),
        }),
    }
}

/// 买得起且买价最高的槽位；同价时取下标小的，保证确定性。
fn best_shop_slot(me: &PlayerViewSelf, catalog: &Catalog) -> Option<(i64, u8)> {
    let mut best: Option<(i64, u8)> = None;
    for affordance in &me.affordances.shop_slots {
        if !affordance.buyable {
            continue;
        }
        let Some(slot) = me.shop.iter().find(|slot| slot.index == affordance.index) else {
            continue;
        };
        let Some(card_id) = slot.card_id.as_deref() else {
            continue;
        };
        let cost = catalog.cards.get(card_id).map_or(0, shop_cost);
        match best {
            Some((best_slot, best_cost))
                if cost < best_cost || (cost == best_cost && affordance.index >= best_slot) => {}
            _ => best = Some((affordance.index, cost)),
        }
    }
    best
}

/// 商店买价：随从固定 3，效果牌取卡面 `cost`（缺失按 3）。
fn shop_cost(card: &rule_engine::data::Card) -> u8 {
    match card.card_type {
        CardType::Minion => MINION_COST,
        CardType::Effect => card.cost.unwrap_or(MINION_COST),
    }
}

/// 下标最小的可打出手牌。
fn first_playable_hand(me: &PlayerViewSelf) -> Option<i64> {
    me.affordances
        .hand_cards
        .iter()
        .filter(|affordance| affordance.playable)
        .map(|affordance| affordance.index)
        .min()
}

/// 出牌时预填的目标：视图声明该牌需要目标时取己方第一个随从。
///
/// 不预填的话，服务端在存在合法目标时一定会拒绝（`NO_VALID_TARGET`），机器人
/// 还要再走一轮「被拒 → 补选 → 重发」。棋盘为空说明该效果可落空，返回 `None`。
fn prefill_target(me: &PlayerViewSelf, hand_index: i64) -> Option<String> {
    let requires_target = me
        .hand
        .iter()
        .find(|card| card.index == hand_index)
        .is_some_and(|card| card.requires_target);
    if !requires_target {
        return None;
    }
    Some(me.board.first()?.minion_id.clone())
}

/// 收到服务端错误后的兜底意图。
///
/// 机器人只有在视图变化时才决策，被拒绝的意图不会带来新视图；没有兜底就会
/// 停在原地直到回合超时。仍在招募且可以结束时补发 `EndRecruit`，其余情况返回
/// `None`，由调用方决定是否降级。
pub fn recover_after_error(view: &PlayerView) -> Option<Intent> {
    if view.phase != Phase::Recruit {
        return None;
    }
    if view.self_.recruited || !view.self_.affordances.can_end_recruit {
        return None;
    }
    Some(Intent::EndRecruit)
}

/// 收到 `NO_VALID_TARGET` 后，给被拒意图补上己方第一个随从并重发。
///
/// 服务端要求「存在合法目标时」必须指定目标；机器人没有交互界面，按棋盘顺序取
/// 第一个己方随从作为确定性选择。棋盘为空说明该效果本可落空（服务端不会返回
/// 该错误），返回 `None` 交给兜底逻辑。
///
/// 补选的目标仍可能不合法：效果的目标池可按种族等过滤器收窄，首个己方随从未必
/// 在池内。因此**已经带目标的意图不再重发**——否则每次拒绝都会补同一个目标，
/// 形成「补选→拒绝→再补选」的无限空转，机器人永远结束不了招募。
pub fn retry_with_target(view: &PlayerView, rejected: &Intent) -> Option<Intent> {
    if view.phase != Phase::Recruit {
        return None;
    }
    let target = view.self_.board.first()?.minion_id.clone();
    match rejected {
        Intent::PlayCard {
            hand_index,
            target_minion_id,
            ..
        } => target_minion_id.is_none().then_some(Intent::PlayCard {
            hand_index: *hand_index,
            position: None,
            target_minion_id: Some(target),
        }),
        Intent::UseHeroPower {
            skill_id,
            target_minion_id,
        } => target_minion_id.is_none().then_some(Intent::UseHeroPower {
            skill_id: skill_id.clone(),
            target_minion_id: Some(target),
        }),
        _ => None,
    }
}
