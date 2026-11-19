//! 好友与邀请链路的端到端测试：真实 WebSocket、申请/同意、邀请入房与全部拒绝分支。

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use common::{ClientWs, connect_with_retry, start_server, start_server_with_clock};
use futures_util::{SinkExt, StreamExt};
use protocol::{ErrorCode, FriendUpdate, RoomUpdate, ServerEnvelope, ServerMessage};
use room_service::FixedClock;
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message;

static PHONE_SEQ: AtomicU32 = AtomicU32::new(1);

struct RawClient {
    ws: ClientWs,
    next_id: u64,
    player_id: String,
    player_code: String,
    name: String,
}

impl RawClient {
    async fn connect(addr: std::net::SocketAddr, name: &str) -> Self {
        let ws = connect_with_retry(&format!("ws://{addr}/ws")).await;
        let mut client = Self {
            ws,
            next_id: 0,
            player_id: String::new(),
            player_code: String::new(),
            name: name.to_string(),
        };
        client.authenticate().await;
        client
    }

    async fn send(&mut self, kind: &str, payload: Value) {
        self.next_id += 1;
        let envelope = json!({
            "v": 1,
            "id": format!("{}-{}", self.name, self.next_id),
            "type": kind,
            "ts": "2026-01-01T00:00:00Z",
            "payload": payload,
        });
        self.ws
            .send(Message::Text(envelope.to_string().into()))
            .await
            .expect("发送应当成功");
    }

    async fn recv(&mut self) -> Option<ServerMessage> {
        while let Some(item) = self.ws.next().await {
            match item {
                Ok(Message::Text(text)) => {
                    let envelope: ServerEnvelope =
                        serde_json::from_str(text.as_str()).expect("下行信封应能解析");
                    return Some(envelope.parse().expect("下行载荷应能解析"));
                }
                Ok(Message::Close(_)) => return None,
                Ok(_) => continue,
                Err(_) => return None,
            }
        }
        None
    }

    /// 等待满足条件的下行消息；期间收到 `Error` 直接 panic（调用方等的是成功路径）。
    async fn wait_for<T>(
        &mut self,
        label: &str,
        mut extract: impl FnMut(&ServerMessage) -> Option<T>,
    ) -> T {
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let message = self.recv().await.expect("等待期间连接不应关闭");
                if let ServerMessage::Error(error) = &message {
                    panic!("{label} 收到错误 {}: {}", error.code, error.message);
                }
                if let Some(value) = extract(&message) {
                    return value;
                }
            }
        })
        .await
        .unwrap_or_else(|_| panic!("{label} 10 秒内未到达"))
    }

    async fn wait_error(&mut self, code: ErrorCode) {
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let message = self.recv().await.expect("等待错误期间连接不应关闭");
                if let ServerMessage::Error(error) = message {
                    assert_eq!(error.code, code);
                    return;
                }
            }
        })
        .await
        .unwrap_or_else(|_| panic!("10 秒内未收到错误 {code:?}"))
    }

    async fn authenticate(&mut self) {
        // 每个客户端用独立手机号：账号持久化后同号会复用同一账号。
        let phone = format!("139{:08}", PHONE_SEQ.fetch_add(1, Ordering::Relaxed));
        self.send(
            "AuthRequest",
            json!({ "phone": phone, "code": "123456", "displayName": self.name }),
        )
        .await;
        let (player_id, player_code) = self
            .wait_for("认证", |message| match message {
                ServerMessage::AuthResult(result) => {
                    Some((result.player_id.clone(), result.player_code.clone()))
                }
                _ => None,
            })
            .await;
        self.player_id = player_id;
        self.player_code = player_code;
    }

    async fn create_room(&mut self) -> RoomUpdate {
        self.send("CreateRoom", json!({})).await;
        self.wait_for("房间更新", |message| match message {
            ServerMessage::RoomUpdate(update) if update.players.len() == 1 => Some(update.clone()),
            _ => None,
        })
        .await
    }

    async fn join_room(&mut self, code: &str) -> RoomUpdate {
        self.send("JoinRoom", json!({ "code": code })).await;
        let player_id = self.player_id.clone();
        self.wait_for("入房更新", |message| match message {
            ServerMessage::RoomUpdate(update)
                if update.players.iter().any(|player| {
                    player.player_id == player_id && player.hero_candidates.is_some()
                }) =>
            {
                Some(update.clone())
            }
            _ => None,
        })
        .await
    }

    /// 取本人候选第一张并确认准备。
    async fn select_hero_and_ready(&mut self, update: &RoomUpdate) {
        let hero = update
            .players
            .iter()
            .find(|player| player.player_id == self.player_id)
            .and_then(|player| player.hero_candidates.as_ref())
            .and_then(|list| list.first())
            .expect("应拿到本人候选")
            .clone();
        self.send("SelectHeroCard", json!({ "cardId": hero })).await;
        self.send("Ready", json!({ "ready": true })).await;
    }

    async fn wait_room_update(&mut self, predicate: impl Fn(&RoomUpdate) -> bool) -> RoomUpdate {
        self.wait_for("房间更新", |message| match message {
            ServerMessage::RoomUpdate(update) if predicate(update) => Some(update.clone()),
            _ => None,
        })
        .await
    }

    async fn wait_game_started(&mut self) {
        self.wait_for("开局", |message| match message {
            ServerMessage::GameStarted(_) => Some(()),
            _ => None,
        })
        .await;
    }

    async fn wait_friend_update(
        &mut self,
        label: &str,
        predicate: impl Fn(&FriendUpdate) -> bool,
    ) -> FriendUpdate {
        self.wait_for(label, |message| match message {
            ServerMessage::FriendUpdate(update) if predicate(update) => Some(update.clone()),
            _ => None,
        })
        .await
    }
}

