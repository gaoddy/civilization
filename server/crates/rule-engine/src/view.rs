//! 逐座位视图：把招募侧只读预检包装成「这个座位现在能看到什么、能做什么」。
//!
//! 视图是纯函数产物：只读 [`Engine`] 与 [`GameState`]，不修改状态、不消耗随机流、
//! 不读时钟或环境。它只含公告信息——其他座位仅以 [`PlayerSummaryView`] 的公开字段
//! 出现，绝不包含对手的金币、手牌、商店、刷新次数或种子。
//!
//! **阶段门控由本模块负责**：`recruit::check_*` 只做意图内校验，不检查阶段、
//! `recruit_done`、`eliminated` 与 `target`。因此 [`view`] 先算
//! `activatable = 处于 Recruit 阶段且未出局`；不可操作时所有布尔值为 `false`，
//! 每槽位的 `disabled_reason` 统一为 [`RejectCode::InvalidStage`]。座位已结束招募
//! 时，除 `can_end_recruit` 外的布尔值一律为 `false`（重复 `EndRecruit` 幂等成功）。

use crate::combat;
use crate::data::{Era, Tribe};
use crate::effects::EVOLVE_CHOICES;
use crate::engine::Engine;
use crate::intent::RejectCode;
use crate::recruit::{
    check_buy, check_end_recruit, check_hero_power, check_play_card, check_reroll, check_sell,
    check_upgrade,
};
use crate::settle;
use crate::state::{
    GameState, MinionInstance, PendingChoice, Phase, PlayerState, Seat, StatComponent,
};

/// 一个座位当前可见的全部状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeatView {
    pub phase: Phase,
    pub turn: u8,
    pub seat: Seat,
    /// 镜像配对或尚未配对时为 `None`。
    pub opponent_seat: Option<Seat>,
    pub self_view: SelfView,
    /// 全部座位的公开摘要，按座位升序。
    pub players: Vec<PlayerSummaryView>,
    /// 本局启用的流派，按 [`Tribe`] 顺序。
    pub enabled_tribes: Vec<String>,
}

/// 本座位自己的私密数据与可操作项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfView {
    pub hp: i32,
    pub era: u8,
    pub gold: u8,
    pub hero_card_id: Option<String>,
    /// 英雄的第一个主动技能（`skill.cost.is_some()`），没有则为 `None`。
    pub hero_power_skill_id: Option<String>,
    pub hero_power_uses_left: u8,
    pub shop: Vec<ShopSlotView>,
    pub hand: Vec<HandCardView>,
    pub board: Vec<BoardMinionView>,
    pub reroll_cost: u8,
    pub upgrade_cost: Option<u16>,
    /// 本回合是否已结束招募。
    pub recruited: bool,
    /// 本座位待选队列中的第一条。
    pub pending_choices: Vec<PendingChoiceView>,
    pub affordances: Affordances,
}

/// 商店槽位的公开视图。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShopSlotView {
    pub index: u8,
    pub card_id: Option<String>,
    pub frozen: bool,
}

/// 手牌的公开视图。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandCardView {
    pub index: u8,
    pub card_id: String,
    /// 该牌是否需要玩家指定单一目标（效果牌口径，随从牌恒为 `false`）。
    pub requires_target: bool,
}

/// 单项属性（攻击或生命）的分层构成视图。
///
/// `permanent` 已把可移除的成长与不可移除的永久增量合并；
/// `damage` 为层和与已夹非负的 `current` 之差，保证
/// `initial + permanent + turn + aura - damage == current`。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StatBreakdownView {
    pub initial: i32,
    pub permanent: i32,
    pub turn: i32,
    pub aura: i32,
    pub damage: i32,
    pub current: i32,
}

/// 随从攻击与生命的分层构成视图。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MinionStatsView {
    pub attack: StatBreakdownView,
    pub health: StatBreakdownView,
}

/// 棋盘随从的公开视图。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardMinionView {
    pub index: u8,
    pub minion_id: u64,
    pub card_id: String,
    pub golden: bool,
    pub attack: i32,
    pub health: i32,
    /// 攻击与生命的当前量与分层构成。
    pub stats: MinionStatsView,
    /// 当前生效的关键词，按字典序。
    pub keywords: Vec<String>,
}

