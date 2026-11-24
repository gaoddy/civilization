//! `sim bot --connect`：真实 WebSocket 客户端。
//!
//! 协议顺序固定为：`AuthRequest` →（`CreateRoom` | `JoinRoom`）→ `Ready` →
//! 房主在全员准备后发 `StartGame` → 等 `HeroPickStarted` 并发送 `SelectHeroCard` →
//! 等 `GameStarted` → 循环按 [`brain`] 决策发送 `IntentMessage`，直到 `GameOver`。
//!
//! 建房机器人同时承担房主：8 名真人全部准备后发送一次 `StartGame`。
//! 机器人房主只在编排式入口（`sim dev` / `sim load`）与手动 `--create` 中出现，
//! 这些流程都先打印房间码、等其余真人入房，因此不能按「补位开启即开始」提前开局。
//! 选英雄优先使用 [`BotConfig::hero`]，不在本人候选内时取候选第一张。
//!
//! 每个意图使用确定性递增的 `clientIntentId`（`{name}-{counter}`）。客户端只对
//! 「自身视角有新变化」的视图做一次决策，避免重连广播导致的重复意图。

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use futures_util::{SinkExt, StreamExt};
use protocol::ErrorCode;
use protocol::ServerEnvelope;
use protocol::ServerMessage;
use protocol::intents::Intent;
use protocol::server::{HeroPickStarted, RoomUpdate};
use protocol::view::PlayerView;
use rule_engine::data::Catalog;
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::tungstenite::Message;

use crate::PLAYER_COUNT;
use crate::bot::brain;

/// 机器人连接配置。
pub struct BotConfig {
    /// WebSocket 地址，例如 `ws://127.0.0.1:8080/ws`。
    pub url: String,
    /// 加入的房间码；`create` 为 true 时忽略。
    pub room: String,
    /// 展示名，同时参与 `clientIntentId` 前缀。
    pub name: String,
    /// 选择的英雄卡 id。
    pub hero: String,
    /// true 表示建房；未提供 `room_code_sink` 时把房间码输出到 stdout（`ROOM_CODE=<code>`），
    /// 提供了则由调用方展示。
    pub create: bool,
    /// brain 在发现候选间做确定性选择时使用的种子。
    pub intent_seed: u64,
    /// 供调用方在进程内取到房间码。提供时由调用方展示房间码，不打印 `ROOM_CODE`。
    pub room_code_sink: Option<tokio::sync::oneshot::Sender<String>>,
}

/// 一局结束后的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotOutcome {
    /// `(playerId, place)`，按服务端排名顺序。
    pub ranking: Vec<(String, u8)>,
    /// 结束时的回合数。
    pub turns: u8,
}

/// 机器人运行期附加选项；[`BotOptions::default`] 等价于原 [`run`] 行为。
#[derive(Default)]
pub struct BotOptions {
    /// 阶段级事件；接收端已丢弃时静默忽略，不阻塞机器人。
    pub events: Option<mpsc::UnboundedSender<BotEvent>>,
    /// 压测停止信号；触发后发送 `LeaveRoom` 并返回 [`BotFinish::Left`]。
    pub stop: Option<watch::Receiver<bool>>,
    /// 覆盖由名字派生的手机号；多房间压测必须保证每个机器人唯一。
    pub phone: Option<String>,
    /// 高频计数器，供压测汇总意图与下行量；用原子计数避免事件通道过载。
    pub counters: Option<Arc<BotCounters>>,
    /// 不在 stderr 逐条输出错误与目标跳过提示。
    pub quiet: bool,
}

/// 阶段级机器人事件，只用于压测计时；不逐视图发送。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BotEvent {
    /// WebSocket 已连接。
    Connected,
    /// 认证完成。
    Authenticated,
    /// 已进入房间并拿到房间码。
    RoomJoined { code: String },
    /// 收到 `GameStarted`。
    GameStarted,
    /// 收到 `GameOver`。
    Finished { turns: u8 },
    /// 运行失败；消息与 [`run_with`] 返回的错误一致。
    Failed { message: String },
}

/// 机器人的结束方式。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BotFinish {
    /// 正常打完一局。
    GameOver(BotOutcome),
    /// 收到停止信号，已发送 `LeaveRoom`。
    Left,
}