/// 让 `from` 向 `to` 发送申请并由 `to` 同意；双方列表都就绪后返回。
async fn befriend(from: &mut RawClient, to: &mut RawClient) {
    let to_code = to.player_code.clone();
    let from_id = from.player_id.clone();
    from.send("FriendRequest", json!({ "toPlayerCode": to_code }))
        .await;
    to.wait_friend_update("收到好友申请", |update| {
        update.incoming.iter().any(|item| item.player_id == from_id)
    })
    .await;

    let from_id = from.player_id.clone();
    to.send(
        "FriendRespond",
        json!({ "fromPlayerId": from_id, "accept": true }),
    )
    .await;
    let to_id = to.player_id.clone();
    from.wait_friend_update("好友列表", |update| {
        update.friends.iter().any(|item| item.player_id == to_id)
    })
    .await;
    to.wait_friend_update("好友列表", |update| {
        update.friends.iter().any(|item| item.player_id == from_id)
    })
    .await;
}

/// 让 `n` 名新玩家入房、选英雄并准备；返回需要保持存活的客户端。
async fn fill_room(
    addr: std::net::SocketAddr,
    code: &str,
    players: usize,
    prefix: &str,
) -> Vec<RawClient> {
    let mut clients = Vec::with_capacity(players);
    for index in 0..players {
        let mut client = RawClient::connect(addr, &format!("{prefix}{index}")).await;
        let update = client.join_room(code).await;
        client.select_hero_and_ready(&update).await;
        clients.push(client);
    }
    clients
}

fn room_is_full(update: &RoomUpdate) -> bool {
    update.players.len() == room_service::SEAT_COUNT
}