/// 其他座位的公开摘要；不含任何私密数据。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerSummaryView {
    pub seat: Seat,
    pub hp: i32,
    pub era: u8,
    pub alive: bool,
    pub board_count: u8,
    pub hero_card_id: Option<String>,
    pub recruited: bool,
    /// 最终名次；未进入排名时为 `None`。
    pub placement: Option<u8>,
}

/// 等待该座位做出的选择。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingChoiceView {
    Discover {
        candidates: Vec<String>,
    },
    Evolve {
        options: Vec<String>,
    },
    /// 战吼目标待选：候选恒为本座位当前棋盘上的全部随从，不在视图里下发。
    Target {
        source_minion_id: u64,
    },
}

/// 本座位各操作的可用性；`*_disabled_reason` 在可操作时为 `None`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Affordances {
    pub shop_slots: Vec<ShopAffordanceView>,
    pub hand_cards: Vec<HandAffordanceView>,
    pub board_minions: Vec<BoardAffordanceView>,
    pub can_reroll: bool,
    pub can_upgrade_era: bool,
    pub upgrade_disabled_reason: Option<RejectCode>,
    pub can_use_hero_power: bool,
    pub hero_power_disabled_reason: Option<RejectCode>,
    pub can_end_recruit: bool,
}

/// 单个商店槽位的可购买性。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShopAffordanceView {
    pub index: u8,
    pub buyable: bool,
    pub disabled_reason: Option<RejectCode>,
}

/// 单张手牌的可打出性。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandAffordanceView {
    pub index: u8,
    pub playable: bool,
    pub disabled_reason: Option<RejectCode>,
}

/// 单个棋盘随从的可出售性。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardAffordanceView {
    pub index: u8,
    pub sellable: bool,
    pub disabled_reason: Option<RejectCode>,
}

