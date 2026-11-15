//! 对局驱动：把房间收到的命令接到规则引擎上，负责计时、意图去重与逐座位视图。
//!
//! 设计约定：
//! - 引擎保持纯库：本模块所有时间判断都走注入的 [`Clock`]，不读 `SystemTime`。
//! - 阶段推进串行且只有一条路径：[`GameTask::advance_phase`]。全员结束招募与计时
//!   到点都调用它，顺序固定为 `finish_recruit → resolve_combat → 逐座位
//!   CombatResult → resolve_settle → start_turn`。
//! - 意图去重键为 `(player_id, clientIntentId)`，表只增不删、随房间销毁。
//! - 每个座位有独立的 [`SeatChannel`]，`seq` 由 [`GameTask::envelope_for`] 在编码
//!   下行消息时按座位各自递增，互不影响。
//! - 下行发送端由 `GameTask` 单一持有：首次建连与重连都经 [`GameTask::bind_sender`]
//!   注册，连接层只借用投递，避免两份所有权在重连后指向不同连接。
//! - 重连与快照请求共用全量 `Snapshot` 路径（M1 不重放历史），快照载荷在编码时由
//!   当前座位视图生成，含未消费的 `pendingChoices`。
//! - 断线保留窗口（[`RECONNECT_GRACE_MS`]）是服务端内部状态：超时只把座位标记为
//!   离线，不释放座位、不结束对局，也不向任何座位下发通知；客户端靠重连快照恢复。
//! - `View` 更新按「该座位视角有变化」推送：私有操作只推操作者；全员可见的变化
//!   （`EndRecruit` 的 `recruited` 标记、阶段推进、回合开始）推给所有座位。
//!   事件按 [`for_seat`] 的规则过滤后随该座位的 `StateUpdate.events` 一起下发；
//!   战斗结果只走逐座位 `CombatResult`。

use std::collections::BTreeMap;
use std::sync::Arc;

use protocol::server::{CombatResult, ErrorMessage, GameOver, GameStarted, Snapshot, StateUpdate};
use protocol::view::GameEvent as ProtocolGameEvent;
use protocol::{ErrorCode, GameOverRankingItem, ProtocolError, ServerEnvelope};
use rule_engine::data::Catalog;
use rule_engine::engine::Engine;
use rule_engine::error::EngineError;
use rule_engine::events::GameEvent as EngineEvent;
use rule_engine::intent::{Intent as EngineIntent, RejectCode};
use rule_engine::rng::seed_from_u64;
use rule_engine::settle::SeatCombatReport;
use rule_engine::state::{GameState, InstanceId, Phase, Seat};
use rule_engine::view::view;
use serde_json::{Map, Value};
use tokio::sync::mpsc;

use crate::clock::Clock;
use crate::envelope::SeatChannel;
use crate::filter::{combat_result, for_seat};
use crate::room::{PlayerSlot, SEAT_COUNT};
use crate::view::{SeatInfo, to_protocol};

/// 每个招募回合的时长（秒）。
pub const RECRUIT_SECONDS: u64 = 90;

/// 剩余多少秒时下发计时警告（由连接层使用，M1a 只暴露常量与 [`GameTask::seconds_left`]）。
pub const TIMER_WARNING_SECONDS: u64 = 10;

/// 断线后座位的保留时长（毫秒）。超过该时长座位被标记离线，但不释放座位、不结束对局。
pub const RECONNECT_GRACE_MS: u64 = 30_000;

/// 一条来自连接层的房间命令。
#[derive(Debug)]
pub enum RoomCommand {
    /// 大厅准备；对局开始后到达即视为当前阶段不允许。
    Ready { player_id: String, ready: bool },
    /// 大厅选英雄；对局开始后到达即视为当前阶段不允许。
    SelectHero { player_id: String, card_id: String },
    /// 招募意图；`client_intent_id` 用于幂等去重。
    Intent {
        player_id: String,
        client_intent_id: String,
        intent: protocol::intents::Intent,
    },
    /// 请求当前全量视图；与重连共用同一条快照路径。
    RequestSnapshot { player_id: String },
    /// 断线重连；本次连接的下行发送端替换为 `out`，并回一条全量快照。
    ///
    /// `last_seq` 在 M1 只用于判断是否需要历史重放；本版本一律不重放，因此新旧
    /// 序号都回全量快照。
    Reconnect {
        player_id: String,
        last_seq: u64,
        out: mpsc::Sender<ServerEnvelope>,
    },
    /// 连接断开；记录断开时刻，超过 [`RECONNECT_GRACE_MS`] 后由 [`GameTask::tick`]
    /// 标记离线。对局状态与座位不因断线改变。
    Disconnect { player_id: String },
}

