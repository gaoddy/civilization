//! 逐座位事件过滤与战斗回放下发。
//!
//! 引擎事件不区分接收者，协议 `StateUpdate.events` 却按座位下发。本模块集中定义
//! 两条规则：
//!
//! - [`for_seat`]：把引擎事件映射为发给指定座位的协议事件。购买、卖出、刷新、
//!   升级、三连与发现属于隐藏信息，只发给操作者；伤害、淘汰、回合、阶段与终局
//!   属于公开信息，发给所有座位；战斗结果不下发（改走 [`combat_result`]）。
//! - [`combat_result`]：把逐座位战斗报告映射为协议 `CombatResult`，动作类型固定
//!   为 `attack`/`death`/`reborn`/`contagion`，下标与侧别都相对接收该消息的座位。
//!
//! 本模块是纯函数层：不读时钟、不做 I/O。

use std::num::NonZeroU64;

use protocol::view::GameEvent as ProtocolGameEvent;
use protocol::{
    CombatResult, CombatResultActionsItem, CombatResultActionsItemSourceSide,
    CombatResultActionsItemTargetSide, GameEventKind,
};
use rule_engine::combat::CombatAction;
use rule_engine::data::Catalog;
use rule_engine::events::GameEvent;
use rule_engine::settle::SeatCombatReport;
use rule_engine::state::{InstanceId, Seat};

use crate::view::{SeatInfo, board_minion_from, phase_to_protocol};

/// 把引擎事件映射为发给 `seat` 的协议事件；不需要发的事件返回 `None`。
///
/// 私有事件只在 `event.seat == seat` 时下发；公开事件对所有座位都返回 `Some`。
/// 事件涉及某个座位时，`playerId` 填该座位的 `player_id`（私有事件即本人），
/// `seat` 也一并携带；`TurnStarted`/`PhaseChanged`/`GameOver` 不指向单个玩家，
/// `playerId` 保持 `null`。
/// 协议没有携带的细节（如购买槽位、升级后的时代）不另行造字段，接收方以
/// `PlayerView` 的公开摘要为准。
pub fn for_seat(event: &GameEvent, seat: Seat, seats: &[SeatInfo; 8]) -> Option<ProtocolGameEvent> {
    match event {
        GameEvent::MinionBought {
            seat: owner,
            card_id,
            ..
        } => {
            if !private(*owner, seat) {
                return None;
            }
            Some(
                protocol_event(GameEventKind::MinionBought, Some(*owner), seats)?
                    .with_card_id(card_id.clone()),
            )
        }
        GameEvent::MinionSold {
            seat: owner,
            minion,
        } => {
            if !private(*owner, seat) {
                return None;
            }
            Some(
                protocol_event(GameEventKind::MinionSold, Some(*owner), seats)?
                    .with_minion_id(minion.to_string()),
            )
        }
        GameEvent::ShopRolled { seat: owner } => {
            if !private(*owner, seat) {
                return None;
            }
            protocol_event(GameEventKind::ShopRolled, Some(*owner), seats)
        }
        GameEvent::EraUpgraded { seat: owner, .. } => {
            if !private(*owner, seat) {
                return None;
            }
            protocol_event(GameEventKind::EraUpgraded, Some(*owner), seats)
        }
        GameEvent::TripleFormed {
            seat: owner,
            card_id,
        } => {
            if !private(*owner, seat) {
                return None;
            }
            Some(
                protocol_event(GameEventKind::TripleFormed, Some(*owner), seats)?
                    .with_card_id(card_id.clone()),
            )
        }
        GameEvent::CardDiscovered {
            seat: owner,
            card_id,
        } => {
            if !private(*owner, seat) {
                return None;
            }
            Some(
                protocol_event(GameEventKind::CardDiscovered, Some(*owner), seats)?
                    .with_card_id(card_id.clone()),
            )
        }
        GameEvent::DamageTaken {
            seat: target,
            amount,
        } => {
            let mut event = protocol_event(GameEventKind::DamageTaken, Some(*target), seats)?;
            event.amount = Some(i64::from(*amount));
            Some(event)
        }
        GameEvent::PlayerEliminated {
            seat: eliminated,
            place,
        } => {
            let mut event =
                protocol_event(GameEventKind::PlayerEliminated, Some(*eliminated), seats)?;
            event.place = NonZeroU64::new(u64::from(*place));
            Some(event)
        }
        GameEvent::TurnStarted { turn } => {
            let mut event = protocol_event(GameEventKind::TurnStarted, None, seats)?;
            event.turn = turn_of(*turn);
            Some(event)
        }
        GameEvent::PhaseChanged { phase, turn } => {
            let mut event = protocol_event(GameEventKind::PhaseChanged, None, seats)?;
            event.phase = Some(phase_to_protocol(*phase));
            event.turn = turn_of(*turn);
            Some(event)
        }
        GameEvent::GameOver { ranking } => {
            let mut event = protocol_event(GameEventKind::GameOver, None, seats)?;
            event.ranking = ranking.iter().map(|seat| i64::from(*seat)).collect();
            Some(event)
        }
        // 战斗结果只走 CombatResult，避免同一场战斗在事件流里出现两次。
        GameEvent::CombatResolved { .. } => None,
    }
}