// ---------------------------------------------------------------------------
// 好友关系
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn friend_request_and_accept_over_websocket() {
    let addr = start_server().await;
    let mut alice = RawClient::connect(addr, "爱丽丝").await;
    let mut bob = RawClient::connect(addr, "鲍勃").await;
    let bob_code = bob.player_code.clone();
    let alice_id = alice.player_id.clone();
    let bob_id = bob.player_id.clone();

    alice
        .send("FriendRequest", json!({ "toPlayerCode": bob_code }))
        .await;

    let incoming = bob
        .wait_friend_update("收到好友申请", |update| !update.incoming.is_empty())
        .await;
    assert_eq!(incoming.incoming.len(), 1);
    assert_eq!(incoming.incoming[0].player_id, alice_id);

    bob.send(
        "FriendRespond",
        json!({ "fromPlayerId": alice_id, "accept": true }),
    )
    .await;

    let alice_update = alice
        .wait_friend_update("爱丽丝好友列表", |update| !update.friends.is_empty())
        .await;
    assert_eq!(alice_update.friends.len(), 1);
    assert_eq!(alice_update.friends[0].player_id, bob_id);
    assert_eq!(
        format!("{:?}", alice_update.friends[0].status),
        "Online",
        "同进程内已认证的玩家应显示在线"
    );

    let bob_update = bob
        .wait_friend_update("鲍勃好友列表", |update| !update.friends.is_empty())
        .await;
    assert_eq!(bob_update.friends.len(), 1);
    assert_eq!(bob_update.friends[0].player_id, alice_id);

    alice
        .send("FriendRemove", json!({ "friendId": bob_id }))
        .await;
    let alice_update = alice
        .wait_friend_update("删除后爱丽丝列表", |update| {
            update.friends.is_empty()
        })
        .await;
    assert!(alice_update.friends.is_empty());
    let bob_update = bob
        .wait_friend_update("删除后鲍勃列表", |update| update.friends.is_empty())
        .await;
    assert!(bob_update.friends.is_empty());
}

// ---------------------------------------------------------------------------
// 邀请成功路径
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invite_friend_to_room_over_websocket() {
    let addr = start_server().await;
    let mut host = RawClient::connect(addr, "房主").await;
    let mut guest = RawClient::connect(addr, "受邀者").await;
    let host_id = host.player_id.clone();
    let guest_id = guest.player_id.clone();
    befriend(&mut host, &mut guest).await;

    let room = host.create_room().await;
    host.send("InviteToRoom", json!({ "friendId": guest_id }))
        .await;
    let invite = guest
        .wait_for("收到房间邀请", |message| match message {
            ServerMessage::FriendInvite(invite) => Some(invite.clone()),
            _ => None,
        })
        .await;
    assert_eq!(invite.room_code, room.code);
    assert_eq!(invite.from_player_id, host_id);

    guest
        .send(
            "RespondInvite",
            json!({ "inviteId": invite.invite_id, "accept": true }),
        )
        .await;
    let accepted = guest
        .wait_for("邀请回执", |message| match message {
            ServerMessage::InviteAccepted(accepted) => Some(accepted.clone()),
            _ => None,
        })
        .await;
    assert_eq!(accepted.room_code, room.code);

    let result = host
        .wait_for("邀请结果", |message| match message {
            ServerMessage::FriendInviteResult(result) => Some(result.clone()),
            _ => None,
        })
        .await;
    assert!(result.accepted);

    let joined = guest.join_room(&accepted.room_code).await;
    assert!(
        joined
            .players
            .iter()
            .any(|player| player.player_id == guest_id),
        "受邀者应出现在房间座位里"
    );
}

