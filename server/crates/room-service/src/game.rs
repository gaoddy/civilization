//! 对局驱动：把房间收到的命令接到规则引擎上，负责计时、意图去重与逐座位视图。
//!
//! 设计约定：
//! - 引擎保持纯库：本模块所有时间判断都走注入的 [`Clock`]，不读 `SystemTime`。
//! - 阶段推进串行且分两步：[`GameTask::enter_combat`] 负责
//!   `finish_recruit → resolve_combat → 逐座位 CombatResult`，停在 `Combat` 并按
//!   演示时长设截止；[`GameTask::finish_combat`] 由 [`GameTask::tick`] 在演示到点后
//!   调用，负责 `resolve_settle → start_turn` 与下回合招募截止（按演示时长抵扣）。
//! - 意图去重键为 `(player_id, clientIntentId)`，表只增不删、随房间销毁。
//! - 每个座位有独立的 [`SeatChannel`]，`seq` 由 [`GameTask::envelope_for`] 在编码
//!   下行消息时按座位各自递增，互不影响。
//! - 下行发送端由 `GameTask` 单一持有：首次建连与重连都经 [`GameTask::bind_sender`]
//!   注册，连接层只借用投递，避免两份所有权在重连后指向不同连接。
//! - 重连与快照请求共用全量 `Snapshot` 路径（M1 不重放历史），快照载荷在编码时由
//!   当前座位视图生成，含未消费的 `pendingChoices`。
//! - 断线保留窗口（[`RECONNECT_GRACE_MS`]）是服务端内部状态：超时只把座位标记为
//!   离线，不释放座位、不结束对局，也不向任何座位下发通知；客户端靠重连快照恢复。
//! - 任何改变房间可见状态的操作都向全部 8 个座位推送 `View`：公开摘要（对手
//!   `boardCount`、`recruited` 标记、阶段与回合）人人都能看到。事件仍按
//!   [`for_seat`] 的规则过滤——本人收到自己的私有事件，其他人收到 `events` 为空的
//!   视图；战斗结果只走逐座位 `CombatResult`。

use std::collections::BTreeMap;
use std::sync::Arc;

use protocol::server::{CombatResult, ErrorMessage, GameOver, GameStarted, Snapshot, StateUpdate};
use protocol::view::{CombatLogEntry, GameEvent as ProtocolGameEvent};
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

/// 每个招募回合的时长（秒）。开局与首回合的固定招募时长也用它。
pub const RECRUIT_SECONDS: u64 = 90;

/// 单条战斗动作的演示时长。
const PACE_MS: u64 = 320;

/// 结果横幅停留。
const TAIL_MS: u64 = 2_000;

