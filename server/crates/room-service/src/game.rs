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

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use protocol::server::{
    AchievementUnlocked, CombatResult, ErrorMessage, GameOver, GameStarted, Snapshot, StateUpdate,
};
use protocol::view::{CombatLogEntry, GameEvent as ProtocolGameEvent};
use protocol::{
    ErrorCode, GameOverRankingItem, ProtocolError, SelfStats, SelfStatsBoardItem, ServerEnvelope,
};
use rule_engine::achievements::Achievement;
use rule_engine::data::{Catalog, Playstyle};
use rule_engine::effects::{TargetRef as EngineTargetRef, Zone as EngineZone};
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

use crate::AchievementCatalog;
use crate::clock::Clock;
use crate::envelope::SeatChannel;
use crate::filter::{combat_result, for_seat};
use crate::room::{PlayerSlot, SEAT_COUNT};
use crate::view::{SeatInfo, to_protocol};

/// 首回合招募时长（秒）。
const INITIAL_RECRUIT_SECONDS: u64 = 60;

/// 每个后续回合增加的招募时长（秒）。
const RECRUIT_STEP_SECONDS: u64 = 15;

/// 招募时长上限（秒）。
const RECRUIT_CAP_SECONDS: u64 = 120;

/// 第 `turn` 回合的招募时长：60 秒起，每回合 +15 秒，封顶 120 秒。
///
/// 招募计时从开战时刻起算，因此战斗演示与结算耗时直接消耗下一回合的招募
/// 时间；战斗越短，剩余招募时间越多。
fn recruit_seconds(turn: u8) -> u64 {
    INITIAL_RECRUIT_SECONDS
        .saturating_add(u64::from(turn.saturating_sub(1)).saturating_mul(RECRUIT_STEP_SECONDS))
        .min(RECRUIT_CAP_SECONDS)
}

/// 客户端战斗演示每条动作的间隔（毫秒），与 `client/scenes/game.gd` 的
/// `COMBAT_PACE_MS` 保持一致。
const COMBAT_ACTION_MS: u64 = 320;

/// 战斗演示结束横幅停留时长（毫秒），与客户端演示时序一致。
const COMBAT_BANNER_MS: u64 = 2_000;

/// 进入战斗后的阶段截止：取「下一回合招募截止」与「按动作数估算的演示时长」
/// 的较大者，避免长战斗被招募兜底提前截断。
///
/// `turn` 是进入战斗时的当前回合；下一回合招募从开战时刻起算。演示时长估算为
/// `最大动作数 × 320ms + 2s`，与客户端逐动作播放的节奏一致。
fn combat_deadline(now_ms: u64, turn: u8, max_actions: usize) -> u64 {
    let recruit_deadline =
        now_ms.saturating_add(recruit_seconds(turn.saturating_add(1)).saturating_mul(1000));
    let demo_ms = u64::try_from(max_actions)
        .unwrap_or(u64::MAX)
        .saturating_mul(COMBAT_ACTION_MS)
        .saturating_add(COMBAT_BANNER_MS);
    recruit_deadline.max(now_ms.saturating_add(demo_ms))
}

/// 剩余多少秒时下发计时警告（由连接层使用，M1a 只暴露常量与 [`GameTask::seconds_left`]）。
pub const TIMER_WARNING_SECONDS: u64 = 10;

/// 断线后座位的保留时长（毫秒）。超过该时长座位被标记离线，但不释放座位、不结束对局。
pub const RECONNECT_GRACE_MS: u64 = 30_000;