// ---------------------------------------------------------------------------
// 邀请发送侧拒绝分支
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invite_requires_friendship_and_lobby_room() {
    let addr = start_server().await;
    let mut host = RawClient::connect(addr, "房主2").await;
    let stranger = RawClient::connect(addr, "陌生人").await;
    let stranger_id = stranger.player_id.clone();

    // 不在房间：先报 NOT_IN_ROOM，不校验好友。
    host.send("InviteToRoom", json!({ "friendId": stranger_id }))
        .await;
    host.wait_error(ErrorCode::NotInRoom).await;

    // 在房间但不是好友：报 NOT_FRIENDS。
    host.create_room().await;
    let stranger_id = stranger.player_id.clone();
    host.send("InviteToRoom", json!({ "friendId": stranger_id }))
        .await;
    host.wait_error(ErrorCode::NotFriends).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invite_rejects_offline_friend() {
    let addr = start_server().await;
    let mut host = RawClient::connect(addr, "房主3").await;
    let mut friend = RawClient::connect(addr, "离线好友").await;
    let friend_id = friend.player_id.clone();
    befriend(&mut host, &mut friend).await;
    host.create_room().await;

    drop(friend);
    // 等到好友状态真正变成离线，避免与断线处理竞态。
    host.wait_friend_update("好友离线", |update| {
        update
            .friends
            .iter()
            .any(|item| item.player_id == friend_id && format!("{:?}", item.status) == "Offline")
    })
    .await;

    host.send("InviteToRoom", json!({ "friendId": friend_id }))
        .await;
    host.wait_error(ErrorCode::FriendOffline).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invite_rejects_friend_in_another_room() {
    let addr = start_server().await;
    let mut host = RawClient::connect(addr, "房主4").await;
    let mut friend = RawClient::connect(addr, "忙碌好友").await;
    let friend_id = friend.player_id.clone();
    befriend(&mut host, &mut friend).await;
    host.create_room().await;
    friend.create_room().await;

    host.send("InviteToRoom", json!({ "friendId": friend_id }))
        .await;
    host.wait_error(ErrorCode::FriendBusy).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn invite_rejects_full_room() {
    let addr = start_server().await;
    let mut host = RawClient::connect(addr, "满房房主").await;
    let mut friend = RawClient::connect(addr, "满房好友").await;
    let friend_id = friend.player_id.clone();
    befriend(&mut host, &mut friend).await;
    let room = host.create_room().await;

    let fillers = fill_room(
        addr,
        &room.code,
        room_service::SEAT_COUNT - 1,
        "full-filler",
    )
    .await;
    host.wait_room_update(room_is_full).await;

    host.send("InviteToRoom", json!({ "friendId": friend_id }))
        .await;
    host.wait_error(ErrorCode::RoomFull).await;
    drop(fillers);
    drop(friend);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn invite_rejects_started_room() {
    let addr = start_server().await;
    let mut host = RawClient::connect(addr, "开局房主").await;
    let mut friend = RawClient::connect(addr, "开局好友").await;
    let friend_id = friend.player_id.clone();
    befriend(&mut host, &mut friend).await;
    let room = host.create_room().await;
    host.select_hero_and_ready(&room).await;

    let fillers = fill_room(
        addr,
        &room.code,
        room_service::SEAT_COUNT - 1,
        "start-filler",
    )
    .await;
    host.wait_game_started().await;

    host.send("InviteToRoom", json!({ "friendId": friend_id }))
        .await;
    host.wait_error(ErrorCode::InvalidStage).await;
    drop(fillers);
    drop(friend);
}

// ---------------------------------------------------------------------------
// 接受侧拒绝分支
// ---------------------------------------------------------------------------

/// 房主建 1 人房间并邀请好友，返回邀请 id 与房主侧的房间更新（含自身候选）。
async fn invite_into_fresh_room(
    host: &mut RawClient,
    guest: &mut RawClient,
) -> (String, RoomUpdate) {
    let room = host.create_room().await;
    let guest_id = guest.player_id.clone();
    host.send("InviteToRoom", json!({ "friendId": guest_id }))
        .await;
    let invite = guest
        .wait_for("收到房间邀请", |message| match message {
            ServerMessage::FriendInvite(invite) => Some(invite.clone()),
            _ => None,
        })
        .await;
    (invite.invite_id, room)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn accept_rejects_room_started_after_invite() {
    let addr = start_server().await;
    let mut host = RawClient::connect(addr, "开局前房主").await;
    let mut guest = RawClient::connect(addr, "迟到的受邀者").await;
    befriend(&mut host, &mut guest).await;
    let (invite_id, room) = invite_into_fresh_room(&mut host, &mut guest).await;

    // 房主创建房间时已在房内，直接用创建返回的更新准备，不能重复 JoinRoom。
    host.select_hero_and_ready(&room).await;
    let fillers = fill_room(
        addr,
        &room.code,
        room_service::SEAT_COUNT - 1,
        "late-filler",
    )
    .await;
    host.wait_game_started().await;

    guest
        .send(
            "RespondInvite",
            json!({ "inviteId": invite_id, "accept": true }),
        )
        .await;
    guest.wait_error(ErrorCode::InvalidStage).await;
    let result = host
        .wait_for("邀请结果", |message| match message {
            ServerMessage::FriendInviteResult(result) => Some(result.clone()),
            _ => None,
        })
        .await;
    assert!(!result.accepted);
    assert_eq!(result.reason.as_deref(), Some("INVALID_STAGE"));
    drop(fillers);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn accept_rejects_full_room() {
    let addr = start_server().await;
    let mut host = RawClient::connect(addr, "满房前房主").await;
    let mut guest = RawClient::connect(addr, "满房受邀者").await;
    befriend(&mut host, &mut guest).await;
    let (invite_id, room) = invite_into_fresh_room(&mut host, &mut guest).await;

    let fillers = fill_room(
        addr,
        &room.code,
        room_service::SEAT_COUNT - 1,
        "accept-full-filler",
    )
    .await;
    host.wait_room_update(room_is_full).await;

    guest
        .send(
            "RespondInvite",
            json!({ "inviteId": invite_id, "accept": true }),
        )
        .await;
    guest.wait_error(ErrorCode::RoomFull).await;
    let result = host
        .wait_for("邀请结果", |message| match message {
            ServerMessage::FriendInviteResult(result) => Some(result.clone()),
            _ => None,
        })
        .await;
    assert!(!result.accepted);
    assert_eq!(result.reason.as_deref(), Some("ROOM_FULL"));
    drop(fillers);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn accept_rejects_room_gone() {
    let addr = start_server().await;
    let mut host = RawClient::connect(addr, "散房房主").await;
    let mut guest = RawClient::connect(addr, "散房受邀者").await;
    befriend(&mut host, &mut guest).await;
    let (invite_id, room) = invite_into_fresh_room(&mut host, &mut guest).await;
    let code = room.code.clone();

    host.send("LeaveRoom", json!({})).await;
    // 等房间真正销毁：再用同一房间码加入应回 ROOM_NOT_FOUND。
    let mut probe = RawClient::connect(addr, "散房探测").await;
    probe.send("JoinRoom", json!({ "code": code })).await;
    probe.wait_error(ErrorCode::RoomNotFound).await;

    guest
        .send(
            "RespondInvite",
            json!({ "inviteId": invite_id, "accept": true }),
        )
        .await;
    guest.wait_error(ErrorCode::RoomNotFound).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn decline_is_reported_and_invite_cannot_be_reused() {
    let addr = start_server().await;
    let mut host = RawClient::connect(addr, "被拒房主").await;
    let mut guest = RawClient::connect(addr, "拒邀好友").await;
    befriend(&mut host, &mut guest).await;
    let (invite_id, _room) = invite_into_fresh_room(&mut host, &mut guest).await;

    guest
        .send(
            "RespondInvite",
            json!({ "inviteId": invite_id, "accept": false }),
        )
        .await;
    let guest_result = guest
        .wait_for("拒绝回执", |message| match message {
            ServerMessage::FriendInviteResult(result) => Some(result.clone()),
            _ => None,
        })
        .await;
    assert!(!guest_result.accepted);
    assert!(guest_result.reason.is_none());

    let host_result = host
        .wait_for("邀请结果", |message| match message {
            ServerMessage::FriendInviteResult(result) => Some(result.clone()),
            _ => None,
        })
        .await;
    assert!(!host_result.accepted);
    assert!(host_result.reason.is_none());

    // 邀请已被处理并移除，再次响应应视为不存在。
    guest
        .send(
            "RespondInvite",
            json!({ "inviteId": invite_id, "accept": true }),
        )
        .await;
    guest.wait_error(ErrorCode::InviteNotFound).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn accept_rejects_expired_and_unknown_invite() {
    let clock = Arc::new(FixedClock::new(1_700_000_000_000));
    let addr = start_server_with_clock(clock.clone()).await;
    let mut host = RawClient::connect(addr, "过期房主").await;
    let mut guest = RawClient::connect(addr, "过期受邀者").await;
    befriend(&mut host, &mut guest).await;
    let (invite_id, _) = invite_into_fresh_room(&mut host, &mut guest).await;

    guest
        .send(
            "RespondInvite",
            json!({ "inviteId": "inv_does_not_exist", "accept": true }),
        )
        .await;
    guest.wait_error(ErrorCode::InviteNotFound).await;

    clock.advance(60_000);
    guest
        .send(
            "RespondInvite",
            json!({ "inviteId": invite_id, "accept": true }),
        )
        .await;
    guest.wait_error(ErrorCode::InviteExpired).await;
}
