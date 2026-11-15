//! WebSocket 接入层与房间写者任务。
//!
//! # task 拓扑
//!
//! - **连接 task**（每个 WS 连接一个）：读端把 `ClientEnvelope` 校验版本、解析为
//!   [`ClientMessage`]，再映射成 [`RoomCommand`] / 房间消息投给房间 task；认证、
//!   建房、入房这类大厅入口也在这里完成。
//! - **写端 task**（每个连接一个）：只从 `mpsc::Receiver<ServerEnvelope>` 取信封，
//!   序列化成 JSON 文本写回 WS；连接断开或发送失败即退出。
//! - **房间 task**（每个房间一个，单写者）：串行消费房间消息，独占 `Room` /
//!   `GameTask` 的可变状态；定时器驱动 [`GameTask::tick`]。连接层从不直接改房间
//!   状态，只投命令。
//!
//! 投递口径沿用 Task 11：每次建连都先 `bind_sender`（或 `Reconnect`），之后
//! `SeatUpdate` 一律由 [`GameTask::envelope_for`] 编码后经 [`GameTask::sender`]
//! 投递；连接层不维护座位到发送端的表。
//!
//! # 锁粒度
//!
//! [`AppState::sessions`] 与 [`AppState::rooms`] 各自一把 `tokio::sync::Mutex`，
//! 只在短操作内持有、不跨网络等待；[`AppState::hub`] 只在建房/查表时短暂持有。
//! 房间内的可变状态由房间 task 独占，因此不需要为每局再加锁。

use std::collections::BTreeMap;
use std::time::Duration;

use axum::Router;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use axum::routing::get;
use futures_util::{SinkExt, StreamExt};
use protocol::{ClientEnvelope, ClientMessage, ServerEnvelope};
use serde::Serialize;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio::time::{self, MissedTickBehavior};

use crate::AppState;
use crate::auth::Session;
use crate::envelope::SeatChannel;
use crate::error::error_message;
use crate::game::{GameTask, RoomCommand, SeatUpdate, UpdateKind};

/// 每个连接下行队列的容量；连接写不动时房间 task 用 `try_send` 丢弃而不是阻塞。
const OUTBOUND_CAPACITY: usize = 64;

/// 房间计时检查的间隔。
const TICK_INTERVAL: Duration = Duration::from_millis(100);

