//! 确定性 AI：只读取 `GameState` 与 `Engine`（卡牌目录、只读目标候选校验），按固定优先级给出下一条意图。
//!
//! 所有比较都使用稳定键：商店槽位下标、实例编号、目录顺序（`BTreeMap` 的卡牌 id 顺序）。
//! 输出只依赖输入状态，不读时钟与环境变量，不使用浮点。

use std::collections::BTreeMap;

use rule_engine::combat::{effective_attack, effective_health};
use rule_engine::data::{Card, CardType, Catalog, CategoryKind, Playstyle, Skill};
use rule_engine::effects::{
    Duration, Effect, EffectCtx, Op, Pick, Selector, Side, TargetRef, Trigger, Zone,
};
use rule_engine::engine::Engine;
use rule_engine::intent::{Intent, Reject, RejectCode};
use rule_engine::state::{GameState, MinionInstance, PendingChoice, Phase, PlayerState, Seat};

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
/// 7. 打出效果牌：手牌中的效果牌先试无目标，被拒后改试己方最左随从。效果牌不花金币，
///    放在会花钱的动作之后，避免「无合法目标」的牌在每次购买成功后被反复重试；
/// 8. `EndRecruit`。
pub fn choose(state: &GameState, seat: Seat, engine: &Engine) -> Option<Intent> {
    plan(state, seat, engine, &[])
}

