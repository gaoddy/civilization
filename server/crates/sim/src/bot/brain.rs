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
//! 视图只告诉机器人哪些目标是合法的：`pendingChoices` 与手牌的 `targetCandidates`
//! 都由服务端下发。机器人按候选顺序取第一个未被拒绝的目标；`NO_VALID_TARGET` 拒绝
//! 后把该目标记入 [`BlockedTargets`]（键为 `zone:id`）并换下一个候选。整张牌都不
//! 合法时才由 [`BlockedTargets`] 把这张牌本回合屏蔽。

use std::collections::BTreeSet;

use protocol::PlayerViewSelf;
use protocol::TargetRef;
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
/// 服务端下发候选池，正常情况下不会拒绝；被拒说明视图与结算口径有偏差，按
/// `zone:id` 记住并换下一个候选，避免同一候选反复空转。
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BlockedTargets {
    /// 本回合放弃的出手，元素为手牌下标。
    hand: BTreeSet<i64>,
    /// 本回合已被拒绝的目标，键为 `zone:id`。
    targets: BTreeSet<String>,
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

    /// 本回合放弃这个目标。
    pub fn block_target(&mut self, target: &TargetRef) {
        self.targets.insert(target_key(target));
    }

    /// 该目标本回合是否已被拒绝。
    pub fn target_blocked(&self, target: &TargetRef) -> bool {
        self.targets.contains(&target_key(target))
    }

    /// 候选里是否还有未被拒绝的目标。
    pub fn has_unblocked_target(&self, candidates: &[TargetRef]) -> bool {
        self.next_target(candidates).is_some()
    }

    /// 第一个未被拒绝的候选目标。
    fn next_target<'a>(&self, candidates: &'a [TargetRef]) -> Option<&'a TargetRef> {
        candidates
            .iter()
            .find(|target| !self.target_blocked(target))
    }
}

/// 目标的稳定键：区域 + 实例编号。
fn target_key(target: &TargetRef) -> String {
    let zone = match target.zone {
        protocol::TargetZone::Hand => "hand",
        protocol::TargetZone::Shop => "shop",
        protocol::TargetZone::Board => "board",
    };
    format!("{zone}:{}", target.id)
}

/// 队首目标待选的候选是否已全部被拒绝。
pub fn pending_targets_exhausted(view: &PlayerView, blocked: &BlockedTargets) -> bool {
    match view.self_.pending_choices.first() {
        Some(PendingChoice::Target { candidates, .. }) => !blocked.has_unblocked_target(candidates),
        _ => true,
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
            target: prefill_target(me, hand_index, blocked),
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
/// 目标待选取服务端下发的候选里第一个未被拒绝的目标；全部候选都被拒绝时本轮
/// 不再发意图，交给回合计时推进。
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
        PendingChoice::Target { candidates, .. } => Some(Intent::ChooseTarget {
            target: blocked.next_target(candidates)?.clone(),
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
        CardType::Hero => 0,
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

/// 出牌时预填的目标：视图声明该牌需要目标时取第一个未被拒绝的候选。
///
/// 候选由服务端下发，正常情况下一次命中；被拒时由 [`retry_with_target`] 换下一个。
/// 不需要目标的牌返回 `None`。
fn prefill_target(
    me: &PlayerViewSelf,
    hand_index: i64,
    blocked: &BlockedTargets,
) -> Option<TargetRef> {
    let card = me.hand.iter().find(|card| card.index == hand_index)?;
    if !card.requires_target {
        return None;
    }
    blocked.next_target(&card.target_candidates).cloned()
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

/// 被 `NO_VALID_TARGET` 拒绝的出手改用该牌候选池里的下一个目标。
///
/// 候选由服务端下发；被拒只说明视图与结算有偏差。`target` 不在候选里时从第一个
/// 重新开始。返回 `None` 表示候选已遍历完，调用方应把该牌本回合屏蔽。
pub fn retry_with_target(view: &PlayerView, rejected: &Intent) -> Option<Intent> {
    if view.phase != Phase::Recruit {
        return None;
    }
    let Intent::PlayCard {
        hand_index, target, ..
    } = rejected
    else {
        return None;
    };
    let card = view
        .self_
        .hand
        .iter()
        .find(|card| card.index == *hand_index)?;
    let candidates = &card.target_candidates;
    let start = target
        .as_ref()
        .and_then(|target| {
            let key = target_key(target);
            candidates
                .iter()
                .position(|candidate| target_key(candidate) == key)
        })
        .map_or(0, |index| index + 1);
    let target = candidates.get(start)?.clone();
    Some(Intent::PlayCard {
        hand_index: *hand_index,
        position: None,
        target: Some(target),
    })
}
