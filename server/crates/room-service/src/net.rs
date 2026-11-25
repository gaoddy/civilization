//! WebSocket 接入层与房间写者任务。
//!
//! # task 拓扑
//!
//! - **连接 task**（每个 WS 连接一个）：读端把 `ClientEnvelope` 校验版本、解析为
//!   [`ClientMessage`]，再映射成 [`RoomCommand`] / 房间消息投给房间 task；认证、
//!   建房、入房与重连这类入口也在这里完成。
//! - **写端 task**（每个连接一个）：只从 `mpsc::Receiver<ServerEnvelope>` 取信封，
//!   序列化成 JSON 文本写回 WS；连接断开或发送失败即退出。
//! - **房间 task**（每个房间一个，单写者）：串行消费房间消息，独占 `Room` /
//!   `GameTask` 的可变状态；定时器驱动 [`GameTask::tick`]。连接层从不直接改房间
//!   状态，只投命令。
//!
//! # 下行序号
//!
//! 同一玩家的所有下行共享一个 [`SeqFloor`]（会话级序号下限）。三条下行来源
//! （连接层 `direct`、大厅 `slot.channel`、对局 `GameTask` 通道）在编码前都先把
//! 通道推进到当前下限，发送后再把新 `seq` 记回下限，因此 `seq` 跨来源也严格递增。
//!
//! # 连接代次
//!
//! 每次 `Attach`/`Reconnect` 都带一个进程内唯一 `generation`。`Detach`/`Leave`
//! 只在代次与当前登记一致时生效，避免新旧连接并存时旧连接迟到关闭误释放座位。
//!
//! # 锁粒度
//!
//! [`AppState::sessions`] 与 [`AppState::rooms`] 各自一把 `tokio::sync::Mutex`，
//! 只在短操作内持有、不跨网络等待；[`AppState::hub`] 只在建房/查表时短暂持有。
//! 房间内的可变状态由房间 task 独占。

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use axum::Router;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::get;
use futures_util::{SinkExt, StreamExt};
use protocol::AchievementStateUnlockedItem;
use protocol::RoomClosedReason;
use protocol::server::{
    AchievementState, FriendInvite as FriendInviteMessage, FriendInviteResult, FriendRequestItem,
    FriendUpdate as FriendUpdateMessage, FriendUpdateFriendsItem, FriendUpdateFriendsItemStatus,
    FriendUpdateOutgoingItem, FriendUpdateRecentItem, FriendUpdateRecentItemRelation,
    InviteAccepted, Leaderboard, LeaderboardEntriesItem, MatchFound, PhoneCodeSent, PlayerProfile,
    PlayerProfileRecentItem, QueueState, RoomClosed, RoomInviteCode,
};
use protocol::{ClientEnvelope, ClientMessage, ServerEnvelope};
use rule_engine::state::Phase;
use serde::Serialize;
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{self, MissedTickBehavior};

use crate::AppState;
use crate::auth::Session;
use crate::envelope::SeatChannel;
use crate::error::error_message;
use crate::game::{GameTask, RoomCommand, SeatUpdate, UpdateKind};
use crate::queue::{QueueEntry, QueueStateData};
use crate::room::{SEAT_COUNT, seed_for_code};
use crate::social::{FriendStatus, InviteOutcome, Relation, SocialError, SocialSnapshot};

/// 每个连接下行队列的容量；连接写不动时房间 task 用 `try_send` 丢弃而不是阻塞。
const OUTBOUND_CAPACITY: usize = 64;

/// 房间计时检查的间隔。
const TICK_INTERVAL: Duration = Duration::from_millis(100);

/// 匹配队列推进间隔。
const QUEUE_TICK_INTERVAL: Duration = Duration::from_millis(200);

/// 匹配房等待 8 个连接挂上房间 task 的宽限；超时后按已连接座位开局。
const RANKED_ATTACH_GRACE_MS: u64 = 3_000;

/// 连接关闭时留给写端冲刷已入队消息的时长。
const FLUSH_TIMEOUT: Duration = Duration::from_millis(200);
/// 顶号接管前探测座位是否仍有效的最长等待；超时按无效处理，新客户端进大厅。
const SEAT_PROBE_TIMEOUT: Duration = Duration::from_millis(500);

/// 会话级下行序号下限；连接 task 与房间 task 共享。
#[derive(Clone, Default)]
pub struct SeqFloor {
    value: Arc<AtomicU64>,
}

impl SeqFloor {
    /// 当前已发送过的最大 `seq`。
    pub fn current(&self) -> u64 {
        self.value.load(Ordering::Acquire)
    }

    /// 记录一条已发送（或即将发送）的 `seq`，只增不减。
    pub fn record(&self, seq: u64) {
        self.value.fetch_max(seq, Ordering::AcqRel);
    }
}

/// 进程内唯一的连接代次。
fn next_generation() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// 房间 task 入口。
#[derive(Clone)]
pub struct RoomHandle {
    /// 投递房间消息的无界通道（背压由房间 task 的串行消费隐含提供）。
    pub tx: mpsc::UnboundedSender<RoomMessage>,
    /// 6 位房间码。
    pub code: String,
    /// 建房时确定的引擎种子。
    pub seed: u64,
}

/// 进程内房间入口表。
#[derive(Default)]
pub struct RoomHub {
    rooms: BTreeMap<String, RoomHandle>,
}

impl RoomHub {
    /// 登记一个房间；同一房间 id 重复登记时覆盖。
    pub fn insert(&mut self, room_id: String, handle: RoomHandle) {
        self.rooms.insert(room_id, handle);
    }

    /// 按房间 id 取入口。
    pub fn get(&self, room_id: &str) -> Option<RoomHandle> {
        self.rooms.get(room_id).cloned()
    }

    /// 摘除房间入口；房间 task 自行回收时调用。
    pub fn remove(&mut self, room_id: &str) -> Option<RoomHandle> {
        self.rooms.remove(room_id)
    }

    /// 当前登记的房间数；供测试断言回收。
    pub fn len(&self) -> usize {
        self.rooms.len()
    }

    /// 是否没有登记任何房间。
    pub fn is_empty(&self) -> bool {
        self.rooms.is_empty()
    }
}

/// 连接 task 的控制通道消息：社交下行与匹配入房由连接自己完成。
pub enum ControlMessage {
    /// 下行消息；连接 task 编码后经自己的序号序列发送。
    Payload {
        /// 下行消息类型。
        kind: &'static str,
        /// 已序列化的载荷。
        payload: Value,
    },
    /// 匹配成功：连接下发 `MatchFound` 并挂到指定房间 task。
    EnterRoom {
        /// 匹配房 id。
        room_id: String,
        /// 服务端分配的座位号（0 起）。
        seat: u8,
    },
    /// 同一账号在别处登录：下发 `SESSION_REPLACED` 后关闭本连接。
    SessionReplaced,
}

/// 在线连接登记项。
#[derive(Clone)]
pub struct ConnectionEntry {
    /// 连接代次；旧连接退出时不得摘除新连接的登记。
    pub generation: u64,
    /// 控制通道发送端。
    pub control: mpsc::Sender<ControlMessage>,
}

/// 在线连接登记表。
#[derive(Default)]
pub struct ConnectionRegistry {
    entries: BTreeMap<String, ConnectionEntry>,
}

impl ConnectionRegistry {
    /// 登记或替换玩家的当前连接，返回被替换的旧连接。
    pub fn insert(&mut self, player_id: String, entry: ConnectionEntry) -> Option<ConnectionEntry> {
        self.entries.insert(player_id, entry)
    }

    /// 仅当代次一致时摘除连接；旧连接迟到关闭不会影响新连接。
    pub fn remove_if(&mut self, player_id: &str, generation: u64) -> bool {
        if self
            .entries
            .get(player_id)
            .is_some_and(|entry| entry.generation == generation)
        {
            self.entries.remove(player_id);
            true
        } else {
            false
        }
    }

    /// 玩家当前是否在线。
    pub fn is_online(&self, player_id: &str) -> bool {
        self.entries.contains_key(player_id)
    }

    /// 向玩家当前连接投递控制消息；不在线返回 `false`。
    pub fn try_send(&self, player_id: &str, message: ControlMessage) -> bool {
        self.entries
            .get(player_id)
            .is_some_and(|entry| entry.control.try_send(message).is_ok())
    }

    /// 当前在线连接数；供测试断言。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否没有在线连接。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// 投递给房间 task 的消息。
pub enum RoomMessage {
    /// 新建连接登记下行发送端；若对局已开始则绑定座位并回快照。
    Attach {
        player_id: String,
        out: mpsc::Sender<ServerEnvelope>,
        floor: SeqFloor,
        generation: u64,
    },
    /// 重连：新连接用已有会话回到既有房间；大厅回 `RoomUpdate`，对局回 `Snapshot`。
    Reconnect {
        player_id: String,
        last_seq: u64,
        out: mpsc::Sender<ServerEnvelope>,
        floor: SeqFloor,
        generation: u64,
    },
    /// 连接断开；只有代次与当前登记一致时才生效。
    Detach { player_id: String, generation: u64 },
    /// 顶号接管前探测座位是否仍有效：对局中要求未弃权，大厅阶段要求仍有座位。
    IsSeatActive {
        player_id: String,
        reply: oneshot::Sender<bool>,
    },
    /// 大厅离开房间；同样受代次保护。
    Leave { player_id: String, generation: u64 },
    /// 一条已映射好的房间命令。
    Command(RoomCommand),
    /// 向房间内某个座位下发非房间消息（社交下行）。
    Push {
        player_id: String,
        kind: &'static str,
        payload: Value,
    },
    /// 无需房间处理的合法上行消息（例如跳过动画）。
    Ignore,
}

/// 构造路由：`GET /ws` 升级为 WebSocket，`GET /content` 下发客户端展示内容，
/// `/healthz` 与 `/readyz` 供探针使用。
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/ws", get(ws_handler))
        .route("/content", get(crate::content::content))
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .with_state(state)
}

/// 存活检查：进程能响应即可。
async fn healthz() -> StatusCode {
    StatusCode::OK
}

/// 就绪检查：无数据库时恒就绪；有数据库时要求 `SELECT 1` 在 2 秒内成功。
async fn readyz(State(state): State<AppState>) -> StatusCode {
    let Some(db) = state.db.as_ref() else {
        return readiness(true);
    };
    let ok = tokio::time::timeout(Duration::from_secs(2), db.ping())
        .await
        .map(|result| result.is_ok())
        .unwrap_or(false);
    readiness(ok)
}

/// 数据库可用性映射为就绪状态码；独立出来便于单测覆盖 503 分支。
fn readiness(is_db_ok: bool) -> StatusCode {
    if is_db_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(state, socket))
}

/// 一个座位的当前下行连接。
struct SeatSender {
    out: mpsc::Sender<ServerEnvelope>,
    floor: SeqFloor,
    generation: u64,
}

