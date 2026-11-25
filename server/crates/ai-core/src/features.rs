//! 候选动作特征：per-candidate 打分的输入，训练与推理共用。
//!
//! 候选由调用方从 `legal_actions` 逐条构造，只把合法动作送进模型；
//! 特征的实体、目标与技能都用 [`CardIndex`] 的整数编号表示，模型负责查表嵌入。

use rule_engine::effects::{TargetRef, Zone};
use rule_engine::state::{GameState, HandCard, InstanceId, MinionInstance, PendingChoice, Seat};

use crate::action::Action;
use crate::catalog::CardIndex;

/// 数值特征长度。
pub const ACTION_FEATURE_DIM: usize = 24;
/// 动作类别 one-hot 占用的前 16 位。
pub const ACTION_KIND_SLOTS: usize = 16;
/// 位置特征：`position / BOARD_POSITION_DIVISOR`。
pub const FEATURE_POSITION: usize = ACTION_KIND_SLOTS;
/// 发现/进化选项特征：`choice / (候选数 - 1)`。
pub const FEATURE_CHOICE: usize = ACTION_KIND_SLOTS + 1;
/// 位置特征的分母：最大合法插入下标。
///
/// 棋盘默认 7 槽位（`GameConfig::board_slots`），打牌与站位的合法下标是 `0..槽位`，
/// 因此最大值为 6；测试 `position_and_choice_features_reach_one_at_the_last_legal_value`
/// 钉住「最大合法下标归一化为 1.0」。
pub const BOARD_POSITION_DIVISOR: f32 = 6.0;
/// 是否有目标。
pub const FEATURE_HAS_TARGET: usize = ACTION_KIND_SLOTS + 2;
/// 目标攻击力 `/ 50`。
pub const FEATURE_TARGET_ATTACK: usize = ACTION_KIND_SLOTS + 3;
/// 目标生命 `/ 50`。
pub const FEATURE_TARGET_HEALTH: usize = ACTION_KIND_SLOTS + 4;
/// 实体攻击力 `/ 50`。
pub const FEATURE_ENTITY_ATTACK: usize = ACTION_KIND_SLOTS + 5;
/// 实体生命 `/ 50`。
pub const FEATURE_ENTITY_HEALTH: usize = ACTION_KIND_SLOTS + 6;
/// 实体是否金色。
pub const FEATURE_ENTITY_GOLDEN: usize = ACTION_KIND_SLOTS + 7;

/// 这批候选是否整批都是选英雄。
///
/// 选英雄是独立头（设计 D2）：整批都是 `PickHero` 时，训练与推理都必须走
/// `hero_head.*` 的参数，其余情况走通用候选头。
pub fn is_hero_choice_batch(features: &[ActionFeatures]) -> bool {
    !features.is_empty()
        && features
            .iter()
            .all(|feature| feature.kind == Action::PICK_HERO_KIND)
}

/// 一条候选动作的特征。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActionFeatures {
    /// 动作类别；取值见 [`Action::kind_index`]。
    pub kind: u8,
    /// 实体卡索引；无实体为 `-1`。
    pub card: i32,
    /// 目标卡索引；无目标为 `-1`。
    pub target_card: i32,
    /// 技能索引；无技能为 `-1`。
    pub skill: i32,
    /// 英雄索引（选英雄或本人当前英雄）；未知为 `-1`。
    pub hero: i32,
    /// 数值特征，布局见本模块常量。
    pub values: [f32; ACTION_FEATURE_DIM],
}

impl ActionFeatures {
    /// 只按动作本身构造特征；实体、目标与技能需要状态才能解析，留空。
    pub fn of(action: &Action) -> Self {
        let mut features = Self {
            kind: action.kind_index(),
            card: -1,
            target_card: -1,
            skill: -1,
            hero: -1,
            values: [0.0; ACTION_FEATURE_DIM],
        };
        features.values[usize::from(action.kind_index())] = 1.0;
        match action {
            Action::PlayCard {
                position: Some(position),
                ..
            } => {
                features.values[FEATURE_POSITION] = f32::from(*position) / BOARD_POSITION_DIVISOR;
            }
            Action::Rearrange { to, .. } => {
                features.values[FEATURE_POSITION] = f32::from(*to) / BOARD_POSITION_DIVISOR;
            }
            // 发现/进化的候选数由待选决定（发现 3 个、进化 4 个），分母要按实际
            // 候选数算，只能在拿到状态的 `from_action` 里填。
            _ => {}
        }
        features
    }

