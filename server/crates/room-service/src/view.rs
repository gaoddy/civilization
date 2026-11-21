//! 引擎视图到协议视图的纯函数映射。
//!
//! 本模块只做结构转换：不含网络、不做 I/O、不读时钟。`phaseDeadline` 由调用方
//! 以 RFC3339 文本传入；文本非法时返回 [`ProtocolError`]，绝不 panic。生成类型的
//! `NonZeroU64`、`[String; 5]` 与 `chrono::DateTime` 约束都在这里一次性适配，
//! 调用方（`game.rs`）只需提供已校验的引擎视图与座位表。

use chrono::{DateTime, Utc};
use protocol::view::{
    Affordances, BoardAffordance, BoardMinion, HandAffordance, HandCard, PendingChoice,
    PlayerSummary, PlayerView, ShopAffordance, ShopSlot,
};
use protocol::{
    HeroPower, MinionStatBreakdown, MinionStats, PlayerViewSelf, ProtocolError, TargetRef,
    TargetZone,
};
use rule_engine::combat;
use rule_engine::data::Catalog;
use rule_engine::effects::TargetRef as EngineTargetRef;
use rule_engine::intent::RejectCode;
use std::num::NonZeroU64;

use rule_engine::state::{MinionInstance, Phase};
use rule_engine::view::{
    MinionStatsView, PendingChoiceView, PlayerSummaryView, SeatView, SelfView, ShopSlotView,
    StatBreakdownView, stats_breakdown,
};

/// 座位上的玩家身份：协议里只需要 `playerId` 与展示名。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeatInfo {
    pub player_id: String,
    pub display_name: String,
}

/// 把引擎的逐座位视图映射为协议 `PlayerView`。
///
/// - `seats` 按座位索引，长度固定为 8；引擎座位越界时返回 `BAD_PAYLOAD`。
/// - `deadline` 是 RFC3339 文本，解析失败返回 `BAD_PAYLOAD`（不 panic）。
/// - `opponent_seat` 映射为对应座位的 `playerId`，`None` 保持 `null`。
/// - 所有 `disabled_reason` 使用 [`RejectCode::as_str`]，与 `errors.schema.json` 一致。
pub fn to_protocol(
    seat_view: &SeatView,
    room_id: &str,
    deadline: &str,
    seats: &[SeatInfo; 8],
    board_slots: usize,
    hp_cap: i32,
) -> Result<PlayerView, ProtocolError> {
    let seat_index = usize::from(seat_view.seat);
    let me = seats.get(seat_index).ok_or(ProtocolError("BAD_PAYLOAD"))?;

    let phase_deadline = DateTime::parse_from_rfc3339(deadline)
        .map_err(|_| ProtocolError("BAD_PAYLOAD"))?
        .with_timezone(&Utc);

    // 引擎回合从 1 起、时代从 1 起；生成类型要求 `NonZeroU64`。
    let turn = NonZeroU64::new(u64::from(seat_view.turn)).ok_or(ProtocolError("BAD_PAYLOAD"))?;

    // 本局固定启用 5 个玩法，长度不符说明引擎状态被破坏；协议字段名保持兼容。
    let enabled_factions: [String; 5] = seat_view
        .enabled_playstyles
        .clone()
        .try_into()
        .map_err(|_| ProtocolError("BAD_PAYLOAD"))?;

    let self_view = &seat_view.self_view;
    let players = seat_view
        .players
        .iter()
        .map(|summary| summary_to_protocol(summary, seats))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(PlayerView {
        board_slots: NonZeroU64::new(u64::try_from(board_slots).unwrap_or(u64::MAX))
            .ok_or(ProtocolError("BAD_PAYLOAD"))?,
        // 战报由调用方在组装后填入 `GameTask.combat_log`。
        combat_log: Vec::new(),
        // 生命上限来自 catalog 配置；非正值说明配置被破坏。
        hp_cap: u64::try_from(hp_cap)
            .ok()
            .and_then(NonZeroU64::new)
            .ok_or(ProtocolError("BAD_PAYLOAD"))?,
        enabled_factions,
        opponent_player_id: seat_view
            .opponent_seat
            .map(|opponent| player_id_of(seats, opponent))
            .transpose()?,
        phase: phase_to_protocol(seat_view.phase),
        phase_deadline,
        players,
        room_id: room_id.to_string(),
        self_: self_to_protocol(seat_view.seat, me, self_view)?,
        turn,
    })
}