/// 构造指定座位的视图；座位不存在时返回 `None`。
///
/// 纯函数：不修改 `state` / `engine`，不推进随机流。
pub fn view(engine: &Engine, state: &GameState, seat: Seat) -> Option<SeatView> {
    let player = state.players.get(usize::from(seat))?;

    // 阶段门控集中在这里；`check_*` 只做意图内校验。
    let activatable = state.phase == Phase::Recruit && !player.eliminated;
    // 队首是目标待选时，该座位只能选定目标：其余操作一律按阶段不可用处理。
    let head_is_target = state
        .pending
        .iter()
        .find(|pending| pending_seat(pending) == seat)
        .is_some_and(|pending| matches!(pending, PendingChoice::Target { .. }));
    let gate = if activatable && !player.recruit_done && !head_is_target {
        None
    } else {
        Some(RejectCode::InvalidStage)
    };

    let shop_slots = (0..player.shop.len())
        .map(|index| {
            let index = u8::try_from(index).unwrap_or(u8::MAX);
            match gate {
                Some(code) => ShopAffordanceView {
                    index,
                    buyable: false,
                    disabled_reason: Some(code),
                },
                None => match check_buy(engine, state, seat, index) {
                    Ok(()) => ShopAffordanceView {
                        index,
                        buyable: true,
                        disabled_reason: None,
                    },
                    Err(reject) => ShopAffordanceView {
                        index,
                        buyable: false,
                        disabled_reason: Some(reject.code),
                    },
                },
            }
        })
        .collect();

    let hand_cards = (0..player.hand.len())
        .map(|index| {
            let index = u8::try_from(index).unwrap_or(u8::MAX);
            match gate {
                Some(code) => HandAffordanceView {
                    index,
                    playable: false,
                    disabled_reason: Some(code),
                },
                None => match check_play_card(engine, state, seat, index) {
                    Ok(()) => HandAffordanceView {
                        index,
                        playable: true,
                        disabled_reason: None,
                    },
                    Err(reject) => HandAffordanceView {
                        index,
                        playable: false,
                        disabled_reason: Some(reject.code),
                    },
                },
            }
        })
        .collect();

    let board_minions = player
        .board
        .iter()
        .enumerate()
        .map(|(index, minion)| {
            let index = u8::try_from(index).unwrap_or(u8::MAX);
            let sellable = match gate {
                Some(code) => (false, Some(code)),
                None => match check_sell(state, seat, minion.instance_id) {
                    Ok(()) => (true, None),
                    Err(reject) => (false, Some(reject.code)),
                },
            };
            BoardAffordanceView {
                index,
                sellable: sellable.0,
                disabled_reason: sellable.1,
            }
        })
        .collect();

    let can_reroll = gate.is_none() && check_reroll(state, seat).is_ok();

    let (can_upgrade_era, upgrade_disabled_reason) = match gate {
        Some(code) => (false, Some(code)),
        None => match check_upgrade(engine, state, seat) {
            Ok(()) => (true, None),
            Err(reject) => (false, Some(reject.code)),
        },
    };

    let hero_power_skill_id = player
        .hero
        .as_deref()
        .and_then(|hero_id| engine.catalog.cards.get(hero_id))
        .and_then(|hero| {
            hero.skills.iter().find(|skill_id| {
                engine
                    .catalog
                    .skills
                    .get(*skill_id)
                    .is_some_and(|skill| skill.cost.is_some())
            })
        })
        .cloned();

    let hero_power_uses_left = hero_power_skill_id
        .as_deref()
        .and_then(|skill_id| {
            engine.catalog.skills.get(skill_id).map(|skill| {
                let limit = skill.uses_per_turn.unwrap_or(1);
                let used = player.hero_uses.get(skill_id).copied().unwrap_or(0);
                limit.saturating_sub(used)
            })
        })
        .unwrap_or(0);

    let (can_use_hero_power, hero_power_disabled_reason) = match gate {
        Some(code) => (false, Some(code)),
        None => match hero_power_skill_id.as_deref() {
            None => (false, Some(RejectCode::HeroPowerLimit)),
            Some(skill_id) => match check_hero_power(engine, state, seat, skill_id) {
                Ok(()) => (true, None),
                Err(reject) => (false, Some(reject.code)),
            },
        },
    };

    // 已结束招募时重复 `EndRecruit` 幂等成功；出局、非招募阶段或目标待选期间不可操作。
    let can_end_recruit = activatable && !head_is_target && check_end_recruit(state, seat).is_ok();

    let shop = player
        .shop
        .iter()
        .enumerate()
        .map(|(index, shop_slot)| ShopSlotView {
            index: u8::try_from(index).unwrap_or(u8::MAX),
            card_id: shop_slot.as_ref().map(|slot| slot.card_id.clone()),
            frozen: shop_slot.as_ref().is_some_and(|slot| slot.frozen),
        })
        .collect();

    let hand = player
        .hand
        .iter()
        .enumerate()
        .map(|(index, hand_card)| HandCardView {
            index: u8::try_from(index).unwrap_or(u8::MAX),
            requires_target: engine
                .requires_target(&hand_card.card_id, crate::effects::Trigger::OnPlayEffect),
            card_id: hand_card.card_id.clone(),
        })
        .collect();

    // 棋盘数值与关键词必须先折叠光环，否则客户端看到的是未加成的数值。
    let folded_board = settle::fold_auras(&engine.catalog, &state.auras, &player.board);
    let board = folded_board
        .iter()
        .enumerate()
        .map(|(index, minion)| BoardMinionView {
            index: u8::try_from(index).unwrap_or(u8::MAX),
            minion_id: minion.instance_id,
            card_id: minion.card_id.clone(),
            golden: minion.golden,
            attack: combat::effective_attack(&engine.catalog, minion),
            health: combat::effective_health(&engine.catalog, minion),
            stats: stats_breakdown(minion),
            keywords: combat::keywords(&engine.catalog, minion)
                .into_iter()
                .collect(),
        })
        .collect();

    let pending_choices = state
        .pending
        .iter()
        .find(|pending| pending_seat(pending) == seat)
        .map(|pending| match pending {
            PendingChoice::Discover { candidates, .. } => PendingChoiceView::Discover {
                candidates: candidates.clone(),
            },
            PendingChoice::Evolve { .. } => PendingChoiceView::Evolve {
                options: EVOLVE_CHOICES
                    .iter()
                    .map(|option| (*option).to_string())
                    .collect(),
            },
            PendingChoice::Target { source, .. } => PendingChoiceView::Target {
                source_minion_id: *source,
            },
        })
        .into_iter()
        .collect();

    let players = state
        .players
        .iter()
        .map(|other| player_summary(state, other))
        .collect();

    let opponent_seat = state.pairings.iter().find_map(|pairing| {
        if pairing.ghost.is_some() || (pairing.a != seat && pairing.b != seat) {
            return None;
        }
        Some(if pairing.a == seat {
            pairing.b
        } else {
            pairing.a
        })
    });

    Some(SeatView {
        phase: state.phase,
        turn: state.turn,
        seat,
        opponent_seat,
        self_view: SelfView {
            hp: player.hp,
            era: player.era as u8,
            gold: player.gold,
            hero_card_id: player.hero.clone(),
            hero_power_skill_id,
            hero_power_uses_left,
            shop,
            hand,
            board,
            reroll_cost: u8::from(player.first_refresh_used),
            upgrade_cost: engine
                .catalog
                .config
                .era_costs
                .get(era_index(player.era))
                .copied(),
            recruited: player.recruit_done,
            pending_choices,
            affordances: Affordances {
                shop_slots,
                hand_cards,
                board_minions,
                can_reroll,
                can_upgrade_era,
                upgrade_disabled_reason,
                can_use_hero_power,
                hero_power_disabled_reason,
                can_end_recruit,
            },
        },
        players,
        enabled_tribes: state
            .enabled_tribes
            .iter()
            .map(|tribe| tribe_name(*tribe).to_string())
            .collect(),
    })
}

