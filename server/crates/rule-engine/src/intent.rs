//! 玩家意图与拒绝码：`Engine::apply` 的唯一输入与错误输出。
//!
//! 意图只描述玩家想做什么，不含座位；座位由 [`crate::engine::Engine::apply`]
//! 显式指定。

use crate::effects::TargetRef;
use crate::events::GameEvent;
use crate::state::InstanceId;

/// 招募阶段玩家可以提交的十二种意图。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    /// 购买商店槽位；买到的牌进入手牌，不需要目标。
    BuyMinion { slot: u8 },
    /// 出售棋盘上的随从。
    SellMinion { minion: InstanceId },
    /// 刷新商店。
    RerollShop,
    /// 升级时代。
    UpgradeEra,
    /// 打出手牌；`position` 是随从的插入下标（缺省追加到最右），
    /// `target` 是效果牌的目标（区域 + 实例编号）。
    PlayCard {
        hand_index: u8,
        position: Option<u8>,
        target: Option<TargetRef>,
    },
    /// 调整棋盘站位。
    RearrangeBoard { from: u8, to: u8 },
    /// 冻结槽位；`None` 表示全部槽位。
    FreezeShop { slot: Option<u8> },
    /// 使用英雄主动技能；`target` 是技能效果的目标（区域 + 实例编号）。
    UseHeroPower {
        skill_id: String,
        target: Option<TargetRef>,
    },
    /// 结束招募，进入战斗阶段。
    EndRecruit,
    /// 从待选发现结果中选择一张；本任务只在 `Recruit` 阶段接受。
    ChooseDiscover { choice: u8 },
    /// 为队首的目标待选指定一个本人区域内的目标。
    ChooseTarget { target: TargetRef },
    /// 放弃队首的目标待选：撤销该次出牌，原牌退回手牌，战吼不结算。
    AbandonTarget,
}

/// 意图被拒绝的原因，与核心规则 §3 的返回码一一对应。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectCode {
    NotEnoughGold,
    ShopSlotEmpty,
    NoSpace,
    InvalidStage,
    NoSuchMinion,
    AlreadyMaxEra,
    CardNotInHand,
    HeroPowerLimit,
    NoValidTarget,
    TargetNotAllowed,
    /// 待选（如发现）的候选下标非法。
    InvalidChoice,
}

impl RejectCode {
    /// 与协议错误码一致的稳定字符串。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotEnoughGold => "NOT_ENOUGH_GOLD",
            Self::ShopSlotEmpty => "SHOP_SLOT_EMPTY",
            Self::NoSpace => "NO_SPACE",
            Self::InvalidStage => "INVALID_STAGE",
            Self::NoSuchMinion => "NO_SUCH_MINION",
            Self::AlreadyMaxEra => "ALREADY_MAX_ERA",
            Self::CardNotInHand => "CARD_NOT_IN_HAND",
            Self::HeroPowerLimit => "HERO_POWER_LIMIT",
            Self::NoValidTarget => "NO_VALID_TARGET",
            Self::TargetNotAllowed => "TARGET_NOT_ALLOWED",
            Self::InvalidChoice => "INVALID_CHOICE",
        }
    }
}

/// 一次被拒绝的意图；拒绝时不改变 `GameState`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reject {
    pub code: RejectCode,
    pub message: String,
}

impl Reject {
    pub(crate) fn new(code: RejectCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// 一次成功意图产生的事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub events: Vec<GameEvent>,
}

impl Outcome {
    pub(crate) fn empty() -> Self {
        Self { events: Vec::new() }
    }

    pub(crate) fn with(events: Vec<GameEvent>) -> Self {
        Self { events }
    }
}
