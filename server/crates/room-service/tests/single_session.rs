//! 单客户端登录：新登录顶掉旧连接、旧令牌失效与大厅房间接管。
//!
//! 全部用真实 WebSocket 驱动，只走公开协议。

mod common;

use std::time::Duration;

use common::{
    RawClient, authenticate, authenticate_with_token, wait_room_update, wait_room_update_until,
};
use protocol::server::ErrorMessage;
use protocol::{ErrorCode, ServerMessage};
use serde_json::json;

/// 等到指定错误码；期间其他下行消息忽略，连接提前关闭视为失败。
async fn wait_error(client: &mut RawClient) -> ErrorMessage {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let (_, message) = client.recv().await.expect("等待错误时连接已关闭");
            if let ServerMessage::Error(error) = message {
                return error;
            }
        }
    })
    .await
    .expect("10 秒内应收到错误")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn second_login_kicks_previous_connection() {
    let addr = common::start_server().await;

    let mut first = RawClient::connect(addr).await;
    let (first_id, first_token) = authenticate(&mut first, "solo-kick").await;

    let mut second = RawClient::connect(addr).await;
    let (second_id, second_token) = authenticate(&mut second, "solo-kick").await;

    assert_eq!(first_id, second_id, "同一手机号应恢复同一账号");
    assert_ne!(first_token, second_token, "每次登录签发新令牌");

    let error = wait_error(&mut first).await;
    assert_eq!(error.code, ErrorCode::SessionReplaced);
    assert_eq!(error.message_id, "error.SESSION_REPLACED");
    assert!(
        tokio::time::timeout(Duration::from_secs(5), first.recv())
            .await
            .expect("旧连接应在提示后关闭")
            .is_none(),
        "被顶下线的连接应关闭"
    );

    // 旧令牌在顶号后不能再认证。
    let mut stale = RawClient::connect(addr).await;
    stale
        .send(
            "AuthRequest",
            json!({ "token": first_token, "displayName": "stale" }),
        )
        .await;
    let error = wait_error(&mut stale).await;
    assert_eq!(error.code, ErrorCode::TokenExpired);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn second_login_takes_over_lobby_room() {
    let addr = common::start_server().await;

    let mut host = RawClient::connect(addr).await;
    authenticate(&mut host, "solo-lobby-host").await;
    host.send("CreateRoom", json!({})).await;
    let created = wait_room_update(&mut host).await;
    let room_id = created.room_id.clone();
    let room_code = created.code.clone();

    let mut member = RawClient::connect(addr).await;
    let (member_id, _member_token) = authenticate(&mut member, "solo-lobby-member").await;
    member
        .send("JoinRoom", json!({ "code": room_code.clone() }))
        .await;
    wait_room_update(&mut member).await;
    member.send("Ready", json!({ "ready": true })).await;
    wait_room_update_until(&mut member, |update| {
        update.players.len() == 2 && update.players.iter().all(|player| player.ready)
    })
    .await;

    // 新客户端登录后直接拿到同一房间的等待视图，不需要额外发 Reconnect。
    let mut next = RawClient::connect(addr).await;
    let (next_id, _next_token) = authenticate(&mut next, "solo-lobby-member").await;
    assert_eq!(next_id, member_id, "同一手机号应恢复同一账号");

    let taken = wait_room_update(&mut next).await;
    assert_eq!(taken.room_id, room_id, "接管应回到原房间");
    assert_eq!(taken.code, room_code, "房间码不变");
    let own = taken
        .players
        .iter()
        .find(|player| player.player_id == next_id)
        .expect("接管视图应包含本人");
    assert!(own.ready, "准备状态随房间保留");
    assert_eq!(taken.players.len(), 2, "座位仍是同一批成员");

    // 旧连接收到顶号提示。
    let error = wait_error(&mut member).await;
    assert_eq!(error.code, ErrorCode::SessionReplaced);

    // 接管后的连接可以继续操作房间。
    next.send("Ready", json!({ "ready": false })).await;
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let (_, message) = next.recv_room().await.expect("等待取消准备时连接关闭");
            match message {
                ServerMessage::RoomUpdate(update)
                    if update
                        .players
                        .iter()
                        .any(|player| player.player_id == next_id && !player.ready) =>
                {
                    return;
                }
                ServerMessage::Error(error) => {
                    panic!("取消准备被拒绝：{} {}", error.code, error.message)
                }
                _ => {}
            }
        }
    })
    .await
    .expect("10 秒内应看到取消准备生效");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn token_relogin_does_not_kick_previous_connection() {
    let addr = common::start_server().await;

    let mut first = RawClient::connect(addr).await;
    let (player_id, token) = authenticate(&mut first, "solo-token").await;

    // 同一令牌续登属于重连，不是第二个客户端登录。
    let mut second = RawClient::connect(addr).await;
    let second_id = authenticate_with_token(&mut second, &token).await;
    assert_eq!(second_id, player_id, "续登应复用同一账号");

    let quiet = tokio::time::timeout(Duration::from_millis(300), async {
        loop {
            match first.recv().await {
                None => return,
                Some((_, ServerMessage::Error(error))) => {
                    panic!("续登不应顶号，实际收到 {}", error.code)
                }
                Some(_) => {}
            }
        }
    })
    .await;
    assert!(quiet.is_err(), "旧连接应保持在线，不被顶号或关闭");
}