/// 高频计数：意图发出数与下行视图数。
#[derive(Debug, Default)]
pub struct BotCounters {
    /// 成功发出的意图数。
    pub intents_sent: AtomicU64,
    /// 收到的 `StateUpdate` 数。
    pub updates_received: AtomicU64,
}

/// 一次连接期间的可变状态：消息序列号、去重记忆与共享计数器。
struct Conn {
    seq: u64,
    memory: BotMemory,
    counters: Option<Arc<BotCounters>>,
}

impl Conn {
    fn new(counters: Option<Arc<BotCounters>>) -> Self {
        Self {
            seq: 0,
            memory: BotMemory::new(),
            counters,
        }
    }
}

type Ws = tokio_tungstenite::WebSocketStream<MaybeTlsStream<TcpStream>>;

/// 连上服务端并打完一局。
///
/// 等价于 [`run_with`] 传入默认选项，行为与压测改造前一致。
///
/// # Errors
///
/// 连接失败、协议消息无法解析、服务端返回认证/入房错误，或连接在对局结束前关闭。
pub async fn run(config: BotConfig) -> Result<BotOutcome, String> {
    match run_with(config, BotOptions::default()).await? {
        BotFinish::GameOver(outcome) => Ok(outcome),
        BotFinish::Left => Err("bot 收到停止信号但未配置停止来源".into()),
    }
}

/// 连上服务端，按 [`BotOptions`] 控制生命周期：正常打完一局，或按停止信号离房。
///
/// # Errors
///
/// 同 [`run`]。
pub async fn run_with(config: BotConfig, mut options: BotOptions) -> Result<BotFinish, String> {
    let mut config = config;
    let result = run_inner(&mut config, &mut options).await;
    if let Err(message) = &result {
        emit(
            &options,
            BotEvent::Failed {
                message: message.clone(),
            },
        );
    }
    result
}