async fn handle_socket(state: AppState, socket: WebSocket) {
    let (mut write, mut read) = socket.split();
    let (out_tx, mut out_rx) = mpsc::channel::<ServerEnvelope>(OUTBOUND_CAPACITY);

    let writer = tokio::spawn(async move {
        while let Some(envelope) = out_rx.recv().await {
            let Ok(text) = serde_json::to_string(&envelope) else {
                continue;
            };
            if write.send(Message::Text(text.into())).await.is_err() {
                break;
            }
        }
    });

    let mut direct = SeatChannel::new();
    let floor = SeqFloor::default();
    let mut session: Option<Session> = None;
    // (room_id, 房间入口, 本连接代次)
    let mut room: Option<(String, mpsc::UnboundedSender<RoomMessage>, u64)> = None;
    // 社交下行等不经过房间的消息走控制通道，保证连接 task 是直接通道的唯一写者。
    let (control_tx, mut control_rx) = mpsc::channel::<ControlMessage>(OUTBOUND_CAPACITY);
    let conn_id = next_generation();

    loop {
        tokio::select! {
            biased;
            Some(control) = control_rx.recv() => {
                match control {
                    ControlMessage::Payload { kind, payload } => {
                        send_value(&out_tx, &mut direct, &floor, kind, &payload);
                    }
                    ControlMessage::EnterRoom { room_id, seat } => {
                        let Some(authorized) = session.clone() else { continue; };
                        // 匹配成局后玩家可能已自行创建/加入房间：以连接本地状态为准。
                        if room.is_some() {
                            continue;
                        }
                        let Some(handle) = state.hub.lock().await.get(&room_id) else {
                            send_error(
                                &out_tx,
                                &mut direct,
                                &floor,
                                "ROOM_NOT_FOUND",
                                "匹配房间不存在",
                                false,
                            );
                            continue;
                        };
                        let generation = next_generation();
                        // `MatchFound` 必须先于 `Attach` 编码：房间 task 一旦开始给该座位
                        // 推送，就会与本连接共用同一个序号下限。
                        let found = MatchFound {
                            room_id: room_id.clone(),
                            seat: i64::from(seat),
                        };
                        send_payload(&out_tx, &mut direct, &floor, "MatchFound", &found);
                        let _ = handle.tx.send(RoomMessage::Attach {
                            player_id: authorized.player_id.clone(),
                            out: out_tx.clone(),
                            floor: floor.clone(),
                            generation,
                        });
                        room = Some((room_id, handle.tx, generation));
                        notify_presence(&state, &authorized.player_id).await;
                    }
                    ControlMessage::SessionReplaced => {
                        // 令牌已在签发新会话时失效；提示旧端后退出，收尾逻辑照常
                        // 刷出已入队消息并关闭写端。
                        send_error(
                            &out_tx,
                            &mut direct,
                            &floor,
                            "SESSION_REPLACED",
                            "账号已在其他设备登录",
                            false,
                        );
                        break;
                    }
                }
            }
            item = read.next() => {
                let Some(item) = item else { break; };
                let text = match item {
            Ok(Message::Text(text)) => text.as_str().to_string(),
            Ok(Message::Close(_)) | Err(_) => break,
            Ok(_) => continue,
        };

        let envelope: ClientEnvelope = match serde_json::from_str(&text) {
            Ok(envelope) => envelope,
            Err(_) => {
                route_error(
                    &room,
                    session.as_ref(),
                    &out_tx,
                    &mut direct,
                    &floor,
                    "BAD_PAYLOAD",
                    "上行信封无法解析",
                );
                break;
            }
        };
        // 生成类型不校验 `v`：连接层显式拒绝非 v1，避免按 v1 语义执行 v2 载荷。
        if envelope.v.as_u64() != Some(1) {
            route_error(
                &room,
                session.as_ref(),
                &out_tx,
                &mut direct,
                &floor,
                "BAD_PAYLOAD",
                "不支持的协议版本",
            );
            break;
        }
        let message = match envelope.parse() {
            Ok(message) => message,
            Err(error) => {
                route_error(
                    &room,
                    session.as_ref(),
                    &out_tx,
                    &mut direct,
                    &floor,
                    error.0,
                    "上行载荷与类型不匹配",
                );
                continue;
            }
        };

        match message {
            ClientMessage::AuthRequest(request) => {
                // 入房后不允许在同一连接上重新认证：否则会切换 player_id 并破坏
                // 该连接已使用的下行序号；重连应使用新连接 + token。
                if room.is_some() {
                    route_error(
                        &room,
                        session.as_ref(),
                        &out_tx,
                        &mut direct,
                        &floor,
                        "INVALID_STAGE",
                        "已在房间中，不能重新认证",
                    );
                    continue;
                }
                // 手机号分支先解析持久化账号，再把身份交给会话存储签发令牌。
                let account = match &request {
                    protocol::client::AuthRequest::Variant0 {
                        phone,
                        display_name,
                        code,
                    } => {
                        if code != crate::auth::DEV_SMS_CODE {
                            send_payload(
                                &out_tx,
                                &mut direct,
                                &floor,
                                "Error",
                                &crate::auth::invalid_code_error(),
                            );
                            continue;
                        }
                        let login = state
                            .accounts
                            .lock()
                            .await
                            .login(
                                phone,
                                display_name.as_ref().map(|name| name.as_str()),
                                state.clock.now_ms(),
                            )
                            .await;
                        match login {
                            Ok(account) => Some(account),
                            Err(error) => {
                                send_payload(&out_tx, &mut direct, &floor, "Error", &error);
                                continue;
                            }
                        }
                    }
                    protocol::client::AuthRequest::Variant1 { .. } => None,
                    protocol::client::AuthRequest::Variant2 {
                        steam_ticket,
                        display_name,
                    } => {
                        // 身份只信服务端验票结果；票据无效或未配置时不建号。
                        let steam_id = match state.steam.verify(steam_ticket.as_str()).await {
                            Ok(steam_id) => steam_id,
                            Err(error) => {
                                send_payload(&out_tx, &mut direct, &floor, "Error", &error);
                                continue;
                            }
                        };
                        let login = state
                            .accounts
                            .lock()
                            .await
                            .login_steam(
                                &steam_id,
                                display_name.as_ref().map(|name| name.as_str()),
                                state.clock.now_ms(),
                            )
                            .await;
                        match login {
                            Ok(account) => Some(account),
                            Err(error) => {
                                send_payload(&out_tx, &mut direct, &floor, "Error", &error);
                                continue;
                            }
                        }
                    }
                };
                // 令牌续登不是新登录：同一客户端重连不踢旧连接、不轮换令牌。
                let is_new_login =
                    !matches!(&request, protocol::client::AuthRequest::Variant1 { .. });
                let result = state.sessions.lock().await.authenticate_with(
                    &request,
                    state.clock.as_ref(),
                    account,
                );
                match result {
                    Ok(authorized) => {
                        state.social.lock().await.register(
                            &authorized.player_id,
                            &authorized.player_code,
                            &authorized.display_name,
                        );
                        let replaced = state.connections.lock().await.insert(
                            authorized.player_id.clone(),
                            ConnectionEntry {
                                generation: conn_id,
                                control: control_tx.clone(),
                            },
                        );
                        // 新连接不继承旧连接的排队状态：客户端重连后无法恢复排队浮层，
                        // 留着会让再次入队返回 QUEUE_ALREADY。
                        state.queue.lock().await.remove(&authorized.player_id);
                        session = Some(authorized.clone());
                        send_payload(
                            &out_tx,
                            &mut direct,
                            &floor,
                            "AuthResult",
                            &authorized.auth_result(),
                        );
                        // 账号视图紧随认证结果；旧客户端按类型忽略。
                        if let Some(view) =
                            account_view_with_rank(&state, &authorized.player_id).await
                        {
                            send_payload(&out_tx, &mut direct, &floor, "AccountView", &view);
                        }
                        // 成就状态：账号级已解锁记录，按成就 id 升序；缺少 Steam
                        // 映射的历史记录不下发。非 Steam 客户端按类型忽略。
                        let unlocked = state
                            .accounts
                            .lock()
                            .await
                            .unlocked_achievements(&authorized.player_id)
                            .into_iter()
                            .filter_map(|achievement_id| {
                                let steam_api_name =
                                    state.achievements.steam_api_name(&achievement_id)?;
                                Some(AchievementStateUnlockedItem {
                                    achievement_id,
                                    steam_api_name: steam_api_name.to_string(),
                                })
                            })
                            .collect();
                        send_payload(
                            &out_tx,
                            &mut direct,
                            &floor,
                            "AchievementState",
                            &AchievementState { unlocked },
                        );
                        // 新登录先把连接挂回账号当前房间，再踢旧连接：接管消息与
                        // 旧连接的 `Detach` 都进同一房间队列，代次先更新，迟到的
                        // `Detach` 会被忽略。
                        if is_new_login
                            && let Some(taken_over) =
                                takeover_room(&state, &authorized.player_id, &out_tx, &floor).await
                        {
                            room = Some(taken_over);
                        }
                        // 同一连接内重新认证不踢自己；旧连接已无法消费控制消息时
                        // 令牌已失效，由心跳超时关闭，不回滚新登录。
                        if is_new_login
                            && let Some(previous) = replaced
                            && previous.generation != conn_id
                        {
                            let _ = previous
                                .control
                                .try_send(ControlMessage::SessionReplaced);
                        }
                        // 本人与好友都需要看到最新的在线状态与社交视图。
                        notify_presence(&state, &authorized.player_id).await;
                    }
                    Err(error) => send_payload(&out_tx, &mut direct, &floor, "Error", &error),
                }
            }
            ClientMessage::RequestAccount(_) => {
                let Some(authorized) = session.clone() else {
                    send_error(&out_tx, &mut direct, &floor, "AUTH_FAILED", "请先认证", false);
                    continue;
                };
                match account_view_with_rank(&state, &authorized.player_id).await {
                    Some(view) => send_payload(&out_tx, &mut direct, &floor, "AccountView", &view),
                    None => send_error(
                        &out_tx,
                        &mut direct,
                        &floor,
                        "INTERNAL_ERROR",
                        "账号不存在",
                        true,
                    ),
                }
            }
            ClientMessage::RequestPhoneCode(request) => {
                let Some(authorized) = session.clone() else {
                    send_error(&out_tx, &mut direct, &floor, "AUTH_FAILED", "请先认证", false);
                    continue;
                };
                if !crate::phone_code::is_valid_phone(&request.phone) {
                    send_error(
                        &out_tx,
                        &mut direct,
                        &floor,
                        "PHONE_INVALID",
                        "手机号格式不正确",
                        false,
                    );
                    continue;
                }
                let issued = state.phone_codes.lock().await.issue(
                    &authorized.player_id,
                    &request.phone,
                    state.clock.now_ms(),
                );
                match issued {
                    Ok(expires_at_ms) => {
                        let sent = PhoneCodeSent {
                            expires_at: crate::db::ms_to_utc(expires_at_ms),
                        };
                        send_payload(&out_tx, &mut direct, &floor, "PhoneCodeSent", &sent);
                    }
                    Err(crate::phone_code::IssueError::Cooldown) => send_error(
                        &out_tx,
                        &mut direct,
                        &floor,
                        "RATE_LIMITED",
                        "验证码发送过于频繁",
                        true,
                    ),
                }
            }
            ClientMessage::BindPhone(request) => {
                let Some(authorized) = session.clone() else {
                    send_error(&out_tx, &mut direct, &floor, "AUTH_FAILED", "请先认证", false);
                    continue;
                };
                if !crate::phone_code::is_valid_phone(&request.phone) {
                    send_error(
                        &out_tx,
                        &mut direct,
                        &floor,
                        "PHONE_INVALID",
                        "手机号格式不正确",
                        false,
                    );
                    continue;
                }
                let now = state.clock.now_ms();
                let checked = state.phone_codes.lock().await.consume(
                    &authorized.player_id,
                    &request.phone,
                    &request.code,
                    now,
                );
                match checked {
                    Ok(()) => {}
                    Err(crate::phone_code::CheckError::Expired) => {
                        send_error(
                            &out_tx,
                            &mut direct,
                            &floor,
                            "CODE_EXPIRED",
                            "验证码已过期",
                            false,
                        );
                        continue;
                    }
                    Err(crate::phone_code::CheckError::Invalid) => {
                        send_error(
                            &out_tx,
                            &mut direct,
                            &floor,
                            "CODE_INVALID",
                            "验证码错误",
                            false,
                        );
                        continue;
                    }
                }
                let bound = state
                    .accounts
                    .lock()
                    .await
                    .bind_phone(&authorized.player_id, &request.phone, now)
                    .await;
                match bound {
                    Ok(mut view) => {
                        apply_rank_to_account_view(&state, &authorized.player_id, &mut view).await;
                        send_payload(&out_tx, &mut direct, &floor, "AccountView", &view);
                    }
                    Err(error) => send_payload(&out_tx, &mut direct, &floor, "Error", &error),
                }
            }
            ClientMessage::CreateRoom(_) => {
                let Some(authorized) = session.clone() else {
                    send_error(
                        &out_tx,
                        &mut direct,
                        &floor,
                        "AUTH_FAILED",
                        "请先认证",
                        false,
                    );
                    continue;
                };
                if room.is_some() {
                    route_error(
                        &room,
                        session.as_ref(),
                        &out_tx,
                        &mut direct,
                        &floor,
                        "INVALID_STAGE",
                        "已在房间中",
                    );
                    continue;
                }
                // 进房即退出匹配队列；先取队列锁再取房间锁，与匹配驱动顺序一致。
                let in_room = {
                    let mut queue = state.queue.lock().await;
                    queue.remove(&authorized.player_id);
                    state
                        .rooms
                        .lock()
                        .await
                        .room_of_player(&authorized.player_id)
                        .is_some()
                };
                if in_room {
                    send_error(
                        &out_tx,
                        &mut direct,
                        &floor,
                        "ALREADY_IN_ROOM",
                        "已在房间中",
                        false,
                    );
                    continue;
                }
                let created = state.rooms.lock().await.create(&authorized, &state.catalog);
                let seed = seed_for_code(&created.code);
                let (tx, rx) = mpsc::unbounded_channel();
                state.hub.lock().await.insert(
                    created.id.clone(),
                    RoomHandle {
                        tx: tx.clone(),
                        code: created.code.clone(),
                        seed,
                    },
                );
                spawn_room_task(state.clone(), created.id.clone(), seed, rx);
                let generation = next_generation();
                let _ = tx.send(RoomMessage::Attach {
                    player_id: authorized.player_id,
                    out: out_tx.clone(),
                    floor: floor.clone(),
                    generation,
                });
                room = Some((created.id, tx, generation));
            }
            ClientMessage::JoinRoom(request) => {
                let Some(authorized) = session.clone() else {
                    send_error(
                        &out_tx,
                        &mut direct,
                        &floor,
                        "AUTH_FAILED",
                        "请先认证",
                        false,
                    );
                    continue;
                };
                if room.is_some() {
                    route_error(
                        &room,
                        session.as_ref(),
                        &out_tx,
                        &mut direct,
                        &floor,
                        "ALREADY_IN_ROOM",
                        "已在房间中",
                    );
                    continue;
                }
                state.queue.lock().await.remove(&authorized.player_id);
                let joined = {
                    let mut rooms = state.rooms.lock().await;
                    rooms
                        .join(&request.code, &authorized, &state.catalog)
                        .map(|joined_room| joined_room.id.clone())
                };
                match joined {
                    Ok(room_id) => {
                        match attach_joined_room(&state, room_id, &authorized, &out_tx, &floor)
                            .await
                        {
                            Ok(attached) => room = Some(attached),
                            Err((code, text)) => {
                                send_error(&out_tx, &mut direct, &floor, code, text, false);
                            }
                        }
                    }
                    Err(error) => {
                        let code = error.code().to_string();
                        send_error(
                            &out_tx,
                            &mut direct,
                            &floor,
                            &code,
                            &error.to_string(),
                            false,
                        );
                    }
                }
            }
            ClientMessage::CreateRoomInvite(_) => {
                let Some(authorized) = session.clone() else {
                    send_error(&out_tx, &mut direct, &floor, "AUTH_FAILED", "请先认证", false);
                    continue;
                };
                let created = state
                    .rooms
                    .lock()
                    .await
                    .create_invite(&authorized.player_id, state.clock.now_ms());
                match created {
                    Ok(invite) => {
                        let payload = RoomInviteCode {
                            token: invite.token,
                            room_code: invite.room_code,
                            expires_at: crate::db::ms_to_utc(invite.expires_at_ms),
                        };
                        send_payload(&out_tx, &mut direct, &floor, "RoomInviteCode", &payload);
                    }
                    Err(error) => send_error(
                        &out_tx,
                        &mut direct,
                        &floor,
                        &error.code().to_string(),
                        &error.to_string(),
                        false,
                    ),
                }
            }
            ClientMessage::RedeemRoomInvite(request) => {
                let Some(authorized) = session.clone() else {
                    send_error(&out_tx, &mut direct, &floor, "AUTH_FAILED", "请先认证", false);
                    continue;
                };
                if room.is_some() {
                    route_error(
                        &room,
                        session.as_ref(),
                        &out_tx,
                        &mut direct,
                        &floor,
                        "ALREADY_IN_ROOM",
                        "已在房间中",
                    );
                    continue;
                }
                state.queue.lock().await.remove(&authorized.player_id);
                let joined = {
                    let mut rooms = state.rooms.lock().await;
                    rooms
                        .redeem_invite(
                            &request.token,
                            &authorized,
                            &state.catalog,
                            state.clock.now_ms(),
                        )
                        .map(|joined_room| joined_room.id.clone())
                };
                match joined {
                    Ok(room_id) => {
                        match attach_joined_room(&state, room_id, &authorized, &out_tx, &floor)
                            .await
                        {
                            Ok(attached) => room = Some(attached),
                            Err((code, text)) => {
                                send_error(&out_tx, &mut direct, &floor, code, text, false);
                            }
                        }
                    }
                    Err(error) => send_error(
                        &out_tx,
                        &mut direct,
                        &floor,
                        &error.code().to_string(),
                        &error.to_string(),
                        false,
                    ),
                }
            }
            ClientMessage::JoinQueue(_) => {
                let Some(authorized) = session.clone() else {
                    send_error(&out_tx, &mut direct, &floor, "AUTH_FAILED", "请先认证", false);
                    continue;
                };
                if room.is_some() {
                    send_error(
                        &out_tx,
                        &mut direct,
                        &floor,
                        "QUEUE_IN_ROOM",
                        "已在房间中，不能匹配",
                        false,
                    );
                    continue;
                }
                let now = state.clock.now_ms();
                // 匹配按账号实际积分；未结算账号为 1000。
                let rating = state.ratings.lock().await.rating_of(&authorized.player_id);
                // 锁顺序固定为 队列 → 房间，与匹配驱动建房保持一致。
                let (joined, driver, snapshot) = {
                    let mut queue = state.queue.lock().await;
                    let in_room = {
                        let rooms = state.rooms.lock().await;
                        rooms.room_of_player(&authorized.player_id).is_some()
                    };
                    if in_room {
                        (Err(crate::queue::QueueError::InRoom), false, None)
                    } else {
                        let result = queue.join(authorized.clone(), rating, now);
                        let driver = result.is_ok() && queue.begin_driver();
                        let snapshot = queue.state(&authorized.player_id, now);
                        (result, driver, snapshot)
                    }
                };
                match joined {
                    Ok(()) => {
                        if let Some(state_data) = snapshot {
                            send_queue_state(&out_tx, &mut direct, &floor, state_data);
                        }
                        if driver {
                            spawn_queue_driver(state.clone());
                        }
                    }
                    Err(error) => send_error(
                        &out_tx,
                        &mut direct,
                        &floor,
                        &error.code().to_string(),
                        &error.to_string(),
                        false,
                    ),
                }
            }
            ClientMessage::LeaveQueue(_) => {
                let Some(authorized) = session.clone() else {
                    send_error(&out_tx, &mut direct, &floor, "AUTH_FAILED", "请先认证", false);
                    continue;
                };
                // 取消排队幂等；客户端自行收起排队浮层，不需要服务端回执。
                state.queue.lock().await.remove(&authorized.player_id);
            }
            ClientMessage::RequestLeaderboard(_) => {
                if session.is_none() {
                    send_error(&out_tx, &mut direct, &floor, "AUTH_FAILED", "请先认证", false);
                    continue;
                }
                let ids = account_ids(&state).await;
                let now = state.clock.now_ms();
                // 先持积分锁完成快照与排序，释放后再取账号锁，避免反向锁序。
                let rows = {
                    let mut ratings = state.ratings.lock().await;
                    if let Err(error) = ratings
                        .ensure_master_snapshot(&state.ranked, now, &ids)
                        .await
                    {
                        eprintln!("room-service: 宗师快照失败: {error}");
                    }
                    match ratings.leaderboard(&ids, crate::rating::LEADERBOARD_SIZE).await {
                        Ok(rows) => rows
                            .iter()
                            .map(|row| {
                                (
                                    row.player_id.clone(),
                                    row.rating,
                                    ratings.tier(&state.ranked, &row.player_id, row.rating),
                                )
                            })
                            .collect::<Vec<_>>(),
                        Err(error) => {
                            eprintln!("room-service: 查询排行榜失败: {error}");
                            Vec::new()
                        }
                    }
                };
                let accounts = state.accounts.lock().await;
                let entries: Vec<LeaderboardEntriesItem> = rows
                    .into_iter()
                    .enumerate()
                    .filter_map(|(index, (player_id, rating, tier))| {
                        let display_name = accounts.display_name(&player_id)?.to_string();
                        Some(LeaderboardEntriesItem {
                            rank: std::num::NonZeroU64::new(
                                u64::try_from(index + 1).ok()?,
                            )?,
                            player_id,
                            display_name,
                            tier,
                            rating: u64::try_from(rating).unwrap_or(0),
                        })
                    })
                    .collect();
                send_payload(
                    &out_tx,
                    &mut direct,
                    &floor,
                    "Leaderboard",
                    &Leaderboard { entries },
                );
            }
            ClientMessage::RequestProfile(request) => {
                let Some(authorized) = session.clone() else {
                    send_error(&out_tx, &mut direct, &floor, "AUTH_FAILED", "请先认证", false);
                    continue;
                };
                let target = request
                    .player_id
                    .clone()
                    .unwrap_or_else(|| authorized.player_id.clone());
                let display_name = {
                    state
                        .accounts
                        .lock()
                        .await
                        .display_name(&target)
                        .map(str::to_string)
                };
                let Some(display_name) = display_name else {
                    send_error(&out_tx, &mut direct, &floor, "FRIEND_NOT_FOUND", "玩家不存在", false);
                    continue;
                };
                let is_friend = {
                    let social = state.social.lock().await;
                    social.is_friend(&authorized.player_id, &target)
                };
                let ids = account_ids(&state).await;
                let now = state.clock.now_ms();
                let visible =
                    crate::rating::recent_visible(&authorized.player_id, &target, is_friend);
                let (rating, tier, highest_tier, recent) = {
                    let mut ratings = state.ratings.lock().await;
                    if let Err(error) = ratings
                        .ensure_master_snapshot(&state.ranked, now, &ids)
                        .await
                    {
                        eprintln!("room-service: 宗师快照失败: {error}");
                    }
                    let summary = ratings.summary(&state.ranked, &target);
                    let recent = if visible {
                        match ratings.recent_for(&target).await {
                            Ok(recent) => recent,
                            Err(error) => {
                                eprintln!("room-service: 查询最近战绩失败: {error}");
                                Vec::new()
                            }
                        }
                    } else {
                        Vec::new()
                    };
                    (
                        summary.rating,
                        summary.tier,
                        summary.highest_tier,
                        recent,
                    )
                };
                let recent_items: Vec<PlayerProfileRecentItem> = recent
                    .iter()
                    .filter_map(|entry| {
                        Some(PlayerProfileRecentItem {
                            placement: std::num::NonZeroU64::new(u64::from(entry.placement))?,
                            rating_delta: i64::from(entry.rating_delta),
                            rating_after: u64::try_from(entry.rating_after).unwrap_or(0),
                            finished_at: crate::db::ms_to_utc(entry.finished_at_ms),
                        })
                    })
                    .collect();
                let top4_ratio = if visible {
                    crate::rating::top4_ratio(&recent)
                } else {
                    0.0
                };
                send_payload(
                    &out_tx,
                    &mut direct,
                    &floor,
                    "PlayerProfile",
                    &PlayerProfile {
                        player_id: target,
                        display_name,
                        rating: u64::try_from(rating).unwrap_or(0),
                        tier,
                        highest_tier,
                        recent_visible: visible,
                        top4_ratio,
                        recent: recent_items,
                    },
                );
            }
            ClientMessage::Reconnect(request) => {
                // 真实重连路径：新 WS 连接 → AuthRequest(token) → Reconnect。
                // 此时连接本地还没有房间，必须用 `roomId` 从 hub 找回房间 task。
                let Some(authorized) = session.clone() else {
                    send_error(
                        &out_tx,
                        &mut direct,
                        &floor,
                        "AUTH_FAILED",
                        "请先认证",
                        false,
                    );
                    continue;
                };
                if room.is_some() {
                    route_error(
                        &room,
                        session.as_ref(),
                        &out_tx,
                        &mut direct,
                        &floor,
                        "INVALID_STAGE",
                        "当前连接已在房间中",
                    );
                    continue;
                }
                let Some(handle) = state.hub.lock().await.get(&request.room_id) else {
                    send_error(
                        &out_tx,
                        &mut direct,
                        &floor,
                        "ROOM_NOT_FOUND",
                        "房间不存在",
                        false,
                    );
                    continue;
                };
                let is_member = {
                    let rooms = state.rooms.lock().await;
                    rooms
                        .room(&request.room_id)
                        .and_then(|room| room.seat_of(&authorized.player_id))
                        .is_some()
                };
                if !is_member {
                    send_error(
                        &out_tx,
                        &mut direct,
                        &floor,
                        "NOT_IN_ROOM",
                        "会话不属于该房间",
                        false,
                    );
                    continue;
                }
                let generation = next_generation();
                let player_id = authorized.player_id;
                let _ = handle.tx.send(RoomMessage::Reconnect {
                    player_id: player_id.clone(),
                    last_seq: request.last_seq,
                    out: out_tx.clone(),
                    floor: floor.clone(),
                    generation,
                });
                room = Some((request.room_id, handle.tx, generation));
                // 认证时的社交视图可能落在旧连接上；重连登记完成后补发一份，
                // 走房间通道保证与快照共用同一条序号序列。
                notify_presence(&state, &player_id).await;
            }
            ClientMessage::LeaveRoom(_) => {
                let Some(authorized) = session.clone() else {
                    send_error(
                        &out_tx,
                        &mut direct,
                        &floor,
                        "AUTH_FAILED",
                        "请先认证",
                        false,
                    );
                    continue;
                };
                let Some((_, tx, generation)) = room.take() else {
                    send_error(
                        &out_tx,
                        &mut direct,
                        &floor,
                        "NOT_IN_ROOM",
                        "尚未加入房间",
                        false,
                    );
                    continue;
                };
                let _ = tx.send(RoomMessage::Leave {
                    player_id: authorized.player_id,
                    generation,
                });
            }
            ClientMessage::FriendRequest(request) => {
                let Some(authorized) = session.clone() else {
                    send_error(&out_tx, &mut direct, &floor, "AUTH_FAILED", "请先认证", false);
                    continue;
                };
                let now = state.clock.now_ms();
                let outcome = {
                    let mut social = state.social.lock().await;
                    social
                        .apply(&authorized.player_id, &request.to_player_code, now)
                        .await
                        .map(|()| {
                            social
                                .profile_by_code(&request.to_player_code)
                                .map(|(player_id, _)| player_id.to_string())
                        })
                };
                match outcome {
                    Ok(target) => {
                        let mut targets = vec![authorized.player_id];
                        if let Some(target) = target {
                            targets.push(target);
                        }
                        notify_social(&state, &targets).await;
                    }
                    Err(error) => route_social_error(
                        &room,
                        session.as_ref(),
                        &out_tx,
                        &mut direct,
                        &floor,
                        &error,
                    ),
                }
            }
            ClientMessage::FriendRespond(request) => {
                let Some(authorized) = session.clone() else {
                    send_error(&out_tx, &mut direct, &floor, "AUTH_FAILED", "请先认证", false);
                    continue;
                };
                let now = state.clock.now_ms();
                let result = state.social.lock().await.respond(
                    &authorized.player_id,
                    &request.from_player_id,
                    request.accept,
                    now,
                )
                .await;
                match result {
                    Ok(()) => {
                        notify_social(
                            &state,
                            &[authorized.player_id, request.from_player_id],
                        )
                        .await;
                    }
                    Err(error) => route_social_error(
                        &room,
                        session.as_ref(),
                        &out_tx,
                        &mut direct,
                        &floor,
                        &error,
                    ),
                }
            }
            ClientMessage::FriendRemove(request) => {
                let Some(authorized) = session.clone() else {
                    send_error(&out_tx, &mut direct, &floor, "AUTH_FAILED", "请先认证", false);
                    continue;
                };
                let result = state
                    .social
                    .lock()
                    .await
                    .remove(&authorized.player_id, &request.friend_id)
                    .await;
                match result {
                    Ok(()) => {
                        notify_social(
                            &state,
                            &[authorized.player_id, request.friend_id],
                        )
                        .await;
                    }
                    Err(error) => route_social_error(
                        &room,
                        session.as_ref(),
                        &out_tx,
                        &mut direct,
                        &floor,
                        &error,
                    ),
                }
            }
            ClientMessage::InviteToRoom(request) => {
                let Some(authorized) = session.clone() else {
                    send_error(&out_tx, &mut direct, &floor, "AUTH_FAILED", "请先认证", false);
                    continue;
                };
                let now = state.clock.now_ms();
                let room_info = {
                    let rooms = state.rooms.lock().await;
                    rooms.room_of_player(&authorized.player_id).map(|(room, _)| {
                        (
                            room.id.clone(),
                            room.code.clone(),
                            room.phase,
                            room.slots().len(),
                        )
                    })
                };
                let Some((room_id, room_code, phase, occupied)) = room_info else {
                    send_error(
                        &out_tx,
                        &mut direct,
                        &floor,
                        "NOT_IN_ROOM",
                        "请先加入房间",
                        false,
                    );
                    continue;
                };
                if phase != Phase::Lobby {
                    send_error(
                        &out_tx,
                        &mut direct,
                        &floor,
                        "INVALID_STAGE",
                        "只有大厅阶段可以邀请好友",
                        false,
                    );
                    continue;
                }
                if occupied >= SEAT_COUNT {
                    send_error(&out_tx, &mut direct, &floor, "ROOM_FULL", "房间已满", false);
                    continue;
                }
                let (online, busy) = {
                    let connections = state.connections.lock().await;
                    let rooms = state.rooms.lock().await;
                    (
                        connections.is_online(&request.friend_id),
                        rooms.room_of_player(&request.friend_id).is_some(),
                    )
                };
                if !online {
                    send_error(
                        &out_tx,
                        &mut direct,
                        &floor,
                        "FRIEND_OFFLINE",
                        "对方不在线",
                        false,
                    );
                    continue;
                }
                if busy {
                    send_error(
                        &out_tx,
                        &mut direct,
                        &floor,
                        "FRIEND_BUSY",
                        "对方已在房间中",
                        false,
                    );
                    continue;
                }
                let invite = state
                    .social
                    .lock()
                    .await
                    .create_invite(
                        &authorized.player_id,
                        &request.friend_id,
                        &room_id,
                        &room_code,
                        now,
                    )
                    .await;
                match invite {
                    Ok(invite) => {
                        let message = FriendInviteMessage {
                            expires_at: utc(invite.expires_at_ms),
                            from_display_name: invite.from_display_name,
                            from_player_code: invite.from_player_code,
                            from_player_id: invite.from_player_id,
                            invite_id: invite.id,
                            room_code: invite.room_code,
                        };
                        send_protocol(&state, &request.friend_id, "FriendInvite", &message).await;
                    }
                    Err(error) => route_social_error(
                        &room,
                        session.as_ref(),
                        &out_tx,
                        &mut direct,
                        &floor,
                        &error,
                    ),
                }
            }
            ClientMessage::RespondInvite(request) => {
                let Some(authorized) = session.clone() else {
                    send_error(&out_tx, &mut direct, &floor, "AUTH_FAILED", "请先认证", false);
                    continue;
                };
                let now = state.clock.now_ms();
                let outcome = state.social.lock().await.respond_invite(
                    &authorized.player_id,
                    &request.invite_id,
                    request.accept,
                    now,
                )
                .await;
                match outcome {
                    Ok(InviteOutcome::Declined(invite)) => {
                        let result = FriendInviteResult {
                            invite_id: invite.id,
                            accepted: false,
                            reason: None,
                        };
                        // 双方都收一份回执：邀请方知道结果，被邀请方据此收起弹窗。
                        send_protocol(
                            &state,
                            &authorized.player_id,
                            "FriendInviteResult",
                            &result,
                        )
                        .await;
                        send_protocol(
                            &state,
                            &invite.from_player_id,
                            "FriendInviteResult",
                            &result,
                        )
                        .await;
                    }
                    Ok(InviteOutcome::Accepted(invite)) => {
                        // 接受邀请意味着即将进房：立刻退出匹配队列。
                        state.queue.lock().await.remove(&authorized.player_id);
                        let joinable = {
                            let rooms = state.rooms.lock().await;
                            match rooms.room(&invite.room_id) {
                                Some(room)
                                    if room.phase == Phase::Lobby
                                        && room.slots().len() < SEAT_COUNT =>
                                {
                                    Ok(())
                                }
                                Some(room) if room.phase != Phase::Lobby => {
                                    Err(("INVALID_STAGE", "房间已开始对局"))
                                }
                                Some(_) => Err(("ROOM_FULL", "房间已满")),
                                None => Err(("ROOM_NOT_FOUND", "房间不存在")),
                            }
                        };
                        match joinable {
                            Ok(()) => {
                                let accepted = InviteAccepted {
                                    invite_id: invite.id.clone(),
                                    room_code: invite.room_code.clone(),
                                };
                                send_protocol(
                                    &state,
                                    &authorized.player_id,
                                    "InviteAccepted",
                                    &accepted,
                                )
                                .await;
                                let result = FriendInviteResult {
                                    invite_id: invite.id,
                                    accepted: true,
                                    reason: None,
                                };
                                send_protocol(
                                    &state,
                                    &invite.from_player_id,
                                    "FriendInviteResult",
                                    &result,
                                )
                                .await;
                            }
                            Err((code, text)) => {
                                send_error(&out_tx, &mut direct, &floor, code, text, false);
                                let result = FriendInviteResult {
                                    invite_id: invite.id,
                                    accepted: false,
                                    reason: Some(code.to_string()),
                                };
                                send_protocol(
                                    &state,
                                    &invite.from_player_id,
                                    "FriendInviteResult",
                                    &result,
                                )
                                .await;
                            }
                        }
                    }
                    Err(error) => route_social_error(
                        &room,
                        session.as_ref(),
                        &out_tx,
                        &mut direct,
                        &floor,
                        &error,
                    ),
                }
            }
            other => {
                let Some(authorized) = session.clone() else {
                    send_error(
                        &out_tx,
                        &mut direct,
                        &floor,
                        "AUTH_FAILED",
                        "请先认证",
                        false,
                    );
                    continue;
                };
                let Some((_, tx, _)) = room.clone() else {
                    send_error(
                        &out_tx,
                        &mut direct,
                        &floor,
                        "NOT_IN_ROOM",
                        "尚未加入房间",
                        false,
                    );
                    continue;
                };
                let message = match map_room_message(other, &authorized.player_id, &out_tx) {
                    Ok(message) => message,
                    Err((code, text)) => {
                        route_error(
                            &room,
                            session.as_ref(),
                            &out_tx,
                            &mut direct,
                            &floor,
                            code,
                            text,
                        );
                        continue;
                    }
                };
                if tx.send(message).is_err() {
                    // 房间 task 已停止，连接无从继续。
                    break;
                }
            }
        }
            }
        }
    }

    // 注销在线登记：仅当代次一致时摘除，避免旧连接误删新连接。
    if let Some(authorized) = session.as_ref()
        && state
            .connections
            .lock()
            .await
            .remove_if(&authorized.player_id, conn_id)
    {
        // 断线即退出匹配队列；驱动下一轮也会按在线状态兜底清理。
        state.queue.lock().await.remove(&authorized.player_id);
        notify_presence(&state, &authorized.player_id).await;
    }

    if let Some((_, tx, generation)) = room {
        let _ = tx.send(RoomMessage::Detach {
            player_id: session
                .map(|authorized| authorized.player_id)
                .unwrap_or_default(),
            generation,
        });
    }
    drop(out_tx);
    // 无论是否入房，都给写端一点时间把已入队的消息（尤其 Error）刷出去；
    // 房间 task 或 `GameTask` 可能仍持有发送端，写端不会自然结束。
    let _ = tokio::time::timeout(FLUSH_TIMEOUT, writer).await;
}