/// 单个座位本局的增量统计；由引擎事件累计，结束时与终局状态合成 `SelfStats`。
#[derive(Debug, Default)]
struct SeatStats {
    buys: u32,
    sells: u32,
    rerolls: u32,
    triples: u32,
    damage_taken: i32,
    combat_wins: u32,
    combat_draws: u32,
    combat_losses: u32,
    /// 英雄主动技能使用次数；`PlayerState.hero_uses` 每回合清空，这里跨回合累计。
    hero_power_uses: u32,
    /// 被淘汰的回合；存活至结束的座位保持 `None`。
    eliminated_turn: Option<u8>,
}

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
    /// 该座位已把本场战斗演示播完，可以结算；重复发送幂等。
    CombatPlayed { player_id: String },
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
    /// 房主移除成员；等待区语义由房间注册表处理，对局中暂不支持（T01 占位）。
    Kick {
        player_id: String,
        target_player_id: String,
    },
    /// 房主解散房间；等待区语义由房间注册表处理，对局中暂不支持（T01 占位）。
    Dissolve { player_id: String },
    /// 对局中主动退出；按弃权结算名次，该座位此后不再收到任何推送。
    Forfeit { player_id: String },
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
    /// 本局首次达成的成就；账号级去重与 Steam 同步在房间投递层完成。
    AchievementUnlocked {
        achievement_id: String,
        steam_api_name: String,
    },
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
    /// 成就 id → Steam API 名；用于把局内解锁转成下行消息。
    achievements: Arc<AchievementCatalog>,
    /// 上次观察到的各座位本局已解锁成就；用于只推送新解锁。
    achievement_seen: [BTreeSet<Achievement>; SEAT_COUNT],
    /// `playerId → 座位`；座位由服务端认定，客户端不能指定。
    seat_of: BTreeMap<String, Seat>,
    /// 按座位索引的玩家身份，长度固定为 [`SEAT_COUNT`]。
    seats: [SeatInfo; SEAT_COUNT],
    /// 每个连接的下行序号分配器；不同座位互相独立。
    channels: BTreeMap<Seat, SeatChannel>,
    clock: Arc<dyn Clock>,
    /// `(playerId, clientIntentId) → 结果`；只增不删。
    dedup: BTreeMap<(String, String), Deduped>,
    /// 按座位累计的本局统计；只由 [`GameTask::record_events`] 增量写入。
    seat_stats: [SeatStats; SEAT_COUNT],
    /// 上次观察到的各座位 `hero_uses` 总和；引擎每回合清空 `hero_uses`，用其与
    /// 当前总和的差累计 [`SeatStats::hero_power_uses`]。
    hero_uses_baseline: [u32; SEAT_COUNT],
    seed: u64,
    room_id: String,
    /// 对局开始时刻（Unix epoch 毫秒），用于观察日志的单局时长。
    started_at_ms: u64,
    /// 按座位累计的意图总数与拒绝数，用于观察日志的意图拒绝率。
    intent_counts: [u32; SEAT_COUNT],
    rejection_counts: [u32; SEAT_COUNT],
    /// 对局结束观察日志是否已输出，保证每局只打印一次。
    game_over_logged: bool,
    /// 当前阶段的截止时刻（Unix epoch 毫秒）。
    ///
    /// `Recruit` 时是本回合招募截止；`Combat` 时是下一回合招募截止，同时也是
    /// 演示回报的兜底时刻：到点仍未全部回报时强制结算。
    phase_deadline_ms: u64,
    /// 本场战斗的逐座位报告；`Combat` 阶段保留供重连补发，结算时取走。
    combat_reports: Option<BTreeMap<Seat, SeatCombatReport>>,
    /// 进入战斗演示前的名次快照（按下标即座位）。
    ///
    /// `resolve_combat` 会立刻改写 `damage_dealt`、`last_won` 与 `last_winner`，
    /// 而设计 §7.2 要求演示期间名次保持战前值；快照只在 `Combat` 阶段覆盖
    /// `PlayerSummary.rank`，[`GameTask::finish_combat`] 结算时清空、恢复实时计算。
    combat_ranks: Option<[Option<u8>; SEAT_COUNT]>,
    /// 本场战斗的开始时刻（Unix epoch 毫秒）；随 `CombatResult.startedAt` 下发，
    /// 客户端据此换算已播放时长。
    combat_started_at_ms: Option<u64>,
    /// 参战座位是否已回报「演示完成」；非参战座位恒为 `false`。
    combat_acks: [bool; SEAT_COUNT],
    /// 参与等待回报的座位（收到本场 `CombatResult` 的座位）。
    combat_participants: [bool; SEAT_COUNT],
    /// 最近 5 个回合的全对局公开战报，按回合升序。
    ///
    /// 只存在 `GameTask` 上，不写入 [`GameState`]，因此不影响 `state_hash` 与
    /// 确定性重放；重连快照从当前视图自然带出。
    combat_log: Vec<CombatLogEntry>,
    /// 本局主动弃权的玩家；座位保留到本局结束，仅用于停止推送与回房重置。
    departed: BTreeSet<String>,
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
    /// 以已确定的英雄表与玩法开局并构造驱动。
    ///
    /// `players` 必须是按座位号升序排列的 8 个座位（`Room::players` 的约定）；
    /// 元素下标即座位号。`seed` 同时用于引擎随机流与 `GameOver.seed`；
    /// `playstyles` 由房间创建时抽取并长期持有，重开与重连复用同一组。
    ///
    /// # Errors
    ///
    /// 座位数不是 8、英雄不合法或缺失时返回 [`EngineError`]。
    pub fn new(
        catalog: Catalog,
        seed: u64,
        playstyles: [Playstyle; 5],
        heroes: BTreeMap<Seat, String>,
        players: Vec<PlayerSlot>,
        clock: Arc<dyn Clock>,
        achievements: Arc<AchievementCatalog>,
    ) -> Result<Self, EngineError> {
        if players.len() != SEAT_COUNT {
            return Err(EngineError(format!(
                "对局需要 {SEAT_COUNT} 个座位，实际为 {}",
                players.len()
            )));
        }

        let (engine, mut state) = Engine::new(catalog, seed_from_u64(seed), playstyles);
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
            .saturating_add(recruit_seconds(state.turn).saturating_mul(1000));
        let started_at_ms = clock.now_ms();

        Ok(Self {
            engine,
            state,
            achievements,
            achievement_seen: std::array::from_fn(|_| BTreeSet::new()),
            seat_of,
            seats,
            channels,
            clock,
            dedup: BTreeMap::new(),
            seat_stats: std::array::from_fn(|_| SeatStats::default()),
            hero_uses_baseline: [0; SEAT_COUNT],
            seed,
            room_id: String::new(),
            started_at_ms,
            intent_counts: [0; SEAT_COUNT],
            rejection_counts: [0; SEAT_COUNT],
            game_over_logged: false,
            phase_deadline_ms,
            combat_reports: None,
            combat_ranks: None,
            combat_started_at_ms: None,
            combat_acks: [false; SEAT_COUNT],
            combat_participants: [false; SEAT_COUNT],
            combat_log: Vec::new(),
            departed: BTreeSet::new(),
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
        // 已弃权的座位不再接收任何消息，重连也不重新绑定。
        if self.departed.contains(player_id) {
            return false;
        }
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
            UpdateKind::AchievementUnlocked {
                achievement_id,
                steam_api_name,
            } => (
                "AchievementUnlocked",
                payload_map(&AchievementUnlocked {
                    achievement_id: achievement_id.clone(),
                    steam_api_name: steam_api_name.clone(),
                })?,
            ),
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
    ///
    /// 命令执行后统一比对成就解锁集合，把本批新解锁追加为对应座位的更新。
    pub fn handle(&mut self, cmd: RoomCommand) -> Vec<SeatUpdate> {
        let mut updates = self.handle_command(cmd);
        self.append_achievement_updates(&mut updates);
        updates
    }

    fn handle_command(&mut self, cmd: RoomCommand) -> Vec<SeatUpdate> {
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
            RoomCommand::Kick { player_id, .. } | RoomCommand::Dissolve { player_id } => {
                // 等待区语义由房间注册表处理；对局中暂不支持。
                self.player_error(&player_id, None, ErrorCode::InvalidStage, "当前阶段不允许")
            }
            RoomCommand::Forfeit { player_id } => self.forfeit(&player_id),
            RoomCommand::CombatPlayed { player_id } => self.mark_combat_played(&player_id),
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
    /// 招募到点进入战斗演示（[`GameTask::enter_combat`]）。战斗演示在全部参战
    /// 座位回报后由 [`GameTask::mark_combat_played`] 立即结算；本方法只在到点仍
    /// 有座位未回报时兜底结算（[`GameTask::finish_combat`]），保证房间继续推进。
    ///
    /// 顺序为「先推进阶段，再处理断线宽限」：阶段推进是时间语义的一部分，必须与
    /// 连接状态无关地按时发生；断线标记只影响可观测的在线状态，安排在推进之后，
    /// 不会改变原有阶段推进的调用次数与顺序。
    pub fn tick(&mut self) -> Vec<SeatUpdate> {
        let mut updates = self.tick_inner();
        self.append_achievement_updates(&mut updates);
        updates
    }

    fn tick_inner(&mut self) -> Vec<SeatUpdate> {
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

    /// 把本批新解锁的成就追加为对应座位的更新。
    ///
    /// `Achievement::ALL` 的顺序由解锁集合的 `BTreeSet` 保证；同一批内按座位升序、
    /// 同一座位内按成就顺序追加，保持确定性。未知成就没有 Steam 映射时跳过。
    fn append_achievement_updates(&mut self, updates: &mut Vec<SeatUpdate>) {
        for (seat, player) in self.state.players.iter().enumerate() {
            let seen = &mut self.achievement_seen[seat];
            let new: Vec<Achievement> = player
                .achievements
                .unlocked
                .difference(seen)
                .copied()
                .collect();
            if new.is_empty() {
                continue;
            }
            seen.extend(new.iter().copied());
            let player_id = self.seats[seat].player_id.clone();
            for achievement in new {
                let Some(steam_api_name) = self.achievements.steam_api_name(achievement.id())
                else {
                    continue;
                };
                updates.push(SeatUpdate {
                    player_id: player_id.clone(),
                    kind: UpdateKind::AchievementUnlocked {
                        achievement_id: achievement.id().to_string(),
                        steam_api_name: steam_api_name.to_string(),
                    },
                });
            }
        }
    }

    /// 记录一个参战座位已播完本场演示；全部参战座位回报后立即结算。
    ///
    /// 非 `Combat` 阶段、非参战座位或重复回报都只回当前视图，不产生错误。
    fn mark_combat_played(&mut self, player_id: &str) -> Vec<SeatUpdate> {
        let Some(seat) = self.seat_of.get(player_id).copied() else {
            return Vec::new();
        };
        if self.is_departed(seat) {
            return Vec::new();
        }
        if self.state.phase != Phase::Combat || !self.combat_participants[usize::from(seat)] {
            return vec![self.view_update(seat)];
        }
        self.combat_acks[usize::from(seat)] = true;
        if self.all_combat_played() {
            return self.finish_combat().unwrap_or_default();
        }
        vec![self.view_update(seat)]
    }

    /// 对局内主动退出：按规则引擎弃权结算，并把该座位从后续推送中移除。
    ///
    /// 未知玩家、已淘汰座位与 `GameOver` 阶段都返回空更新。弃权座位记录在
    /// `departed` 中，直到回房重置才统一移除；本局内它的座位与名次仍保留在
    /// 其他人的视图与终局排名里。
    fn forfeit(&mut self, player_id: &str) -> Vec<SeatUpdate> {
        let Some(seat) = self.seat_of.get(player_id).copied() else {
            return Vec::new();
        };
        // 离开必须登记，无论座位是否已淘汰或本局是否已终局：回房重置只按
        // `departed` 移除座位，漏记会让该座位（乃至房主身份）永久残留。
        self.departed.insert(player_id.to_string());
        if self.state.phase == Phase::GameOver || self.state.players[usize::from(seat)].eliminated {
            return Vec::new();
        }

        let events = rule_engine::settle::forfeit(&mut self.state, seat);
        if events.is_empty() {
            return Vec::new();
        }
        // 后续视图与结果都不再投递给该座位。
        self.record_events(&events);

        let all_seats: Vec<Seat> = (0..SEAT_COUNT as Seat).collect();
        if self.state.phase == Phase::GameOver {
            // 终结时清除演示期临时状态，结果与视图一并下发（视图在前）。
            self.combat_reports = None;
            self.combat_ranks = None;
            self.combat_started_at_ms = None;
            self.combat_acks = [false; SEAT_COUNT];
            self.combat_participants = [false; SEAT_COUNT];
            self.log_game_over_if_needed();

            let mut updates = self.view_updates(&events, &all_seats);
            for seat in all_seats {
                if self.is_departed(seat) {
                    continue;
                }
                updates.push(SeatUpdate {
                    player_id: self.seats[usize::from(seat)].player_id.clone(),
                    kind: UpdateKind::GameOver(self.game_over_payload(seat)),
                });
            }
            return updates;
        }

        if self.state.phase == Phase::Combat {
            // 本场演示不再等待该座位：视为已回报，其余座位播完后立即结算。
            let index = usize::from(seat);
            self.combat_participants[index] = false;
            self.combat_acks[index] = true;
            if self.all_combat_played() {
                return self.finish_combat().unwrap_or_default();
            }
        }

        self.view_updates(&events, &all_seats)
    }

    /// 座位是否已在本局弃权。
    fn is_departed(&self, seat: Seat) -> bool {
        self.departed
            .contains(&self.seats[usize::from(seat)].player_id)
    }

    /// 本场全部参战座位是否都已回报。
    fn all_combat_played(&self) -> bool {
        (0..SEAT_COUNT).all(|seat| !self.combat_participants[seat] || self.combat_acks[seat])
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
        // 已弃权的座位不再接收任何更新；其意图也一并忽略。
        if self.is_departed(seat) {
            return Vec::new();
        }

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

        // 去重命中属于重传，不计入意图数；这里开始统计本座位的意图与拒绝。
        let seat_index = usize::from(seat);
        self.intent_counts[seat_index] = self.intent_counts[seat_index].saturating_add(1);

        if self.state.phase != Phase::Recruit {
            let message = format!("当前阶段 {:?} 不接受招募意图", self.state.phase);
            self.rejection_counts[seat_index] = self.rejection_counts[seat_index].saturating_add(1);
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
                self.rejection_counts[seat_index] =
                    self.rejection_counts[seat_index].saturating_add(1);
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
                self.record_events(&outcome.events);
                let all_seats: Vec<Seat> = (0..SEAT_COUNT as Seat).collect();
                self.view_updates(&outcome.events, &all_seats)
            }
            Err(reject) => {
                let code = error_code_of(reject.code);
                let message = reject.message;
                self.rejection_counts[seat_index] =
                    self.rejection_counts[seat_index].saturating_add(1);
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

    /// 按引擎事件累计各座位统计：只做增量累计，不读取时钟。
    ///
    /// 调用方负责保证同一批事件只记录一次；`CombatResolved` 的负场与承受伤害只在
    /// `damage > 0` 时计入，因此幽灵战的败者事件（`damage == 0`）自然被排除。
    /// 每批事件末尾同步一次英雄技能累计（`start_turn` 清空 `hero_uses` 时只更新基线）。
    fn record_events(&mut self, events: &[EngineEvent]) {
        self.record_events_filtered(events, true);
    }

    /// 同 [`GameTask::record_events`]；`count_rerolls = false` 时跳过 `ShopRolled`。
    ///
    /// `start_turn` 内部的自动刷新也发 `ShopRolled`，它不是玩家主动刷新。
    fn record_events_filtered(&mut self, events: &[EngineEvent], count_rerolls: bool) {
        for event in events {
            match event {
                EngineEvent::MinionBought { seat, .. } => {
                    let stats = &mut self.seat_stats[usize::from(*seat)];
                    stats.buys = stats.buys.saturating_add(1);
                }
                EngineEvent::MinionSold { seat, .. } => {
                    let stats = &mut self.seat_stats[usize::from(*seat)];
                    stats.sells = stats.sells.saturating_add(1);
                }
                EngineEvent::ShopRolled { seat } if count_rerolls => {
                    let stats = &mut self.seat_stats[usize::from(*seat)];
                    stats.rerolls = stats.rerolls.saturating_add(1);
                }
                EngineEvent::TripleFormed { seat, .. } => {
                    let stats = &mut self.seat_stats[usize::from(*seat)];
                    stats.triples = stats.triples.saturating_add(1);
                }
                EngineEvent::PlayerEliminated { seat, .. } => {
                    let turn = self.state.turn;
                    let stats = &mut self.seat_stats[usize::from(*seat)];
                    if stats.eliminated_turn.is_none() {
                        stats.eliminated_turn = Some(turn);
                    }
                }
                EngineEvent::CombatResolved {
                    winner,
                    loser,
                    damage,
                } => {
                    if *damage > 0 {
                        let stats = &mut self.seat_stats[usize::from(*loser)];
                        stats.combat_losses = stats.combat_losses.saturating_add(1);
                        stats.damage_taken = stats.damage_taken.saturating_add(*damage);
                    }
                    // 幽灵获胜时胜者座位已淘汰，不计胜场。
                    if !self.state.players[usize::from(*winner)].eliminated {
                        let stats = &mut self.seat_stats[usize::from(*winner)];
                        stats.combat_wins = stats.combat_wins.saturating_add(1);
                    }
                }
                EngineEvent::CombatDrawn { a, b } => {
                    // 幽灵平局双方是同一座位，只计一次。
                    let seats = [*a, *b];
                    let count = if a == b { 1 } else { 2 };
                    for seat in seats.into_iter().take(count) {
                        if !self.state.players[usize::from(seat)].eliminated {
                            let stats = &mut self.seat_stats[usize::from(seat)];
                            stats.combat_draws = stats.combat_draws.saturating_add(1);
                        }
                    }
                }
                _ => {}
            }
        }
        self.sync_hero_uses();
    }

    /// 把各座位 `hero_uses` 总和的变化累计到 [`SeatStats::hero_power_uses`]。
    ///
    /// 引擎在 `start_turn` 清空 `hero_uses`，因此只按「当前总和 > 基线」计入
    /// 增量；下降或其他变化只把基线更新为当前值，不回退已累计次数。
    fn sync_hero_uses(&mut self) {
        for (index, player) in self.state.players.iter().enumerate() {
            let current: u32 = player.hero_uses.values().map(|uses| u32::from(*uses)).sum();
            let baseline = self.hero_uses_baseline[index];
            if current > baseline {
                let stats = &mut self.seat_stats[index];
                stats.hero_power_uses = stats.hero_power_uses.saturating_add(current - baseline);
            }
            self.hero_uses_baseline[index] = current;
        }
    }

    /// 把逐座位战斗报告转成 `CombatResult` 更新：一份报告对应一条更新。
    ///
    /// 幽灵战在 `outcome.reports` 里只有该座位自己的一条，因此只会产生 1 条更新。
    pub fn combat_updates(&self, reports: &BTreeMap<Seat, SeatCombatReport>) -> Vec<SeatUpdate> {
        let started_at = self.combat_started_at();
        reports
            .iter()
            .filter(|(seat, _)| !self.is_departed(**seat))
            .map(|(&seat, report)| SeatUpdate {
                player_id: self.seats[usize::from(seat)].player_id.clone(),
                kind: UpdateKind::CombatResult(Box::new(combat_result(
                    &self.engine.catalog,
                    report,
                    &self.seats,
                    started_at,
                ))),
            })
            .collect()
    }

    /// 本场演示开始时刻；尚未进入战斗时退回当前时钟，避免编码失败。
    fn combat_started_at(&self) -> chrono::DateTime<chrono::Utc> {
        let ms = self
            .combat_started_at_ms
            .unwrap_or_else(|| self.clock.now_ms());
        chrono::DateTime::from_timestamp_millis(i64::try_from(ms).unwrap_or(i64::MAX))
            .unwrap_or_else(chrono::Utc::now)
    }

    /// 招募 → 战斗演示：配对、模拟战斗、逐座位下发 `CombatResult`。
    ///
    /// `finish_recruit → resolve_combat → 存本场报告与开战时刻 → 设下一回合招募截止`。
    /// 阶段停在 `Combat`，等待全部参战座位回报「演示完成」；到点仍未全部回报时
    /// 由 [`GameTask::finish_combat`] 兜底结算。
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
        // 进入战斗前的事件只在这里记录一次：`handle_intent` 的战斗分支不记录，
        // 到点进入战斗时本方法收到空事件。
        self.record_events(&events);

        let now = self.clock.now_ms();
        self.combat_started_at_ms = Some(now);
        // 下一回合招募计时从开战时刻起算；本截止同时是演示回报的兜底时刻。
        // 演示动作数超过招募时长时延长截止，保证客户端播完再结算。
        let max_actions = outcome
            .reports
            .values()
            .map(|report| report.actions.len())
            .max()
            .unwrap_or(0);
        self.phase_deadline_ms = combat_deadline(now, self.state.turn, max_actions);
        self.combat_acks = [false; SEAT_COUNT];
        self.combat_participants = [false; SEAT_COUNT];
        for &seat in outcome.reports.keys() {
            self.combat_participants[usize::from(seat)] = true;
        }
        self.combat_reports = Some(outcome.reports);

        let mut updates = Vec::with_capacity(SEAT_COUNT * 2);
        updates.extend(self.combat_updates(self.combat_reports.as_ref().expect("刚写入")));

        let all_seats: Vec<Seat> = (0..SEAT_COUNT as Seat).collect();
        updates.extend(self.view_updates(&events, &all_seats));
        Ok(updates)
    }

    /// 战斗演示 → 结算 →（非 `GameOver`）下一回合招募。
    ///
    /// 全部参战座位回报后立即调用，或在下一回合招募截止时兜底调用。结算后不重算
    /// 截止时间：进入战斗时设定的下一回合招募截止继续生效，战斗越短剩余招募时间
    /// 越多。
    fn finish_combat(&mut self) -> Result<Vec<SeatUpdate>, EngineError> {
        let reports = self.combat_reports.take();
        // 战斗已在 `enter_combat` 里算完，阶段被演示临时留在 `Combat`；结算前拨回
        // 引擎期望的 `Settle`。
        self.state.phase = Phase::Settle;
        let mut events = self.engine.resolve_settle(&mut self.state)?;
        // 结算事件的回合号仍是本回合：必须在 `start_turn` 递增回合之前记录，
        // 否则出局座位的 `survivedTurns` 会多算一回合。
        self.record_events(&events);
        // 战报在结算时才公开，避免演示期间提前看到本场结果。
        if let Some(reports) = &reports {
            self.record_combat_log(reports);
        }
        // 演示结束：名次恢复实时计算（若已进入 `GameOver`，`rank_of` 与
        // `placement` 一致）。
        self.combat_ranks = None;
        if self.state.phase != Phase::GameOver {
            let turn_events = self.engine.start_turn(&mut self.state)?;
            // 回合开始效果与自动刷新同属新回合的事件，随后记录一次；自动刷新
            // 不是玩家主动刷新，不计入 `rerolls`。
            self.record_events_filtered(&turn_events, false);
            events.extend(turn_events);
        }
        self.combat_started_at_ms = None;
        self.combat_acks = [false; SEAT_COUNT];
        self.combat_participants = [false; SEAT_COUNT];

        let all_seats: Vec<Seat> = (0..SEAT_COUNT as Seat).collect();
        let mut updates = self.view_updates(&events, &all_seats);

        if self.state.phase == Phase::GameOver {
            self.log_game_over_if_needed();
            // 逐座位生成载荷：`selfStats` 只填接收者自己的统计，不能共享同一份。
            for seat in 0..SEAT_COUNT as Seat {
                if self.is_departed(seat) {
                    continue;
                }
                updates.push(SeatUpdate {
                    player_id: self.seats[usize::from(seat)].player_id.clone(),
                    kind: UpdateKind::GameOver(self.game_over_payload(seat)),
                });
            }
        }
        Ok(updates)
    }

    /// 对局结束观测：一行 JSON，供内测的拒绝率与单局时长记录使用。
    ///
    /// 意图数只统计首次提交（去重命中属于重传，不计数）；任何一次拒绝
    /// （阶段不符、参数非法、引擎拒绝）都计入拒绝数。拒绝率为千分比，
    /// 没有意图时为 0。
    fn observation_json(&self) -> serde_json::Value {
        let intents_total: u64 = self
            .intent_counts
            .iter()
            .map(|count| u64::from(*count))
            .sum();
        let rejections_total: u64 = self
            .rejection_counts
            .iter()
            .map(|count| u64::from(*count))
            .sum();
        let rejection_rate_permille = if intents_total == 0 {
            0
        } else {
            rejections_total.saturating_mul(1_000) / intents_total
        };
        serde_json::json!({
            "service": "room-service",
            "event": "game_over",
            "roomId": self.room_id,
            "seed": format!("{:016x}", self.seed),
            "turns": self.state.turn,
            "durationMs": self.clock.now_ms().saturating_sub(self.started_at_ms),
            "intentsBySeat": self.intent_counts,
            "intentsTotal": intents_total,
            "rejectionsBySeat": self.rejection_counts,
            "rejectionsTotal": rejections_total,
            "rejectionRatePermille": rejection_rate_permille,
            "survivors": self.state.players.iter().filter(|player| !player.eliminated).count(),
            "winner": self.state.ranking.first().copied(),
        })
    }

    /// 进入 `GameOver` 时输出一次观测日志；重复调用不会重复打印。
    fn log_game_over_if_needed(&mut self) {
        if self.state.phase != Phase::GameOver || self.game_over_logged {
            return;
        }
        self.game_over_logged = true;
        eprintln!("{}", self.observation_json());
    }

    /// 对局结束载荷：排名（`place` 从 1 起）与 16 位十六进制种子。
    ///
    /// `ranking` 对每个座位都完整下发；`selfStats` 只填 `seat` 本人的统计。
    fn game_over_payload(&self, seat: Seat) -> GameOver {
        let ranking = self
            .state
            .ranking
            .iter()
            .enumerate()
            .filter_map(|(index, ranked_seat)| {
                let place = u64::try_from(index + 1)
                    .ok()
                    .and_then(std::num::NonZeroU64::new)?;
                Some(GameOverRankingItem {
                    place,
                    player_id: self.seats[usize::from(*ranked_seat)].player_id.clone(),
                    survived_turns: Some(self.survived_turns(*ranked_seat)),
                    damage_dealt: Some(self.damage_dealt(*ranked_seat)),
                })
            })
            .collect();
        GameOver {
            ranking,
            seed: format!("{:016x}", self.seed),
            self_stats: Some(self.self_stats(seat)),
        }
    }

    /// 座位存活到的回合：出局者取淘汰事件所在回合，存活者取结束回合。
    fn survived_turns(&self, seat: Seat) -> i64 {
        let stats = &self.seat_stats[usize::from(seat)];
        match stats.eliminated_turn {
            Some(turn) => i64::from(turn),
            None => i64::from(self.state.turn),
        }
    }

    /// 座位本局造成的总伤害；负值按 0 处理。
    fn damage_dealt(&self, seat: Seat) -> u64 {
        u64::try_from(self.state.players[usize::from(seat)].damage_dealt.max(0)).unwrap_or(0)
    }

    /// 座位本人的本局统计；终局读 `PlayerState` 与棋盘，增量字段读事件累计表。
    fn self_stats(&self, seat: Seat) -> SelfStats {
        let index = usize::from(seat);
        let player = &self.state.players[index];
        let stats = &self.seat_stats[index];
        // `GameOver` 时 `ranking` 一定包含全部座位；缺失时按最后一名兜底。
        let place = self
            .state
            .ranking
            .iter()
            .position(|ranked| *ranked == seat)
            .map_or(SEAT_COUNT, |position| position + 1);
        let place = u64::try_from(place)
            .ok()
            .and_then(std::num::NonZeroU64::new)
            .unwrap_or(std::num::NonZeroU64::MIN);
        let final_era = u64::from(player.era as u8);
        let final_era = std::num::NonZeroU64::new(final_era).unwrap_or(std::num::NonZeroU64::MIN);
        SelfStats {
            board: player
                .board
                .iter()
                .take(7)
                .map(|minion| SelfStatsBoardItem {
                    card_id: minion.card_id.clone(),
                    golden: minion.golden,
                    attack: u64::try_from(minion.attack().max(0)).unwrap_or(0),
                    health: u64::try_from(minion.health().max(0)).unwrap_or(0),
                })
                .collect(),
            buys: u64::from(stats.buys),
            combat_draws: u64::from(stats.combat_draws),
            combat_losses: u64::from(stats.combat_losses),
            combat_wins: u64::from(stats.combat_wins),
            damage_dealt: self.damage_dealt(seat),
            damage_taken: u64::try_from(stats.damage_taken.max(0)).unwrap_or(0),
            final_era,
            final_hp: u64::try_from(player.hp.max(0)).unwrap_or(0),
            hero_power_uses: u64::from(stats.hero_power_uses),
            place,
            remaining_gold: u64::from(player.gold),
            rerolls: u64::from(stats.rerolls),
            sells: u64::from(stats.sells),
            survived_turns: self.survived_turns(seat),
            triples: u64::from(stats.triples),
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
            if self.is_departed(seat) {
                continue;
            }
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
        if self.departed.contains(player_id) {
            return Vec::new();
        }
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
        // 已弃权的座位不再接收任何消息：不绑定发送端、不产出更新。
        if self.departed.contains(player_id) {
            return Vec::new();
        }
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
                    self.combat_started_at(),
                ))),
            });
        }

        // 对局结束即销毁房间，重连不会再到达本 task；这里不再补发 `GameOver`。
        updates
    }

    /// 记录断线时刻；座位与对局状态保持不变。
    ///
    /// 重复断线只保留第一次时刻，避免连接层抖动无限延长宽限期。未知玩家的断开不是
    /// 错误，直接忽略。战斗演示期间断线的参战座位视为已回报，避免阻塞整局结算。
    fn mark_disconnect(&mut self, player_id: &str) -> Vec<SeatUpdate> {
        let Some(seat) = self.seat_of.get(player_id).copied() else {
            return Vec::new();
        };
        let index = usize::from(seat);
        if self.disconnected_at_ms[index].is_none() {
            self.disconnected_at_ms[index] = Some(self.clock.now_ms());
        }
        if self.state.phase == Phase::Combat && self.combat_participants[index] {
            self.combat_acks[index] = true;
            if self.all_combat_played() {
                return self.finish_combat().unwrap_or_default();
            }
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
                message_id: format!("error.{code}"),
                message_params: Vec::new(),
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

/// 把协议目标解析为引擎目标（区域 + 实例 id）；id 解析失败返回 `NO_SUCH_MINION`。
fn parse_target(target: Option<protocol::TargetRef>) -> Result<Option<EngineTargetRef>, ErrorCode> {
    target.map(map_target).transpose()
}

/// 把协议目标映射为引擎目标；区域字符串由生成类型保证合法。
fn map_target(target: protocol::TargetRef) -> Result<EngineTargetRef, ErrorCode> {
    let zone = match target.zone {
        protocol::TargetZone::Hand => EngineZone::Hand,
        protocol::TargetZone::Shop => EngineZone::Shop,
        protocol::TargetZone::Board => EngineZone::Board,
    };
    Ok(EngineTargetRef {
        zone,
        id: target
            .id
            .parse::<InstanceId>()
            .map_err(|_| ErrorCode::NoSuchMinion)?,
    })
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
            target,
        } => Ok(EngineIntent::PlayCard {
            hand_index: to_u8(hand_index)?,
            position: position.map(to_u8).transpose()?,
            target: parse_target(target)?,
        }),
        Protocol::ChooseTarget { target } => Ok(EngineIntent::ChooseTarget {
            target: map_target(target)?,
        }),
        Protocol::AbandonTarget => Ok(EngineIntent::AbandonTarget),
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
        Protocol::UnfreezeShop { slot } => Ok(EngineIntent::UnfreezeShop {
            slot: slot.map(to_u8).transpose()?,
        }),
        Protocol::UseHeroPower { skill_id, target } => Ok(EngineIntent::UseHeroPower {
            skill_id,
            target: parse_target(target)?,
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
        RejectCode::NotActiveSkill => ErrorCode::NotActiveSkill,
        RejectCode::NoValidTarget => ErrorCode::NoValidTarget,
        RejectCode::TargetNotAllowed => ErrorCode::TargetNotAllowed,
        RejectCode::InvalidChoice => ErrorCode::InvalidChoice,
        RejectCode::SevenVoyagesActive => ErrorCode::SevenVoyagesActive,
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

    use rule_engine::achievements::Achievement;
    use rule_engine::data::{Catalog, Playstyle};
    use rule_engine::state::{MinionInstance, Pairing, Phase, Seat};

    use super::{RoomCommand, UpdateKind, combat_deadline, recruit_seconds};
    use crate::clock::FixedClock;
    use crate::envelope::SeatChannel;
    use crate::game::GameTask;
    use crate::room::PlayerSlot;

    const CARDS_JSON: &str = include_str!("../../../data/v0.1/cards.json");
    const HEROES_JSON: &str = include_str!("../../../data/v0.1/heroes.json");
    const SKILLS_JSON: &str = include_str!("../../../data/v0.1/skills.json");
    const CONFIG_JSON: &str = include_str!("../../../data/v0.1/config.json");
    const TAXONOMY_JSON: &str = include_str!("../../../data/v0.1/taxonomy.json");

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
        task_at(FixedClock::new(1_000_000))
    }

    #[test]
    fn achievement_updates_are_emitted_once_with_steam_mapping() {
        let mut task = task();
        task.state.players[0]
            .achievements
            .unlocked
            .insert(Achievement::BiologicalEvolution);

        let mut updates = Vec::new();
        task.append_achievement_updates(&mut updates);
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].player_id, task.seats[0].player_id);
        match &updates[0].kind {
            UpdateKind::AchievementUnlocked {
                achievement_id,
                steam_api_name,
            } => {
                assert_eq!(achievement_id, "biological-evolution");
                assert_eq!(steam_api_name, "BIOLOGICAL_EVOLUTION");
            }
            other => panic!("期望成就解锁更新，实际为 {other:?}"),
        }

        updates.clear();
        task.append_achievement_updates(&mut updates);
        assert!(updates.is_empty(), "同一解锁不应重复推送");
    }

    /// 测试固定玩法：覆盖各级随从，保证商店不至于因池过小而全空。
    fn playstyles() -> [Playstyle; 5] {
        [
            Playstyle::Mammal,
            Playstyle::Bird,
            Playstyle::Plant,
            Playstyle::Virus,
            Playstyle::Machine,
        ]
    }

    fn task_at(clock: FixedClock) -> GameTask {
        let catalog = Catalog::from_json(
            CARDS_JSON,
            HEROES_JSON,
            SKILLS_JSON,
            CONFIG_JSON,
            TAXONOMY_JSON,
        )
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
                hero_candidates: vec![HEROES[seat].to_string()],
                candidate_segment: None,
                channel: SeatChannel::new(),
                connected: true,
                disconnected_at_ms: None,
            })
            .collect();
        GameTask::new(
            catalog,
            7,
            playstyles(),
            heroes,
            players,
            Arc::new(clock),
            Arc::new(crate::AchievementCatalog::embedded()),
        )
        .expect("开局应当成功")
    }

    /// 构造 0 号与 1 号座位对战、其余座位已淘汰的局面；调用后阶段为 `Combat`，
    /// 可直接进入 [`GameTask::enter_combat`]。
    fn prepare_two_seat_combat(task: &mut GameTask) {
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
        task.state.last_won = Default::default();
        task.state.pending_settle.clear();
        task.state.pairings = vec![Pairing {
            a: 0,
            b: 1,
            ghost: None,
        }];
        let catalog = &task.engine.catalog;
        task.state.players[0].board = vec![MinionInstance::new(catalog, 1, "bruce-lee", false)];
        task.state.players[1].board = vec![MinionInstance::new(catalog, 2, "baiyi", false)];
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
    fn recruit_seconds_follows_battlegrounds_curve() {
        assert_eq!(recruit_seconds(1), 60);
        assert_eq!(recruit_seconds(2), 75);
        assert_eq!(recruit_seconds(3), 90);
        assert_eq!(recruit_seconds(4), 105);
        assert_eq!(recruit_seconds(5), 120);
        assert_eq!(recruit_seconds(6), 120);
    }

    #[test]
    fn combat_deadline_extends_only_for_long_demos() {
        let now = 1_000_000;
        // 第 2 回合进入战斗，下一回合招募 75 秒；10 条动作约 5.2 秒，不延长。
        assert_eq!(combat_deadline(now, 1, 10), now + 75_000);
        // 300 条动作约 98 秒，超过招募截止，按演示时长延长。
        assert_eq!(combat_deadline(now, 1, 300), now + 98_000);
        // 招募已封顶 120 秒时，普通长战斗仍以招募截止为准。
        assert_eq!(combat_deadline(now, 5, 100), now + 120_000);
    }

    /// 进入战斗后不会按估算时长自动结算；全部参战座位回报后才结算。
    #[test]
    fn combat_waits_for_all_participants_before_settling() {
        let mut task = task();
        prepare_two_seat_combat(&mut task);
        task.enter_combat(Vec::new()).expect("进入战斗应成功");

        assert_eq!(task.state.phase, Phase::Combat);
        assert!(task.combat_participants[0] && task.combat_participants[1]);
        assert!(!task.combat_participants[2], "已淘汰座位不参与等待");
        // 下一回合是第 3 回合：招募计时从开战时刻起算，时长 90 秒。
        assert_eq!(task.phase_deadline_ms, 1_000_000 + 90_000);

        task.mark_combat_played("p0");
        assert_eq!(task.state.phase, Phase::Combat, "还有参战座位未回报");

        task.mark_combat_played("p1");
        assert_eq!(task.state.phase, Phase::Recruit);
        assert_eq!(task.state.turn, 3);
        assert_eq!(task.phase_deadline_ms, 1_090_000, "结算不重算招募截止");
    }

    /// 断线的参战座位视为已回报，不阻塞其余座位结算。
    #[test]
    fn combat_disconnect_counts_as_played() {
        let mut task = task();
        prepare_two_seat_combat(&mut task);
        task.enter_combat(Vec::new()).expect("进入战斗应成功");

        task.mark_disconnect("p0");
        assert_eq!(task.state.phase, Phase::Combat);

        task.mark_combat_played("p1");
        assert_eq!(task.state.phase, Phase::Recruit);
    }

    /// 8 个座位看到同一组本局玩法，且与引擎状态一致。
    #[test]
    fn enabled_factions_are_shared_by_every_seat() {
        let task = task();
        let expected: Vec<String> = task
            .state
            .enabled_playstyles
            .iter()
            .map(|playstyle| playstyle.id().to_string())
            .collect();
        assert_eq!(expected.len(), 5, "本局玩法固定 5 个");
        for seat in 0..8u8 {
            let view = task.player_view(seat).expect("视图应可用");
            assert_eq!(
                view.enabled_factions.as_slice(),
                expected.as_slice(),
                "座位 {seat} 的 enabledFactions 应与本局玩法一致"
            );
        }
    }

    /// 未回报时到点兜底结算，保证房间继续推进。
    #[test]
    fn combat_settles_at_deadline_when_acks_missing() {
        let clock = FixedClock::new(1_000_000);
        let mut task = task_at(clock.clone());
        prepare_two_seat_combat(&mut task);
        task.enter_combat(Vec::new()).expect("进入战斗应成功");

        clock.advance(90_000);
        task.tick();

        assert_eq!(task.state.phase, Phase::Recruit);
        assert_eq!(task.state.turn, 3);
    }

    /// 战报在结算时才公开，演示期间不出现本场结果。
    #[test]
    fn combat_log_is_published_after_settle() {
        let mut task = task();
        task.combat_log.clear();
        prepare_two_seat_combat(&mut task);
        task.enter_combat(Vec::new()).expect("进入战斗应成功");
        assert!(task.combat_log.is_empty(), "演示期间不公开本场战报");

        task.mark_combat_played("p0");
        assert!(task.combat_log.is_empty());

        task.mark_combat_played("p1");
        assert_eq!(task.combat_log.len(), 1, "结算后公开本场战报");
    }

    /// 协议目标三个区域逐一映射到引擎区域。
    #[test]
    fn map_target_maps_every_zone() {
        use protocol::{TargetRef as ProtocolTargetRef, TargetZone};
        use rule_engine::effects::{TargetRef as EngineTargetRef, Zone as EngineZone};

        for (zone, engine_zone) in [
            (TargetZone::Hand, EngineZone::Hand),
            (TargetZone::Shop, EngineZone::Shop),
            (TargetZone::Board, EngineZone::Board),
        ] {
            let mapped = super::map_target(ProtocolTargetRef {
                zone,
                id: "42".to_string(),
            })
            .expect("合法编号应能映射");
            assert_eq!(
                mapped,
                EngineTargetRef {
                    zone: engine_zone,
                    id: 42
                },
                "{zone:?} 应映射到对应引擎区域"
            );
        }
    }

    /// 实例编号不是数字时按「目标不存在」拒绝。
    #[test]
    fn map_target_rejects_non_numeric_id() {
        use protocol::{TargetRef as ProtocolTargetRef, TargetZone};

        let error = super::map_target(ProtocolTargetRef {
            zone: TargetZone::Board,
            id: "minion-1".to_string(),
        })
        .expect_err("非数字编号应被拒绝");
        assert_eq!(error, protocol::ErrorCode::NoSuchMinion);
    }

    /// 首次提交计入意图数，重传命中去重表不重复计数。
    #[test]
    fn observation_counts_only_first_time_intents() {
        let mut task = task();
        let intent = || protocol::intents::Intent::RerollShop;
        task.handle(RoomCommand::Intent {
            player_id: "p0".to_string(),
            client_intent_id: "i1".to_string(),
            intent: intent(),
        });
        task.handle(RoomCommand::Intent {
            player_id: "p0".to_string(),
            client_intent_id: "i1".to_string(),
            intent: intent(),
        });

        let json = task.observation_json();
        assert_eq!(json["intentsBySeat"][0], 1, "重传不应重复计数");
        assert_eq!(json["intentsTotal"], 1);
        assert_eq!(json["rejectionsTotal"], 0, "成功的意图不算拒绝");
    }

    /// 被拒绝的意图计入拒绝数，拒绝率为千分比。
    #[test]
    fn observation_counts_rejections_and_rate() {
        let mut task = task();
        task.handle(RoomCommand::Intent {
            player_id: "p0".to_string(),
            client_intent_id: "bad".to_string(),
            // 越界槽位：引擎按「商店槽位为空」拒绝。
            intent: protocol::intents::Intent::BuyMinion { slot: 99 },
        });
        task.handle(RoomCommand::Intent {
            player_id: "p0".to_string(),
            client_intent_id: "ok".to_string(),
            intent: protocol::intents::Intent::RerollShop,
        });

        let json = task.observation_json();
        assert_eq!(json["intentsTotal"], 2);
        assert_eq!(json["rejectionsBySeat"][0], 1);
        assert_eq!(json["rejectionsTotal"], 1);
        assert_eq!(json["rejectionRatePermille"], 500);
    }

    /// 对局结束观测每局只输出一次；字段覆盖时长、名次与存活人数。
    #[test]
    fn observation_logs_once_at_game_over() {
        let mut task = task();
        task.state.ranking = vec![0, 1, 2, 3, 4, 5, 6, 7];
        task.state.phase = Phase::GameOver;

        task.log_game_over_if_needed();
        assert!(task.game_over_logged, "进入 GameOver 应标记已记录");
        task.log_game_over_if_needed();
        assert!(task.game_over_logged);

        let json = task.observation_json();
        assert_eq!(json["event"], "game_over");
        assert_eq!(json["winner"], 0);
        assert_eq!(json["survivors"], 8);
        assert_eq!(json["durationMs"], 0, "固定时钟下开赛即结束");
        assert!(json["seed"].as_str().is_some_and(|seed| seed.len() == 16));
    }
}