async fn run_inner(config: &mut BotConfig, options: &mut BotOptions) -> Result<BotFinish, String> {
    let (mut ws, _response) = tokio_tungstenite::connect_async(&config.url)
        .await
        .map_err(|err| format!("连接 {} 失败: {err}", config.url))?;
    emit(options, BotEvent::Connected);

    let mut conn = Conn::new(options.counters.clone());
    let catalog = crate::embedded_catalog().map_err(|err| err.to_string())?;

    if !authenticate(&mut ws, config, options, &mut conn.seq).await? {
        return leave_room(&mut ws, &mut conn.seq).await;
    }
    emit(options, BotEvent::Authenticated);
    if stop_requested(&options.stop) {
        return leave_room(&mut ws, &mut conn.seq).await;
    }

    if config.create {
        send(&mut ws, "CreateRoom", json!({}), &mut conn.seq).await?;
    } else {
        if config.room.is_empty() {
            return Err("加入房间时必须提供 --room".into());
        }
        send(
            &mut ws,
            "JoinRoom",
            json!({ "code": config.room }),
            &mut conn.seq,
        )
        .await?;
    }
    let Some(room_update) = wait_room_update(&mut ws, options).await? else {
        return Ok(BotFinish::Left);
    };
    let room_code = room_update.code.clone();
    if config.create {
        // 有 sink 时调用方自己负责展示房间码（`sim dev` 会打印两行提示），此处不重复。
        match config.room_code_sink.take() {
            Some(sink) => {
                let _ = sink.send(room_code.clone());
            }
            None => println!("ROOM_CODE={room_code}"),
        }
    }
    emit(
        options,
        BotEvent::RoomJoined {
            code: room_code.clone(),
        },
    );
    if stop_requested(&options.stop) {
        return leave_room(&mut ws, &mut conn.seq).await;
    }

    // 大厅只提交准备；开局由房主显式 `StartGame` 触发，选英雄在其后的阶段进行。
    send(&mut ws, "Ready", json!({ "ready": true }), &mut conn.seq).await?;

    let mut turns = 1_u8;
    let mut start_sent = false;
    let mut hero_pick_sent = false;
    loop {
        let message = tokio::select! {
            _ = wait_stop(&mut options.stop) => {
                return leave_room(&mut ws, &mut conn.seq).await;
            }
            message = receive(&mut ws) => message?,
        };
        let Some(message) = message else {
            return Err("连接在对局结束前关闭".into());
        };
        match message {
            ServerMessage::RoomUpdate(update) => {
                // 建房机器人同时是房主：8 名真人全部准备后由它开始对局。
                if config.create && !start_sent && all_ready(&update) {
                    start_sent = true;
                    if !options.quiet {
                        eprintln!("bot {} 全员已准备，发送 StartGame", config.name);
                    }
                    send(&mut ws, "StartGame", json!({}), &mut conn.seq).await?;
                }
            }
            ServerMessage::HeroPickStarted(pick) => {
                // 每次有人确认都会再推一次；本人已选或已被超时自动选则跳过。
                if !hero_pick_sent && !picked_for_name(&pick, &config.name) {
                    let hero = pick_hero(&pick, &config.hero)?;
                    send(
                        &mut ws,
                        "SelectHeroCard",
                        json!({ "cardId": hero }),
                        &mut conn.seq,
                    )
                    .await?;
                    hero_pick_sent = true;
                }
            }
            ServerMessage::GameStarted(started) => {
                emit(options, BotEvent::GameStarted);
                turns = turn_of(&started.view);
                decide(&mut ws, config, &catalog, &started.view, &mut conn, false).await?;
            }
            ServerMessage::Snapshot(snapshot) => {
                turns = turn_of(&snapshot.view);
                decide(&mut ws, config, &catalog, &snapshot.view, &mut conn, false).await?;
            }
            ServerMessage::StateUpdate(update) => {
                if let Some(counters) = &conn.counters {
                    counters.updates_received.fetch_add(1, Ordering::Relaxed);
                }
                turns = turn_of(&update.view);
                decide(&mut ws, config, &catalog, &update.view, &mut conn, false).await?;
            }
            ServerMessage::GameOver(game_over) => {
                let ranking = game_over
                    .ranking
                    .into_iter()
                    .map(|item| {
                        (
                            item.player_id,
                            u8::try_from(item.place.get()).unwrap_or(u8::MAX),
                        )
                    })
                    .collect();
                emit(options, BotEvent::Finished { turns });
                return Ok(BotFinish::GameOver(BotOutcome { ranking, turns }));
            }
            ServerMessage::Error(error) => {
                // 目标不合法是预期反馈之外的偏差：候选由服务端下发，正常情况下不会被拒。
                // 出手沿该牌的候选池换下一个，候选遍历完才放弃这张牌；`ChooseTarget`
                // 同样按 `zone:id` 轮换候选。
                if error.code == ErrorCode::NoValidTarget
                    && let Some(rejected) = error
                        .client_intent_id
                        .as_deref()
                        .and_then(|id| conn.memory.rejected(id).cloned())
                {
                    let turn = conn.memory.view.as_ref().map_or(0, turn_of);
                    conn.memory.reset_blocked_for_turn(turn);
                    match &rejected {
                        Intent::PlayCard { hand_index, .. } => {
                            let retried = conn
                                .memory
                                .view
                                .as_ref()
                                .and_then(|view| brain::retry_with_target(view, &rejected));
                            match retried {
                                Some(intent) => {
                                    send_intent(&mut ws, &config.name, &mut conn, intent).await?;
                                }
                                None => {
                                    conn.memory.blocked.block_hand(*hand_index);
                                    if !options.quiet {
                                        log_target_skip(&config.name, &rejected);
                                    }
                                    if let Some(view) = conn.memory.view.clone() {
                                        decide(&mut ws, config, &catalog, &view, &mut conn, true)
                                            .await?;
                                    }
                                }
                            }
                        }
                        Intent::ChooseTarget { target } => {
                            conn.memory.blocked.block_target(target);
                            let exhausted = conn.memory.view.as_ref().is_some_and(|view| {
                                brain::pending_targets_exhausted(view, &conn.memory.blocked)
                            });
                            if exhausted {
                                if !options.quiet {
                                    log_target_skip(&config.name, &rejected);
                                }
                            } else if let Some(view) = conn.memory.view.clone() {
                                decide(&mut ws, config, &catalog, &view, &mut conn, true).await?;
                            }
                        }
                        _ => {}
                    }
                    continue;
                }
                if !options.quiet {
                    eprintln!(
                        "bot {} 收到错误 {}: {}",
                        config.name, error.code, error.message
                    );
                }
                // 被拒绝的意图不会带来新视图，机器人不会自己重试；补发兜底意图，
                // 避免整局停在招募阶段等超时。每个回合最多补发一次。
                let recovery = conn.memory.view.as_ref().and_then(|view| {
                    let turn = turn_of(view);
                    if conn.memory.recovery_turn == Some(turn) {
                        return None;
                    }
                    brain::recover_after_error(view).map(|intent| (turn, intent))
                });
                if let Some((turn, intent)) = recovery {
                    conn.memory.recovery_turn = Some(turn);
                    send_intent(&mut ws, &config.name, &mut conn, intent).await?;
                }
            }
            ServerMessage::CombatResult(_) => {
                // 机器人不做演示动画：收到即回报，避免阻塞同一局的真人客户端。
                send(&mut ws, "CombatPlayed", json!({}), &mut conn.seq).await?;
            }
            _ => {}
        }
    }
}