/// 新登录接管账号当前房间：座位仍有效时按重连路径绑定新连接。
///
/// 返回连接后续使用的房间上下文；账号不在房间、座位已弃权或探测超时都返回 `None`，
/// 新客户端按正常流程进大厅。
async fn takeover_room(
    state: &AppState,
    player_id: &str,
    out_tx: &mpsc::Sender<ServerEnvelope>,
    floor: &SeqFloor,
) -> Option<(String, mpsc::UnboundedSender<RoomMessage>, u64)> {
    let room_id = {
        let rooms = state.rooms.lock().await;
        rooms
            .room_of_player(player_id)
            .map(|(room, _)| room.id.clone())
    }?;
    let handle = state.hub.lock().await.get(&room_id)?;
    let (reply, active) = oneshot::channel();
    handle
        .tx
        .send(RoomMessage::IsSeatActive {
            player_id: player_id.to_string(),
            reply,
        })
        .ok()?;
    if !matches!(
        time::timeout(SEAT_PROBE_TIMEOUT, active).await,
        Ok(Ok(true))
    ) {
        return None;
    }
    let generation = next_generation();
    let _ = handle.tx.send(RoomMessage::Reconnect {
        player_id: player_id.to_string(),
        last_seq: 0,
        out: out_tx.clone(),
        floor: floor.clone(),
        generation,
    });
    Some((room_id, handle.tx, generation))
}