    /// 结合状态解析实体、目标与技能编号。
    pub fn from_action(action: &Action, state: &GameState, seat: Seat, index: &CardIndex) -> Self {
        let mut features = Self::of(action);
        match action {
            Action::Buy { slot } => {
                if let Some(shop_slot) = shop_slot(state, seat, *slot) {
                    features.card = index.card(&shop_slot.card_id);
                }
            }
            Action::Sell { minion } => {
                if let Some(minion) = board_minion(state, seat, *minion) {
                    features.card = index.card(&minion.card_id);
                    fill_entity(&mut features, Some(minion));
                }
            }
            Action::PlayCard { hand, target, .. } => {
                if let Some(hand_card) = hand_card(state, seat, *hand) {
                    features.card = index.card(hand_card.card_id());
                    fill_entity(&mut features, hand_card.minion());
                }
                if let Some(target) = target {
                    fill_target(&mut features, state, seat, *target, index);
                }
            }
            Action::Rearrange { from, .. } => {
                if let Some(minion) = board_at(state, seat, *from) {
                    features.card = index.card(&minion.card_id);
                    fill_entity(&mut features, Some(minion));
                }
            }
            Action::Freeze { slot: Some(slot) } | Action::Unfreeze { slot: Some(slot) } => {
                if let Some(shop_slot) = shop_slot(state, seat, *slot) {
                    features.card = index.card(&shop_slot.card_id);
                }
            }
            Action::HeroPower { skill, target } => {
                features.skill = index.skill(skill);
                if let Some(target) = target {
                    fill_target(&mut features, state, seat, *target, index);
                }
            }
            Action::MinionAbility {
                minion,
                skill,
                target,
            } => {
                features.skill = index.skill(skill);
                if let Some(minion) = board_minion(state, seat, *minion) {
                    features.card = index.card(&minion.card_id);
                    fill_entity(&mut features, Some(minion));
                }
                if let Some(target) = target {
                    fill_target(&mut features, state, seat, *target, index);
                }
            }
            Action::ChooseTarget { target } => {
                fill_target(&mut features, state, seat, *target, index);
            }
            Action::PickHero { hero } => {
                features.hero = index.hero(hero);
                features.card = -1;
            }
            Action::ChooseDiscover { choice } => {
                if let Some(candidate) = discover_candidates(state, seat) {
                    if let Some(card_id) = candidate.get(usize::from(*choice)) {
                        features.card = index.card(card_id);
                    }
                    let divisor = candidate.len().saturating_sub(1).max(1) as f32;
                    features.values[FEATURE_CHOICE] = f32::from(*choice) / divisor;
                }
            }
            Action::Freeze { slot: None } | Action::Unfreeze { slot: None } => {}
            Action::Reroll | Action::Upgrade | Action::End | Action::AbandonTarget => {}
        }
        features
    }
}

fn player(state: &GameState, seat: Seat) -> Option<&rule_engine::state::PlayerState> {
    state.players.get(usize::from(seat))
}

fn shop_slot(state: &GameState, seat: Seat, slot: u8) -> Option<&rule_engine::state::ShopSlot> {
    player(state, seat)?.shop.get(usize::from(slot))?.as_ref()
}

/// 本座位队首发现待选的第 `choice` 个候选卡 id。
///
/// 进化待选没有具体卡牌候选（选项是固定的进化类别），不提供卡索引。
/// 该座位当前发现/进化待选的候选表。
fn discover_candidates(state: &GameState, seat: Seat) -> Option<&[String]> {
    state.pending.iter().find_map(|pending| match pending {
        PendingChoice::Discover {
            seat: pending_seat,
            candidates,
            ..
        } if *pending_seat == seat => Some(candidates.as_slice()),
        _ => None,
    })
}

fn hand_card(state: &GameState, seat: Seat, hand: u8) -> Option<&HandCard> {
    player(state, seat)?.hand.get(usize::from(hand))
}

fn board_at(state: &GameState, seat: Seat, position: u8) -> Option<&MinionInstance> {
    player(state, seat)?.board.get(usize::from(position))
}

fn board_minion(state: &GameState, seat: Seat, id: InstanceId) -> Option<&MinionInstance> {
    player(state, seat)?
        .board
        .iter()
        .find(|minion| minion.instance_id == id)
}

fn fill_entity(features: &mut ActionFeatures, minion: Option<&MinionInstance>) {
    if let Some(minion) = minion {
        features.values[FEATURE_ENTITY_ATTACK] = minion.attack() as f32 / 50.0;
        features.values[FEATURE_ENTITY_HEALTH] = minion.health() as f32 / 50.0;
        features.values[FEATURE_ENTITY_GOLDEN] = if minion.golden { 1.0 } else { 0.0 };
    }
}

fn fill_target(
    features: &mut ActionFeatures,
    state: &GameState,
    seat: Seat,
    target: TargetRef,
    index: &CardIndex,
) {
    features.values[FEATURE_HAS_TARGET] = 1.0;
    match target.zone {
        Zone::Board => {
            if let Some(minion) = board_minion(state, seat, target.id) {
                features.target_card = index.card(&minion.card_id);
                features.values[FEATURE_TARGET_ATTACK] = minion.attack() as f32 / 50.0;
                features.values[FEATURE_TARGET_HEALTH] = minion.health() as f32 / 50.0;
            }
        }
        Zone::Hand => {
            if let Some(hand_card) = player(state, seat).and_then(|player| {
                player
                    .hand
                    .iter()
                    .find(|card| card.instance_id() == target.id)
            }) {
                features.target_card = index.card(hand_card.card_id());
            }
        }
        Zone::Shop => {
            if let Some(shop_slot) = player(state, seat).and_then(|player| {
                player
                    .shop
                    .iter()
                    .flatten()
                    .find(|slot| slot.instance_id == target.id)
            }) {
                features.target_card = index.card(&shop_slot.card_id);
            }
        }
    }
}
