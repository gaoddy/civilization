//! `sim bot --connect`：真实 WebSocket 客户端。
//!
//! 协议顺序固定为：`AuthRequest` →（`CreateRoom` | `JoinRoom`）→
//! `SelectHeroCard` → `Ready` → 等 `GameStarted` → 循环按 [`brain`] 决策发送
//! `IntentMessage`，直到 `GameOver`。
//!
//! 每个意图使用确定性递增的 `clientIntentId`（`{name}-{counter}`）。客户端只对
//! 「自身视角有新变化」的视图做一次决策，避免重连广播导致的重复意图。

use futures_util::{SinkExt, StreamExt};
use protocol::ErrorCode;
use protocol::ServerEnvelope;
use protocol::ServerMessage;
use protocol::intents::Intent;
use protocol::server::RoomUpdate;
use protocol::view::PlayerView;
use rule_engine::data::Catalog;
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::tungstenite::Message;

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

type Ws = tokio_tungstenite::WebSocketStream<MaybeTlsStream<TcpStream>>;

/// 连上服务端并打完一局。
///
/// # Errors
///
/// 连接失败、协议消息无法解析、服务端返回认证/入房错误，或连接在对局结束前关闭。
pub async fn run(config: BotConfig) -> Result<BotOutcome, String> {
    let mut config = config;
    let (mut ws, _response) = tokio_tungstenite::connect_async(&config.url)
        .await
        .map_err(|err| format!("连接 {} 失败: {err}", config.url))?;

    let mut seq = 0_u64;
    let catalog = crate::embedded_catalog().map_err(|err| err.to_string())?;

    authenticate(&mut ws, &config, &mut seq).await?;

    if config.create {
        send(&mut ws, "CreateRoom", json!({}), &mut seq).await?;
    } else {
        if config.room.is_empty() {
            return Err("加入房间时必须提供 --room".into());
        }
        send(
            &mut ws,
            "JoinRoom",
            json!({ "code": config.room }),
            &mut seq,
        )
        .await?;
    }
    let room_update = wait_room_update(&mut ws).await?;
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

    // 英雄只能从服务端分配给本人的候选里选：优先用配置的英雄，不在候选里就退到第一张。
    let hero = pick_hero(&room_update, &config.hero)?;
    send(
        &mut ws,
        "SelectHeroCard",
        json!({ "cardId": hero }),
        &mut seq,
    )
    .await?;
    send(&mut ws, "Ready", json!({ "ready": true }), &mut seq).await?;

    let mut turns = 1_u8;
    let mut memory = BotMemory::new();
    while let Some(message) = receive(&mut ws).await? {
        match message {
            ServerMessage::GameStarted(started) => {
                turns = turn_of(&started.view);
                decide(
                    &mut ws,
                    &config,
                    &catalog,
                    &started.view,
                    &mut memory,
                    &mut seq,
                    false,
                )
                .await?;
            }
            ServerMessage::Snapshot(snapshot) => {
                turns = turn_of(&snapshot.view);
                decide(
                    &mut ws,
                    &config,
                    &catalog,
                    &snapshot.view,
                    &mut memory,
                    &mut seq,
                    false,
                )
                .await?;
            }
            ServerMessage::StateUpdate(update) => {
                turns = turn_of(&update.view);
                decide(
                    &mut ws,
                    &config,
                    &catalog,
                    &update.view,
                    &mut memory,
                    &mut seq,
                    false,
                )
                .await?;
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
                return Ok(BotOutcome { ranking, turns });
            }
            ServerMessage::Error(error) => {
                // 目标不合法是预期反馈：视图只给 `requiresTarget`、不给目标池，
                // 预填的随从可能不在池内。出手沿棋盘静默换下一个，整盘都不合法时
                // 才放弃这张牌；`ChooseTarget` 逐个轮换候选。
                if error.code == ErrorCode::NoValidTarget
                    && let Some(rejected) = error
                        .client_intent_id
                        .as_deref()
                        .and_then(|id| memory.rejected(id).cloned())
                {
                    let turn = memory.view.as_ref().map_or(0, turn_of);
                    memory.reset_blocked_for_turn(turn);
                    match &rejected {
                        Intent::PlayCard { hand_index, .. } => {
                            let retried = memory
                                .view
                                .as_ref()
                                .and_then(|view| brain::retry_with_target(view, &rejected));
                            match retried {
                                Some(intent) => {
                                    send_intent(
                                        &mut ws,
                                        &config.name,
                                        &mut memory,
                                        &mut seq,
                                        intent,
                                    )
                                    .await?;
                                }
                                None => {
                                    memory.blocked.block_hand(*hand_index);
                                    log_target_skip(&config.name, &rejected);
                                    if let Some(view) = memory.view.clone() {
                                        decide(
                                            &mut ws,
                                            &config,
                                            &catalog,
                                            &view,
                                            &mut memory,
                                            &mut seq,
                                            true,
                                        )
                                        .await?;
                                    }
                                }
                            }
                        }
                        Intent::ChooseTarget { minion_id } => {
                            memory.blocked.block_minion(minion_id.clone());
                            let exhausted = memory.view.as_ref().is_some_and(|view| {
                                !memory.blocked.has_unblocked_minion(&view.self_)
                            });
                            if exhausted {
                                log_target_skip(&config.name, &rejected);
                            } else if let Some(view) = memory.view.clone() {
                                decide(
                                    &mut ws,
                                    &config,
                                    &catalog,
                                    &view,
                                    &mut memory,
                                    &mut seq,
                                    true,
                                )
                                .await?;
                            }
                        }
                        _ => {}
                    }
                    continue;
                }
                eprintln!(
                    "bot {} 收到错误 {}: {}",
                    config.name, error.code, error.message
                );
                // 被拒绝的意图不会带来新视图，机器人不会自己重试；补发兜底意图，
                // 避免整局停在招募阶段等超时。每个回合最多补发一次。
                let recovery = memory.view.as_ref().and_then(|view| {
                    let turn = turn_of(view);
                    if memory.recovery_turn == Some(turn) {
                        return None;
                    }
                    brain::recover_after_error(view).map(|intent| (turn, intent))
                });
                if let Some((turn, intent)) = recovery {
                    memory.recovery_turn = Some(turn);
                    send_intent(&mut ws, &config.name, &mut memory, &mut seq, intent).await?;
                }
            }
            ServerMessage::CombatResult(_) => {
                // 机器人不做演示动画：收到即回报，避免阻塞同一局的真人客户端。
                send(&mut ws, "CombatPlayed", json!({}), &mut seq).await?;
            }
            _ => {}
        }
    }

    Err("连接在对局结束前关闭".into())
}