/// 是否满足开始条件：8 名真人全部准备。
///
/// `RoomUpdate.players` 只含真人（机器人由服务端在开局时补入）。机器人房主只出现在
/// 编排式入口与手动 `--create` 中，这些流程都在打印房间码后等待其余真人入房，因此
/// 不能因为补位开启就在真人不足 8 时提前开局：后到的入房者会被 `INVALID_STAGE` 拒绝。
fn all_ready(update: &RoomUpdate) -> bool {
    update.players.len() == usize::from(PLAYER_COUNT)
        && update.players.iter().all(|player| player.ready)
}

/// 本人在选英雄视图里是否已确认；按展示名匹配，机器人名字在房内唯一。
fn picked_for_name(pick: &HeroPickStarted, name: &str) -> bool {
    pick.players
        .iter()
        .any(|player| player.display_name == name && player.picked)
}

/// 从本人候选里选英雄：优先配置英雄，不在候选内则取第一张。
///
/// 候选只来自服务端 `HeroPickStarted`，机器人不做随机或卡表推断。
///
/// # Errors
///
/// 本人候选为空时返回错误，避免发送空 `cardId` 被服务端拒绝。
pub fn pick_hero(pick: &HeroPickStarted, preferred: &str) -> Result<String, String> {
    if pick.candidates.is_empty() {
        return Err("选英雄阶段没有本人候选".into());
    }
    if pick.candidates.iter().any(|card| card == preferred) {
        Ok(preferred.to_string())
    } else {
        Ok(pick.candidates[0].clone())
    }
}

/// 由机器人名字派生稳定且互不相同的手机号：同一手机号会恢复同一账号，
/// 多台机器人必须各自拥有独立身份。
fn phone_for(name: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in name.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("139{:08}", hash % 100_000_000)
}

/// 发送阶段级事件；接收端不存在时静默丢弃。
fn emit(options: &BotOptions, event: BotEvent) {
    if let Some(events) = &options.events {
        let _ = events.send(event);
    }
}

/// 等待停止信号；未配置停止来源时永不返回。
async fn wait_stop(stop: &mut Option<watch::Receiver<bool>>) {
    let Some(receiver) = stop.as_mut() else {
        std::future::pending::<()>().await;
        return;
    };
    if *receiver.borrow() {
        return;
    }
    let _ = receiver.changed().await;
}

fn stop_requested(stop: &Option<watch::Receiver<bool>>) -> bool {
    stop.as_ref().is_some_and(|receiver| *receiver.borrow())
}

/// 发送 `LeaveRoom` 并关闭连接；发送失败也照常按已停止返回。
async fn leave_room(ws: &mut Ws, seq: &mut u64) -> Result<BotFinish, String> {
    let _ = send(ws, "LeaveRoom", json!({}), seq).await;
    let _ = ws.close(None).await;
    Ok(BotFinish::Left)
}

/// 认证；返回 `false` 表示收到停止信号、尚未认证完成。
async fn authenticate(
    ws: &mut Ws,
    config: &BotConfig,
    options: &mut BotOptions,
    seq: &mut u64,
) -> Result<bool, String> {
    let phone = options
        .phone
        .clone()
        .unwrap_or_else(|| phone_for(&config.name));
    send(
        ws,
        "AuthRequest",
        json!({
            "phone": phone,
            "code": crate::bot::DEV_SMS_CODE,
            "displayName": config.name,
        }),
        seq,
    )
    .await?;

    loop {
        let message = tokio::select! {
            _ = wait_stop(&mut options.stop) => return Ok(false),
            message = receive(ws) => message?,
        };
        let Some(message) = message else {
            return Err("连接在认证完成前关闭".into());
        };
        match message {
            ServerMessage::AuthResult(_) => return Ok(true),
            ServerMessage::Error(error) => {
                return Err(format!("认证失败: {} {}", error.code, error.message));
            }
            _ => {}
        }
    }
}