/// 要发给某个座位的一条更新信号。
///
/// 载荷不在本结构里：调用方用 [`GameTask::envelope_for`] 按该座位自己的
/// `SeatChannel` 编码并分配 `seq`。
#[derive(Debug)]
pub struct SeatUpdate {
    pub player_id: String,
    pub kind: UpdateKind,
}

/// 更新类型。
#[derive(Debug)]
pub enum UpdateKind {
    /// 该座位视角有变化，应下发 `StateUpdate`；`events` 是已按该座位过滤的
    /// 协议事件，连同视图一起下发给同一个接收者。
    View { events: Vec<ProtocolGameEvent> },
    /// 全量快照；载荷在编码时由 [`GameTask::player_view`] 生成，`Snapshot.seq`
    /// 与该条信封的 `seq` 一致。
    Snapshot,
    /// 一场战斗的逐座位回放。
    CombatResult(Box<CombatResult>),
    /// 对局结束：排名与公开种子。
    GameOver(GameOver),
    /// 被拒绝的意图；`clientIntentId` 与请求一致。
    Error(ErrorMessage),
}

/// 去重表中的一个结果。
#[derive(Debug, Clone)]
pub enum Deduped {
    /// 已成功执行；重发只回当前视图，不再执行、不再广播。
    Applied,
    /// 已拒绝；重发同一条错误。
    Rejected { code: ErrorCode, message: String },
}

/// 对局驱动。
pub struct GameTask {
    engine: Engine,
    state: GameState,
    /// `playerId → 座位`；座位由服务端认定，客户端不能指定。
    seat_of: BTreeMap<String, Seat>,
    /// 按座位索引的玩家身份，长度固定为 [`SEAT_COUNT`]。
    seats: [SeatInfo; SEAT_COUNT],
    /// 每个连接的下行序号分配器；不同座位互相独立。
    channels: BTreeMap<Seat, SeatChannel>,
    clock: Arc<dyn Clock>,
    /// `(playerId, clientIntentId) → 结果`；只增不删。
    dedup: BTreeMap<(String, String), Deduped>,
    seed: u64,
    room_id: String,
    /// 当前招募阶段的截止时刻（Unix epoch 毫秒）。
    phase_deadline_ms: u64,
    /// 每个座位当前连接的下行发送端；首次建连经 [`GameTask::bind_sender`] 注册，
    /// 重连时被替换。所有权只在这里，连接层通过 [`GameTask::sender`] 借用投递。
    senders: BTreeMap<Seat, mpsc::Sender<ServerEnvelope>>,
    /// 座位是否在线；属于服务端内部状态，不通过下行消息下发。
    ///
    /// 断线宽限期结束后置为 `false`，重连（`Reconnect` / `bind_sender`）恢复。
    connected: [bool; SEAT_COUNT],
    /// 断线起始时刻（Unix epoch 毫秒）；`None` 表示当前未处于断线宽限期。
    disconnected_at_ms: [Option<u64>; SEAT_COUNT],
}