/// 一个回合的总预算（招募 + 演示）。
const ROUND_BUDGET_MS: u64 = 150_000;

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
    /// 入房之后的连接层错误；由房间 task 统一走该座位的下行通道编码下发，
    /// 保证同一玩家的 `seq` 单调递增（不另开连接层计数器）。
    Error {
        player_id: String,
        code: String,
        message: String,
    },
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
    /// 本场战斗的逐座位报告；`Combat` 阶段保留供重连补发，结算时取走。
    combat_reports: Option<BTreeMap<Seat, SeatCombatReport>>,
    /// 进入战斗演示前的名次快照（按下标即座位）。
    ///
    /// `resolve_combat` 会立刻改写 `damage_dealt`、`last_won` 与 `last_winner`，
    /// 而设计 §7.2 要求演示期间名次保持战前值；快照只在 `Combat` 阶段覆盖
    /// `PlayerSummary.rank`，[`GameTask::finish_combat`] 结算时清空、恢复实时计算。
    combat_ranks: Option<[Option<u8>; SEAT_COUNT]>,
    /// 本场战斗的演示时长（毫秒）；参与下回合招募预算抵扣。
    combat_duration_ms: u64,
    /// 最近 5 个回合的全对局公开战报，按回合升序。
    ///
    /// 只存在 `GameTask` 上，不写入 [`GameState`]，因此不影响 `state_hash` 与
    /// 确定性重放；重连快照从当前视图自然带出。
    combat_log: Vec<CombatLogEntry>,
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
            combat_reports: None,
            combat_ranks: None,
            combat_duration_ms: 0,
            combat_log: Vec::new(),
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
        let mut protocol_view = to_protocol(
            &seat_view,
            &self.room_id,
            &self.deadline_rfc3339(),
            &self.seats,
            self.engine.catalog.config.board_slots,
            self.engine.catalog.config.initial_hp,
        )
        .ok()?;
        // 演示期间名次保持战前值：`resolve_combat` 已写入本轮伤害与胜者，
        // 实时 `rank_of` 会提前跳动，这里用进入战斗前的快照覆盖。
        if self.state.phase == Phase::Combat
            && let Some(ranks) = &self.combat_ranks
        {
            for player in &mut protocol_view.players {
                if let Ok(index) = usize::try_from(player.seat)
                    && let Some(rank) = ranks.get(index).copied()
                {
                    player.rank = rank.and_then(|rank| std::num::NonZeroU64::new(u64::from(rank)));
                }
            }
        }
        protocol_view.combat_log = self.combat_log.clone();
        Some(protocol_view)
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

    /// 把该座位的下行序号推进到至少 `seq`。
    ///
    /// 连接层与房间 task 共享会话级序号下限：对局通道可能是在连接层发过若干
    /// 大厅消息之后才绑定/使用的，编码前先把通道抬到当前下限，保证 `seq` 跨来源
    /// 也严格递增（`Snapshot.seq` 与信封 `seq` 因此仍一致）。
    pub fn set_seq_floor(&mut self, seat: Seat, seq: u64) {
        if let Some(channel) = self.channels.get_mut(&seat) {
            channel.set_next_seq(seq);
        }
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
            RoomCommand::Error {
                player_id,
                code,
                message,
            } => {
                let code = code.parse().unwrap_or(ErrorCode::InternalError);
                self.player_error(&player_id, None, code, message)
            }
        }
    }

    /// 计时检查：未到点返回空；到点推进当前阶段。
    ///
    /// 招募到点进入战斗演示（[`GameTask::enter_combat`]），演示到点结算并进入下一
    /// 回合（[`GameTask::finish_combat`]）。推进后阶段离开本阶段，同一毫秒再次调用
    /// 返回空。
    ///
    /// 顺序为「先推进阶段，再处理断线宽限」：阶段推进是时间语义的一部分，必须与
    /// 连接状态无关地按时发生；断线标记只影响可观测的在线状态，安排在推进之后，
    /// 不会改变原有阶段推进的调用次数与顺序。
    pub fn tick(&mut self) -> Vec<SeatUpdate> {
        let mut updates = Vec::new();
        if self.clock.now_ms() >= self.phase_deadline_ms {
            updates = match self.state.phase {
                Phase::Recruit => self.enter_combat(Vec::new()).unwrap_or_default(),
                Phase::Combat => self.finish_combat().unwrap_or_default(),
                Phase::Lobby | Phase::Settle | Phase::GameOver => Vec::new(),
            };
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
        match self.engine.apply(&mut self.state, seat, engine_intent) {
            Ok(outcome) => {
                self.dedup.insert(key, Deduped::Applied);
                if self.state.phase == Phase::Combat {
                    return match self.enter_combat(outcome.events) {
                        Ok(updates) => updates,
                        Err(error) => {
                            self.player_error(&player_id, None, ErrorCode::InternalError, error.0)
                        }
                    };
                }
                // 私有意图也会改变公开摘要（例如对手的 `boardCount`），因此对全部座位
                // 推送视图；事件仍按座位过滤，其他人的 `events` 为空。
                let all_seats: Vec<Seat> = (0..SEAT_COUNT as Seat).collect();
                self.view_updates(&outcome.events, &all_seats)
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

    /// 把本场逐座位报告归一为战报并追加到最近五回合窗口。
    ///
    /// 真实战在 `reports` 里两个座位各一条，只保留座位号较小的一条；
    /// 幽灵战本来只有落单座位一条。战报回合号取结算时刻的 `state.turn`，
    /// 追加后丢弃早于 `turn - 4` 的条目。
    fn record_combat_log(&mut self, reports: &BTreeMap<Seat, SeatCombatReport>) {
        let Some(turn) = std::num::NonZeroU64::new(u64::from(self.state.turn)) else {
            return;
        };
        for report in reports.values() {
            let opponent = report.opponent_seat;
            // 真实战的另一半报告指向更小座位时跳过，保证一场只记一条。
            if !report.mirror {
                let Some(opponent) = opponent else {
                    continue;
                };
                if report.seat > opponent {
                    continue;
                }
            }
            let participant_ids = if report.mirror {
                vec![self.player_id(report.seat)]
            } else {
                let Some(opponent) = opponent else { continue };
                let mut participants = [report.seat, opponent];
                // 按座位号升序，再映射为 playerId；不是按 playerId 字符串排序。
                participants.sort_unstable();
                participants
                    .into_iter()
                    .map(|seat| self.player_id(seat))
                    .collect()
            };
            let (winner_id, loser_id) = match report.winner {
                // 平局：双方都为 null，伤害显式写 0，不依赖引擎当前保证。
                None => (None, None),
                Some(winner) if winner == report.seat => (
                    Some(self.player_id(winner)),
                    // 幽灵没有玩家 id，玩家赢下幽灵战时 loserId 为 null。
                    if report.mirror {
                        None
                    } else {
                        opponent.map(|opponent| self.player_id(opponent))
                    },
                ),
                Some(winner) => (
                    // 幽灵赢时 `winner` 是幽灵归属座位，同样没有玩家 id。
                    if report.mirror {
                        None
                    } else {
                        Some(self.player_id(winner))
                    },
                    Some(self.player_id(report.seat)),
                ),
            };
            let damage = match report.winner {
                None => 0,
                Some(_) => u64::try_from(report.damage).unwrap_or(0),
            };
            self.combat_log.push(CombatLogEntry {
                damage,
                ghost_id: if report.mirror {
                    opponent.map(|seat| self.player_id(seat))
                } else {
                    None
                },
                loser_id,
                mirror: report.mirror,
                participant_ids,
                turn,
                winner_id,
            });
        }
        let oldest = u64::from(self.state.turn.saturating_sub(4));
        self.combat_log.retain(|entry| entry.turn.get() >= oldest);
    }

    fn player_id(&self, seat: Seat) -> String {
        self.seats[usize::from(seat)].player_id.clone()
    }

    /// 把逐座位战斗报告转成 `CombatResult` 更新：一份报告对应一条更新。
    ///
    /// 幽灵战在 `outcome.reports` 里只有该座位自己的一条，因此只会产生 1 条更新。
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

    /// 招募 → 战斗演示：配对、模拟战斗、逐座位下发 `CombatResult`。
    ///
    /// `finish_recruit → resolve_combat → 存本场报告与演示时长 → 设演示截止`。
    /// 阶段停在 `Combat`，之后由 [`GameTask::finish_combat`] 在截止时刻结算，
    /// 这样 `Combat` 成为真实存在的时间段，客户端可见战斗视图。
    ///
    /// `events` 是进入本方法前已产生的事件（例如最后一条 `EndRecruit` 的
    /// `PhaseChanged`）；本方法产生的事件合并后按座位过滤随视图下发。
    fn enter_combat(
        &mut self,
        mut events: Vec<EngineEvent>,
    ) -> Result<Vec<SeatUpdate>, EngineError> {
        events.extend(self.engine.finish_recruit(&mut self.state)?);
        // 演示期间左栏名次必须停在战前值：`resolve_combat` 会立即改写伤害与
        // 胜者，因此在这之前按下标（即座位）取一次快照。
        self.combat_ranks = Some(std::array::from_fn(|seat| {
            rule_engine::settle::rank_of(&self.state, seat as Seat)
        }));
        let outcome = self.engine.resolve_combat(&mut self.state)?;
        events.extend(outcome.events);
        self.record_combat_log(&outcome.reports);

        // `resolve_combat` 是纯计算，按引擎口径把阶段推到 `Settle`。演示期间要停在
        // `Combat`：结算产物（伤害、淘汰）已写入状态，但结算本身推迟到
        // [`GameTask::finish_combat`]，届时再把阶段拨回 `Settle` 交给引擎。
        self.state.phase = Phase::Combat;
        // 对应地不下发 `Settle` 的阶段事件：客户端在本场演示里应一直停在 `Combat`。
        events.retain(|event| {
            !matches!(
                event,
                EngineEvent::PhaseChanged {
                    phase: Phase::Settle,
                    ..
                }
            )
        });

        let mut updates = Vec::with_capacity(SEAT_COUNT * 2 + outcome.reports.len());
        updates.extend(self.combat_updates(&outcome.reports));

        self.combat_duration_ms = combat_duration_ms(&outcome.reports);
        self.combat_reports = Some(outcome.reports);
        self.phase_deadline_ms = self.clock.now_ms().saturating_add(self.combat_duration_ms);

        let all_seats: Vec<Seat> = (0..SEAT_COUNT as Seat).collect();
        updates.extend(self.view_updates(&events, &all_seats));
        Ok(updates)
    }

    /// 战斗演示 → 结算 →（非 `GameOver`）下一回合招募。
    ///
    /// 结算后若未结束，下一回合招募截止为 `now + (150_000 − 演示时长)`：演示占用的
    /// 时间从回合预算里扣除，不设下限（演示超预算时截止即 `now`）。
    fn finish_combat(&mut self) -> Result<Vec<SeatUpdate>, EngineError> {
        // 报告只服务本场演示期间的重连补发，结算时取走。
        self.combat_reports = None;
        // 战斗已在 `enter_combat` 里算完，阶段被演示临时留在 `Combat`；结算前拨回
        // 引擎期望的 `Settle`。
        self.state.phase = Phase::Settle;
        let mut events = self.engine.resolve_settle(&mut self.state)?;
        // 演示结束：名次恢复实时计算（若已进入 `GameOver`，`rank_of` 与
        // `placement` 一致）。
        self.combat_ranks = None;
        if self.state.phase != Phase::GameOver {
            events.extend(self.engine.start_turn(&mut self.state)?);
            self.phase_deadline_ms = self
                .clock
                .now_ms()
                .saturating_add(recruit_budget_ms(self.combat_duration_ms));
        }
        self.combat_duration_ms = 0;

        let all_seats: Vec<Seat> = (0..SEAT_COUNT as Seat).collect();
        let mut updates = self.view_updates(&events, &all_seats);

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
    ///
    /// 演示阶段（`Combat`）重连时，快照之后补发本场该座位的 `CombatResult`：
    /// 客户端靠 `phaseDeadline` 从当前动作续播，但动作序列本身只在战报里，因此
    /// 服务端在报告被结算取走前补发一次。
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
        let mut updates = vec![self.snapshot_update(seat)];

        if self.state.phase == Phase::Combat
            && let Some(report) = self
                .combat_reports
                .as_ref()
                .and_then(|reports| reports.get(&seat))
        {
            updates.push(SeatUpdate {
                player_id: player_id.to_string(),
                kind: UpdateKind::CombatResult(Box::new(combat_result(
                    &self.engine.catalog,
                    report,
                    &self.seats,
                ))),
            });
        }

        updates
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

/// 演示时长：本场所有座位动作数的最大者 × [`PACE_MS`]，再加结果横幅 [`TAIL_MS`]。
///
/// 取最大值保证 8 个座位同时结束演示；不设上限，超长演示由下回合招募抵扣。
fn combat_duration_ms(reports: &BTreeMap<Seat, SeatCombatReport>) -> u64 {
    let max_actions = reports
        .values()
        .map(|report| report.actions.len())
        .max()
        .unwrap_or(0);
    let actions = u64::try_from(max_actions).unwrap_or(u64::MAX);
    actions.saturating_mul(PACE_MS).saturating_add(TAIL_MS)
}

/// 抵扣后的招募预算：回合总预算减去演示时长，下限为 0。
fn recruit_budget_ms(combat_ms: u64) -> u64 {
    ROUND_BUDGET_MS.saturating_sub(combat_ms)
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
        Protocol::BuyMinion { slot } => Ok(EngineIntent::BuyMinion { slot: to_u8(slot)? }),
        Protocol::SellMinion { minion_id } => Ok(EngineIntent::SellMinion {
            minion: minion_id
                .parse::<InstanceId>()
                .map_err(|_| ErrorCode::NoSuchMinion)?,
        }),
        Protocol::RerollShop => Ok(EngineIntent::RerollShop),
        Protocol::UpgradeEra => Ok(EngineIntent::UpgradeEra),
        Protocol::PlayCard {
            hand_index,
            position,
            target_minion_id,
        } => Ok(EngineIntent::PlayCard {
            hand_index: to_u8(hand_index)?,
            position: position.map(to_u8).transpose()?,
            target: parse_target(target_minion_id)?,
        }),
        Protocol::ChooseTarget { minion_id } => Ok(EngineIntent::ChooseTarget {
            minion: minion_id
                .parse::<InstanceId>()
                .map_err(|_| ErrorCode::NoSuchMinion)?,
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use rule_engine::data::Catalog;
    use rule_engine::state::{MinionInstance, Pairing, Phase, Seat};

    use super::{PACE_MS, ROUND_BUDGET_MS, TAIL_MS, combat_duration_ms, recruit_budget_ms};
    use crate::clock::FixedClock;
    use crate::envelope::SeatChannel;
    use crate::game::GameTask;
    use crate::room::PlayerSlot;

    const CARDS_JSON: &str = include_str!("../../../data/v0.1/cards.json");
    const SKILLS_JSON: &str = include_str!("../../../data/v0.1/skills.json");
    const TOKENS_JSON: &str = include_str!("../../../data/v0.1/tokens.json");
    const CONFIG_JSON: &str = include_str!("../../../data/v0.1/config.json");

    /// 8 个座位各用一张不同的英雄卡。
    const HEROES: [&str; 8] = [
        "adam-smith",
        "darwin",
        "davinci",
        "edison",
        "einstein",
        "faraday",
        "galileo",
        "knuth",
    ];

    fn task() -> GameTask {
        let catalog = Catalog::from_json(CARDS_JSON, SKILLS_JSON, TOKENS_JSON, CONFIG_JSON)
            .expect("卡表应能加载");
        let heroes: BTreeMap<Seat, String> = HEROES
            .iter()
            .enumerate()
            .map(|(seat, hero)| (seat as Seat, (*hero).to_string()))
            .collect();
        let players = (0..8)
            .map(|seat| PlayerSlot {
                player_id: format!("p{seat}"),
                display_name: format!("玩家{seat}"),
                ready: true,
                hero_card_id: Some(HEROES[seat].to_string()),
                channel: SeatChannel::new(),
                connected: true,
            })
            .collect();
        GameTask::new(
            catalog,
            7,
            heroes,
            players,
            Arc::new(FixedClock::new(1_000_000)),
        )
        .expect("开局应当成功")
    }

    fn rank_of(view: &protocol::view::PlayerView, seat: i64) -> Option<u64> {
        view.players
            .iter()
            .find(|player| player.seat == seat)
            .and_then(|player| player.rank)
            .map(std::num::NonZeroU64::get)
    }

    /// 演示期间名次取进入战斗前的快照；结算后恢复实时名次。
    ///
    /// 构造：座位 0、1 生命与累计伤害相同，座位 1 是上一场赢家（战前名次 1），
    /// 座位 0 本轮获胜后 `last_won` 改写，实时名次会反超。
    #[test]
    fn combat_view_keeps_pre_combat_ranks_until_settle() {
        let mut task = task();
        for player in &mut task.state.players {
            player.hp = 30;
            player.damage_dealt = 0;
            player.eliminated = false;
            player.board.clear();
        }
        for seat in 2..8 {
            task.state.players[seat].eliminated = true;
            task.state.players[seat].hp = 0;
        }
        task.state.ranking = vec![2, 3, 4, 5, 6, 7];
        task.state.turn = 2;
        task.state.phase = Phase::Combat;
        task.state.first_seat = 0;
        task.state.last_winner = None;
        task.state.last_won = [1].into();
        task.state.pending_settle.clear();
        task.state.pairings = vec![Pairing {
            a: 0,
            b: 1,
            ghost: None,
        }];
        let catalog = &task.engine.catalog;
        task.state.players[0].board = vec![MinionInstance::new(catalog, 1, "bruce-lee", false)];
        task.state.players[1].board = vec![MinionInstance::new(catalog, 2, "baiyi", false)];

        // 战前：座位 1 是上战胜者，名次为 1；座位 0 为 2。
        assert_eq!(
            rank_of(&task.player_view(0).expect("视图应可用"), 0),
            Some(2)
        );
        assert_eq!(
            rank_of(&task.player_view(0).expect("视图应可用"), 1),
            Some(1)
        );

        task.enter_combat(Vec::new()).expect("进入战斗应成功");
        assert_eq!(task.state.phase, Phase::Combat, "演示期间停在 Combat");
        assert_eq!(task.state.last_won, [0].into(), "本轮座位 0 获胜");
        let view = task.player_view(0).expect("视图应可用");
        assert_eq!(rank_of(&view, 0), Some(2), "演示期间名次保持战前值");
        assert_eq!(rank_of(&view, 1), Some(1), "演示期间名次保持战前值");

        task.finish_combat().expect("结算应成功");
        let view = task.player_view(0).expect("视图应可用");
        assert_eq!(rank_of(&view, 0), Some(1), "结算后座位 0 反超");
        assert_eq!(rank_of(&view, 1), Some(2), "结算后座位 1 落到第二");
    }

    #[test]
    fn combat_duration_over_budget_sets_deadline_at_now() {
        // 演示时长超过回合预算时，招募预算夹到 0（截止即 now）。
        assert_eq!(recruit_budget_ms(200_000), 0);
        assert_eq!(recruit_budget_ms(30_000), 120_000);
        assert_eq!(ROUND_BUDGET_MS, 150_000);
    }

    #[test]
    fn combat_duration_counts_max_actions_plus_tail() {
        // 空报告只有横幅停留；不同座位取动作数最大者。
        assert_eq!(combat_duration_ms(&BTreeMap::new()), TAIL_MS);
        assert_eq!(PACE_MS, 320);
    }
}