/// 把已加入注册表的玩家接到房间 task；失败时回滚刚占的座位。
async fn attach_joined_room(
    state: &AppState,
    room_id: String,
    authorized: &Session,
    out_tx: &mpsc::Sender<ServerEnvelope>,
    floor: &SeqFloor,
) -> Result<(String, mpsc::UnboundedSender<RoomMessage>, u64), (&'static str, &'static str)> {
    let Some(handle) = state.hub.lock().await.get(&room_id) else {
        // 先改注册表后查 hub：失败必须回滚刚加入的座位。
        let _ = state
            .rooms
            .lock()
            .await
            .leave(&room_id, &authorized.player_id);
        return Err(("ROOM_NOT_FOUND", "房间任务不存在"));
    };
    let generation = next_generation();
    let _ = handle.tx.send(RoomMessage::Attach {
        player_id: authorized.player_id.clone(),
        out: out_tx.clone(),
        floor: floor.clone(),
        generation,
    });
    Ok((room_id, handle.tx, generation))
}

/// 把非大厅入口的上行消息映射成房间消息。
fn map_room_message(
    message: ClientMessage,
    player_id: &str,
    out: &mpsc::Sender<ServerEnvelope>,
) -> Result<RoomMessage, (&'static str, &'static str)> {
    let player_id = player_id.to_string();
    match message {
        ClientMessage::Ready(request) => Ok(RoomMessage::Command(RoomCommand::Ready {
            player_id,
            ready: request.ready,
        })),
        ClientMessage::StartGame(_) => {
            Ok(RoomMessage::Command(RoomCommand::StartGame { player_id }))
        }
        ClientMessage::SetBotFill(request) => Ok(RoomMessage::Command(RoomCommand::SetBotFill {
            player_id,
            enabled: request.enabled,
        })),
        ClientMessage::SelectHeroCard(request) => {
            Ok(RoomMessage::Command(RoomCommand::SelectHero {
                player_id,
                card_id: request.card_id,
            }))
        }
        ClientMessage::Intent(request) => Ok(RoomMessage::Command(RoomCommand::Intent {
            player_id,
            client_intent_id: request.client_intent_id.to_string(),
            intent: request.intent,
        })),
        ClientMessage::CombatPlayed(_) => Ok(RoomMessage::Command(RoomCommand::CombatPlayed {
            player_id,
        })),
        ClientMessage::RequestSnapshot(_) => {
            Ok(RoomMessage::Command(RoomCommand::RequestSnapshot {
                player_id,
            }))
        }
        ClientMessage::Reconnect(_) => {
            let _ = out;
            Err(("INTERNAL_ERROR", "重连入口不应进入房间命令路由"))
        }
        ClientMessage::KickMember(request) => Ok(RoomMessage::Command(RoomCommand::Kick {
            player_id,
            target_player_id: request.target_player_id,
        })),
        ClientMessage::DissolveRoom(_) => {
            Ok(RoomMessage::Command(RoomCommand::Dissolve { player_id }))
        }
        ClientMessage::LeaveRoom(_)
        | ClientMessage::AuthRequest(_)
        | ClientMessage::CreateRoom(_)
        | ClientMessage::JoinRoom(_)
        | ClientMessage::JoinQueue(_)
        | ClientMessage::LeaveQueue(_)
        | ClientMessage::RequestAccount(_)
        | ClientMessage::RequestPhoneCode(_)
        | ClientMessage::BindPhone(_)
        | ClientMessage::CreateRoomInvite(_)
        | ClientMessage::RedeemRoomInvite(_)
        | ClientMessage::RequestLeaderboard(_)
        | ClientMessage::RequestProfile(_) => {
            Err(("INTERNAL_ERROR", "大厅入口消息不应进入房间路由"))
        }
        ClientMessage::FriendRequest(_)
        | ClientMessage::FriendRespond(_)
        | ClientMessage::FriendRemove(_)
        | ClientMessage::InviteToRoom(_)
        | ClientMessage::RespondInvite(_) => Err(("INTERNAL_ERROR", "社交消息不应进入房间路由")),
    }
}

fn send_payload<T: Serialize>(
    out: &mpsc::Sender<ServerEnvelope>,
    channel: &mut SeatChannel,
    floor: &SeqFloor,
    kind: &str,
    payload: &T,
) {
    let Ok(value) = serde_json::to_value(payload) else {
        return;
    };
    send_value(out, channel, floor, kind, &value);
}

fn send_value(
    out: &mpsc::Sender<ServerEnvelope>,
    channel: &mut SeatChannel,
    floor: &SeqFloor,
    kind: &str,
    payload: &Value,
) {
    let Some(map) = payload.as_object().cloned() else {
        return;
    };
    channel.set_next_seq(floor.current());
    if let Ok(envelope) = channel.bind(kind, map) {
        floor.record(envelope.seq.get());
        let _ = out.try_send(envelope);
    }
}

fn send_error(
    out: &mpsc::Sender<ServerEnvelope>,
    channel: &mut SeatChannel,
    floor: &SeqFloor,
    code: &str,
    message: &str,
    retryable: bool,
) {
    send_payload(
        out,
        channel,
        floor,
        "Error",
        &error_message(code, message, retryable),
    );
}

/// 下发一条连接层错误。
///
/// 入房之后一律经房间 task 走该座位的统一下行通道（大厅用 `slot.channel`、对局用
/// `GameTask` 通道），避免连接层独立计数器产生倒退的 `seq`。
/// 只有入房之前才走连接自己的 `direct` 通道。
fn route_error(
    room: &Option<(String, mpsc::UnboundedSender<RoomMessage>, u64)>,
    session: Option<&Session>,
    out: &mpsc::Sender<ServerEnvelope>,
    direct: &mut SeatChannel,
    floor: &SeqFloor,
    code: &str,
    message: &str,
) {
    if let Some((_, tx, _)) = room
        && let Some(authorized) = session
    {
        let _ = tx.send(RoomMessage::Command(RoomCommand::Error {
            player_id: authorized.player_id.clone(),
            code: code.to_string(),
            message: message.to_string(),
        }));
        return;
    }
    send_error(out, direct, floor, code, message, false);
}

/// 把社交错误按当前连接状态下发：入房后走房间 task，入房前走直接通道。
fn route_social_error(
    room: &Option<(String, mpsc::UnboundedSender<RoomMessage>, u64)>,
    session: Option<&Session>,
    out: &mpsc::Sender<ServerEnvelope>,
    direct: &mut SeatChannel,
    floor: &SeqFloor,
    error: &SocialError,
) {
    let code = error.code().to_string();
    route_error(room, session, out, direct, floor, &code, &error.to_string());
}

/// 按连接与房间状态计算好友可见状态；调用方需已持有对应的锁。
fn presence_status(
    connections: &ConnectionRegistry,
    rooms: &crate::room::RoomRegistry,
    player_id: &str,
) -> FriendStatus {
    if let Some((room, _)) = rooms.room_of_player(player_id)
        && matches!(room.phase, Phase::Recruit | Phase::Combat)
    {
        return FriendStatus::InGame;
    }
    if connections.is_online(player_id) {
        FriendStatus::Online
    } else {
        FriendStatus::Offline
    }
}