/// 把一条战斗报告里的随从转换为协议 `BoardMinion`。
///
/// 战斗报告里的数值来自引擎折叠后的快照；攻击/生命下限为 0，协议字段为无符号。
/// `sellable` 由调用方决定（玩家视图取预检结果，战斗回放恒为 `false`）。
pub fn board_minion_from(
    catalog: &Catalog,
    index: usize,
    minion: &MinionInstance,
    sellable: bool,
) -> BoardMinion {
    BoardMinion {
        attack: non_negative(combat::effective_attack(catalog, minion)),
        card_id: minion.card_id.clone(),
        golden: minion.golden,
        health: non_negative(combat::effective_health(catalog, minion)),
        index: u64::try_from(index).unwrap_or(u64::MAX),
        keywords: combat::keyword_set(catalog, minion).names(),
        minion_id: minion.instance_id.to_string(),
        sellable,
        stats: Some(stats_to_protocol(&stats_breakdown(minion))),
        stealth: combat::has_stealth(catalog, minion),
    }
}

// 战斗回放的映射见 `filter::combat_result`；本模块只保留玩家视图转换。
fn self_to_protocol(
    seat: u8,
    seat_info: &SeatInfo,
    view: &SelfView,
) -> Result<PlayerViewSelf, ProtocolError> {
    let era = NonZeroU64::new(u64::from(view.era)).ok_or(ProtocolError("BAD_PAYLOAD"))?;

    // 手牌的可打出性由预检给出，按同一下标合并进 `HandCard`。
    let hand = view
        .hand
        .iter()
        .map(|card| {
            let affordance = view
                .affordances
                .hand_cards
                .iter()
                .find(|candidate| candidate.index == card.index);
            HandCard {
                instance_id: card.instance_id.to_string(),
                card_id: card.card_id.clone(),
                golden: card.golden,
                attack: card.attack.map(i64::from),
                health: card.health.map(i64::from),
                reward_level: card
                    .reward_level
                    .and_then(|level| NonZeroU64::new(u64::from(level))),
                requires_target: card.requires_target,
                target_candidates: card
                    .target_candidates
                    .iter()
                    .map(target_to_protocol)
                    .collect(),
                disabled_reason: affordance
                    .and_then(|candidate| candidate.disabled_reason)
                    .map(reject_reason),
                index: i64::from(card.index),
                playable: affordance.is_some_and(|candidate| candidate.playable),
            }
        })
        .collect();

    let board = view
        .board
        .iter()
        .map(|minion| {
            let affordance = view
                .affordances
                .board_minions
                .iter()
                .find(|candidate| candidate.index == minion.index);
            BoardMinion {
                attack: non_negative(minion.attack),
                card_id: minion.card_id.clone(),
                golden: minion.golden,
                health: non_negative(minion.health),
                index: u64::from(minion.index),
                keywords: minion.keywords.clone(),
                minion_id: minion.minion_id.to_string(),
                sellable: affordance.is_some_and(|candidate| candidate.sellable),
                stats: Some(stats_to_protocol(&minion.stats)),
                stealth: minion.stealth,
            }
        })
        .collect();

    Ok(PlayerViewSelf {
        affordances: Affordances {
            board_minions: view
                .affordances
                .board_minions
                .iter()
                .map(|affordance| BoardAffordance {
                    disabled_reason: affordance.disabled_reason.map(reject_reason),
                    index: i64::from(affordance.index),
                    sellable: affordance.sellable,
                })
                .collect(),
            can_end_recruit: view.affordances.can_end_recruit,
            can_reroll: view.affordances.can_reroll,
            can_unfreeze: view.affordances.can_unfreeze,
            can_upgrade_era: view.affordances.can_upgrade_era,
            hand_cards: view
                .affordances
                .hand_cards
                .iter()
                .map(|affordance| HandAffordance {
                    disabled_reason: affordance.disabled_reason.map(reject_reason),
                    index: i64::from(affordance.index),
                    playable: affordance.playable,
                })
                .collect(),
            shop_slots: view
                .affordances
                .shop_slots
                .iter()
                .map(|affordance| ShopAffordance {
                    buyable: affordance.buyable,
                    disabled_reason: affordance.disabled_reason.map(reject_reason),
                    index: i64::from(affordance.index),
                })
                .collect(),
            upgrade_disabled_reason: view.affordances.upgrade_disabled_reason.map(reject_reason),
        },
        board,
        era,
        gold: u64::from(view.gold),
        hand,
        hero_card_id: view.hero_card_id.clone(),
        hero_powers: view
            .hero_powers
            .iter()
            .map(|power| HeroPower {
                can_use: power.can_use,
                disabled_reason: power.disabled_reason.map(reject_reason),
                requires_target: power.requires_target,
                skill_id: power.skill_id.clone(),
                target_candidates: power
                    .target_candidates
                    .iter()
                    .map(target_to_protocol)
                    .collect(),
                uses_left: u64::from(power.uses_left),
            })
            .collect(),
        hp: i64::from(view.hp),
        pending_choices: view
            .pending_choices
            .iter()
            .map(pending_to_protocol)
            .collect(),
        player_id: seat_info.player_id.clone(),
        recruited: view.recruited,
        reroll_cost: u64::from(view.reroll_cost),
        seat: i64::from(seat),
        shop: view.shop.iter().map(shop_slot_to_protocol).collect(),
        upgrade_cost: view.upgrade_cost.map(i64::from),
    })
}

