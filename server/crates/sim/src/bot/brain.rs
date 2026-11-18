//! 只读视图驱动的确定性机器人决策。
//!
//! 输入只有服务端下发的 [`PlayerView`]（含服务端算好的 `affordances`）与静态卡表；
//! 本模块不链接规则引擎的判定函数、不读时钟与环境变量、不使用浮点。
//!
//! 优先级严格固定：
//! 1. 消费 `pendingChoices`（跳过本回合已被拒绝的目标）；
//! 2. 本回合被拒绝的出手达到上限且可结束招募时直接 `EndRecruit`；
//! 3. 打出一张可打出且未被拒绝的手牌（买入一律入手牌，先腾出手牌空位）；
//! 4. 购买买得起的最贵商店槽位；
//! 5. 升时代；
//! 6. 刷新商店；
//! 7. `EndRecruit`。
//!
//! 视图只告诉机器人「这张牌需要目标」，不给合法目标池（池子会按玩法、分类等收窄）。
//! 因此 `targetMinionId` 只在手牌声明 `requiresTarget` 时预填己方第一个随从；被
//! `NO_VALID_TARGET` 拒绝后沿棋盘静默换下一个随从，整盘都不合法时才由
//! [`BlockedTargets`] 把这张牌本回合屏蔽。`ChooseTarget` 同样按候选轮换。

use std::collections::BTreeSet;

use protocol::PlayerViewSelf;
use protocol::intents::Intent;
use protocol::view::{PendingChoice, Phase, PlayerView};
use rule_engine::data::{CardType, Catalog};

/// 随从的商店买价：与规则引擎一致，随从固定 3 金。
const MINION_COST: u8 = 3;

/// 本回合被拒绝的定向出手达到该数量后，不再尝试出牌，直接结束招募。
///
/// 与无头客户端的 `BLOCKED_LIMIT` 一致：棋盘满时手牌没有别的出路，继续试错只会
/// 每来一次视图就空转一轮，直到回合超时。
pub const BLOCKED_LIMIT: usize = 3;

/// 本回合内已确认不合法的目标选择。
///
/// 机器人看不到效果的目标池，只能靠服务端拒绝学习。定向出手会先沿棋盘换下一个
/// 己方随从静默重试；整盘都没有合法目标时才把这张牌本回合屏蔽。`ChooseTarget`
/// 则逐个轮换候选，全部失败时才放弃。
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BlockedTargets {
    /// 本回合放弃的出手，元素为手牌下标。
    hand: BTreeSet<i64>,
    /// 本回合已被拒绝的 `ChooseTarget` 候选。
    minions: BTreeSet<String>,
}

impl BlockedTargets {
    /// 本回合该手牌是否已放弃。
    pub fn blocks_hand(&self, hand_index: i64) -> bool {
        self.hand.contains(&hand_index)
    }

    /// 本回合已放弃的出手数量。
    pub fn hand_count(&self) -> usize {
        self.hand.len()
    }

    /// 本回合放弃这张手牌。
    pub fn block_hand(&mut self, hand_index: i64) {
        self.hand.insert(hand_index);
    }

    /// 本回合放弃这个 `ChooseTarget` 候选。
    pub fn block_minion(&mut self, minion_id: String) {
        self.minions.insert(minion_id);
    }

    /// 棋盘上是否还有未被拒绝的候选目标。
    pub fn has_unblocked_minion(&self, me: &PlayerViewSelf) -> bool {
        self.next_minion(me).is_some()
    }

    /// 未被拒绝的下一个候选目标；棋盘为空或全部被拒时返回 `None`。
    fn next_minion<'a>(&self, me: &'a PlayerViewSelf) -> Option<&'a str> {
        me.board
            .iter()
            .find(|minion| !self.minions.contains(&minion.minion_id))
            .map(|minion| minion.minion_id.as_str())
    }
}

/// 给出下一条意图；该座位已结束招募、不在招募阶段或无事可做时返回 `None`。
///
/// `seed` 只用于在三个发现候选间做确定性选择，不引入任何外部随机源。
/// `blocked` 是本回合已确认不合法的目标选择，由调用方按回合维护。
pub fn next_intent(
    view: &PlayerView,
    catalog: &Catalog,
    seed: u64,
    blocked: &BlockedTargets,
) -> Option<Intent> {
    if view.phase != Phase::Recruit {
        return None;
    }
    let me = &view.self_;
    if me.recruited {
        return None;
    }

    if let Some(intent) = pending_intent(me, seed, blocked) {
        return Some(intent);
    }
    // 棋盘满时手牌没有别的出路：已连续被拒多张就直接结束招募，不再试错。
    if blocked.hand_count() >= BLOCKED_LIMIT && me.affordances.can_end_recruit {
        return Some(Intent::EndRecruit);
    }
    if let Some(hand_index) = first_playable_hand(me, blocked) {
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

/// 待选队列的第一条。发现按 `seed` 取候选下标，进化固定取 0（与 M0 AI 一致）。
///
/// 目标待选取棋盘上第一个未被拒绝的随从：战吼的合法池可能带筛选（如「一个友方
/// 鱼纲随从」），来源随从在棋盘末尾时未必在池内。被拒的候选记入 `blocked`，
/// 下一次决策换下一个；全部候选都被拒绝时本轮不再发意图，交给回合计时推进。
fn pending_intent(me: &PlayerViewSelf, seed: u64, blocked: &BlockedTargets) -> Option<Intent> {
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
            minion_id: blocked.next_minion(me)?.to_string(),
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

/// 下标最小的可打出且本回合未被拒绝的手牌。
fn first_playable_hand(me: &PlayerViewSelf, blocked: &BlockedTargets) -> Option<i64> {
    me.affordances
        .hand_cards
        .iter()
        .filter(|affordance| affordance.playable && !blocked.blocks_hand(affordance.index))
        .map(|affordance| affordance.index)
        .min()
}

/// 出牌时预填的目标：视图声明该牌需要目标时取己方第一个随从。
///
/// 不预填的话，服务端在存在合法目标时一定会拒绝（`NO_VALID_TARGET`）；预填猜中
/// 即可省掉一轮往返。猜错只产生一次拒绝，由 [`BlockedTargets`] 记住并跳过。
/// 棋盘为空说明该效果可落空，返回 `None`。
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

/// 被 `NO_VALID_TARGET` 拒绝的出手改用棋盘上的下一个己方随从。
///
/// 目标池可能被玩法、分类等筛选收窄，预填的第一个随从未必在池内；被拒后沿棋盘
/// 后移一位继续尝试，直到找到合法目标。`target` 已不在棋盘上时从第一个重新开始。
/// 返回 `None` 表示棋盘已遍历完，调用方应把该牌本回合屏蔽。
pub fn retry_with_target(view: &PlayerView, rejected: &Intent) -> Option<Intent> {
    if view.phase != Phase::Recruit {
        return None;
    }
    let Intent::PlayCard {
        hand_index,
        target_minion_id,
        ..
    } = rejected
    else {
        return None;
    };
    let board = &view.self_.board;
    let start = target_minion_id
        .as_deref()
        .and_then(|id| board.iter().position(|minion| minion.minion_id == id))
        .map_or(0, |index| index + 1);
    let target = board.get(start)?.minion_id.clone();
    Some(Intent::PlayCard {
        hand_index: *hand_index,
        position: None,
        target_minion_id: Some(target),
    })
}