fn utc(ms: u64) -> chrono::DateTime<chrono::Utc> {
    i64::try_from(ms)
        .ok()
        .and_then(chrono::DateTime::<chrono::Utc>::from_timestamp_millis)
        .unwrap_or(chrono::DateTime::UNIX_EPOCH)
}

/// 把社交快照映射为协议 `FriendUpdate`。
fn to_protocol_update(snapshot: SocialSnapshot) -> FriendUpdateMessage {
    FriendUpdateMessage {
        friends: snapshot
            .friends
            .into_iter()
            .map(|entry| FriendUpdateFriendsItem {
                display_name: entry.display_name,
                player_code: entry.player_code,
                player_id: entry.player_id,
                status: match entry.status {
                    FriendStatus::Online => FriendUpdateFriendsItemStatus::Online,
                    FriendStatus::InGame => FriendUpdateFriendsItemStatus::InGame,
                    FriendStatus::Offline => FriendUpdateFriendsItemStatus::Offline,
                },
            })
            .collect(),
        incoming: snapshot
            .incoming
            .into_iter()
            .map(|entry| FriendRequestItem {
                display_name: entry.display_name,
                player_code: entry.player_code,
                player_id: entry.player_id,
                sent_at: utc(entry.sent_at_ms),
            })
            .collect(),
        outgoing: snapshot
            .outgoing
            .into_iter()
            .map(|entry| FriendUpdateOutgoingItem {
                display_name: entry.display_name,
                player_code: entry.player_code,
                player_id: entry.player_id,
                sent_at: utc(entry.sent_at_ms),
            })
            .collect(),
        recent: snapshot
            .recent
            .into_iter()
            .map(|entry| FriendUpdateRecentItem {
                display_name: entry.display_name,
                player_code: entry.player_code,
                player_id: entry.player_id,
                relation: match entry.relation {
                    Relation::None => FriendUpdateRecentItemRelation::None,
                    Relation::Friend => FriendUpdateRecentItemRelation::Friend,
                    Relation::Incoming => FriendUpdateRecentItemRelation::Incoming,
                    Relation::Outgoing => FriendUpdateRecentItemRelation::Outgoing,
                },
            })
            .collect(),
    }
}

/// 构造指定玩家的 `FriendUpdate` 载荷。
///
/// 锁顺序固定为 连接 → 房间 → 社交，避免与通知路径交叉死锁。
async fn build_friend_update(state: &AppState, player_id: &str) -> Option<Value> {
    let now = state.clock.now_ms();
    let connections = state.connections.lock().await;
    let rooms = state.rooms.lock().await;
    let mut social = state.social.lock().await;
    let snapshot = social.snapshot(player_id, now, |id| {
        presence_status(&connections, &rooms, id)
    });
    drop(social);
    drop(rooms);
    drop(connections);
    serde_json::to_value(to_protocol_update(snapshot)).ok()
}

/// 向玩家下发一份最新社交视图。
async fn send_friend_update(state: &AppState, player_id: &str) {
    if let Some(payload) = build_friend_update(state, player_id).await {
        send_social(state, player_id, "FriendUpdate", payload).await;
    }
}

/// 玩家在线状态变化：通知本人与其好友。
async fn notify_presence(state: &AppState, player_id: &str) {
    let targets = {
        let social = state.social.lock().await;
        let mut targets = social.friends_of(player_id);
        targets.push(player_id.to_string());
        targets
    };
    for target in targets {
        send_friend_update(state, &target).await;
    }
}

/// 社交关系变化：只通知直接相关的玩家。
async fn notify_social(state: &AppState, player_ids: &[String]) {
    for player_id in player_ids {
        send_friend_update(state, player_id).await;
    }
}

/// 下行社交消息：房间内走房间 task，房间外走连接控制通道。
async fn send_social(state: &AppState, player_id: &str, kind: &'static str, payload: Value) {
    let room_id = {
        let rooms = state.rooms.lock().await;
        rooms
            .room_of_player(player_id)
            .map(|(room, _)| room.id.clone())
    };
    if let Some(room_id) = room_id
        && let Some(handle) = state.hub.lock().await.get(&room_id)
    {
        let _ = handle.tx.send(RoomMessage::Push {
            player_id: player_id.to_string(),
            kind,
            payload,
        });
        return;
    }
    let connections = state.connections.lock().await;
    connections.try_send(player_id, ControlMessage::Payload { kind, payload });
}

/// 下发一条已序列化的协议消息；序列化失败时静默丢弃。
async fn send_protocol<T: Serialize>(
    state: &AppState,
    player_id: &str,
    kind: &'static str,
    message: &T,
) {
    if let Ok(value) = serde_json::to_value(message) {
        send_social(state, player_id, kind, value).await;
    }
}

/// 把排队状态转成协议载荷。
fn queue_state_payload(data: QueueStateData) -> QueueState {
    QueueState {
        waiting_ms: data.waiting_ms,
        // 契约上限 8：队列里可能暂存第九人，展示只按一局需要的人数封顶。
        waiting_players: data.waiting_players.min(SEAT_COUNT) as i64,
    }
}

/// 补全天梯字段：段位与积分只来自积分存储，账号层不自行计算。
async fn apply_rank_to_account_view(
    state: &AppState,
    player_id: &str,
    view: &mut protocol::AccountView,
) {
    let summary = state.ratings.lock().await.summary(&state.ranked, player_id);
    view.rating = u64::try_from(summary.rating).unwrap_or(0);
    view.tier = summary.tier;
    view.highest_tier = summary.highest_tier;
}

/// 账号视图（含天梯段位）；账号不存在时返回 `None`。
async fn account_view_with_rank(
    state: &AppState,
    player_id: &str,
) -> Option<protocol::AccountView> {
    let mut view = state.accounts.lock().await.view(player_id)?;
    apply_rank_to_account_view(state, player_id, &mut view).await;
    Some(view)
}

/// 账号全集：排行榜与宗师快照在内存模式下需要它补齐未结算账号。
async fn account_ids(state: &AppState) -> Vec<String> {
    state.accounts.lock().await.player_ids()
}

/// 经连接自己的下行通道发送一条 `QueueState`。
fn send_queue_state(
    out: &mpsc::Sender<ServerEnvelope>,
    channel: &mut SeatChannel,
    floor: &SeqFloor,
    data: QueueStateData,
) {
    send_payload(
        out,
        channel,
        floor,
        "QueueState",
        &queue_state_payload(data),
    );
}

/// 宗师榜快照检查间隔：UTC 日切后最迟一分钟内补齐当天快照。
const MASTER_SNAPSHOT_INTERVAL: Duration = Duration::from_secs(60);

/// 启动宗师快照驱动：首个 tick 立即执行，启动即补齐当天；之后每分钟检查日切。
///
/// 时间统一取服务端 [`crate::Clock`]，不直接读系统时间，测试可用 `FixedClock`。
pub(crate) fn spawn_master_snapshot_driver(state: AppState) {
    tokio::spawn(async move {
        let mut ticker = time::interval(MASTER_SNAPSHOT_INTERVAL);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            let now = state.clock.now_ms();
            // 锁序：账号 → 积分，与请求路径一致。
            let ids = account_ids(&state).await;
            let mut ratings = state.ratings.lock().await;
            if let Err(error) = ratings
                .ensure_master_snapshot(&state.ranked, now, &ids)
                .await
            {
                eprintln!("room-service: 宗师快照失败: {error}");
            }
        }
    });
}

/// 启动匹配驱动：每 200ms 推进一次队列，成局后建房并通知连接。
///
/// 队列为空时驱动自行退出；连接层的下一次入队会重新拉起。
fn spawn_queue_driver(state: AppState) {
    tokio::spawn(async move {
        let mut ticker = time::interval(QUEUE_TICK_INTERVAL);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            let now = state.clock.now_ms();
            let (notices, formed, empty) = {
                // 锁顺序固定为 队列 → 连接 → 房间，与入队/建房路径一致。
                let mut queue = state.queue.lock().await;
                let stale = {
                    let connections = state.connections.lock().await;
                    let rooms = state.rooms.lock().await;
                    queue.retain(|entry| {
                        connections.is_online(&entry.session.player_id)
                            && rooms.room_of_player(&entry.session.player_id).is_none()
                    })
                };
                if !stale.is_empty() {
                    eprintln!("room-service: 匹配移出 {} 名失效玩家", stale.len());
                }
                let mut tick = queue.tick(now);
                let formed = match tick.formed.take() {
                    Some(entries) => {
                        let sessions: Vec<Session> =
                            entries.iter().map(|entry| entry.session.clone()).collect();
                        let created = {
                            // 持队列锁建房：与 CreateRoom 的「先移出队列、再建房」串行，
                            // 避免同一玩家同时被匹配进房又自己建房。
                            let mut rooms = state.rooms.lock().await;
                            rooms.create_match(&sessions, &state.catalog)
                        };
                        match created {
                            Ok(room) => Some((room, entries)),
                            Err(error) => {
                                eprintln!("room-service: 匹配建房失败: {error}");
                                None
                            }
                        }
                    }
                    None => None,
                };
                let empty = queue.is_empty();
                if empty {
                    queue.end_driver();
                }
                (tick.notices, formed, empty)
            };
            if !notices.is_empty() {
                let connections = state.connections.lock().await;
                for notice in notices {
                    connections.try_send(
                        &notice.player_id,
                        ControlMessage::Payload {
                            kind: "QueueState",
                            payload: serde_json::to_value(queue_state_payload(notice.state))
                                .unwrap_or_else(|_| Value::Object(serde_json::Map::new())),
                        },
                    );
                }
            }
            if let Some((room, entries)) = formed {
                activate_match_room(&state, room, entries).await;
            }
            if empty {
                break;
            }
        }
    });
}

/// 匹配房建房成功后登记 hub、启动房间 task，并通知 8 名玩家入房。
async fn activate_match_room(state: &AppState, room: crate::room::Room, entries: Vec<QueueEntry>) {
    let seed = room.seed;
    let (tx, rx) = mpsc::unbounded_channel();
    state.hub.lock().await.insert(
        room.id.clone(),
        RoomHandle {
            tx,
            code: room.code.clone(),
            seed,
        },
    );
    spawn_room_task(state.clone(), room.id.clone(), seed, rx);
    let connections = state.connections.lock().await;
    for (seat, entry) in entries.iter().enumerate() {
        let _ = connections.try_send(
            &entry.session.player_id,
            ControlMessage::EnterRoom {
                room_id: room.id.clone(),
                seat: seat as u8,
            },
        );
    }
    eprintln!(
        "room-service: match formed room={} players={}",
        room.id,
        entries.len()
    );
}

/// 房间码派生的确定性对局种子（FNV-1a 64 位）。
fn spawn_room_task(
    state: AppState,
    room_id: String,
    seed: u64,
    rx: mpsc::UnboundedReceiver<RoomMessage>,
) {
    let mut task = RoomTask {
        state,
        room_id,
        seed,
        game: None,
        senders: BTreeMap::new(),
        presence_end_notified: false,
        ranked_start_deadline_ms: None,
        ranked_start_ratings: BTreeMap::new(),
    };
    tokio::spawn(async move {
        let mut ticker = time::interval(TICK_INTERVAL);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let mut rx = rx;
        loop {
            tokio::select! {
                message = rx.recv() => match message {
                    Some(message) => task.on_message(message).await,
                    None => break,
                },
                _ = ticker.tick() => task.on_tick().await,
            }
            if task.retire_if_done().await {
                break;
            }
        }
    });
}

/// 单个房间的写者 task：独占房间与对局状态。
struct RoomTask {
    state: AppState,
    room_id: String,
    seed: u64,
    game: Option<GameTask>,
    senders: BTreeMap<String, SeatSender>,
    /// 是否已向好友推送过「对局结束」状态，避免重复通知。
    presence_end_notified: bool,
    /// 匹配房等待连接挂载的截止时刻；非匹配房恒为 `None`。
    ranked_start_deadline_ms: Option<u64>,
    /// 匹配局开局积分快照；结算使用开局积分，避免中途读取造成偏差。
    ranked_start_ratings: BTreeMap<String, i32>,
}

impl RoomTask {
    /// 房间 task 是否应自行回收：注册表里的房间已不存在。
    ///
    /// `hub` 持有房间入口的发送端，是 task 存活的唯一原因；因此摘除 hub 条目必须在
    /// 房间 task 内完成，随后 loop 才能退出。对局结束、解散与全员离开都会删除注册表
    /// 房间，这里统一摘除 hub，避免房间码与 task 泄漏。
    async fn retire_if_done(&mut self) -> bool {
        let gone = {
            let rooms = self.state.rooms.lock().await;
            rooms.room(&self.room_id).is_none()
        };
        if gone {
            self.state.hub.lock().await.remove(&self.room_id);
            return true;
        }
        false
    }

    async fn on_message(&mut self, message: RoomMessage) {
        match message {
            RoomMessage::Attach {
                player_id,
                out,
                floor,
                generation,
            } => self.attach(player_id, out, floor, generation).await,
            RoomMessage::Reconnect {
                player_id,
                last_seq,
                out,
                floor,
                generation,
            } => {
                self.reconnect(player_id, last_seq, out, floor, generation)
                    .await;
            }
            RoomMessage::Detach {
                player_id,
                generation,
            } => self.detach(player_id, generation).await,
            RoomMessage::IsSeatActive { player_id, reply } => {
                let _ = reply.send(self.seat_active(&player_id).await);
            }
            RoomMessage::Leave {
                player_id,
                generation,
            } => self.leave(player_id, generation).await,
            RoomMessage::Command(command) => self.on_command(command).await,
            RoomMessage::Push {
                player_id,
                kind,
                payload,
            } => self.push_to(&player_id, kind, &payload),
            RoomMessage::Ignore => {}
        }
    }

    /// 顶号接管前座位是否仍有效：对局中要求座位存在且未弃权，大厅阶段要求
    /// 注册表中仍有该座位。
    async fn seat_active(&self, player_id: &str) -> bool {
        if let Some(game) = self.game.as_ref() {
            return game
                .seat_of(player_id)
                .is_some_and(|_| !game.is_departed(player_id));
        }
        let rooms = self.state.rooms.lock().await;
        rooms
            .room(&self.room_id)
            .and_then(|room| room.seat_of(player_id))
            .is_some()
    }

