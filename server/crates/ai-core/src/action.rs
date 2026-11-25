//! 动作表示：与协议意图一一对应，供训练侧记录与回放。

use rule_engine::effects::TargetRef;
use rule_engine::intent::Intent;
use rule_engine::state::InstanceId;

/// 策略可执行的动作。
///
/// 与 [`Intent`] 的差别只有一处：开局选英雄没有对应的协议意图（由
/// `Engine::start_game` 直接写入），因此 [`Action::to_intent`] 对它返回 `None`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Buy {
        slot: u8,
    },
    Sell {
        minion: InstanceId,
    },
    Reroll,
    Upgrade,
    End,
    PlayCard {
        hand: u8,
        position: Option<u8>,
        target: Option<TargetRef>,
    },
    Rearrange {
        from: u8,
        to: u8,
    },
    Freeze {
        slot: Option<u8>,
    },
    Unfreeze {
        slot: Option<u8>,
    },
    HeroPower {
        skill: String,
        target: Option<TargetRef>,
    },
    MinionAbility {
        minion: InstanceId,
        skill: String,
        target: Option<TargetRef>,
    },
    ChooseDiscover {
        choice: u8,
    },
    ChooseTarget {
        target: TargetRef,
    },
    AbandonTarget,
    PickHero {
        hero: String,
    },
}

impl Action {
    /// 选英雄的类别编号；选英雄走独立头（设计 D2），训练与推理都按它分派。
    pub const PICK_HERO_KIND: u8 = 14;

    /// 动作类别编号，供候选特征的 one-hot 使用。
    pub fn kind_index(&self) -> u8 {
        match self {
            Self::Buy { .. } => 0,
            Self::Sell { .. } => 1,
            Self::Reroll => 2,
            Self::Upgrade => 3,
            Self::End => 4,
            Self::PlayCard { .. } => 5,
            Self::Rearrange { .. } => 6,
            Self::Freeze { .. } => 7,
            Self::Unfreeze { .. } => 8,
            Self::HeroPower { .. } => 9,
            Self::MinionAbility { .. } => 10,
            Self::ChooseDiscover { .. } => 11,
            Self::ChooseTarget { .. } => 12,
            Self::AbandonTarget => 13,
            Self::PickHero { .. } => Self::PICK_HERO_KIND,
        }
    }

    /// 由协议意图构造动作；覆盖全部 14 种意图。
    pub fn from_intent(intent: &Intent) -> Option<Self> {
        Some(match intent {
            Intent::BuyMinion { slot } => Self::Buy { slot: *slot },
            Intent::SellMinion { minion } => Self::Sell { minion: *minion },
            Intent::RerollShop => Self::Reroll,
            Intent::UpgradeEra => Self::Upgrade,
            Intent::PlayCard {
                hand_index,
                position,
                target,
            } => Self::PlayCard {
                hand: *hand_index,
                position: *position,
                target: *target,
            },
            Intent::RearrangeBoard { from, to } => Self::Rearrange {
                from: *from,
                to: *to,
            },
            Intent::FreezeShop { slot } => Self::Freeze { slot: *slot },
            Intent::UnfreezeShop { slot } => Self::Unfreeze { slot: *slot },
            Intent::UseHeroPower { skill_id, target } => Self::HeroPower {
                skill: skill_id.clone(),
                target: *target,
            },
            Intent::UseMinionAbility {
                minion,
                skill_id,
                target,
            } => Self::MinionAbility {
                minion: *minion,
                skill: skill_id.clone(),
                target: *target,
            },
            Intent::EndRecruit => Self::End,
            Intent::ChooseDiscover { choice } => Self::ChooseDiscover { choice: *choice },
            Intent::ChooseTarget { target } => Self::ChooseTarget { target: *target },
            Intent::AbandonTarget => Self::AbandonTarget,
        })
    }

    /// 转回协议意图；`PickHero` 没有对应意图。
    pub fn to_intent(&self) -> Option<Intent> {
        Some(match self {
            Self::Buy { slot } => Intent::BuyMinion { slot: *slot },
            Self::Sell { minion } => Intent::SellMinion { minion: *minion },
            Self::Reroll => Intent::RerollShop,
            Self::Upgrade => Intent::UpgradeEra,
            Self::End => Intent::EndRecruit,
            Self::PlayCard {
                hand,
                position,
                target,
            } => Intent::PlayCard {
                hand_index: *hand,
                position: *position,
                target: *target,
            },
            Self::Rearrange { from, to } => Intent::RearrangeBoard {
                from: *from,
                to: *to,
            },
            Self::Freeze { slot } => Intent::FreezeShop { slot: *slot },
            Self::Unfreeze { slot } => Intent::UnfreezeShop { slot: *slot },
            Self::HeroPower { skill, target } => Intent::UseHeroPower {
                skill_id: skill.clone(),
                target: *target,
            },
            Self::MinionAbility {
                minion,
                skill,
                target,
            } => Intent::UseMinionAbility {
                minion: *minion,
                skill_id: skill.clone(),
                target: *target,
            },
            Self::ChooseDiscover { choice } => Intent::ChooseDiscover { choice: *choice },
            Self::ChooseTarget { target } => Intent::ChooseTarget { target: *target },
            Self::AbandonTarget => Intent::AbandonTarget,
            Self::PickHero { .. } => return None,
        })
    }
}