async fn wait_room_update(
    ws: &mut Ws,
    options: &mut BotOptions,
) -> Result<Option<RoomUpdate>, String> {
    loop {
        let message = tokio::select! {
            _ = wait_stop(&mut options.stop) => return Ok(None),
            message = receive(ws) => message?,
        };
        let Some(message) = message else {
            return Err("连接在进入房间前关闭".into());
        };
        match message {
            ServerMessage::RoomUpdate(update) => return Ok(Some(update)),
            ServerMessage::Error(error) => {
                return Err(format!("进入房间失败: {} {}", error.code, error.message));
            }
            _ => {}
        }
    }
}

/// 机器人跨消息保留的决策记忆。
struct BotMemory {
    /// 上一次已决策视图的指纹。
    fingerprint: String,
    /// 最近一次已决策的视图，用于错误兜底。
    view: Option<PlayerView>,
    /// 已发送的意图数，参与 `clientIntentId`。
    intents: u64,
    /// 最近一次补发兜底意图的回合。
    recovery_turn: Option<u8>,
    /// 尚未确认的意图，按 `clientIntentId` 找回被拒的意图。
    pending: Vec<(String, Intent)>,
    /// 本回合已确认不合法的目标选择；换回合时清空。
    blocked: brain::BlockedTargets,
    /// `blocked` 对应的回合。
    blocked_turn: Option<u8>,
}

/// 未确认意图的保留上限；只需覆盖「发出一条后收到下一条错误」的流水线深度。
const PENDING_LIMIT: usize = 8;

impl BotMemory {
    fn new() -> Self {
        Self {
            fingerprint: String::new(),
            view: None,
            intents: 0,
            recovery_turn: None,
            pending: Vec::new(),
            blocked: brain::BlockedTargets::default(),
            blocked_turn: None,
        }
    }

    /// 换回合时清空目标拒绝记录；同回合的多次决策保留。
    fn reset_blocked_for_turn(&mut self, turn: u8) {
        if self.blocked_turn != Some(turn) {
            self.blocked_turn = Some(turn);
            self.blocked = brain::BlockedTargets::default();
        }
    }

    fn next_intent_id(&mut self, name: &str) -> String {
        let id = format!("{name}-{}", self.intents);
        self.intents = self.intents.saturating_add(1);
        id
    }

    /// 记下一条已发出的意图，供被拒后识别被拒的是哪一条。
    fn remember(&mut self, client_intent_id: String, intent: Intent) {
        if self.pending.len() >= PENDING_LIMIT {
            self.pending.remove(0);
        }
        self.pending.push((client_intent_id, intent));
    }

    /// 按 `clientIntentId` 找回被拒意图。
    fn rejected(&self, client_intent_id: &str) -> Option<&Intent> {
        self.pending
            .iter()
            .rev()
            .find(|(id, _)| id == client_intent_id)
            .map(|(_, intent)| intent)
    }
}

/// 对一份视图做一次决策；视图与上次已决策的完全一致时跳过。
///
/// `force` 用于目标拒绝之后：拒绝不改变视图，但本回合的拒绝集合变了，必须立即
/// 换一张牌或换一个待选目标，否则机器人会停在原地等回合计时。
async fn decide(
    ws: &mut Ws,
    config: &BotConfig,
    catalog: &Catalog,
    view: &PlayerView,
    conn: &mut Conn,
    force: bool,
) -> Result<(), String> {
    let self_json = serde_json::to_string(&view.self_).unwrap_or_default();
    let fingerprint = format!("{:?}|{}|{self_json}", view.phase, view.turn);
    if !force && fingerprint == conn.memory.fingerprint {
        return Ok(());
    }
    conn.memory.fingerprint = fingerprint;
    conn.memory.view = Some(view.clone());
    conn.memory.reset_blocked_for_turn(turn_of(view));

    let Some(intent) = brain::next_intent(view, catalog, config.intent_seed, &conn.memory.blocked)
    else {
        return Ok(());
    };
    send_intent(ws, &config.name, conn, intent).await
}