/// 房间 task 入口。
#[derive(Clone)]
pub struct RoomHandle {
    /// 投递房间消息的无界通道（带背压由房间 task 的串行消费隐含提供）。
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

/// 投递给房间 task 的消息。
pub enum RoomMessage {
    /// 一条已映射好的房间命令。
    Command(RoomCommand),
    /// 新建连接登记下行发送端；若对局已开始则同时绑定座位发送端并回快照。
    Attach {
        player_id: String,
        out: mpsc::Sender<ServerEnvelope>,
    },
    /// 连接断开。
    Detach { player_id: String },
    /// 大厅离开房间。
    Leave { player_id: String },
    /// 无需房间处理的合法上行消息（例如跳过动画）。
    Ignore,
}

/// 构造路由：`GET /ws` 升级为 WebSocket。
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state)
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(state, socket))
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
    let mut session: Option<Session> = None;
    let mut room: Option<(String, mpsc::UnboundedSender<RoomMessage>)> = None;

    while let Some(item) = read.next().await {
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
                        "INVALID_STAGE",
                        "已在房间中，不能重新认证",
                    );
                    continue;
                }
                let result = state
                    .sessions
                    .lock()
                    .await
                    .authenticate(&request, state.clock.as_ref());
                match result {
                    Ok(authorized) => {
                        session = Some(authorized.clone());
                        send_payload(
                            &out_tx,
                            &mut direct,
                            "AuthResult",
                            &authorized.auth_result(),
                        );
                    }
                    Err(error) => send_payload(&out_tx, &mut direct, "Error", &error),
                }
            }
            ClientMessage::CreateRoom(_) => {
                let Some(authorized) = session.clone() else {
                    send_error(&out_tx, &mut direct, "AUTH_FAILED", "请先认证", false);
                    continue;
                };
                if room.is_some() {
                    route_error(
                        &room,
                        session.as_ref(),
                        &out_tx,
                        &mut direct,
                        "INVALID_STAGE",
                        "已在房间中",
                    );
                    continue;
                }
                let created = state.rooms.lock().await.create(&authorized);
                bump_lobby_seq(
                    &state,
                    &created.id,
                    &authorized.player_id,
                    direct.next_seq(),
                )
                .await;
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
                let _ = tx.send(RoomMessage::Attach {
                    player_id: authorized.player_id,
                    out: out_tx.clone(),
                });
                room = Some((created.id, tx));
            }
            ClientMessage::JoinRoom(request) => {
                let Some(authorized) = session.clone() else {
                    send_error(&out_tx, &mut direct, "AUTH_FAILED", "请先认证", false);
                    continue;
                };
                if room.is_some() {
                    route_error(
                        &room,
                        session.as_ref(),
                        &out_tx,
                        &mut direct,
                        "INVALID_STAGE",
                        "已在房间中",
                    );
                    continue;
                }
                let joined = {
                    let mut rooms = state.rooms.lock().await;
                    rooms
                        .join(&request.code, &authorized)
                        .map(|joined_room| joined_room.id.clone())
                };
                match joined {
                    Ok(room_id) => {
                        bump_lobby_seq(&state, &room_id, &authorized.player_id, direct.next_seq())
                            .await;
                        let Some(handle) = state.hub.lock().await.get(&room_id) else {
                            // 先改注册表后查 hub：失败必须回滚刚加入的座位。
                            let _ = state
                                .rooms
                                .lock()
                                .await
                                .leave(&room_id, &authorized.player_id);
                            send_error(
                                &out_tx,
                                &mut direct,
                                "ROOM_NOT_FOUND",
                                "房间任务不存在",
                                false,
                            );
                            continue;
                        };
                        let _ = handle.tx.send(RoomMessage::Attach {
                            player_id: authorized.player_id,
                            out: out_tx.clone(),
                        });
                        room = Some((room_id, handle.tx));
                    }
                    Err(error) => {
                        let code = error.code().to_string();
                        send_error(&out_tx, &mut direct, &code, &error.to_string(), false);
                    }
                }
            }
            ClientMessage::Reconnect(request) => {
                // 真实重连路径：新 WS 连接 → AuthRequest(token) → Reconnect。
                // 此时连接本地还没有房间，必须用 `roomId` 从 hub 找回房间 task。
                let Some(authorized) = session.clone() else {
                    send_error(&out_tx, &mut direct, "AUTH_FAILED", "请先认证", false);
                    continue;
                };
                if room.is_some() {
                    route_error(
                        &room,
                        session.as_ref(),
                        &out_tx,
                        &mut direct,
                        "INVALID_STAGE",
                        "当前连接已在房间中",
                    );
                    continue;
                }
                let Some(handle) = state.hub.lock().await.get(&request.room_id) else {
                    send_error(&out_tx, &mut direct, "ROOM_NOT_FOUND", "房间不存在", false);
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
                        "NOT_IN_ROOM",
                        "会话不属于该房间",
                        false,
                    );
                    continue;
                }
                let _ = handle.tx.send(RoomMessage::Command(RoomCommand::Reconnect {
                    player_id: authorized.player_id.clone(),
                    last_seq: request.last_seq,
                    out: out_tx.clone(),
                }));
                room = Some((request.room_id, handle.tx));
            }
            ClientMessage::LeaveRoom(_) => {
                let Some(authorized) = session.clone() else {
                    send_error(&out_tx, &mut direct, "AUTH_FAILED", "请先认证", false);
                    continue;
                };
                let Some((_, tx)) = room.take() else {
                    send_error(&out_tx, &mut direct, "NOT_IN_ROOM", "尚未加入房间", false);
                    continue;
                };
                let _ = tx.send(RoomMessage::Leave {
                    player_id: authorized.player_id,
                });
            }
            other => {
                let Some(authorized) = session.clone() else {
                    send_error(&out_tx, &mut direct, "AUTH_FAILED", "请先认证", false);
                    continue;
                };
                let Some((_, tx)) = room.clone() else {
                    send_error(&out_tx, &mut direct, "NOT_IN_ROOM", "尚未加入房间", false);
                    continue;
                };
                let message = match map_room_message(other, &authorized.player_id, &out_tx) {
                    Ok(message) => message,
                    Err((code, text)) => {
                        route_error(&room, session.as_ref(), &out_tx, &mut direct, code, text);
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

    let had_room = room.is_some();
    if let Some((_, tx)) = room {
        let _ = tx.send(RoomMessage::Detach {
            player_id: session
                .map(|authorized| authorized.player_id)
                .unwrap_or_default(),
        });
    }
    drop(out_tx);
    if had_room {
        // 对局中 `GameTask` 仍持有发送端，写端不会自然结束，直接中止。
        writer.abort();
    } else {
        // 未入房时给写端一点时间把最后一条 Error 刷出去，再退出。
        let _ = tokio::time::timeout(Duration::from_millis(200), writer).await;
    }
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
        ClientMessage::RequestSnapshot(_) => {
            Ok(RoomMessage::Command(RoomCommand::RequestSnapshot {
                player_id,
            }))
        }
        ClientMessage::Reconnect(request) => Ok(RoomMessage::Command(RoomCommand::Reconnect {
            player_id,
            last_seq: request.last_seq,
            out: out.clone(),
        })),
        ClientMessage::LeaveRoom(_) => Ok(RoomMessage::Leave { player_id }),
        ClientMessage::SkipAnimation(_) => Ok(RoomMessage::Ignore),
        ClientMessage::AuthRequest(_)
        | ClientMessage::CreateRoom(_)
        | ClientMessage::JoinRoom(_) => Err(("INTERNAL_ERROR", "大厅入口消息不应进入房间路由")),
        ClientMessage::FriendRequest(_) => Err(("UNKNOWN_TYPE", "M1 不支持好友请求")),
    }
}

fn send_payload<T: Serialize>(
    out: &mpsc::Sender<ServerEnvelope>,
    channel: &mut SeatChannel,
    kind: &str,
    payload: &T,
) {
    let Ok(value) = serde_json::to_value(payload) else {
        return;
    };
    let Value::Object(map) = value else {
        return;
    };
    if let Ok(envelope) = channel.bind(kind, map) {
        let _ = out.try_send(envelope);
    }
}

fn send_error(
    out: &mpsc::Sender<ServerEnvelope>,
    channel: &mut SeatChannel,
    code: &str,
    message: &str,
    retryable: bool,
) {
    send_payload(
        out,
        channel,
        "Error",
        &error_message(code, message, retryable),
    );
}

/// 下发一条连接层错误。
///
/// 入房之后一律经房间 task 走该座位的统一下行通道（大厅用 `slot.channel`、对局用
/// `GameTask::envelope_for`），避免连接层独立计数器产生倒退的 `seq`。
/// 只有入房之前才走连接自己的 `direct` 通道。
fn route_error(
    room: &Option<(String, mpsc::UnboundedSender<RoomMessage>)>,
    session: Option<&Session>,
    out: &mpsc::Sender<ServerEnvelope>,
    direct: &mut SeatChannel,
    code: &str,
    message: &str,
) {
    if let Some((_, tx)) = room
        && let Some(authorized) = session
    {
        let _ = tx.send(RoomMessage::Command(RoomCommand::Error {
            player_id: authorized.player_id.clone(),
            code: code.to_string(),
            message: message.to_string(),
        }));
        return;
    }
    send_error(out, direct, code, message, false);
}

/// 让该座位的大厅下行序号从连接层已用过的序号之后继续。
async fn bump_lobby_seq(state: &AppState, room_id: &str, player_id: &str, used_seq: u64) {
    let mut rooms = state.rooms.lock().await;
    if let Some(slot) = rooms
        .room_mut(room_id)
        .and_then(|room| room.slot_entry_mut(player_id))
    {
        slot.channel.set_next_seq(used_seq);
    }
}

/// 房间码派生的确定性对局种子（FNV-1a 64 位）。
pub fn seed_for_code(code: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in code.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

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
        game_over: false,
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
                _ = ticker.tick() => task.on_tick(),
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
    /// 大厅阶段的下行发送端；对局阶段发送端由 `GameTask` 持有副本。
    senders: BTreeMap<String, mpsc::Sender<ServerEnvelope>>,
    /// 是否已经向所有座位下发过 `GameOver`。
    game_over: bool,
}

impl RoomTask {
    /// 房间 task 是否应自行回收：注册表里的房间已不存在，或对局结束且所有连接已离开。
    ///
    /// `hub` 持有房间入口的发送端，是 task 存活的唯一原因；因此摘除 hub 条目必须在
    /// 房间 task 内完成，随后 loop 才能退出。
    async fn retire_if_done(&mut self) -> bool {
        if self.game_over && self.senders.is_empty() {
            self.state.hub.lock().await.remove(&self.room_id);
            return true;
        }
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
            RoomMessage::Attach { player_id, out } => self.attach(player_id, out).await,
            RoomMessage::Detach { player_id } => self.detach(player_id).await,
            RoomMessage::Leave { player_id } => self.leave(player_id).await,
            RoomMessage::Command(command) => self.on_command(command).await,
            RoomMessage::Ignore => {}
        }
    }

    fn on_tick(&mut self) {
        if let Some(game) = self.game.as_mut() {
            let updates = game.tick();
            self.deliver(updates);
        }
    }

    async fn attach(&mut self, player_id: String, out: mpsc::Sender<ServerEnvelope>) {
        self.senders.insert(player_id.clone(), out.clone());
        if self.game.is_some() {
            let updates = {
                let game = self.game.as_mut().expect("刚判空");
                game.bind_sender(&player_id, out);
                game.handle(RoomCommand::RequestSnapshot { player_id })
            };
            self.deliver(updates);
        } else {
            self.send_room_update_to(&player_id).await;
        }
    }

    async fn detach(&mut self, player_id: String) {
        self.senders.remove(&player_id);
        if self.game.is_some() {
            if let Some(game) = self.game.as_mut() {
                game.handle(RoomCommand::Disconnect { player_id });
            }
            return;
        }
        // 大厅没有对局状态需要保护：断线立即离开并释放座位，避免永久占位。
        let result = {
            let mut rooms = self.state.rooms.lock().await;
            rooms.leave(&self.room_id, &player_id)
        };
        if result.is_ok() {
            self.broadcast_room_update().await;
        }
    }

    async fn leave(&mut self, player_id: String) {
        if self.game.is_some() {
            self.send_lobby_error(&player_id, "INVALID_STAGE", "对局已开始，不能离开房间")
                .await;
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
            self.deliver(updates);
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
                    Ok(()) => {
                        let all_ready = {
                            let rooms = self.state.rooms.lock().await;
                            rooms.start_ready(&self.room_id).is_some()
                        };
                        if all_ready {
                            self.start_game().await;
                        } else {
                            self.broadcast_room_update().await;
                        }
                    }
                    Err(error) => {
                        let code = error.code().to_string();
                        let message = error.to_string();
                        self.send_lobby_error(&player_id, &code, &message).await;
                    }
                }
            }
            RoomCommand::SelectHero { player_id, card_id } => {
                let result = {
                    let mut rooms = self.state.rooms.lock().await;
                    rooms.select_hero(&self.room_id, &player_id, &card_id, &self.state.catalog)
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
            RoomCommand::RequestSnapshot { player_id } => {
                self.send_room_update_to(&player_id).await;
            }
            RoomCommand::Reconnect {
                player_id,
                last_seq: _,
                out,
            } => self.attach(player_id, out).await,
            RoomCommand::Disconnect { player_id } => self.detach(player_id).await,
            RoomCommand::Intent { player_id, .. } => {
                self.send_lobby_error(&player_id, "INVALID_STAGE", "对局尚未开始")
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

    /// 大厅 → 对局交接：`try_start` 成功后在同一 task 内构造 `GameTask`。
    async fn start_game(&mut self) {
        let prepared = {
            let mut rooms = self.state.rooms.lock().await;
            let Some(host) = rooms.room(&self.room_id).map(|room| room.host_id.clone()) else {
                return;
            };
            match rooms.try_start(&self.room_id, &host) {
                Ok(heroes) => rooms
                    .room(&self.room_id)
                    .map(|room| (heroes, room.players())),
                Err(error) => {
                    let code = error.code().to_string();
                    let message = error.to_string();
                    drop(rooms);
                    self.broadcast_lobby_error(&code, &message).await;
                    None
                }
            }
        };
        let Some((heroes, players)) = prepared else {
            return;
        };

        let mut game = match GameTask::new(
            self.state.catalog.clone(),
            self.seed,
            heroes,
            players,
            self.state.clock.clone(),
        ) {
            Ok(game) => game.with_room_id(self.room_id.clone()),
            Err(error) => {
                self.broadcast_lobby_error("INTERNAL_ERROR", &error.to_string())
                    .await;
                return;
            }
        };

        // 每次建连的发送端都先绑定（首次连接也走这里），随后 `GameStarted` 用同一
        // 座位通道编号，保证序号连续。
        for (player_id, out) in &self.senders {
            game.bind_sender(player_id, out.clone());
        }
        let mut started = Vec::new();
        for player_id in self.senders.keys() {
            if let Some(seat) = game.seat_of(player_id)
                && let Some(envelope) = game.game_started_for(seat)
            {
                started.push((player_id.clone(), envelope));
            }
        }
        self.game = Some(game);
        for (player_id, envelope) in started {
            if let Some(out) = self.senders.get(&player_id) {
                let _ = out.try_send(envelope);
            }
        }
    }

    /// 把 `SeatUpdate` 编码后经该座位当前发送端投递；发送失败只丢弃。
    fn deliver(&mut self, updates: Vec<SeatUpdate>) {
        let Some(game) = self.game.as_mut() else {
            return;
        };
        for update in updates {
            if matches!(&update.kind, UpdateKind::GameOver(_)) {
                self.game_over = true;
            }
            let Ok(envelope) = game.envelope_for(&update) else {
                continue;
            };
            let Some(seat) = game.seat_of(&update.player_id) else {
                continue;
            };
            if let Some(out) = game.sender(seat) {
                let _ = out.try_send(envelope);
            }
        }
    }

    async fn broadcast_room_update(&mut self) {
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
        let Some(room) = rooms.room_mut(&self.room_id) else {
            return;
        };
        let mut envelopes = Vec::new();
        for player_id in self.senders.keys() {
            let Some(slot) = room.slot_entry_mut(player_id) else {
                continue;
            };
            if let Ok(envelope) = slot.channel.bind("RoomUpdate", payload.clone()) {
                envelopes.push((player_id.clone(), envelope));
            }
        }
        drop(rooms);
        for (player_id, envelope) in envelopes {
            if let Some(out) = self.senders.get(&player_id) {
                let _ = out.try_send(envelope);
            }
        }
    }

    async fn send_room_update_to(&mut self, player_id: &str) {
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
        let Ok(envelope) = slot.channel.bind("RoomUpdate", payload) else {
            return;
        };
        drop(rooms);
        if let Some(out) = self.senders.get(player_id) {
            let _ = out.try_send(envelope);
        }
    }

    async fn send_lobby_error(&mut self, player_id: &str, code: &str, message: &str) {
        let mut rooms = self.state.rooms.lock().await;
        let Some(slot) = rooms
            .room_mut(&self.room_id)
            .and_then(|room| room.slot_entry_mut(player_id))
        else {
            return;
        };
        let payload = error_message(code, message, false);
        let Ok(value) = serde_json::to_value(&payload) else {
            return;
        };
        let Value::Object(payload) = value else {
            return;
        };
        let Ok(envelope) = slot.channel.bind("Error", payload) else {
            return;
        };
        drop(rooms);
        if let Some(out) = self.senders.get(player_id) {
            let _ = out.try_send(envelope);
        }
    }

    async fn broadcast_lobby_error(&mut self, code: &str, message: &str) {
        let player_ids: Vec<String> = self.senders.keys().cloned().collect();
        for player_id in player_ids {
            self.send_lobby_error(&player_id, code, message).await;
        }
    }
}