impl GameTask {
    /// 以已确定的英雄表开局并构造驱动。
    ///
    /// `players` 必须是按座位号升序排列的 8 个座位（`Room::players` 的约定）；
    /// 元素下标即座位号。`seed` 同时用于引擎随机流与 `GameOver.seed`。
    ///
    /// # Errors
    ///
    /// 座位数不是 8、英雄不合法或缺失时返回 [`EngineError`]。
    pub fn new(
        catalog: Catalog,
        seed: u64,
        heroes: BTreeMap<Seat, String>,
        players: Vec<PlayerSlot>,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, EngineError> {
        if players.len() != SEAT_COUNT {
            return Err(EngineError(format!(
                "对局需要 {SEAT_COUNT} 个座位，实际为 {}",
                players.len()
            )));
        }

        let (mut engine, mut state) = Engine::new(catalog, seed_from_u64(seed));
        engine.start_game(&mut state, &heroes)?;

        // 下标即座位号：`Room::players` 按 `BTreeMap<Seat, _>` 升序遍历，
        // 开局要求 8 个座位全满，因此下标 0..8 与座位 0..8 一一对应。
        let seats: [SeatInfo; SEAT_COUNT] = std::array::from_fn(|index| SeatInfo {
            player_id: players[index].player_id.clone(),
            display_name: players[index].display_name.clone(),
        });
        let seat_of = seats
            .iter()
            .enumerate()
            .map(|(seat, info)| (info.player_id.clone(), seat as Seat))
            .collect();
        let channels = players
            .iter()
            .enumerate()
            .map(|(seat, slot)| (seat as Seat, slot.channel.clone()))
            .collect();

        let phase_deadline_ms = clock
            .now_ms()
            .saturating_add(RECRUIT_SECONDS.saturating_mul(1000));

        Ok(Self {
            engine,
            state,
            seat_of,
            seats,
            channels,
            clock,
            dedup: BTreeMap::new(),
            seed,
            room_id: String::new(),
            phase_deadline_ms,
            senders: BTreeMap::new(),
            connected: [true; SEAT_COUNT],
            disconnected_at_ms: [None; SEAT_COUNT],
        })
    }

    /// 设置房间标识，供 `PlayerView.roomId` 与下行信封使用。
    #[must_use]
    pub fn with_room_id(mut self, room_id: impl Into<String>) -> Self {
        self.room_id = room_id.into();
        self
    }

    /// 玩家所在座位；不在房间里返回 `None`。
    pub fn seat_of(&self, player_id: &str) -> Option<Seat> {
        self.seat_of.get(player_id).copied()
    }

    /// 指定座位当前剩余招募秒数；已过点为 0。
    pub fn seconds_left(&self) -> u64 {
        self.phase_deadline_ms.saturating_sub(self.clock.now_ms()) / 1000
    }

    /// 指定座位当前协议视图；座位不存在或内部数据不合法时返回 `None`。
    pub fn player_view(&self, seat: Seat) -> Option<protocol::view::PlayerView> {
        let seat_view = view(&self.engine, &self.state, seat)?;
        to_protocol(
            &seat_view,
            &self.room_id,
            &self.deadline_rfc3339(),
            &self.seats,
        )
        .ok()
    }

    /// 指定座位当前的全量 `StateUpdate`；快照请求不携带事件。
    pub fn state_update(&self, seat: Seat) -> Option<StateUpdate> {
        self.state_update_with(seat, &[])
    }

    /// 带事件的 `StateUpdate`；`events` 必须已按该座位过滤。
    fn state_update_with(&self, seat: Seat, events: &[ProtocolGameEvent]) -> Option<StateUpdate> {
        Some(StateUpdate {
            events: events.to_vec(),
            view: self.player_view(seat)?,
        })
    }

    /// 指定座位已分配到的序号（尚未发送任何消息时为 0）。
    pub fn channel_seq(&self, seat: Seat) -> u64 {
        self.channels
            .get(&seat)
            .map(SeatChannel::next_seq)
            .unwrap_or(0)
    }

    /// 座位是否在线。宽限期内仍返回 `true`；超过 [`RECONNECT_GRACE_MS`] 后由
    /// [`GameTask::tick`] 置为 `false`，重连后恢复。越界座位视为离线。
    pub fn is_connected(&self, seat: Seat) -> bool {
        self.connected
            .get(usize::from(seat))
            .copied()
            .unwrap_or(false)
    }

    /// 座位当前绑定的下行发送端；连接层（Task 12）用它投递编码后的信封。
    ///
    /// 首次 WS 建连也必须先通过 [`GameTask::bind_sender`] 或
    /// [`RoomCommand::Reconnect`] 注册发送端；两条路径写同一张表，所有权只在本结构。
    pub fn sender(&self, seat: Seat) -> Option<&mpsc::Sender<ServerEnvelope>> {
        self.senders.get(&seat)
    }

    /// 绑定或替换座位当前连接的下行发送端。
    ///
    /// 首次连接与断线重连共用本方法：连接层在每次 WS 建连时调用（或发
    /// [`RoomCommand::Reconnect`]），避免 `GameTask` 与连接层各持一份发送端。
    /// 绑定同时恢复在线状态并清空断线宽限。
    ///
    /// 返回 `false` 表示 `player_id` 不在房间里，此时不保存发送端。
    pub fn bind_sender(&mut self, player_id: &str, out: mpsc::Sender<ServerEnvelope>) -> bool {
        let Some(seat) = self.seat_of.get(player_id).copied() else {
            return false;
        };
        let index = usize::from(seat);
        self.senders.insert(seat, out);
        self.connected[index] = true;
        self.disconnected_at_ms[index] = None;
        true
    }

    /// 构造该座位的 `GameStarted` 下行信封。
    ///
    /// 大厅 → 对局交接时使用：载荷是开局瞬间的 `PlayerView`，`seq` 继续沿用该座位
    /// 在大厅阶段已经推进过的通道，保证同一连接的序号单调递增。
    ///
    /// 座位不存在或视图/载荷无法编码时返回 `None`。
    pub fn game_started_for(&mut self, seat: Seat) -> Option<ServerEnvelope> {
        let view = self.player_view(seat)?;
        let payload = payload_map(&GameStarted { view }).ok()?;
        let channel = self.channels.get_mut(&seat)?;
        let mut envelope = channel.bind("GameStarted", payload).ok()?;
        envelope.player_id = Some(self.seats[usize::from(seat)].player_id.clone());
        if !self.room_id.is_empty() {
            envelope.room_id = Some(self.room_id.clone());
        }
        Some(envelope)
    }

    /// 把一条更新编码成该座位自己的下行信封，并按座位递增 `seq`。
    /// 不同座位的序号互相独立；同一座位每调用一次递增一次。
    ///
    /// # Errors
    ///
    /// 更新指向未知玩家或载荷无法序列化为 JSON 对象时返回 `BAD_PAYLOAD`。
    pub fn envelope_for(&mut self, update: &SeatUpdate) -> Result<ServerEnvelope, ProtocolError> {
        let seat = self
            .seat_of
            .get(&update.player_id)
            .copied()
            .ok_or(ProtocolError("BAD_PAYLOAD"))?;

        let (kind, payload) = match &update.kind {
            UpdateKind::View { events } => {
                let state_update = self
                    .state_update_with(seat, events)
                    .ok_or(ProtocolError("BAD_PAYLOAD"))?;
                ("StateUpdate", payload_map(&state_update)?)
            }
            UpdateKind::CombatResult(result) => ("CombatResult", payload_map(result.as_ref())?),
            UpdateKind::Snapshot => {
                // 先取该座位下一个序号，再把它写进载荷；随后 `bind` 分配的序号
                // 与载荷里的 `Snapshot.seq` 必然一致（单线程 `&mut self` 下无并发）。
                let next = self
                    .channels
                    .get(&seat)
                    .ok_or(ProtocolError("BAD_PAYLOAD"))?
                    .next_seq()
                    .saturating_add(1);
                let seq = std::num::NonZeroU64::new(next).ok_or(ProtocolError("BAD_PAYLOAD"))?;
                let snapshot = Snapshot {
                    seq: Some(seq),
                    view: self.player_view(seat).ok_or(ProtocolError("BAD_PAYLOAD"))?,
                };
                ("Snapshot", payload_map(&snapshot)?)
            }
            UpdateKind::GameOver(game_over) => ("GameOver", payload_map(game_over)?),
            UpdateKind::Error(error) => ("Error", payload_map(error)?),
        };

        let channel = self
            .channels
            .get_mut(&seat)
            .ok_or(ProtocolError("BAD_PAYLOAD"))?;
        let mut envelope = channel.bind(kind, payload)?;
        envelope.player_id = Some(update.player_id.clone());
        if !self.room_id.is_empty() {
            envelope.room_id = Some(self.room_id.clone());
        }
        Ok(envelope)
    }

    /// 处理一条命令，返回要发给各座位的更新；不等待网络。
    pub fn handle(&mut self, cmd: RoomCommand) -> Vec<SeatUpdate> {
        match cmd {
            RoomCommand::Ready { player_id, .. } | RoomCommand::SelectHero { player_id, .. } => {
                // 对局开始后大厅操作一律属于当前阶段不允许。
                self.player_error(&player_id, None, ErrorCode::InvalidStage, "对局已开始")
            }
            RoomCommand::RequestSnapshot { player_id } => self.snapshot_for_player(&player_id),
            RoomCommand::Reconnect {
                player_id,
                last_seq,
                out,
            } => self.reconnect(&player_id, last_seq, out),
            RoomCommand::Disconnect { player_id } => self.mark_disconnect(&player_id),
            RoomCommand::Intent {
                player_id,
                client_intent_id,
                intent,
            } => self.handle_intent(player_id, client_intent_id, intent),
        }
    }

    /// 计时检查：未到点返回空；到点串行推进到下一回合。
    ///
    /// 幂等：推进后阶段离开 `Recruit`，同一毫秒再次调用返回空。
    ///
    /// 顺序为「先推进阶段，再处理断线宽限」：阶段推进是时间语义的一部分，必须与
    /// 连接状态无关地按时发生；断线标记只影响可观测的在线状态，安排在推进之后，
    /// 不会改变原有阶段推进的调用次数与顺序。
    pub fn tick(&mut self) -> Vec<SeatUpdate> {
        let mut updates = Vec::new();
        if self.state.phase == Phase::Recruit && self.clock.now_ms() >= self.phase_deadline_ms {
            updates = self.advance_phase(Vec::new()).unwrap_or_default();
        }
        self.expire_disconnects();
        updates
    }

    /// 意图处理：先查去重表，再映射到引擎意图并执行。
    fn handle_intent(
        &mut self,
        player_id: String,
        client_intent_id: String,
        intent: protocol::intents::Intent,
    ) -> Vec<SeatUpdate> {
        let Some(seat) = self.seat_of.get(&player_id).copied() else {
            return vec![self.error_update(
                &player_id,
                Some(&client_intent_id),
                ErrorCode::NotInRoom,
                "玩家不在房间内",
            )];
        };

        let key = (player_id.clone(), client_intent_id.clone());
        if let Some(previous) = self.dedup.get(&key) {
            return match previous {
                Deduped::Applied => vec![self.view_update(seat)],
                Deduped::Rejected { code, message } => vec![self.error_update(
                    &player_id,
                    Some(&client_intent_id),
                    *code,
                    message.clone(),
                )],
            };
        }

        if self.state.phase != Phase::Recruit {
            let message = format!("当前阶段 {:?} 不接受招募意图", self.state.phase);
            self.dedup.insert(
                key,
                Deduped::Rejected {
                    code: ErrorCode::InvalidStage,
                    message: message.clone(),
                },
            );
            return vec![self.error_update(
                &player_id,
                Some(&client_intent_id),
                ErrorCode::InvalidStage,
                message,
            )];
        }

        let engine_intent = match map_intent(intent) {
            Ok(intent) => intent,
            Err(code) => {
                let message = format!("意图参数非法：{code}");
                self.dedup.insert(
                    key,
                    Deduped::Rejected {
                        code,
                        message: message.clone(),
                    },
                );
                return vec![self.error_update(&player_id, Some(&client_intent_id), code, message)];
            }
        };
        let ends_recruit = matches!(engine_intent, EngineIntent::EndRecruit);

        match self.engine.apply(&mut self.state, seat, engine_intent) {
            Ok(outcome) => {
                self.dedup.insert(key, Deduped::Applied);
                if self.state.phase == Phase::Combat {
                    return match self.advance_phase(outcome.events) {
                        Ok(updates) => updates,
                        Err(error) => {
                            self.player_error(&player_id, None, ErrorCode::InternalError, error.0)
                        }
                    };
                }
                let mut recipients = vec![seat];
                if ends_recruit {
                    // `recruited` 是所有座位可见的公开摘要字段，全员推视图。
                    recipients.extend((0..SEAT_COUNT as Seat).filter(|other| *other != seat));
                }
                self.view_updates(&outcome.events, &recipients)
            }
            Err(reject) => {
                let code = error_code_of(reject.code);
                let message = reject.message;
                self.dedup.insert(
                    key,
                    Deduped::Rejected {
                        code,
                        message: message.clone(),
                    },
                );
                vec![self.error_update(&player_id, Some(&client_intent_id), code, message)]
            }
        }
    }

    /// 把逐座位战斗报告转成 `CombatResult` 更新：一份报告对应一条更新。
    ///
    /// 镜像战在 `outcome.reports` 里只有该座位自己的一条，因此只会产生 1 条更新。
    pub fn combat_updates(&self, reports: &BTreeMap<Seat, SeatCombatReport>) -> Vec<SeatUpdate> {
        reports
            .iter()
            .map(|(&seat, report)| SeatUpdate {
                player_id: self.seats[usize::from(seat)].player_id.clone(),
                kind: UpdateKind::CombatResult(Box::new(combat_result(
                    &self.engine.catalog,
                    report,
                    &self.seats,
                ))),
            })
            .collect()
    }

    /// 招募阶段结束后的唯一推进路径。
    ///
    /// `finish_recruit → resolve_combat → 逐座位 CombatResult → resolve_settle →
    /// （非 GameOver）start_turn`，随后重设招募截止时刻。`Combat`/`Settle` 不设
    /// 玩家输入窗口，全部同步完成。
    ///
    /// `events` 是进入本方法前已产生的事件（例如最后一条 `EndRecruit` 的
    /// `PhaseChanged`）；四步推进各自返回的事件会按时间顺序合并后，再按座位
    /// 过滤随视图下发。
    fn advance_phase(
        &mut self,
        mut events: Vec<EngineEvent>,
    ) -> Result<Vec<SeatUpdate>, EngineError> {
        events.extend(self.engine.finish_recruit(&mut self.state)?);
        let outcome = self.engine.resolve_combat(&mut self.state)?;
        events.extend(outcome.events);

        let mut updates = Vec::with_capacity(SEAT_COUNT * 2 + outcome.reports.len());
        updates.extend(self.combat_updates(&outcome.reports));

        events.extend(self.engine.resolve_settle(&mut self.state)?);
        if self.state.phase != Phase::GameOver {
            events.extend(self.engine.start_turn(&mut self.state)?);
            self.phase_deadline_ms = self
                .clock
                .now_ms()
                .saturating_add(RECRUIT_SECONDS.saturating_mul(1000));
        }

        let all_seats: Vec<Seat> = (0..SEAT_COUNT as Seat).collect();
        updates.extend(self.view_updates(&events, &all_seats));

        if self.state.phase == Phase::GameOver {
            let game_over = self.game_over_payload();
            for seat in 0..SEAT_COUNT as Seat {
                updates.push(SeatUpdate {
                    player_id: self.seats[usize::from(seat)].player_id.clone(),
                    kind: UpdateKind::GameOver(game_over.clone()),
                });
            }
        }

        Ok(updates)
    }

    /// 对局结束载荷：排名（`place` 从 1 起）与 16 位十六进制种子。
    fn game_over_payload(&self) -> GameOver {
        let ranking = self
            .state
            .ranking
            .iter()
            .enumerate()
            .filter_map(|(index, seat)| {
                let place = u64::try_from(index + 1)
                    .ok()
                    .and_then(std::num::NonZeroU64::new)?;
                Some(GameOverRankingItem {
                    place,
                    player_id: self.seats[usize::from(*seat)].player_id.clone(),
                })
            })
            .collect();
        GameOver {
            ranking,
            seed: format!("{:016x}", self.seed),
        }
    }

    fn view_update(&self, seat: Seat) -> SeatUpdate {
        SeatUpdate {
            player_id: self.seats[usize::from(seat)].player_id.clone(),
            kind: UpdateKind::View { events: Vec::new() },
        }
    }

    /// 快照更新：载荷由编码时的当前视图生成。
    fn snapshot_update(&self, seat: Seat) -> SeatUpdate {
        SeatUpdate {
            player_id: self.seats[usize::from(seat)].player_id.clone(),
            kind: UpdateKind::Snapshot,
        }
    }

    /// 按座位过滤事件并生成视图更新。
    ///
    /// 每个座位最多收到一条视图更新：`recipients` 里的座位即使没有事件也推送
    /// （用于公开摘要变化或快照）；其余座位只有在本批事件过滤后非空时才推送。
    fn view_updates(&self, events: &[EngineEvent], recipients: &[Seat]) -> Vec<SeatUpdate> {
        let mut updates = Vec::new();
        for seat in 0..SEAT_COUNT as Seat {
            let filtered: Vec<ProtocolGameEvent> = events
                .iter()
                .filter_map(|event| for_seat(event, seat, &self.seats))
                .collect();
            if filtered.is_empty() && !recipients.contains(&seat) {
                continue;
            }
            updates.push(SeatUpdate {
                player_id: self.seats[usize::from(seat)].player_id.clone(),
                kind: UpdateKind::View { events: filtered },
            });
        }
        updates
    }

    /// 已知玩家回全量快照；未知玩家直接忽略。
    ///
    /// `RequestSnapshot` 与 `Reconnect` 共用本路径，快照内容只由该座位当前视图决定，
    /// 不依赖是否发生过断线。未知玩家不产出更新：`envelope_for` 无法为不存在的
    /// 座位编码，返回错误更新只会制造无法投递的消息。
    fn snapshot_for_player(&self, player_id: &str) -> Vec<SeatUpdate> {
        match self.seat_of.get(player_id).copied() {
            Some(seat) => vec![self.snapshot_update(seat)],
            None => Vec::new(),
        }
    }

    /// 绑定新的下行发送端并回全量快照。
    ///
    /// M1 不做历史重放：无论 `last_seq` 是否落后于当前序号，都以全量快照补齐。
    /// 快照序号由 [`GameTask::envelope_for`] 在该座位下一个序号上分配，必然大于
    /// 重连前的最后一条消息。
    fn reconnect(
        &mut self,
        player_id: &str,
        last_seq: u64,
        out: mpsc::Sender<ServerEnvelope>,
    ) -> Vec<SeatUpdate> {
        // M1 只回全量快照，`last_seq` 不参与重放判断；保留参数供后续版本使用。
        let _ = last_seq;
        if !self.bind_sender(player_id, out) {
            // 未知玩家的重连直接忽略，不产出无法编码的更新。
            return Vec::new();
        }
        let Some(seat) = self.seat_of.get(player_id).copied() else {
            return Vec::new();
        };
        vec![self.snapshot_update(seat)]
    }

    /// 记录断线时刻；座位与对局状态保持不变。
    ///
    /// 重复断线只保留第一次时刻，避免连接层抖动无限延长宽限期。未知玩家的断开不是
    /// 错误，直接忽略。
    fn mark_disconnect(&mut self, player_id: &str) -> Vec<SeatUpdate> {
        let Some(seat) = self.seat_of.get(player_id).copied() else {
            return Vec::new();
        };
        let index = usize::from(seat);
        if self.disconnected_at_ms[index].is_none() {
            self.disconnected_at_ms[index] = Some(self.clock.now_ms());
        }
        Vec::new()
    }

    /// 把超过 [`RECONNECT_GRACE_MS`] 的断线座位标记离线。
    ///
    /// 断线保留窗口是服务端内部状态，不产出任何下行更新：槽位与在线状态只供
    /// 连接层查询，客户端重连时靠 [`RoomCommand::Reconnect`] 拿全量快照。
    fn expire_disconnects(&mut self) {
        let now = self.clock.now_ms();
        for seat in 0..SEAT_COUNT as Seat {
            let index = usize::from(seat);
            let Some(since) = self.disconnected_at_ms[index] else {
                continue;
            };
            if now < since.saturating_add(RECONNECT_GRACE_MS) {
                continue;
            }
            self.disconnected_at_ms[index] = None;
            self.connected[index] = false;
        }
    }

    /// 只针对已知玩家的错误；未知玩家退化为 `NOT_IN_ROOM`。
    fn player_error(
        &self,
        player_id: &str,
        client_intent_id: Option<&str>,
        code: ErrorCode,
        message: impl Into<String>,
    ) -> Vec<SeatUpdate> {
        if self.seat_of.contains_key(player_id) {
            vec![self.error_update(player_id, client_intent_id, code, message.into())]
        } else {
            vec![self.error_update(
                player_id,
                client_intent_id,
                ErrorCode::NotInRoom,
                "玩家不在房间内",
            )]
        }
    }

    fn error_update(
        &self,
        player_id: &str,
        client_intent_id: Option<&str>,
        code: ErrorCode,
        message: impl Into<String>,
    ) -> SeatUpdate {
        SeatUpdate {
            player_id: player_id.to_string(),
            kind: UpdateKind::Error(ErrorMessage {
                client_intent_id: client_intent_id.map(str::to_string),
                code,
                message: message.into(),
                retryable: false,
            }),
        }
    }

    /// 当前截止时刻的 RFC3339 文本；由毫秒值直接派生，必然可被重新解析。
    fn deadline_rfc3339(&self) -> String {
        chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
            i64::try_from(self.phase_deadline_ms).unwrap_or(i64::MAX),
        )
        .map(|time| time.to_rfc3339())
        .unwrap_or_default()
    }
}