    /// 向房间内某个座位下发非房间消息；玩家已断开时丢弃。
    fn push_to(&self, player_id: &str, kind: &str, payload: &Value) {
        let Some(sender) = self.senders.get(player_id) else {
            return;
        };
        let mut channel = SeatChannel::new();
        send_value(&sender.out, &mut channel, &sender.floor, kind, payload);
    }

    async fn on_tick(&mut self) {
        if let Some(game) = self.game.as_mut() {
            let updates = game.tick();
            self.deliver(updates).await;
            return;
        }
        let now = self.state.clock.now_ms();
        // 匹配房没有大厅阶段：连接挂上后立即开局，最多等一个短暂宽限。
        if self.try_start_ranked(now).await {
            return;
        }
        // 大厅没有对局：只处理断线宽限超时的座位释放。
        let expired = {
            let mut rooms = self.state.rooms.lock().await;
            rooms.tick_disconnects(&self.room_id, now)
        };
        if expired.is_empty() {
            return;
        }
        for player_id in expired {
            self.senders.remove(&player_id);
        }
        self.broadcast_room_update().await;
    }

    /// 匹配房是否已满足开局条件；满足时直接构造对局并启动。
    ///
    /// 8 个连接全部挂上时立即开局；有玩家迟迟未挂上（例如入房通知丢失）时，
    /// 等待 [`RANKED_ATTACH_GRACE_MS`] 后按已挂载座位开局，未挂载座位仍可通过
    /// 既有 `Reconnect` 回到原座位。
    async fn try_start_ranked(&mut self, now: u64) -> bool {
        let ready = {
            let rooms = self.state.rooms.lock().await;
            rooms
                .room(&self.room_id)
                .is_some_and(|room| room.ranked && room.phase == Phase::Recruit)
        };
        if !ready {
            return false;
        }
        let deadline = *self
            .ranked_start_deadline_ms
            .get_or_insert_with(|| now.saturating_add(RANKED_ATTACH_GRACE_MS));
        if self.senders.len() < SEAT_COUNT && now < deadline {
            return false;
        }
        self.start_ranked_game().await;
        true
    }

    async fn attach(
        &mut self,
        player_id: String,
        out: mpsc::Sender<ServerEnvelope>,
        floor: SeqFloor,
        generation: u64,
    ) {
        self.senders.insert(
            player_id.clone(),
            SeatSender {
                out: out.clone(),
                floor: floor.clone(),
                generation,
            },
        );
        if self.game.is_some() {
            // 对局中的断线曾写回注册表；重连回来要清掉宽限标记，避免对局内被误释放。
            {
                let mut rooms = self.state.rooms.lock().await;
                let _ = rooms.mark_reconnected(&self.room_id, &player_id);
            }
            let updates = {
                let game = self.game.as_mut().expect("刚判空");
                game.bind_sender(&player_id, out);
                if let Some(seat) = game.seat_of(&player_id) {
                    game.set_seq_floor(seat, floor.current());
                }
                game.handle(RoomCommand::RequestSnapshot { player_id })
            };
            self.deliver(updates).await;
        } else {
            let (result, ranked) = {
                let mut rooms = self.state.rooms.lock().await;
                let ranked = rooms.room(&self.room_id).is_some_and(|room| room.ranked);
                (rooms.mark_reconnected(&self.room_id, &player_id), ranked)
            };
            if result.is_err() {
                // 宽限期已过、座位已被释放：连接不再属于该房间。
                self.send_lobby_error(&player_id, "NOT_IN_ROOM", "座位已释放")
                    .await;
                self.senders.remove(&player_id);
                return;
            }
            // 匹配房跳过大厅阶段：不向客户端下发 `RoomUpdate`，等 `GameStarted`。
            if !ranked {
                self.send_room_update_to(&player_id).await;
            }
        }
    }

    async fn reconnect(
        &mut self,
        player_id: String,
        last_seq: u64,
        out: mpsc::Sender<ServerEnvelope>,
        floor: SeqFloor,
        generation: u64,
    ) {
        self.senders.insert(
            player_id.clone(),
            SeatSender {
                out: out.clone(),
                floor: floor.clone(),
                generation,
            },
        );
        if self.game.is_some() {
            // 同 `attach`：重连清掉对局断线写回的宽限标记。
            {
                let mut rooms = self.state.rooms.lock().await;
                let _ = rooms.mark_reconnected(&self.room_id, &player_id);
            }
            let updates = {
                let game = self.game.as_mut().expect("刚判空");
                game.bind_sender(&player_id, out.clone());
                if let Some(seat) = game.seat_of(&player_id) {
                    game.set_seq_floor(seat, floor.current());
                }
                game.handle(RoomCommand::Reconnect {
                    player_id,
                    last_seq,
                    out,
                })
            };
            self.deliver(updates).await;
        } else {
            let (result, ranked) = {
                let mut rooms = self.state.rooms.lock().await;
                let ranked = rooms.room(&self.room_id).is_some_and(|room| room.ranked);
                (rooms.mark_reconnected(&self.room_id, &player_id), ranked)
            };
            if result.is_err() {
                self.send_lobby_error(&player_id, "NOT_IN_ROOM", "座位已释放")
                    .await;
                self.senders.remove(&player_id);
                return;
            }
            // 匹配房跳过大厅阶段：重连不补发 `RoomUpdate`，等 `GameStarted`。
            if !ranked {
                self.send_room_update_to(&player_id).await;
            }
        }
    }

    async fn detach(&mut self, player_id: String, generation: u64) {
        // 代次不匹配说明这是被新连接替换掉的旧连接，迟到关闭不得影响新连接。
        if self.senders.get(&player_id).map(|sender| sender.generation) != Some(generation) {
            return;
        }
        self.senders.remove(&player_id);
        if self.game.is_some() {
            if let Some(game) = self.game.as_mut() {
                game.handle(RoomCommand::Disconnect {
                    player_id: player_id.clone(),
                });
            }
            // 对局中的断线同样写回注册表：大厅 tick 才能在宽限期后释放该座位；
            // 只记在 GameTask 里会让座位一直占着。对局结束房间直接销毁，不回房。
            let now = self.state.clock.now_ms();
            let _ = {
                let mut rooms = self.state.rooms.lock().await;
                rooms.mark_disconnected(&self.room_id, &player_id, now)
            };
            return;
        }
        // 大厅断线保留座位与候选一个宽限期，重连能拿回同一组；超时由 tick 释放。
        let now = self.state.clock.now_ms();
        let (result, ranked) = {
            let mut rooms = self.state.rooms.lock().await;
            let ranked = rooms.room(&self.room_id).is_some_and(|room| room.ranked);
            (
                rooms.mark_disconnected(&self.room_id, &player_id, now),
                ranked,
            )
        };
        // 匹配房没有大厅视图需要广播。
        if result.is_ok() && !ranked {
            self.broadcast_room_update().await;
        }
    }

    async fn leave(&mut self, player_id: String, generation: u64) {
        if self.senders.get(&player_id).map(|sender| sender.generation) != Some(generation) {
            return;
        }
        if self.game.is_some() {
            // 对局中退出按弃权结算：座位保留到本局结束，随后随房间一起销毁。
            let updates = {
                let game = self.game.as_mut().expect("刚判空");
                game.handle(RoomCommand::Forfeit {
                    player_id: player_id.clone(),
                })
            };
            self.deliver_excluding(updates, Some(&player_id)).await;
            self.senders.remove(&player_id);
            notify_presence(&self.state, &player_id).await;
            eprintln!(
                "room-service: forfeit room={} player={} result=ok",
                self.room_id, player_id
            );
            return;
        }
        let result = {
            let mut rooms = self.state.rooms.lock().await;
            rooms.leave(&self.room_id, &player_id)
        };
        match result {
            Ok(()) => {
                self.senders.remove(&player_id);
                self.broadcast_room_update().await;
            }
            Err(error) => {
                let code = error.code().to_string();
                let message = error.to_string();
                self.send_lobby_error(&player_id, &code, &message).await;
            }
        }
    }

    async fn on_command(&mut self, command: RoomCommand) {
        if self.game.is_some() {
            let updates = {
                let game = self.game.as_mut().expect("刚判空");
                game.handle(command)
            };
            self.deliver(updates).await;
        } else {
            self.on_lobby_command(command).await;
        }
    }

    async fn on_lobby_command(&mut self, command: RoomCommand) {
        match command {
            RoomCommand::Ready { player_id, ready } => {
                let result = {
                    let mut rooms = self.state.rooms.lock().await;
                    rooms.ready(&self.room_id, &player_id, ready)
                };
                match result {
                    // 准备只更新状态：开局改由房主显式 `StartGame` 触发。
                    Ok(()) => self.broadcast_room_update().await,
                    Err(error) => {
                        let code = error.code().to_string();
                        let message = error.to_string();
                        self.send_lobby_error(&player_id, &code, &message).await;
                    }
                }
            }
            RoomCommand::StartGame { player_id } => {
                self.start_game(&player_id).await;
            }
            RoomCommand::SetBotFill { player_id, enabled } => {
                let result = {
                    let mut rooms = self.state.rooms.lock().await;
                    rooms.set_bot_fill(&self.room_id, &player_id, enabled)
                };
                match result {
                    Ok(()) => self.broadcast_room_update().await,
                    Err(error) => {
                        let code = error.code().to_string();
                        let message = error.to_string();
                        self.send_lobby_error(&player_id, &code, &message).await;
                    }
                }
            }
            RoomCommand::SelectHero { player_id, .. } => {
                // 选英雄只在对局内的选英雄阶段有效。
                self.send_lobby_error(&player_id, "INVALID_STAGE", "对局尚未开始")
                    .await;
            }
            RoomCommand::RequestSnapshot { player_id } => {
                self.send_room_update_to(&player_id).await;
            }
            // 重连由 `RoomMessage::Reconnect` 专门处理，这里只兜底。
            RoomCommand::Reconnect {
                player_id,
                last_seq: _,
                out: _,
            } => self.send_room_update_to(&player_id).await,
            RoomCommand::Disconnect { player_id } => {
                // 大厅阶段不经过 `Detach` 的座位释放语义；这里只移除发送端。
                self.senders.remove(&player_id);
            }
            RoomCommand::Intent { player_id, .. } => {
                self.send_lobby_error(&player_id, "INVALID_STAGE", "对局尚未开始")
                    .await;
            }
            RoomCommand::CombatPlayed { player_id } => {
                self.send_lobby_error(&player_id, "INVALID_STAGE", "对局尚未开始")
                    .await;
            }
            RoomCommand::Kick {
                player_id,
                target_player_id,
            } => {
                let result = {
                    let mut rooms = self.state.rooms.lock().await;
                    rooms.kick(&self.room_id, &player_id, &target_player_id)
                };
                match result {
                    Ok(()) => {
                        eprintln!(
                            "room-service: kick room={} actor={} target={} result=ok",
                            self.room_id, player_id, target_player_id
                        );
                        // 先经该座位当前下行通道发出通知，再移除发送端：
                        // 被移除后座位已不在注册表，后续只能靠发送端自己的序号下限。
                        self.send_room_closed_to(&target_player_id, RoomClosedReason::Kicked)
                            .await;
                        self.senders.remove(&target_player_id);
                        self.broadcast_room_update().await;
                        notify_presence(&self.state, &player_id).await;
                        notify_presence(&self.state, &target_player_id).await;
                    }
                    Err(error) => {
                        eprintln!(
                            "room-service: kick room={} actor={} target={} result=err({error})",
                            self.room_id, player_id, target_player_id
                        );
                        let code = error.code().to_string();
                        self.send_lobby_error(&player_id, &code, &error.to_string())
                            .await;
                    }
                }
            }
            RoomCommand::Dissolve { player_id } => {
                let result = {
                    let mut rooms = self.state.rooms.lock().await;
                    rooms.dissolve(&self.room_id, &player_id)
                };
                match result {
                    Ok(members) => {
                        eprintln!(
                            "room-service: dissolve room={} actor={} members={} result=ok",
                            self.room_id,
                            player_id,
                            members.len()
                        );
                        // 注册表已删除房间：逐个下发 RoomClosed 后清空发送端，
                        // 由下一轮 `retire_if_done` 摘除 hub。
                        for member in &members {
                            self.send_room_closed_to(member, RoomClosedReason::Dissolved)
                                .await;
                        }
                        self.senders.clear();
                    }
                    Err(error) => {
                        eprintln!(
                            "room-service: dissolve room={} actor={} result=err({error})",
                            self.room_id, player_id
                        );
                        let code = error.code().to_string();
                        self.send_lobby_error(&player_id, &code, &error.to_string())
                            .await;
                    }
                }
            }
            RoomCommand::Forfeit { player_id } => {
                // `Forfeit` 只由对局中的 `LeaveRoom` 生成；大厅阶段不会有合法来源。
                self.send_lobby_error(&player_id, "INVALID_STAGE", "当前阶段不允许")
                    .await;
            }
            RoomCommand::Error {
                player_id,
                code,
                message,
            } => {
                self.send_lobby_error(&player_id, &code, &message).await;
            }
        }
    }