/// 把逐座位战斗报告映射为协议 `CombatResult`。
///
/// `own_board` 是接收座位自己的棋盘，因此己方下标标 `own`、对手下标标
/// `opponent`；召唤物等不在战前快照里的实例找不到下标时，下标与侧别都写
/// `null`。`mirror` 原样取自报告；镜像战由调用方只发给该座位。
pub fn combat_result(
    catalog: &Catalog,
    report: &SeatCombatReport,
    seats: &[SeatInfo; 8],
) -> CombatResult {
    let own_board = report
        .own_board
        .iter()
        .enumerate()
        .map(|(index, minion)| board_minion_from(catalog, index, minion, false))
        .collect();
    let opponent_board = report
        .opponent_board
        .iter()
        .enumerate()
        .map(|(index, minion)| board_minion_from(catalog, index, minion, false))
        .collect();
    let actions = report
        .actions
        .iter()
        .map(|action| combat_action(report, action))
        .collect();

    CombatResult {
        actions,
        damage: u64::try_from(report.damage).unwrap_or(0),
        mirror: Some(report.mirror),
        opponent_board,
        own_board,
        winner_id: report
            .winner
            .and_then(|seat| seats.get(usize::from(seat)))
            .map(|info| info.player_id.clone()),
    }
}

/// 私有事件是否应发给该座位。
fn private(owner: Seat, seat: Seat) -> bool {
    owner == seat
}

/// 构造一条只有 `kind` 与 `seat` 的事件；其余可选字段保持缺省。
///
/// `seat` 指向事件涉及的座位时，`playerId` 填该座位的玩家 id；座位越界说明
/// 调用方数据损坏，返回 `None`。不指向单个玩家的事件 `playerId` 为 `null`。
fn protocol_event(
    kind: GameEventKind,
    seat: Option<Seat>,
    seats: &[SeatInfo; 8],
) -> Option<ProtocolGameEvent> {
    let player_id = match seat {
        Some(seat) => Some(seats.get(usize::from(seat))?.player_id.clone()),
        None => None,
    };
    Some(ProtocolGameEvent {
        amount: None,
        card_id: None,
        kind,
        minion_id: None,
        phase: None,
        place: None,
        player_id,
        ranking: Vec::new(),
        seat: seat.map(i64::from),
        turn: None,
    })
}

/// 引擎回合号转协议非零回合号；回合从 1 起，0 视为不携带。
fn turn_of(turn: u8) -> Option<NonZeroU64> {
    NonZeroU64::new(u64::from(turn))
}

trait ProtocolEventExt {
    fn with_card_id(self, card_id: String) -> Self;
    fn with_minion_id(self, minion_id: String) -> Self;
}

impl ProtocolEventExt for ProtocolGameEvent {
    fn with_card_id(mut self, card_id: String) -> Self {
        self.card_id = Some(card_id);
        self
    }

    fn with_minion_id(mut self, minion_id: String) -> Self {
        self.minion_id = Some(minion_id);
        self
    }
}

/// 一条战斗动作涉及的棋盘侧别（相对接收该消息的座位）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Own,
    Opponent,
}

/// 战斗动作转协议：`kind` 固定为 `attack|death|reborn|contagion`。
fn combat_action(report: &SeatCombatReport, action: &CombatAction) -> CombatResultActionsItem {
    match action {
        CombatAction::Attack {
            attacker,
            target,
            damage,
            blocked,
        } => {
            let source = locate(report, *attacker);
            let target = locate(report, *target);
            CombatResultActionsItem {
                damage: Some(if *blocked { 0 } else { i64::from(*damage) }),
                kind: "attack".to_string(),
                source_index: source.map(|(index, _)| index as i64),
                source_side: source.map(|(_, side)| source_side(side)),
                target_index: target.map(|(index, _)| index as i64),
                target_side: target.map(|(_, side)| target_side(side)),
            }
        }
        CombatAction::Death { minion } => {
            let source = locate(report, *minion);
            CombatResultActionsItem {
                damage: None,
                kind: "death".to_string(),
                source_index: source.map(|(index, _)| index as i64),
                source_side: source.map(|(_, side)| source_side(side)),
                target_index: None,
                target_side: None,
            }
        }
        CombatAction::Reborn { minion } => {
            let source = locate(report, *minion);
            CombatResultActionsItem {
                damage: None,
                kind: "reborn".to_string(),
                source_index: source.map(|(index, _)| index as i64),
                source_side: source.map(|(_, side)| source_side(side)),
                target_index: None,
                target_side: None,
            }
        }
        CombatAction::ContagionApplied { target } => {
            let target = locate(report, *target);
            CombatResultActionsItem {
                damage: None,
                kind: "contagion".to_string(),
                source_index: None,
                source_side: None,
                target_index: target.map(|(index, _)| index as i64),
                target_side: target.map(|(_, side)| target_side(side)),
            }
        }
    }
}

fn source_side(side: Side) -> CombatResultActionsItemSourceSide {
    match side {
        Side::Own => CombatResultActionsItemSourceSide::Own,
        Side::Opponent => CombatResultActionsItemSourceSide::Opponent,
    }
}

fn target_side(side: Side) -> CombatResultActionsItemTargetSide {
    match side {
        Side::Own => CombatResultActionsItemTargetSide::Own,
        Side::Opponent => CombatResultActionsItemTargetSide::Opponent,
    }
}

/// 在该座位视角的两份棋盘里定位实例：先本方、后对手。
///
/// 真实战斗的实例编号全局唯一；镜像战的幽灵快照会复用编号，此时先命中本方即
/// 可（两份棋盘内容相同）。
fn locate(report: &SeatCombatReport, minion: InstanceId) -> Option<(usize, Side)> {
    if let Some(index) = report
        .own_board
        .iter()
        .position(|candidate| candidate.instance_id == minion)
    {
        return Some((index, Side::Own));
    }
    report
        .opponent_board
        .iter()
        .position(|candidate| candidate.instance_id == minion)
        .map(|index| (index, Side::Opponent))
}