/// 把意图里的字符串目标解析为实例 id；解析失败返回 `NO_SUCH_MINION`。
fn parse_target(target: Option<String>) -> Result<Option<InstanceId>, ErrorCode> {
    target
        .map(|id| {
            id.parse::<InstanceId>()
                .map_err(|_| ErrorCode::NoSuchMinion)
        })
        .transpose()
}

/// 把协议意图映射为引擎意图。
///
/// 座位不在意图里：客户端只能通过 `player_id` 路由，座位由服务端查表得到。
/// 下表之外的参数越界（负数、超出 `u8`）返回 `INVALID_CHOICE`；目标解析失败
/// 返回 `NO_SUCH_MINION`。
fn map_intent(intent: protocol::intents::Intent) -> Result<EngineIntent, ErrorCode> {
    use protocol::intents::Intent as Protocol;

    match intent {
        Protocol::BuyMinion {
            slot,
            target_minion_id,
        } => Ok(EngineIntent::BuyMinion {
            slot: to_u8(slot)?,
            target: parse_target(target_minion_id)?,
        }),
        Protocol::SellMinion { minion_id } => Ok(EngineIntent::SellMinion {
            minion: minion_id
                .parse::<InstanceId>()
                .map_err(|_| ErrorCode::NoSuchMinion)?,
        }),
        Protocol::RerollShop => Ok(EngineIntent::RerollShop),
        Protocol::UpgradeEra => Ok(EngineIntent::UpgradeEra),
        Protocol::PlayCard {
            hand_index,
            target_minion_id,
        } => Ok(EngineIntent::PlayCard {
            hand_index: to_u8(hand_index)?,
            target: parse_target(target_minion_id)?,
        }),
        Protocol::RearrangeBoard {
            from_index,
            to_index,
        } => Ok(EngineIntent::RearrangeBoard {
            from: to_u8(from_index)?,
            to: to_u8(to_index)?,
        }),
        Protocol::FreezeShop { slot } => Ok(EngineIntent::FreezeShop {
            slot: slot.map(to_u8).transpose()?,
        }),
        Protocol::UseHeroPower {
            skill_id,
            target_minion_id,
        } => Ok(EngineIntent::UseHeroPower {
            skill_id,
            target: parse_target(target_minion_id)?,
        }),
        Protocol::EndRecruit => Ok(EngineIntent::EndRecruit),
        Protocol::ChooseDiscover { choice } => Ok(EngineIntent::ChooseDiscover {
            choice: to_u8(choice)?,
        }),
    }
}