/// 发送一条意图并登记 `clientIntentId`，供被拒后找回原始参数。
async fn send_intent(
    ws: &mut Ws,
    name: &str,
    conn: &mut Conn,
    intent: Intent,
) -> Result<(), String> {
    let intent_value =
        serde_json::to_value(&intent).map_err(|err| format!("序列化意图失败: {err}"))?;
    let client_intent_id = conn.memory.next_intent_id(name);
    conn.memory.remember(client_intent_id.clone(), intent);
    send(
        ws,
        "Intent",
        json!({ "clientIntentId": client_intent_id, "intent": intent_value }),
        &mut conn.seq,
    )
    .await?;
    if let Some(counters) = &conn.counters {
        counters.intents_sent.fetch_add(1, Ordering::Relaxed);
    }
    Ok(())
}

fn turn_of(view: &PlayerView) -> u8 {
    u8::try_from(view.turn.get()).unwrap_or(u8::MAX)
}

/// 目标被拒且已无候选可换时的提示。
///
/// `NO_VALID_TARGET` 是合法拒绝（目标池被玩法、分类等筛选收窄，或视图与结算有偏差）；
/// 能换候选时机器人静默重试，只有候选确实耗尽、本轮放弃时才输出一行。
fn log_target_skip(name: &str, rejected: &Intent) {
    match rejected {
        Intent::PlayCard { hand_index, .. } => {
            eprintln!("bot {name} 候选目标都已拒绝，本回合跳过手牌 {hand_index}");
        }
        Intent::ChooseTarget { target } => {
            eprintln!("bot {name} 目标待选的候选都已拒绝，放弃目标 {target:?}");
        }
        _ => eprintln!("bot {name} 目标不合法，本回合跳过该操作"),
    }
}

async fn send(ws: &mut Ws, kind: &str, payload: Value, seq: &mut u64) -> Result<(), String> {
    *seq = seq.saturating_add(1);
    let envelope = json!({
        "v": 1,
        "id": format!("bot_{seq}"),
        "type": kind,
        "ts": "2026-01-01T00:00:00Z",
        "payload": payload,
    });
    let text =
        serde_json::to_string(&envelope).map_err(|err| format!("序列化上行信封失败: {err}"))?;
    ws.send(Message::Text(text.into()))
        .await
        .map_err(|err| format!("WebSocket 发送失败: {err}"))
}

async fn receive(ws: &mut Ws) -> Result<Option<ServerMessage>, String> {
    while let Some(item) = ws.next().await {
        match item {
            Ok(Message::Text(text)) => {
                let envelope: ServerEnvelope = serde_json::from_str(text.as_str())
                    .map_err(|err| format!("解析下行信封失败: {err}"))?;
                let message = envelope
                    .parse()
                    .map_err(|err| format!("解析下行载荷失败: {err}"))?;
                return Ok(Some(message));
            }
            Ok(Message::Close(_)) => return Ok(None),
            Ok(_) => continue,
            Err(err) => return Err(format!("WebSocket 读取失败: {err}")),
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::pick_hero;
    use protocol::HeroPickStartedPlayersItem;
    use protocol::server::HeroPickStarted;

    fn pick(candidates: &[&str]) -> HeroPickStarted {
        HeroPickStarted {
            candidates: candidates.iter().map(|card| (*card).to_string()).collect(),
            deadline: "2026-01-01T00:00:00Z".to_string(),
            players: vec![HeroPickStartedPlayersItem {
                player_id: "bot0".to_string(),
                display_name: "bot0".to_string(),
                is_bot: false,
                picked: false,
                seat: 0,
            }],
        }
    }

    #[test]
    fn pick_hero_prefers_configured_card() {
        assert_eq!(pick_hero(&pick(&["a", "b", "c"]), "b").unwrap(), "b");
    }

    #[test]
    fn pick_hero_falls_back_to_first_candidate() {
        assert_eq!(pick_hero(&pick(&["a", "b"]), "missing").unwrap(), "a");
    }

    #[test]
    fn pick_hero_rejects_empty_candidates() {
        assert!(pick_hero(&pick(&[]), "b").is_err());
    }
}
