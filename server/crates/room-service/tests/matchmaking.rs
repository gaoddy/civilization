//! 自动匹配集成测试：真实 WebSocket、固定时钟与 8 人成局。
//!
//! 覆盖入队回执、重复入队、进房拒绝、取消排队、断线移出队列，以及 8 名真实会话
//! 经排队进入同一 `ranked` 房间并收到 `MatchFound` 与 `GameStarted`。

mod common;

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use common::{ClientWs, connect_with_retry, start_server_with_clock};
use futures_util::{SinkExt, StreamExt};
use protocol::server::{GameStarted, MatchFound, QueueState};
use protocol::{ErrorCode, ServerEnvelope, ServerMessage};
use room_service::clock::FixedClock;
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message;

struct RawClient {
    ws: ClientWs,
    next_id: u64,
}

impl RawClient {
    async fn connect(addr: std::net::SocketAddr) -> Self {
        let ws = connect_with_retry(&format!("ws://{addr}/ws")).await;
        Self { ws, next_id: 0 }
    }

    async fn send(&mut self, kind: &str, payload: Value) {
        self.next_id += 1;
        let envelope = json!({
            "v": 1,
            "id": format!("raw_{}", self.next_id),
            "type": kind,
            "ts": "2026-01-01T00:00:00Z",
            "payload": payload,
        });
        self.ws
            .send(Message::Text(envelope.to_string().into()))
            .await
            .expect("发送应当成功");
    }

    /// 取下一条下行消息；跳过认证后附带的账号与社交推送。
    async fn recv(&mut self) -> Option<ServerMessage> {
        loop {
            let item = tokio::time::timeout(Duration::from_secs(5), self.ws.next())
                .await
                .expect("5 秒内应有下行消息");
            match item {
                Some(Ok(Message::Text(text))) => {
                    let envelope: ServerEnvelope =
                        serde_json::from_str(text.as_str()).expect("下行信封应能解析");
                    let message = envelope.parse().expect("下行载荷应能解析");
                    if matches!(
                        message,
                        ServerMessage::FriendUpdate(_)
                            | ServerMessage::AccountView(_)
                            | ServerMessage::AchievementState(_)
                            | ServerMessage::AchievementUnlocked(_)
                    ) {
                        continue;
                    }
                    return Some(message);
                }
                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => return None,
                Some(Ok(_)) => continue,
            }
        }
    }
}

/// 由名字派生稳定且互不相同的手机号：同一手机号会恢复同一账号。
fn phone_for(name: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in name.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("137{:08}", hash % 100_000_000)
}

async fn authenticate(client: &mut RawClient, name: &str) -> String {
    client
        .send(
            "AuthRequest",
            json!({ "phone": phone_for(name), "code": "123456", "displayName": name }),
        )
        .await;
    loop {
        match client.recv().await.expect("认证期间连接不应关闭") {
            ServerMessage::AuthResult(result) => return result.player_id,
            ServerMessage::Error(error) => panic!("认证失败: {} {}", error.code, error.message),
            _ => {}
        }
    }
}

/// 读一条 `QueueState`；遇到其他消息跳过。
async fn wait_queue_state(client: &mut RawClient) -> QueueState {
    loop {
        match client.recv().await.expect("等待 QueueState 时连接关闭") {
            ServerMessage::QueueState(state) => return state,
            ServerMessage::Error(error) => panic!("等待 QueueState 时收到错误: {error:?}"),
            _ => {}
        }
    }
}

/// 读一条 `MatchFound`，同时断言入房之前没有下发大厅 `RoomUpdate`。
async fn wait_match_found(client: &mut RawClient) -> MatchFound {
    loop {
        match client.recv().await.expect("等待 MatchFound 时连接关闭") {
            ServerMessage::MatchFound(found) => return found,
            ServerMessage::RoomUpdate(_) => panic!("匹配房不应下发大厅 RoomUpdate"),
            ServerMessage::QueueState(_) => {}
            other => panic!("匹配前收到意外消息: {other:?}"),
        }
    }
}

async fn wait_game_started(client: &mut RawClient) -> GameStarted {
    loop {
        match client.recv().await.expect("等待 GameStarted 时连接关闭") {
            ServerMessage::GameStarted(started) => return started,
            _ => {}
        }
    }
}