    /// 房主开始对局：先只读构造开局计划与 `GameTask`，成功后才提交阶段。
    ///
    /// `prepare_start` 失败（非房主、阶段、准备或补位人数不足）只回大厅错误，
    /// 注册表不变；`GameTask::new_picking` 失败同样不提交，房间仍可离开。
    async fn start_game(&mut self, actor: &str) {
        let prepared = {
            let rooms = self.state.rooms.lock().await;
            let bot_fill = rooms.room(&self.room_id).is_some_and(|room| room.bot_fill);
            match rooms.prepare_start(&self.room_id, actor, &self.state.catalog) {
                Ok(plan) => Some((plan, bot_fill)),
                Err(error) => {
                    let code = error.code().to_string();
                    let message = error.to_string();
                    drop(rooms);
                    self.broadcast_lobby_error(&code, &message).await;
                    None
                }
            }
        };
        let Some((plan, bot_fill)) = prepared else {
            return;
        };
        let humans = plan.slots.iter().filter(|slot| !slot.is_bot).count();
        let bots = plan.slots.len().saturating_sub(humans);

        let game = match GameTask::new_picking(
            self.state.catalog.clone(),
            plan.seed,
            plan.playstyles,
            plan.candidates.clone(),
            plan.slots.clone(),
            self.state.clock.clone(),
            self.state.achievements.clone(),
        ) {
            Ok(game) => {
                let mut game = game
                    .with_room_id(self.room_id.clone())
                    .with_ranked(plan.ranked);
                game.apply_env_policy();
                game
            }
            Err(error) => {
                // 阶段尚未提交，房间仍在 `Lobby`；错误只通知当前连接，玩家可重试或离开。
                self.broadcast_lobby_error("INTERNAL_ERROR", &error.to_string())
                    .await;
                return;
            }
        };

        // 对局构造成功后才提交阶段与机器人座位。
        {
            let mut rooms = self.state.rooms.lock().await;
            if let Err(error) = rooms.commit_start(&self.room_id, plan) {
                let code = error.code().to_string();
                let message = error.to_string();
                drop(rooms);
                self.broadcast_lobby_error(&code, &message).await;
                return;
            }
        }

        eprintln!(
            "room-service: start room={} humans={} bots={} bot_fill={}",
            self.room_id, humans, bots, bot_fill
        );
        self.activate_game(game).await;
    }

    /// 匹配房开局：候选与座位已由 `RoomRegistry::create_match` 写入，直接进入选英雄阶段。
    ///
    /// 与大厅开局共用 [`RoomTask::activate_game`] 的发送端绑定与开局通知。
    async fn start_ranked_game(&mut self) {
        self.ranked_start_deadline_ms = None;
        let prepared = {
            let rooms = self.state.rooms.lock().await;
            let Some(room) = rooms.room(&self.room_id) else {
                return;
            };
            (room.players(), room.candidates().clone(), room.playstyles)
        };
        let (players, candidates, playstyles) = prepared;
        // 开局积分快照：后续结算一律使用这里的值，不中途再读存储。
        self.ranked_start_ratings = {
            let ratings = self.state.ratings.lock().await;
            players
                .iter()
                .map(|player| {
                    (
                        player.player_id.clone(),
                        ratings.rating_of(&player.player_id),
                    )
                })
                .collect()
        };
        let game = match GameTask::new_picking(
            self.state.catalog.clone(),
            self.seed,
            playstyles,
            candidates,
            players,
            self.state.clock.clone(),
            self.state.achievements.clone(),
        ) {
            Ok(game) => {
                let mut game = game.with_room_id(self.room_id.clone()).with_ranked(true);
                game.apply_env_policy();
                game
            }
            Err(error) => {
                eprintln!(
                    "room-service: 匹配房构造对局失败 room={}: {error}",
                    self.room_id
                );
                return;
            }
        };
        self.activate_game(game).await;
    }

    /// 对局已构造成功：绑定各连接发送端、下发 `HeroPickStarted` 并进入对局状态。
    async fn activate_game(&mut self, mut game: GameTask) {
        // 每次建连的发送端都先绑定（首次连接也走这里），随后 `HeroPickStarted` 用同一
        // 座位通道编号，保证序号连续。
        for (player_id, sender) in &self.senders {
            game.bind_sender(player_id, sender.out.clone());
        }
        let mut started = Vec::new();
        for (player_id, sender) in &self.senders {
            if let Some(seat) = game.seat_of(player_id) {
                game.set_seq_floor(seat, sender.floor.current());
                if let Some(envelope) = game.hero_pick_started_for(seat) {
                    sender.floor.record(envelope.seq.get());
                    started.push((sender.out.clone(), envelope));
                }
            }
        }
        self.game = Some(game);
        self.presence_end_notified = false;
        for (out, envelope) in started {
            let _ = out.try_send(envelope);
        }
        // 进入对局后好友列表显示「对局中」；记录本局参与者用于「最近对局」。
        let participants: Vec<String> = self.senders.keys().cloned().collect();
        {
            let now = self.state.clock.now_ms();
            let mut social = self.state.social.lock().await;
            // 落库失败只影响「最近对局」展示，不阻断开局；错误记日志便于排查。
            if let Err(error) = social.record_match(&participants, now).await {
                eprintln!("room-service: 记录最近对局失败: {error}");
            }
        }
        for player in participants {
            notify_presence(&self.state, &player).await;
        }
    }

    /// 把 `SeatUpdate` 编码后经该座位当前发送端投递；发送失败只丢弃。
    async fn deliver(&mut self, updates: Vec<SeatUpdate>) {
        self.deliver_excluding(updates, None).await;
    }

    /// 同 [`RoomTask::deliver`]，但跳过 `excluded` 玩家自己的更新（对局中退出时使用）。
    ///
    /// 本批出现 `GameOver` 时，全部投递完成后结束对局并销毁房间：清空发送端、
    /// 从注册表删除房间（房间码立即失效），随后 `retire_if_done` 摘除 hub 条目并退出。
    async fn deliver_excluding(&mut self, updates: Vec<SeatUpdate>, excluded: Option<&str>) {
        // 成就解锁先做账号级去重：同一账号只同步一次 Steam 解锁；落库失败不阻断
        // 对局，按首次解锁继续下发，客户端仍按 `steamApiName` 幂等处理。
        let now_ms = self.state.clock.now_ms();
        let mut synchronized = Vec::with_capacity(updates.len());
        for update in updates {
            if let UpdateKind::AchievementUnlocked { achievement_id, .. } = &update.kind {
                match self
                    .state
                    .accounts
                    .lock()
                    .await
                    .unlock_achievement(&update.player_id, achievement_id, now_ms)
                    .await
                {
                    Ok(true) => {}
                    Ok(false) => continue,
                    Err(error) => eprintln!(
                        "room-service: 记录成就解锁失败 player={} achievement={achievement_id}: {}",
                        update.player_id, error.message
                    ),
                }
            }
            synchronized.push(update);
        }
        let updates = synchronized;

        let Some(game) = self.game.as_mut() else {
            return;
        };
        // 天梯结算必须发生在 `GameOver` 投递之前：一次性按开局积分算好全桌，
        // 再按 playerId 把结果注入每个座位的载荷；房间局保持四项为 null。
        let has_game_over = updates
            .iter()
            .any(|update| matches!(&update.kind, UpdateKind::GameOver(_)));
        let mut settled: BTreeMap<String, crate::rating::SettleResult> = BTreeMap::new();
        if has_game_over && crate::rating::should_settle(game.ranked()) {
            let placements = game.placements();
            if placements.len() == rule_engine::rating::PLAYERS_PER_MATCH {
                let match_id = format!("m_{}", uuid::Uuid::new_v4().simple());
                let seed = format!("{:016x}", self.seed);
                let record = crate::rating::MatchRecord {
                    match_id: &match_id,
                    room_id: &self.room_id,
                    seed: &seed,
                    finished_at_ms: now_ms,
                };
                let mut ratings = self.state.ratings.lock().await;
                match ratings
                    .settle_match(
                        &self.state.ranked,
                        &record,
                        &placements,
                        &self.ranked_start_ratings,
                    )
                    .await
                {
                    Ok(results) => {
                        settled = results
                            .into_iter()
                            .map(|result| (result.player_id.clone(), result))
                            .collect();
                    }
                    Err(error) => eprintln!(
                        "room-service: 天梯结算失败 room={} match={match_id}: {error}",
                        self.room_id
                    ),
                }
            } else {
                eprintln!(
                    "room-service: 天梯结算名次不完整 room={} placements={}",
                    self.room_id,
                    placements.len()
                );
            }
        }

        let mut ended = false;
        for mut update in updates {
            if let UpdateKind::GameOver(payload) = &mut update.kind {
                ended = true;
                if let Some(result) = settled.get(&update.player_id) {
                    payload.rating_delta = Some(i64::from(result.rating_delta));
                    payload.rating_after = Some(u64::try_from(result.rating_after).unwrap_or(0));
                    payload.tier_before = Some(result.tier_before.clone());
                    payload.tier_after = Some(result.tier_after.clone());
                }
            }
            if excluded.is_some_and(|player_id| player_id == update.player_id) {
                continue;
            }
            let Some(sender) = self.senders.get(&update.player_id) else {
                continue;
            };
            let floor = sender.floor.clone();
            let Some(seat) = game.seat_of(&update.player_id) else {
                continue;
            };
            game.set_seq_floor(seat, floor.current());
            let Ok(envelope) = game.envelope_for(&update) else {
                continue;
            };
            floor.record(envelope.seq.get());
            if let Some(out) = game.sender(seat) {
                let _ = out.try_send(envelope);
            }
        }
        if !ended {
            return;
        }

        // 全部结果已投递：对局结束即销毁房间，不保留等待区也不重置给下一局。
        if self.game.take().is_none() {
            return;
        }
        let players: Vec<String> = self.senders.keys().cloned().collect();
        self.senders.clear();
        {
            let mut rooms = self.state.rooms.lock().await;
            rooms.remove(&self.room_id);
        }
        if !self.presence_end_notified {
            self.presence_end_notified = true;
            for player in players {
                notify_presence(&self.state, &player).await;
            }
        }
    }

    async fn broadcast_room_update(&mut self) {
        let targets: Vec<(String, mpsc::Sender<ServerEnvelope>, SeqFloor)> = self
            .senders
            .iter()
            .map(|(player_id, sender)| {
                (player_id.clone(), sender.out.clone(), sender.floor.clone())
            })
            .collect();

        let mut rooms = self.state.rooms.lock().await;
        // 同一载荷广播：等待区不再按座位下发不同候选。
        let Some(update) = rooms.room_update(&self.room_id) else {
            return;
        };
        let Ok(value) = serde_json::to_value(&update) else {
            return;
        };
        let Value::Object(payload) = value else {
            return;
        };
        let mut envelopes = Vec::new();
        for (player_id, out, floor) in targets {
            let Some(slot) = rooms
                .room_mut(&self.room_id)
                .and_then(|room| room.slot_entry_mut(&player_id))
            else {
                continue;
            };
            slot.channel.set_next_seq(floor.current());
            if let Ok(envelope) = slot.channel.bind("RoomUpdate", payload.clone()) {
                floor.record(envelope.seq.get());
                envelopes.push((out, envelope));
            }
        }
        drop(rooms);
        for (out, envelope) in envelopes {
            let _ = out.try_send(envelope);
        }
    }

    async fn send_room_update_to(&mut self, player_id: &str) {
        let Some(sender) = self.senders.get(player_id) else {
            return;
        };
        let out = sender.out.clone();
        let floor = sender.floor.clone();
        let mut rooms = self.state.rooms.lock().await;
        let Some(update) = rooms.room_update(&self.room_id) else {
            return;
        };
        let Ok(value) = serde_json::to_value(&update) else {
            return;
        };
        let Value::Object(payload) = value else {
            return;
        };
        let Some(slot) = rooms
            .room_mut(&self.room_id)
            .and_then(|room| room.slot_entry_mut(player_id))
        else {
            return;
        };
        slot.channel.set_next_seq(floor.current());
        let Ok(envelope) = slot.channel.bind("RoomUpdate", payload) else {
            return;
        };
        floor.record(envelope.seq.get());
        drop(rooms);
        let _ = out.try_send(envelope);
    }

    /// 向该玩家下发 `RoomClosed`；座位可能已从注册表移除（kick/dissolve），
    /// 此时用携带当前序号下限的新通道保证 `seq` 继续递增。
    async fn send_room_closed_to(&mut self, player_id: &str, reason: RoomClosedReason) {
        let payload = RoomClosed { reason };
        let Ok(value) = serde_json::to_value(&payload) else {
            return;
        };
        let Value::Object(payload) = value else {
            return;
        };
        self.send_slot_message(player_id, "RoomClosed", payload)
            .await;
    }

    /// 经座位通道（若仍在注册表）或新通道下发一条消息。
    async fn send_slot_message(
        &mut self,
        player_id: &str,
        kind: &str,
        payload: serde_json::Map<String, Value>,
    ) {
        let Some(sender) = self.senders.get(player_id) else {
            return;
        };
        let out = sender.out.clone();
        let floor = sender.floor.clone();

        let mut fresh = SeatChannel::new();
        fresh.set_next_seq(floor.current());
        let mut rooms = self.state.rooms.lock().await;
        let channel = rooms
            .room_mut(&self.room_id)
            .and_then(|room| room.slot_entry_mut(player_id))
            .map(|slot| &mut slot.channel)
            .unwrap_or(&mut fresh);
        channel.set_next_seq(floor.current());
        let Ok(envelope) = channel.bind(kind, payload) else {
            return;
        };
        floor.record(envelope.seq.get());
        drop(rooms);
        let _ = out.try_send(envelope);
    }

    async fn send_lobby_error(&mut self, player_id: &str, code: &str, message: &str) {
        let Some(sender) = self.senders.get(player_id) else {
            return;
        };
        let out = sender.out.clone();
        let floor = sender.floor.clone();
        let payload = error_message(code, message, false);
        let Ok(value) = serde_json::to_value(&payload) else {
            return;
        };
        let Value::Object(payload) = value else {
            return;
        };

        // 座位可能已被宽限超时释放，此时没有 `slot.channel` 可用；但仍然必须把错误
        // 发出去，否则客户端收不到任何回包、会停在一个不能离开的大厅页面上。
        // 这种情况下用一条以当前下行序号为起点的新通道，保证 `seq` 继续递增。
        let mut fresh = SeatChannel::new();
        fresh.set_next_seq(floor.current());
        let mut rooms = self.state.rooms.lock().await;
        let channel = rooms
            .room_mut(&self.room_id)
            .and_then(|room| room.slot_entry_mut(player_id))
            .map(|slot| &mut slot.channel)
            .unwrap_or(&mut fresh);
        channel.set_next_seq(floor.current());
        let Ok(envelope) = channel.bind("Error", payload) else {
            return;
        };
        floor.record(envelope.seq.get());
        drop(rooms);
        let _ = out.try_send(envelope);
    }