/// 与 [`choose`] 相同的优先级；`blocked` 中的意图被跳过（本轮曾被引擎拒绝、状态未变）。
///
/// 跳过只会改变选中哪一步，不会改变比较规则，因此结果仍是确定的。
pub fn plan(state: &GameState, seat: Seat, engine: &Engine, blocked: &[Intent]) -> Option<Intent> {
    let player = state.players.get(usize::from(seat))?;
    if player.eliminated {
        return None;
    }
    let catalog = &engine.catalog;
    let candidates = [
        choose_pending(state, player, catalog, blocked),
        choose_hero_power(state, player, engine),
        choose_fill_board(state, player, catalog),
        choose_buy(state, player, catalog),
        can_upgrade(player, catalog).then_some(Intent::UpgradeEra),
        choose_reroll(player, catalog),
        choose_play_effect(player, catalog, blocked),
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
    let Some(intent) = choose(state, seat, engine) else {
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

fn choose_pending(
    state: &GameState,
    player: &PlayerState,
    catalog: &Catalog,
    blocked: &[Intent],
) -> Option<Intent> {
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
        // 目标待选：按「棋盘 → 手牌 → 商店」顺序取第一个未被拒过的目标。战吼的目标池
        // 可以按玩法、种族或区域收窄；若只提议棋盘最左随从，被拒后待选无法消费，
        // 其余招募意图又都会因待选存在被引擎拒绝（`InvalidStage`），整局模拟中断。
        PendingChoice::Target { .. } => own_targets(player)
            .into_iter()
            .map(|target| Intent::ChooseTarget { target })
            .find(|intent| !blocked.contains(intent)),
    }
}

/// 本座位三个区域的实例编号，按「棋盘 → 手牌 → 商店」排列。
fn own_targets(player: &PlayerState) -> Vec<TargetRef> {
    let mut targets: Vec<TargetRef> = player
        .board
        .iter()
        .map(|minion| TargetRef {
            zone: Zone::Board,
            id: minion.instance_id,
        })
        .collect();
    targets.extend(player.hand.iter().map(|card| TargetRef {
        zone: Zone::Hand,
        id: card.instance_id(),
    }));
    targets.extend(player.shop.iter().flatten().map(|slot| TargetRef {
        zone: Zone::Shop,
        id: slot.instance_id,
    }));
    targets
}

/// 发现选项：优先能凑成三连的候选（手牌 + 棋盘已有 2 张）；其次卡牌等级高者；
/// 再按候选顺序靠前者。
pub fn discover_choice(player: &PlayerState, candidates: &[String], catalog: &Catalog) -> u8 {
    let copies = |card_id: &str| {
        player
            .hand
            .iter()
            .filter(|card| card.card_id() == card_id)
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
        PendingChoice::Discover { seat, .. }
        | PendingChoice::Evolve { seat, .. }
        | PendingChoice::Target { seat, .. } => *seat,
    }
}

fn choose_hero_power(state: &GameState, player: &PlayerState, engine: &Engine) -> Option<Intent> {
    let hero_id = player.hero.as_deref()?;
    let hero = engine.catalog.heroes.get(hero_id)?;
    for skill_id in &hero.skills {
        let skill = engine.catalog.skill(*skill_id);
        let Some(cost) = skill.cost else {
            continue;
        };
        let limit = skill.uses_per_turn.unwrap_or(1);
        let used = player.hero_uses.get(&skill.id).copied().unwrap_or(0);
        if used >= limit || player.gold < cost {
            continue;
        }
        let Some(target) = hero_power_use(state, player, engine, skill) else {
            continue;
        };
        return Some(Intent::UseHeroPower {
            skill_id: skill.id.clone(),
            target,
        });
    }
    None
}

/// 英雄技能的使用与目标；返回 `None` 表示本回合不应使用该技能（没有能收益的目标）。
///
/// 只检查技能的 `OnActivate` 效果：
///
/// - 不需要目标的效果（`friendly:all`、`self`、群聚等）直接使用；但选择器只在敌方或
///   空棋盘上生效时跳过，避免在招募期空放。
/// - 需要单一目标时，用引擎的 `target_candidates` 按选择器筛选与组合条件求出合法目标，
///   再按效果挑选：成长取成长层数最少者；永久关键词取尚未拥有该关键词者；其余取最强。
///
/// `duration=combat` 的授予不写入 `MinionInstance::granted`，不能作为「是否已拥有」的
/// 判据，但仍是有效目标来源。
fn hero_power_use(
    state: &GameState,
    player: &PlayerState,
    engine: &Engine,
    skill: &Skill,
) -> Option<Option<TargetRef>> {
    let effects: Vec<Effect> = skill
        .effects
        .iter()
        .filter(|effect| effect.trigger == Trigger::OnActivate)
        .cloned()
        .collect();
    if effects.is_empty() {
        return None;
    }
    if !effects.iter().any(effect_requires_target) {
        return effects
            .iter()
            .any(|effect| effect_has_benefit(state, player, effect))
            .then_some(None);
    }
    let ctx = EffectCtx {
        seat: player.seat,
        source: None,
        target: None,
        event_target: None,
        golden: false,
        level_override: None,
    };
    let candidates = engine.target_candidates(state, &effects, &ctx, false);
    if candidates.is_empty() {
        return None;
    }
    let effect = effects
        .iter()
        .find(|effect| effect_requires_target(effect))?;
    choose_hero_power_target(effect, &candidates, player, &engine.catalog).map(Some)
}

fn effect_requires_target(effect: &Effect) -> bool {
    matches!(
        effect.selector,
        Selector::Target { .. }
            | Selector::Side {
                pick: Pick::Single,
                ..
            }
    )
}

/// 不需要目标的效果是否有实际作用：发现与商店类效果总是有收益；光环类效果要求
/// 目标阵营在当前阶段有随从（招募期敌方为空）。
fn effect_has_benefit(state: &GameState, player: &PlayerState, effect: &Effect) -> bool {
    if effect.op == Op::Discover
        || matches!(
            effect.op,
            Op::RefreshShop | Op::AddShopSlot | Op::GainMinion
        )
    {
        return true;
    }
    match &effect.selector {
        Selector::SelfCard => false,
        Selector::Target { .. } => true,
        Selector::Side { side, .. } => match side {
            Side::Enemy => {
                state.phase != Phase::Recruit
                    && state.players.iter().any(|other| {
                        other.seat != player.seat && !other.eliminated && !other.board.is_empty()
                    })
            }
            Side::Friendly | Side::Both => !player.board.is_empty(),
        },
    }
}

fn choose_hero_power_target(
    effect: &Effect,
    candidates: &[TargetRef],
    player: &PlayerState,
    catalog: &Catalog,
) -> Option<TargetRef> {
    if effect.op == Op::Growth {
        return candidates
            .iter()
            .copied()
            .filter(|target| target.zone == Zone::Board)
            .filter_map(|target| {
                board_minion(player, target.id)
                    .map(|minion| (minion.stats.attack.growth, target.id, target))
            })
            .min_by_key(|(growth, id, _)| (*growth, *id))
            .map(|(_, _, target)| target)
            .or_else(|| candidates.first().copied());
    }
    if effect.op == Op::GrantKeyword
        && effect.params.duration == Some(Duration::Permanent)
        && let Some(keyword) = &effect.params.keyword
    {
        return candidates
            .iter()
            .copied()
            .filter(|target| target.zone == Zone::Board)
            .filter(|target| {
                board_minion(player, target.id).is_some_and(|minion| {
                    !rule_engine::combat::keyword_set(catalog, minion)
                        .contains_name(&keyword.keyword)
                })
            })
            .min_by_key(|target| target.id);
    }
    candidates.iter().copied().max_by_key(|target| {
        (
            hero_power_target_strength(player, catalog, *target),
            std::cmp::Reverse(target.id),
        )
    })
}

fn board_minion(player: &PlayerState, id: u64) -> Option<&MinionInstance> {
    player.board.iter().find(|minion| minion.instance_id == id)
}

fn hero_power_target_strength(player: &PlayerState, catalog: &Catalog, target: TargetRef) -> i32 {
    match target.zone {
        Zone::Board => board_minion(player, target.id)
            .map(|minion| strength(catalog, minion))
            .unwrap_or(i32::MIN),
        Zone::Hand => player
            .hand
            .iter()
            .filter_map(|card| card.minion())
            .find(|minion| minion.instance_id == target.id)
            .map(|minion| strength(catalog, minion))
            .unwrap_or(i32::MIN),
        Zone::Shop => player
            .shop
            .iter()
            .flatten()
            .find(|slot| slot.instance_id == target.id)
            .and_then(|slot| catalog.cards.get(&slot.card_id))
            .map(|card| {
                card.attack
                    .unwrap_or(0)
                    .saturating_add(card.health.unwrap_or(0))
            })
            .unwrap_or(i32::MIN),
    }
}

/// 购买候选的排序键：价格 → 卡牌等级 → 槽位下标小者优先。
type CandidateKey = (u8, u8, std::cmp::Reverse<u8>);

/// 定向效果牌是否适合当前阵容：没有主导玩法时始终可买；有主导玩法时，
/// 任一条 `play:` 等于主导玩法才可买；没有 `play:` 的效果牌按可买处理
/// （加载期已拒绝，这里防御性兜底）。
fn effect_matches_playstyle(card: &Card, dominant: Playstyle) -> bool {
    if card.card_type != CardType::Effect || dominant == Playstyle::Neutral {
        return true;
    }
    let mut any = false;
    for category in &card.categories {
        if category.kind != CategoryKind::Play {
            continue;
        }
        any = true;
        if Playstyle::from_id(&category.value) == Some(dominant) {
            return true;
        }
    }
    !any
}

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
    let dominant = dominant_playstyle(
        player.board.iter().map(|minion| minion.card_id.as_str()),
        catalog,
    );
    let mut best: Option<(CandidateKey, u8)> = None;
    for (slot_index, slot) in player.shop.iter().enumerate() {
        let Some(slot) = slot else { continue };
        let Some(card) = catalog.cards.get(&slot.card_id) else {
            continue;
        };
        if !effect_matches_playstyle(card, dominant) {
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
fn choose_play_effect(
    player: &PlayerState,
    catalog: &Catalog,
    blocked: &[Intent],
) -> Option<Intent> {
    for (index, hand) in player.hand.iter().enumerate() {
        let Some(card) = catalog.cards.get(hand.card_id()) else {
            continue;
        };
        if card.card_type != CardType::Effect {
            continue;
        }
        let hand_index = u8::try_from(index).ok()?;
        let plain = Intent::PlayCard {
            hand_index,
            position: None,
            target: None,
        };
        if !blocked.contains(&plain) {
            return Some(plain);
        }
        // 定向效果牌的目标池可能带玩法、分类或区域筛选；只试最左会让棋盘第一个随从
        // 不合法时这张牌永远打不出。按「棋盘 → 手牌 → 商店」逐个尝试，被拒的候选由
        // `blocked` 跳过，遍历完仍无候选则留给后续购买/结束招募。
        for target in own_targets(player) {
            let targeted = Intent::PlayCard {
                hand_index,
                position: None,
                target: Some(target),
            };
            if !blocked.contains(&targeted) {
                return Some(targeted);
            }
        }
    }
    None
}

fn choose_fill_board(state: &GameState, player: &PlayerState, catalog: &Catalog) -> Option<Intent> {
    // 三连奖励牌先于普通随从打出：它是零费发现，保证奖励路径被覆盖。
    if let Some(index) = player
        .hand
        .iter()
        .position(|card| card.reward_level().is_some())
    {
        return u8::try_from(index).ok().map(|hand_index| Intent::PlayCard {
            hand_index,
            position: None,
            target: None,
        });
    }
    let minion_in_hand = player.hand.iter().enumerate().find(|(_, card)| {
        catalog
            .cards
            .get(card.card_id())
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
        catalog
            .cards
            .get(&slot.card_id)
            .is_some_and(|card| card.card_type == CardType::Minion && buy_cost(card) <= player.gold)
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
            | RejectCode::NotActiveSkill
            | RejectCode::NoValidTarget
            | RejectCode::TargetNotAllowed
            | RejectCode::InvalidChoice
    )
}

/// 玩法判定：统计阵容随从的全部 `play:` 归属，每张卡按每条 `play:` 各计 1 次，
/// 取数量最多的玩法；并列时按玩法声明顺序（`Playstyle` 的 `Ord`）取靠前者。
/// 没有 `play:` 的卡跳过；没有可统计归属时返回 `Neutral`。
pub fn dominant_playstyle<'a>(
    board: impl IntoIterator<Item = &'a str>,
    catalog: &Catalog,
) -> Playstyle {
    let mut counts: BTreeMap<Playstyle, usize> = BTreeMap::new();
    for card_id in board {
        let Some(card) = catalog.cards.get(card_id) else {
            continue;
        };
        for category in &card.categories {
            if category.kind != CategoryKind::Play {
                continue;
            }
            if let Some(playstyle) = Playstyle::from_id(&category.value) {
                *counts.entry(playstyle).or_insert(0) += 1;
            }
        }
    }
    let mut best: Option<(Playstyle, usize)> = None;
    for (&playstyle, &count) in &counts {
        let better = match best {
            None => true,
            Some((current_playstyle, current_count)) => {
                if count != current_count {
                    count > current_count
                } else {
                    playstyle < current_playstyle
                }
            }
        };
        if better {
            best = Some((playstyle, count));
        }
    }
    best.map_or(Playstyle::Neutral, |(playstyle, _)| playstyle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedded_catalog;
    use rule_engine::state::{HandCard, PendingChoice, Phase};

    /// 棋盘目标。
    fn board_target(id: u64) -> TargetRef {
        TargetRef {
            zone: Zone::Board,
            id,
        }
    }

    /// 座位 0 的棋盘按给定卡牌 id 从左到右摆放，并挂一条目标待选。
    fn state_with_target(cards: &[&str]) -> (Engine, GameState) {
        let (engine, mut state) = Engine::new(
            embedded_catalog().expect("内置卡表应能加载"),
            [7; 32],
            rule_engine::playstyles::draw(7),
        );
        state.phase = Phase::Recruit;
        state.turn = 1;
        let player = state.players.first_mut().expect("8 个座位");
        player.board = cards
            .iter()
            .enumerate()
            .map(|(index, card_id)| {
                MinionInstance::new(&engine.catalog, index as u64 + 1, card_id, false)
            })
            .collect();
        state.pending.push_back(PendingChoice::Target {
            seat: 0,
            source: 1,
            hand_card: HandCard::Minion(MinionInstance::new(
                &engine.catalog,
                1,
                "clownfish",
                false,
            )),
            hand_index: 0,
            fire_ally_evolve: false,
        });
        (engine, state)
    }

    /// 指定英雄与棋盘卡牌的招募态座位，金币由调用方给足。
    fn hero_state(hero_id: &str, cards: &[&str], gold: u8) -> (Engine, GameState) {
        let (engine, mut state) = Engine::new(
            embedded_catalog().expect("内置卡表应能加载"),
            [7; 32],
            rule_engine::playstyles::draw(7),
        );
        state.phase = Phase::Recruit;
        state.turn = 1;
        let player = state.players.first_mut().expect("8 个座位");
        player.hero = Some(hero_id.to_string());
        player.gold = gold;
        player.board = cards
            .iter()
            .enumerate()
            .map(|(index, card_id)| {
                MinionInstance::new(&engine.catalog, index as u64 + 1, card_id, false)
            })
            .collect();
        (engine, state)
    }

    #[test]
    fn hero_power_uses_all_target_skill_without_target() {
        // 质能方程只作用于全体友方，不需要选目标；旧 AI 因无法识别 All 选择器而跳过。
        let (engine, state) = hero_state("einstein", &["saman"], 10);
        assert_eq!(
            choose_hero_power(&state, &state.players[0], &engine),
            Some(Intent::UseHeroPower {
                skill_id: "mass-energy-equivalence".to_string(),
                target: None,
            })
        );
    }

    #[test]
    fn hero_power_targets_matching_filtered_minion() {
        // 本草纲目只作用于友方植物；目标必须在筛选后的候选内，不能固定取最左。
        let (engine, state) = hero_state("lisshi", &["saman", "wandou"], 10);
        assert_eq!(
            choose_hero_power(&state, &state.players[0], &engine),
            Some(Intent::UseHeroPower {
                skill_id: "materia-medica".to_string(),
                target: Some(board_target(2)),
            })
        );
    }

    #[test]
    fn hero_power_skips_filtered_skill_without_candidate() {
        let (engine, state) = hero_state("lisshi", &["saman"], 10);
        assert_eq!(choose_hero_power(&state, &state.players[0], &engine), None);
    }

    #[test]
    fn hero_power_uses_combat_keyword_skill() {
        // 光速出击只持续到本场战斗结束，不能靠「永久授予」判据识别；仍应选一个目标。
        let (engine, state) = hero_state("einstein", &["saman"], 10);
        let skill = engine
            .catalog
            .skill_by_name("light-speed-strike")
            .expect("技能应存在");
        assert_eq!(
            hero_power_use(&state, &state.players[0], &engine, skill),
            Some(Some(board_target(1)))
        );
    }

    #[test]
    fn pending_target_defaults_to_leftmost_minion() {
        let (engine, state) = state_with_target(&["saman", "clownfish"]);
        assert_eq!(
            choose(&state, 0, &engine),
            Some(Intent::ChooseTarget {
                target: board_target(1),
            })
        );
    }

    #[test]
    fn pending_target_skips_blocked_minions() {
        // 筛选型战吼的目标池可能不含最左侧随从；被拒后必须沿棋盘后移，
        // 否则待选无法消费，其余意图都会因待选存在而被引擎拒绝。
        let (engine, state) = state_with_target(&["saman", "clownfish"]);
        assert_eq!(
            plan(
                &state,
                0,
                &engine,
                &[Intent::ChooseTarget {
                    target: board_target(1),
                }]
            ),
            Some(Intent::ChooseTarget {
                target: board_target(2),
            })
        );
    }

    #[test]
    fn plays_effect_card_from_hand() {
        let catalog = embedded_catalog().expect("内置卡表应能加载");
        let (_, mut state) =
            Engine::new(catalog.clone(), [7; 32], rule_engine::playstyles::draw(7));
        state.phase = Phase::Recruit;
        state.turn = 1;
        state.players[0].board = vec![MinionInstance::new(&catalog, 1, "saman", false)];
        state.players[0].hand = vec![rule_engine::state::HandCard::effect(20, "aolinpike-jidian")];
        assert_eq!(
            choose_play_effect(&state.players[0], &catalog, &[]),
            Some(Intent::PlayCard {
                hand_index: 0,
                position: None,
                target: None
            })
        );
    }

    #[test]
    fn effect_play_falls_back_to_leftmost_target() {
        let catalog = embedded_catalog().expect("内置卡表应能加载");
        let (_, mut state) =
            Engine::new(catalog.clone(), [7; 32], rule_engine::playstyles::draw(7));
        state.phase = Phase::Recruit;
        state.turn = 1;
        state.players[0].board = vec![MinionInstance::new(&catalog, 1, "saman", false)];
        state.players[0].hand = vec![rule_engine::state::HandCard::effect(20, "faraday-cage")];
        let plain = Intent::PlayCard {
            hand_index: 0,
            position: None,
            target: None,
        };
        assert_eq!(
            choose_play_effect(&state.players[0], &catalog, &[plain]),
            Some(Intent::PlayCard {
                hand_index: 0,
                position: None,
                target: Some(board_target(1))
            })
        );
    }

    #[test]
    fn effect_play_tries_every_friendly_target() {
        // 带玩法筛选的定向效果牌（洋流只作用于友方鱼纲）在棋盘第一个随从不合法时
        // 必须继续尝试后面的随从；只试最左会让这张牌永远打不出、长期占用一手牌。
        let catalog = embedded_catalog().expect("内置卡表应能加载");
        let (_, mut state) =
            Engine::new(catalog.clone(), [7; 32], rule_engine::playstyles::draw(7));
        state.phase = Phase::Recruit;
        state.turn = 1;
        state.players[0].board = vec![
            MinionInstance::new(&catalog, 1, "saman", false),
            MinionInstance::new(&catalog, 2, "clownfish", false),
        ];
        state.players[0].hand = vec![rule_engine::state::HandCard::effect(20, "yangliu")];
        let plain = Intent::PlayCard {
            hand_index: 0,
            position: None,
            target: None,
        };
        let leftmost = Intent::PlayCard {
            hand_index: 0,
            position: None,
            target: Some(board_target(1)),
        };
        assert_eq!(
            choose_play_effect(&state.players[0], &catalog, &[plain, leftmost]),
            Some(Intent::PlayCard {
                hand_index: 0,
                position: None,
                target: Some(board_target(2))
            })
        );
    }

    #[test]
    fn dominant_playstyle_counts_every_play_category() {
        let catalog = embedded_catalog().expect("内置卡表应能加载");
        // agi 同时带 play:mammal 与 play:machine，各计 1 次；并列时取声明顺序靠前者。
        assert_eq!(dominant_playstyle(["agi"], &catalog), Playstyle::Mammal);
        // agi 的 machine 与 nanobot 的 machine 合计 2 次，压过 mammal 的 1 次。
        assert_eq!(
            dominant_playstyle(["agi", "nanobot"], &catalog),
            Playstyle::Machine
        );
        // agi 的 mammal 与 saman 的 mammal 合计 2 次。
        assert_eq!(
            dominant_playstyle(["agi", "saman"], &catalog),
            Playstyle::Mammal
        );
        assert_eq!(
            dominant_playstyle(std::iter::empty(), &catalog),
            Playstyle::Neutral
        );
    }

    #[test]
    fn dominant_playstyle_skips_cards_without_play() {
        let mut catalog = embedded_catalog().expect("内置卡表应能加载");
        let mut card = catalog.cards.get("saman").expect("saman 存在").clone();
        card.id = "no-play".to_string();
        card.categories.clear();
        catalog.cards.insert(card.id.clone(), card);
        assert_eq!(
            dominant_playstyle(["no-play"], &catalog),
            Playstyle::Neutral
        );
    }

    #[test]
    fn effect_matches_playstyle_uses_any_play_category() {
        let catalog = embedded_catalog().expect("内置卡表应能加载");
        // faraday-cage 带 play:reptile 与 play:machine。
        let faraday = catalog
            .cards
            .get("faraday-cage")
            .expect("faraday-cage 存在");
        assert!(effect_matches_playstyle(faraday, Playstyle::Machine));
        assert!(effect_matches_playstyle(faraday, Playstyle::Reptile));
        assert!(!effect_matches_playstyle(faraday, Playstyle::Mammal));
        // 没有主导玩法时始终可买。
        assert!(effect_matches_playstyle(faraday, Playstyle::Neutral));

        // aolinpike-jidian 带 play:mammal 与 play:plant。
        let aolinpike = catalog
            .cards
            .get("aolinpike-jidian")
            .expect("aolinpike-jidian 存在");
        assert!(effect_matches_playstyle(aolinpike, Playstyle::Mammal));
        assert!(effect_matches_playstyle(aolinpike, Playstyle::Plant));
        assert!(!effect_matches_playstyle(aolinpike, Playstyle::Machine));
    }

    #[test]
    fn effect_without_play_is_buyable_defensively() {
        let catalog = embedded_catalog().expect("内置卡表应能加载");
        let mut card = catalog
            .cards
            .get("faraday-cage")
            .expect("faraday-cage 存在")
            .clone();
        card.categories.clear();
        assert!(
            effect_matches_playstyle(&card, Playstyle::Machine),
            "没有 play: 的效果牌按可买处理"
        );
    }

    #[test]
    fn buy_accepts_effect_card_matching_any_dominant_play() {
        let catalog = embedded_catalog().expect("内置卡表应能加载");
        let (_, mut state) =
            Engine::new(catalog.clone(), [7; 32], rule_engine::playstyles::draw(7));
        state.phase = Phase::Recruit;
        state.turn = 1;
        let player = &mut state.players[0];
        player.board = vec![
            MinionInstance::new(&catalog, 1, "nanobot", false),
            MinionInstance::new(&catalog, 2, "agi", false),
        ];
        player.gold = 10;
        player.shop = vec![Some(rule_engine::state::ShopSlot {
            instance_id: 10,
            card_id: "faraday-cage".into(),
            frozen: false,
        })];
        assert_eq!(
            choose_buy(&state, &state.players[0], &catalog),
            Some(Intent::BuyMinion { slot: 0 }),
            "机械主导时可买带 play:machine 的跨玩法效果牌"
        );

        state.players[0].board = vec![
            MinionInstance::new(&catalog, 3, "saman", false),
            MinionInstance::new(&catalog, 4, "agi", false),
        ];
        state.players[0].shop = vec![Some(rule_engine::state::ShopSlot {
            instance_id: 11,
            card_id: "aolinpike-jidian".into(),
            frozen: false,
        })];
        assert_eq!(
            choose_buy(&state, &state.players[0], &catalog),
            Some(Intent::BuyMinion { slot: 0 }),
            "哺乳纲主导时可买带 play:mammal 的跨玩法效果牌"
        );

        state.players[0].shop = vec![Some(rule_engine::state::ShopSlot {
            instance_id: 12,
            card_id: "faraday-cage".into(),
            frozen: false,
        })];
        assert_eq!(
            choose_buy(&state, &state.players[0], &catalog),
            None,
            "哺乳纲主导时不买只有爬行纲/机械归属的效果牌"
        );
    }

    #[test]
    fn buy_skips_targeted_effect_of_other_playstyle() {
        let catalog = embedded_catalog().expect("内置卡表应能加载");
        let (_, mut state) =
            Engine::new(catalog.clone(), [7; 32], rule_engine::playstyles::draw(7));
        state.phase = Phase::Recruit;
        state.turn = 1;
        state.players[0].board = vec![MinionInstance::new(&catalog, 1, "saman", false)];
        state.players[0].gold = 10;
        state.players[0].shop = vec![Some(rule_engine::state::ShopSlot {
            instance_id: 10,
            card_id: "yangliu".into(),
            frozen: false,
        })];
        assert_eq!(
            choose_buy(&state, &state.players[0], &catalog),
            None,
            "哺乳玩家不买鱼的定向牌"
        );

        state.players[0].board = vec![MinionInstance::new(&catalog, 2, "clownfish", false)];
        assert_eq!(
            choose_buy(&state, &state.players[0], &catalog),
            Some(Intent::BuyMinion { slot: 0 }),
            "鱼纲玩家可以买鱼的定向牌"
        );
    }
}
