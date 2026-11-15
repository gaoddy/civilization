//! `sim bot --connect`：真实 WebSocket 客户端。
//!
//! 协议顺序固定为：`AuthRequest` →（`CreateRoom` | `JoinRoom`）→
//! `SelectHeroCard` → `Ready` → 等 `GameStarted` → 循环按 [`brain`] 决策发送
//! `IntentMessage`，直到 `GameOver`。
//!
//! 每个意图使用确定性递增的 `clientIntentId`（`{name}-{counter}`）。客户端只对
//! 「自身视角有新变化」的视图做一次决策，避免重连广播导致的重复意图。

use futures_util::{SinkExt, StreamExt};
use protocol::ServerEnvelope;
use protocol::ServerMessage;
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
    /// true 表示建房并把房间码输出到 stdout（首行 `ROOM_CODE=<code>`）。
    pub create: bool,
    /// brain 在发现候选间做确定性选择时使用的种子。
    pub intent_seed: u64,
    /// 供测试在进程内取到房间码；CLI 不使用。
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
    let mut intent_counter = 0_u64;
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
    let room_code = wait_room_update(&mut ws).await?;
    if config.create {
        if let Some(sink) = config.room_code_sink.take() {
            let _ = sink.send(room_code.clone());
        }
        println!("ROOM_CODE={room_code}");
    }

    send(
        &mut ws,
        "SelectHeroCard",
        json!({ "cardId": config.hero }),
        &mut seq,
    )
    .await?;
    send(&mut ws, "Ready", json!({ "ready": true }), &mut seq).await?;

    let mut turns = 1_u8;
    let mut last_fingerprint = String::new();
    while let Some(message) = receive(&mut ws).await? {
        match message {
            ServerMessage::GameStarted(started) => {
                turns = turn_of(&started.view);
                decide(
                    &mut ws,
                    &config,
                    &catalog,
                    &started.view,
                    &mut last_fingerprint,
                    &mut intent_counter,
                    &mut seq,
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
                    &mut last_fingerprint,
                    &mut intent_counter,
                    &mut seq,
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
                    &mut last_fingerprint,
                    &mut intent_counter,
                    &mut seq,
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
                eprintln!(
                    "bot {} 收到错误 {}: {}",
                    config.name, error.code, error.message
                );
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

async fn wait_room_update(ws: &mut Ws) -> Result<String, String> {
    while let Some(message) = receive(ws).await? {
        match message {
            ServerMessage::RoomUpdate(update) => return Ok(update.code),
            ServerMessage::Error(error) => {
                return Err(format!("进入房间失败: {} {}", error.code, error.message));
            }
            _ => {}
        }
    }
    Err("连接在进入房间前关闭".into())
}

/// 对一份视图做一次决策；视图与上次已决策的完全一致时跳过。
async fn decide(
    ws: &mut Ws,
    config: &BotConfig,
    catalog: &Catalog,
    view: &PlayerView,
    last_fingerprint: &mut String,
    intent_counter: &mut u64,
    seq: &mut u64,
) -> Result<(), String> {
    let self_json = serde_json::to_string(&view.self_).unwrap_or_default();
    let fingerprint = format!("{:?}|{}|{self_json}", view.phase, view.turn);
    if fingerprint == *last_fingerprint {
        return Ok(());
    }
    *last_fingerprint = fingerprint;

    let Some(intent) = brain::next_intent(view, catalog, config.intent_seed) else {
        return Ok(());
    };
    let client_intent_id = format!("{}-{intent_counter}", config.name);
    *intent_counter = intent_counter.saturating_add(1);
    let intent_value =
        serde_json::to_value(&intent).map_err(|err| format!("序列化意图失败: {err}"))?;
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