fn summary_to_protocol(
    summary: &PlayerSummaryView,
    seats: &[SeatInfo; 8],
) -> Result<PlayerSummary, ProtocolError> {
    let info = seats
        .get(usize::from(summary.seat))
        .ok_or(ProtocolError("BAD_PAYLOAD"))?;
    let era = NonZeroU64::new(u64::from(summary.era)).ok_or(ProtocolError("BAD_PAYLOAD"))?;
    Ok(PlayerSummary {
        alive: summary.alive,
        board_count: i64::from(summary.board_count),
        display_name: info.display_name.clone(),
        era,
        hero_card_id: summary.hero_card_id.clone(),
        hp: i64::from(summary.hp),
        managed: false,
        placement: summary.placement.map(i64::from),
        player_id: info.player_id.clone(),
        // `rank_of` 恒 ≥ 1，`NonZeroU64` 映射不会丢值。
        rank: summary
            .rank
            .and_then(|rank| NonZeroU64::new(u64::from(rank))),
        recruited: summary.recruited,
        seat: i64::from(summary.seat),
    })
}

fn shop_slot_to_protocol(slot: &ShopSlotView) -> ShopSlot {
    ShopSlot {
        instance_id: slot.instance_id.map(|id| id.to_string()),
        card_id: slot.card_id.clone(),
        frozen: slot.frozen,
        index: i64::from(slot.index),
    }
}

fn pending_to_protocol(pending: &PendingChoiceView) -> PendingChoice {
    match pending {
        PendingChoiceView::Discover { candidates } => PendingChoice::Discover {
            candidates: candidates.clone(),
        },
        PendingChoiceView::Evolve { options } => PendingChoice::Evolve {
            options: options.clone(),
        },
        PendingChoiceView::Target {
            source_minion_id,
            candidates,
        } => PendingChoice::Target {
            source_minion_id: source_minion_id.to_string(),
            candidates: candidates.iter().map(target_to_protocol).collect(),
        },
    }
}

/// 引擎目标（区域 + 实例编号）转协议；编号统一为字符串。
fn target_to_protocol(target: &EngineTargetRef) -> TargetRef {
    TargetRef {
        zone: match target.zone {
            rule_engine::effects::Zone::Hand => TargetZone::Hand,
            rule_engine::effects::Zone::Shop => TargetZone::Shop,
            rule_engine::effects::Zone::Board => TargetZone::Board,
        },
        id: target.id.to_string(),
    }
}

/// 引擎分层构成转协议；字段一一对应，`current` 已在引擎视图夹非负。
fn stats_to_protocol(stats: &MinionStatsView) -> MinionStats {
    MinionStats {
        attack: breakdown_to_protocol(&stats.attack),
        health: breakdown_to_protocol(&stats.health),
    }
}

fn breakdown_to_protocol(breakdown: &StatBreakdownView) -> MinionStatBreakdown {
    MinionStatBreakdown {
        aura: i64::from(breakdown.aura),
        current: non_negative(breakdown.current),
        damage: non_negative(breakdown.damage),
        initial: non_negative(breakdown.initial),
        permanent: i64::from(breakdown.permanent),
        turn: i64::from(breakdown.turn),
    }
}

fn player_id_of(seats: &[SeatInfo; 8], seat: u8) -> Result<String, ProtocolError> {
    seats
        .get(usize::from(seat))
        .map(|info| info.player_id.clone())
        .ok_or(ProtocolError("BAD_PAYLOAD"))
}

/// 拒绝码转协议字符串；取值与 `errors.schema.json` 一致。
fn reject_reason(code: RejectCode) -> String {
    code.as_str().to_string()
}

/// 引擎阶段转协议阶段；两边变体一一对应。
pub(crate) fn phase_to_protocol(phase: Phase) -> protocol::view::Phase {
    match phase {
        Phase::Lobby => protocol::view::Phase::Lobby,
        Phase::Recruit => protocol::view::Phase::Recruit,
        Phase::Combat => protocol::view::Phase::Combat,
        Phase::Settle => protocol::view::Phase::Settle,
        Phase::GameOver => protocol::view::Phase::GameOver,
    }
}

/// 协议无符号字段：引擎数值下限为 0，负值按 0 处理。
fn non_negative(value: i32) -> u64 {
    u64::try_from(value.max(0)).unwrap_or(0)
}