#[tokio::test]
async fn eight_players_match_into_one_ranked_room() {
    let clock = Arc::new(FixedClock::new(1_000_000));
    let addr = start_server_with_clock(clock).await;

    let mut clients = Vec::with_capacity(8);
    let mut waiting = Vec::with_capacity(8);
    for index in 0..8 {
        let mut client = RawClient::connect(addr).await;
        let player_id = authenticate(&mut client, &format!("match{index}")).await;
        client.send("JoinQueue", json!({})).await;
        let state = wait_queue_state(&mut client).await;
        assert_eq!(state.waiting_ms, 0, "刚入队等待时长应为 0");
        assert!(state.waiting_players >= 1, "至少包含本人");
        assert_eq!(
            state.rating_window,
            Some(150),
            "初始匹配窗口应为 ±150 分"
        );
        waiting.push(player_id);
        clients.push(client);
    }

    let mut room_ids = BTreeSet::new();
    let mut seats = BTreeSet::new();
    for client in &mut clients {
        let found = wait_match_found(client).await;
        room_ids.insert(found.room_id.clone());
        assert!((0..8).contains(&found.seat), "座位号应在 0..8");
        assert!(seats.insert(found.seat), "同一局不能出现重复座位");
    }
    assert_eq!(room_ids.len(), 1, "8 人应进入同一个匹配房");
    assert_eq!(seats.len(), 8, "8 个座位应全部占用");

    for client in &mut clients {
        let started = wait_game_started(client).await;
        assert_eq!(started.view.players.len(), 8, "开局视图应包含 8 名玩家");
    }
    assert_eq!(waiting.len(), 8);
}

#[tokio::test]
async fn duplicate_queue_join_is_rejected() {
    let clock = Arc::new(FixedClock::new(1_000_000));
    let addr = start_server_with_clock(clock).await;
    let mut client = RawClient::connect(addr).await;
    authenticate(&mut client, "duplicate").await;

    client.send("JoinQueue", json!({})).await;
    let _ = wait_queue_state(&mut client).await;
    client.send("JoinQueue", json!({})).await;
    match client.recv().await.expect("应收到错误") {
        ServerMessage::Error(error) => assert_eq!(error.code, ErrorCode::QueueAlready),
        other => panic!("重复入队应返回 QUEUE_ALREADY，实际 {other:?}"),
    }
}

#[tokio::test]
async fn leaving_queue_frees_the_slot() {
    let clock = Arc::new(FixedClock::new(1_000_000));
    let addr = start_server_with_clock(clock).await;
    let mut client = RawClient::connect(addr).await;
    authenticate(&mut client, "cancel").await;

    client.send("JoinQueue", json!({})).await;
    let first = wait_queue_state(&mut client).await;
    assert_eq!(first.waiting_players, 1);

    client.send("LeaveQueue", json!({})).await;
    // 取消不扣分也不回错误；再次入队必须成功而不是 QUEUE_ALREADY。
    client.send("JoinQueue", json!({})).await;
    let second = wait_queue_state(&mut client).await;
    assert_eq!(second.waiting_players, 1, "取消后队列应只剩本人");
}

#[tokio::test]
async fn join_queue_while_in_room_is_rejected() {
    let clock = Arc::new(FixedClock::new(1_000_000));
    let addr = start_server_with_clock(clock).await;
    let mut client = RawClient::connect(addr).await;
    authenticate(&mut client, "in-room").await;

    client.send("CreateRoom", json!({})).await;
    loop {
        match client.recv().await.expect("建房期间连接不应关闭") {
            ServerMessage::RoomUpdate(_) => break,
            ServerMessage::Error(error) => panic!("建房失败: {error:?}"),
            _ => {}
        }
    }
    client.send("JoinQueue", json!({})).await;
    match client.recv().await.expect("应收到错误") {
        ServerMessage::Error(error) => assert_eq!(error.code, ErrorCode::QueueInRoom),
        other => panic!("房间内入队应返回 QUEUE_IN_ROOM，实际 {other:?}"),
    }
}

#[tokio::test]
async fn disconnect_removes_player_from_queue() {
    let clock = Arc::new(FixedClock::new(1_000_000));
    let addr = start_server_with_clock(clock).await;

    let mut first = RawClient::connect(addr).await;
    authenticate(&mut first, "drop").await;
    first.send("JoinQueue", json!({})).await;
    let state = wait_queue_state(&mut first).await;
    assert_eq!(state.waiting_players, 1);
    drop(first);

    // 等驱动按在线状态清理；随后新玩家入队时应只看到自己。
    tokio::time::sleep(Duration::from_millis(600)).await;
    let mut second = RawClient::connect(addr).await;
    authenticate(&mut second, "after-drop").await;
    second.send("JoinQueue", json!({})).await;
    let state = wait_queue_state(&mut second).await;
    assert_eq!(state.waiting_players, 1, "断线玩家应已移出队列");
}