/// 构造随从的分层构成视图。
///
/// `current` 先取 [`MinionInstance::attack`] / [`MinionInstance::health`]
/// （已夹非负），再反推 `damage`，因此招募期负血量会被 `damage` 吸收。
/// 学者攻击沿用 `attack()` 的口径：六层全部投影为 0，存储层保留加成。
///
/// 服务层的协议映射复用它，保证规则引擎与服务端视图口径一致。
pub fn stats_breakdown(minion: &MinionInstance) -> MinionStatsView {
    MinionStatsView {
        attack: if minion.is_scholar {
            StatBreakdownView::default()
        } else {
            breakdown(&minion.stats.attack, minion.attack())
        },
        health: breakdown(&minion.stats.health, minion.health()),
    }
}

/// 单项属性的分层构成；`permanent` 合并成长与其他永久增量。
fn breakdown(stat: &StatComponent, current: i32) -> StatBreakdownView {
    let permanent = stat.growth.saturating_add(stat.permanent);
    StatBreakdownView {
        initial: stat.initial,
        permanent,
        turn: stat.turn,
        aura: stat.aura,
        damage: stat.layers().saturating_sub(current),
        current,
    }
}

/// `ranking` 下标 + 1；仅在 `GameOver` 阶段公开名次，其余阶段返回 `None`。
///
/// 对局进行中 `ranking` 只是「已出局者在前、存活者补齐在尾部」的临时块，
/// 下标 + 1 并不等于最终名次；协议要求结束前 `placement` 为 `null`。
pub fn placement_of(state: &GameState, seat: Seat) -> Option<u8> {
    if state.phase != Phase::GameOver {
        return None;
    }
    state
        .ranking
        .iter()
        .position(|ranked| *ranked == seat)
        .map(|index| u8::try_from(index + 1).unwrap_or(u8::MAX))
}

fn pending_seat(pending: &PendingChoice) -> Seat {
    match pending {
        PendingChoice::Discover { seat, .. }
        | PendingChoice::Evolve { seat, .. }
        | PendingChoice::Target { seat, .. } => *seat,
    }
}

fn player_summary(state: &GameState, player: &PlayerState) -> PlayerSummaryView {
    PlayerSummaryView {
        seat: player.seat,
        hp: player.hp,
        era: player.era as u8,
        alive: !player.eliminated,
        board_count: u8::try_from(player.board.len()).unwrap_or(u8::MAX),
        hero_card_id: player.hero.clone(),
        recruited: player.recruit_done,
        placement: placement_of(state, player.seat),
    }
}

/// 时代在 `config.era_costs` 中的下标；未来时代为 6，超出费用表。
fn era_index(era: Era) -> usize {
    (era as usize).saturating_sub(1)
}

/// 流派的中文名，与数据文件中的 `tribe` 取值一致。
fn tribe_name(tribe: Tribe) -> &'static str {
    match tribe {
        Tribe::Human => "人类",
        Tribe::Machine => "机械",
        Tribe::Biology => "生物",
        Tribe::Plant => "植物",
        Tribe::Virus => "病毒",
        Tribe::Scholar => "学者",
        Tribe::General => "通用",
    }
}