    async fn broadcast_lobby_error(&mut self, code: &str, message: &str) {
        let player_ids: Vec<String> = self.senders.keys().cloned().collect();
        for player_id in player_ids {
            self.send_lobby_error(&player_id, code, message).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use protocol::client::AuthRequest;
    use rule_engine::state::Phase;

    use super::*;
    use crate::auth::SessionStore;
    use crate::clock::FixedClock;

    fn session(store: &mut SessionStore, clock: &FixedClock, name: &str) -> Session {
        store
            .authenticate(
                &AuthRequest::Variant0 {
                    code: "123456".to_string(),
                    display_name: Some(name.to_string().try_into().expect("展示名合法")),
                    phone: "13800000000".to_string(),
                },
                clock,
            )
            .expect("开发验证码应登录成功")
    }

    /// 构造一个大厅房间，返回房间 id 与按座位升序的 playerId。
    async fn lobby_with_humans(
        state: &AppState,
        clock: &FixedClock,
        names: &[&str],
    ) -> (String, Vec<String>) {
        let mut store = SessionStore::new();
        let mut rooms = state.rooms.lock().await;
        let host = session(&mut store, clock, names[0]);
        let host_id = host.player_id.clone();
        let created = rooms.create(&host, &state.catalog);
        let room_id = created.id.clone();
        let code = created.code.clone();
        let mut players = vec![host_id];
        for name in &names[1..] {
            let player = session(&mut store, clock, name);
            players.push(player.player_id.clone());
            rooms
                .join(&code, &player, &state.catalog)
                .expect("加入成功");
        }
        (room_id, players)
    }

    fn task_for(state: AppState, room_id: String) -> RoomTask {
        RoomTask {
            state,
            room_id,
            seed: 7,
            game: None,
            senders: BTreeMap::new(),
            presence_end_notified: false,
            ranked_start_deadline_ms: None,
            ranked_start_ratings: BTreeMap::new(),
        }
    }

    fn add_sender(task: &mut RoomTask, player_id: &str) -> mpsc::Receiver<ServerEnvelope> {
        let (out, rx) = mpsc::channel::<ServerEnvelope>(32);
        task.senders.insert(
            player_id.to_string(),
            SeatSender {
                out,
                floor: SeqFloor::default(),
                generation: 1,
            },
        );
        rx
    }

    async fn recv_envelope(rx: &mut mpsc::Receiver<ServerEnvelope>) -> ServerEnvelope {
        tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv())
            .await
            .expect("应在超时前收到下行消息")
            .expect("下行通道不应关闭")
    }

    async fn recv_error_code(rx: &mut mpsc::Receiver<ServerEnvelope>) -> protocol::ErrorCode {
        match recv_envelope(rx).await.parse().expect("载荷可解析") {
            protocol::ServerMessage::Error(error) => error.code,
            other => panic!("期望 Error，实际 {other:?}"),
        }
    }

    #[tokio::test]
    async fn start_game_command_requires_host() {
        let clock = FixedClock::new(1_000_000);
        let state = AppState::new(Arc::new(clock.clone())).expect("状态应能构造");
        let (room_id, players) = lobby_with_humans(&state, &clock, &["房主", "玩家2"]).await;
        for player_id in &players {
            let mut rooms = state.rooms.lock().await;
            rooms.ready(&room_id, player_id, true).expect("准备成功");
        }
        let mut task = task_for(state, room_id.clone());
        let mut rx = add_sender(&mut task, &players[1]);

        task.on_lobby_command(RoomCommand::StartGame {
            player_id: players[1].clone(),
        })
        .await;

        assert_eq!(recv_error_code(&mut rx).await, protocol::ErrorCode::NotHost);
        assert!(task.game.is_none(), "非房主开始不应创建对局");
        let rooms = task.state.rooms.lock().await;
        assert_eq!(rooms.room(&room_id).expect("房间仍在").phase, Phase::Lobby);
    }

    #[tokio::test]
    async fn ready_does_not_auto_start() {
        let clock = FixedClock::new(1_000_000);
        let state = AppState::new(Arc::new(clock.clone())).expect("状态应能构造");
        let names = ["p0", "p1", "p2", "p3", "p4", "p5", "p6", "p7"];
        let (room_id, players) = lobby_with_humans(&state, &clock, &names).await;
        let mut task = task_for(state, room_id.clone());

        for player_id in &players {
            task.on_lobby_command(RoomCommand::Ready {
                player_id: player_id.clone(),
                ready: true,
            })
            .await;
        }

        assert!(task.game.is_none(), "全员准备不得自动开局");
        {
            let rooms = task.state.rooms.lock().await;
            assert_eq!(rooms.room(&room_id).expect("房间仍在").phase, Phase::Lobby);
        }

        let mut rx = add_sender(&mut task, &players[0]);
        task.on_lobby_command(RoomCommand::StartGame {
            player_id: players[0].clone(),
        })
        .await;
        assert!(task.game.is_some(), "房主 StartGame 后才开局");
        match recv_envelope(&mut rx).await.parse().expect("载荷可解析") {
            protocol::ServerMessage::HeroPickStarted(_) => {}
            other => panic!("期望先收到 HeroPickStarted，实际 {other:?}"),
        }
    }

    #[tokio::test]
    async fn set_bot_fill_broadcasts_room_update() {
        let clock = FixedClock::new(1_000_000);
        let state = AppState::new(Arc::new(clock.clone())).expect("状态应能构造");
        let (room_id, players) = lobby_with_humans(&state, &clock, &["房主"]).await;
        let mut task = task_for(state, room_id);
        let mut rx = add_sender(&mut task, &players[0]);

        task.on_lobby_command(RoomCommand::SetBotFill {
            player_id: players[0].clone(),
            enabled: false,
        })
        .await;

        match recv_envelope(&mut rx).await.parse().expect("载荷可解析") {
            protocol::ServerMessage::RoomUpdate(update) => {
                assert!(!update.bot_fill, "广播应反映关闭后的补位开关");
                assert_eq!(update.players.len(), 1);
                assert_eq!(update.players[0].seat, 0);
            }
            other => panic!("期望 RoomUpdate，实际 {other:?}"),
        }
    }

    #[tokio::test]
    async fn select_hero_in_lobby_is_invalid_stage() {
        let clock = FixedClock::new(1_000_000);
        let state = AppState::new(Arc::new(clock.clone())).expect("状态应能构造");
        let (room_id, players) = lobby_with_humans(&state, &clock, &["房主"]).await;
        let mut task = task_for(state, room_id);
        let mut rx = add_sender(&mut task, &players[0]);

        task.on_lobby_command(RoomCommand::SelectHero {
            player_id: players[0].clone(),
            card_id: "darwin".to_string(),
        })
        .await;

        assert_eq!(
            recv_error_code(&mut rx).await,
            protocol::ErrorCode::InvalidStage
        );
        assert!(task.game.is_none());
    }

    #[tokio::test]
    async fn start_game_sends_hero_pick_then_game_started() {
        let clock = FixedClock::new(1_000_000);
        let state = AppState::new(Arc::new(clock.clone())).expect("状态应能构造");
        let (room_id, players) = lobby_with_humans(&state, &clock, &["房主"]).await;
        {
            let mut rooms = state.rooms.lock().await;
            rooms.ready(&room_id, &players[0], true).expect("准备成功");
        }
        let mut task = task_for(state, room_id.clone());
        let mut rx = add_sender(&mut task, &players[0]);

        task.on_lobby_command(RoomCommand::StartGame {
            player_id: players[0].clone(),
        })
        .await;

        let hero = match recv_envelope(&mut rx).await.parse().expect("载荷可解析") {
            protocol::ServerMessage::HeroPickStarted(payload) => {
                assert_eq!(payload.candidates.len(), 4, "本人候选应下发");
                assert!(payload.players.iter().skip(1).all(|item| item.is_bot));
                payload.candidates[0].clone()
            }
            other => panic!("期望先收到 HeroPickStarted，实际 {other:?}"),
        };

        task.on_command(RoomCommand::SelectHero {
            player_id: players[0].clone(),
            card_id: hero,
        })
        .await;

        match recv_envelope(&mut rx).await.parse().expect("载荷可解析") {
            protocol::ServerMessage::GameStarted(_) => {}
            other => panic!("期望选完后收到 GameStarted，实际 {other:?}"),
        }
        assert_eq!(
            task.game.as_ref().expect("对局已开始").stage(),
            crate::game::GameStage::Playing
        );
    }

    /// 构造 `GameTask` 失败时，房间必须仍在 `Lobby`，且玩家可以正常离开。
    #[tokio::test]
    async fn start_game_without_heroes_leaves_room_in_lobby() {
        let clock = FixedClock::new(1_000_000);
        let mut state = AppState::new(Arc::new(clock.clone())).expect("状态应能构造");
        let names = ["p0", "p1", "p2", "p3", "p4", "p5", "p6", "p7"];
        let (room_id, players) = lobby_with_humans(&state, &clock, &names).await;
        for player_id in &players {
            let mut rooms = state.rooms.lock().await;
            rooms.ready(&room_id, player_id, true).expect("准备成功");
        }
        // 清空英雄卡：`prepare_start` 仍可构造计划，但候选为空，`new_picking` 必然失败。
        state.catalog.heroes.clear();
        let host = players[0].clone();

        let mut task = task_for(state, room_id.clone());
        task.start_game(&host).await;

        assert!(task.game.is_none(), "构造失败不应保存对局");
        let mut rooms = task.state.rooms.lock().await;
        let phase = rooms.room(&room_id).expect("房间应仍存在").phase;
        assert_eq!(phase, Phase::Lobby, "构造失败后房间必须留在 Lobby");
        assert!(
            rooms.leave(&room_id, &host).is_ok(),
            "Lobby 阶段房主应能正常离开"
        );
    }

    /// 匹配房重连不补发 `RoomUpdate`：匹配局没有大厅视图，等 `GameStarted`。
    #[tokio::test]
    async fn ranked_room_reconnect_skips_room_update() {
        let clock = FixedClock::new(1_000_000);
        let state = AppState::new(Arc::new(clock.clone())).expect("状态应能构造");
        let mut store = SessionStore::new();
        let sessions: Vec<_> = (0..8)
            .map(|index| session(&mut store, &clock, &format!("p{index}")))
            .collect();
        let player_id = sessions[2].player_id.clone();
        let room_id = {
            let mut rooms = state.rooms.lock().await;
            rooms
                .create_match(&sessions, &state.catalog)
                .expect("匹配房应能创建")
                .id
        };
        let mut task = task_for(state, room_id);
        let mut rx = add_sender(&mut task, &player_id);
        let (out, mut out_rx) = mpsc::channel::<ServerEnvelope>(4);

        task.reconnect(player_id.clone(), 0, out, SeqFloor::default(), 1)
            .await;

        // 重连会用新发送端替换同一 `player_id` 的旧表项，所以断言必须看新连接的接收端；
        // 旧 `rx` 因发送端被替换而必然关闭，只看它无法发现回归。
        assert!(out_rx.try_recv().is_err(), "匹配房重连不得补发 RoomUpdate");
        assert!(rx.try_recv().is_err(), "旧发送端已被替换，不应再收到消息");
    }

    /// 座位已被宽限超时释放时，重连仍必须收到 `NOT_IN_ROOM`，不能静默丢弃。
    ///
    /// 客户端在这条路径上没有任何超时兜底：收不到回包就会停在选英雄页，
    /// 而该页没有离开入口。
    #[tokio::test]
    async fn released_seat_still_receives_lobby_error() {
        let clock = FixedClock::new(1_000_000);
        let state = AppState::new(Arc::new(clock.clone())).expect("状态应能构造");
        let mut store = SessionStore::new();

        let (room_id, player_id) = {
            let mut rooms = state.rooms.lock().await;
            let host = session(&mut store, &clock, "掉线玩家");
            let player_id = host.player_id.clone();
            let created = rooms.create(&host, &state.catalog);
            let room_id = created.id.clone();
            // 模拟宽限超时：座位已被注册表释放。
            rooms.leave(&room_id, &player_id).expect("大厅可以离开");
            (room_id, player_id)
        };

        let (out, mut rx) = mpsc::channel::<ServerEnvelope>(8);
        let mut task = RoomTask {
            state,
            room_id,
            seed: 7,
            game: None,
            senders: BTreeMap::new(),
            presence_end_notified: false,
            ranked_start_deadline_ms: None,
            ranked_start_ratings: BTreeMap::new(),
        };
        task.senders.insert(
            player_id.clone(),
            SeatSender {
                out,
                floor: SeqFloor::default(),
                generation: 1,
            },
        );

        task.send_lobby_error(&player_id, "NOT_IN_ROOM", "座位已释放")
            .await;

        let envelope = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv())
            .await
            .expect("座位已释放时也必须下发错误")
            .expect("下行通道不应关闭");
        match envelope.parse().expect("下行载荷应能解析") {
            protocol::ServerMessage::Error(error) => {
                assert_eq!(error.code, protocol::ErrorCode::NotInRoom);
            }
            other => panic!("期望 Error，实际 {other:?}"),
        }
    }
}

#[cfg(test)]
mod readiness_tests {
    use super::readiness;
    use axum::http::StatusCode;

    #[test]
    fn readiness_maps_database_state() {
        assert_eq!(readiness(true), StatusCode::OK);
        assert_eq!(readiness(false), StatusCode::SERVICE_UNAVAILABLE);
    }
}

#[cfg(test)]
mod probe_tests {
    use super::router;
    use crate::{AppState, SystemClock, db::Db};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use std::sync::Arc;
    use tower::ServiceExt;

    #[tokio::test]
    async fn readyz_is_unavailable_when_database_is_unreachable() {
        let db = Db::lazy_for_test("postgres://postgres:postgres@127.0.0.1:1/civilization")
            .expect("连接池应能装配");
        let mut state = AppState::new(Arc::new(SystemClock)).expect("状态应能装配");
        state.db = Some(Arc::new(db));
        let response = router(state)
            .oneshot(
                Request::builder()
                    .uri("/readyz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn content_serves_bundle_and_honors_etag() {
        let state = AppState::new(Arc::new(SystemClock)).expect("状态应能装配");
        let app = router(state);
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/content")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let etag = response
            .headers()
            .get("etag")
            .expect("响应应带 ETag")
            .to_str()
            .expect("ETag 应为 ASCII")
            .to_string();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&body).expect("响应应是 JSON");
        assert!(
            parsed["cards"]
                .as_array()
                .is_some_and(|cards| !cards.is_empty())
        );

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/content")
                    .header("if-none-match", etag)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
    }
}