fn to_u8(value: i64) -> Result<u8, ErrorCode> {
    u8::try_from(value).map_err(|_| ErrorCode::InvalidChoice)
}

/// 引擎拒绝码转协议错误码；两者取值一一对应。
fn error_code_of(code: RejectCode) -> ErrorCode {
    match code {
        RejectCode::NotEnoughGold => ErrorCode::NotEnoughGold,
        RejectCode::ShopSlotEmpty => ErrorCode::ShopSlotEmpty,
        RejectCode::NoSpace => ErrorCode::NoSpace,
        RejectCode::InvalidStage => ErrorCode::InvalidStage,
        RejectCode::NoSuchMinion => ErrorCode::NoSuchMinion,
        RejectCode::AlreadyMaxEra => ErrorCode::AlreadyMaxEra,
        RejectCode::CardNotInHand => ErrorCode::CardNotInHand,
        RejectCode::HeroPowerLimit => ErrorCode::HeroPowerLimit,
        RejectCode::NoValidTarget => ErrorCode::NoValidTarget,
        RejectCode::TargetNotAllowed => ErrorCode::TargetNotAllowed,
        RejectCode::InvalidChoice => ErrorCode::InvalidChoice,
    }
}

/// 任意可序列化载荷转 JSON 对象；不是对象时返回 `BAD_PAYLOAD`。
fn payload_map<T: serde::Serialize>(payload: &T) -> Result<Map<String, Value>, ProtocolError> {
    match serde_json::to_value(payload).map_err(|_| ProtocolError("BAD_PAYLOAD"))? {
        Value::Object(map) => Ok(map),
        _ => Err(ProtocolError("BAD_PAYLOAD")),
    }
}