async fn authenticate(ws: &mut Ws, config: &BotConfig, seq: &mut u64) -> Result<(), String> {
    send(
        ws,
        "AuthRequest",
        json!({
            "phone": "13800000000",
            "code": crate::bot::DEV_SMS_CODE,
            "displayName": config.name,
        }),
        seq,
    )
    .await?;

    while let Some(message) = receive(ws).await? {
        match message {
            ServerMessage::AuthResult(_) => return Ok(()),
            ServerMessage::Error(error) => {
                return Err(format!("认证失败: {} {}", error.code, error.message));
            }
            _ => {}
        }
    }
    Err("连接在认证完成前关闭".into())
}

/// 从本人的 `RoomUpdate` 里挑一张候选英雄。
///
/// 服务端只给本人填充 `heroCandidates`，机器人必须在这组候选里选；
/// 配置里的英雄不在候选内时退到第一张，保证机器人总能进入准备状态。
fn pick_hero(update: &RoomUpdate, preferred: &str) -> Result<String, String> {
    let candidates = update
        .players
        .iter()
        .flat_map(|item| item.hero_candidates.clone().unwrap_or_default())
        .collect::<Vec<_>>();
    if candidates.iter().any(|id| id == preferred) {
        return Ok(preferred.to_string());
    }
    candidates
        .into_iter()
        .next()
        .ok_or_else(|| "服务端未下发英雄候选".to_string())
}

async fn wait_room_update(ws: &mut Ws) -> Result<RoomUpdate, String> {
    while let Some(message) = receive(ws).await? {
        match message {
            ServerMessage::RoomUpdate(update) => return Ok(update),
            ServerMessage::Error(error) => {
                return Err(format!("进入房间失败: {} {}", error.code, error.message));
            }
            _ => {}
        }
    }
    Err("连接在进入房间前关闭".into())
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
    memory: &mut BotMemory,
    seq: &mut u64,
    force: bool,
) -> Result<(), String> {
    let self_json = serde_json::to_string(&view.self_).unwrap_or_default();
    let fingerprint = format!("{:?}|{}|{self_json}", view.phase, view.turn);
    if !force && fingerprint == memory.fingerprint {
        return Ok(());
    }
    memory.fingerprint = fingerprint;
    memory.view = Some(view.clone());
    memory.reset_blocked_for_turn(turn_of(view));

    let Some(intent) = brain::next_intent(view, catalog, config.intent_seed, &memory.blocked)
    else {
        return Ok(());
    };
    send_intent(ws, &config.name, memory, seq, intent).await
}

/// 发送一条意图并登记 `clientIntentId`，供被拒后找回原始参数。
async fn send_intent(
    ws: &mut Ws,
    name: &str,
    memory: &mut BotMemory,
    seq: &mut u64,
    intent: Intent,
) -> Result<(), String> {
    let intent_value =
        serde_json::to_value(&intent).map_err(|err| format!("序列化意图失败: {err}"))?;
    let client_intent_id = memory.next_intent_id(name);
    memory.remember(client_intent_id.clone(), intent);
    send(
        ws,
        "Intent",
        json!({ "clientIntentId": client_intent_id, "intent": intent_value }),
        seq,
    )
    .await
}

fn turn_of(view: &PlayerView) -> u8 {
    u8::try_from(view.turn.get()).unwrap_or(u8::MAX)
}

/// 目标被拒且已无候选可换时的提示。
///
/// `NO_VALID_TARGET` 是合法拒绝（目标池被玩法、分类等筛选收窄）；能换候选时
/// 机器人静默重试，只有整盘随从都不合法、本轮确实放弃时才输出一行。
fn log_target_skip(name: &str, rejected: &Intent) {
    match rejected {
        Intent::PlayCard { hand_index, .. } => {
            eprintln!("bot {name} 棋盘上没有合法目标，本回合跳过手牌 {hand_index}");
        }
        Intent::ChooseTarget { minion_id } => {
            eprintln!("bot {name} 目标待选的候选都已拒绝，放弃随从 {minion_id}");
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
