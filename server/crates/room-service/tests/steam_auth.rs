//! Steam 登录：假校验器驱动真实 WebSocket，覆盖建号、复用、失败与降级。

mod common;

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use futures_util::{SinkExt, StreamExt};
use protocol::client::AuthRequest;
use protocol::server::{AccountView, AchievementState, AuthResult, ErrorMessage};
use protocol::{ErrorCode, ServerEnvelope, ServerMessage};
use room_service::clock::FixedClock;
use room_service::phone_code::{CODE_TTL_MS, DEV_PHONE_CODE, RESEND_COOLDOWN_MS};
use room_service::steam::{SteamVerifier, auth_failed_error, unavailable_error};
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message;

const NOW_MS: u64 = 1_700_000_000_000;
const STEAM_ID: &str = "76561198000000001";

/// 测试用最小 WebSocket 客户端；只解析下行信封与认证结果。
struct WsClient {
    ws: common::ClientWs,
    next_id: u64,
}

impl WsClient {
    async fn connect(addr: std::net::SocketAddr) -> Self {
        let ws = common::connect_with_retry(&format!("ws://{addr}/ws")).await;
        Self { ws, next_id: 0 }
    }

    async fn send(&mut self, kind: &str, payload: Value) {
        self.next_id += 1;
        let envelope = json!({
            "v": 1,
            "id": format!("steam_ws_{}", self.next_id),
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

    /// 发送 Steam 登录并等待 `AuthResult` 或 `Error`。
    async fn login(
        &mut self,
        ticket: &str,
        display_name: Option<&str>,
    ) -> Result<AuthResult, ErrorMessage> {
        let mut payload = json!({ "steamTicket": ticket });
        if let Some(name) = display_name {
            payload["displayName"] = json!(name);
        }
        self.send("AuthRequest", payload).await;
        loop {
            match self.recv().await.expect("连接在认证前关闭") {
                ServerMessage::AuthResult(result) => return Ok(result),
                ServerMessage::Error(error) => return Err(error),
                _ => continue,
            }
        }
    }

    /// 手机号 + 开发验证码登录。
    async fn login_phone(&mut self, phone: &str, code: &str) -> Result<AuthResult, ErrorMessage> {
        self.send("AuthRequest", json!({ "phone": phone, "code": code }))
            .await;
        loop {
            match self.recv().await.expect("连接在认证前关闭") {
                ServerMessage::AuthResult(result) => return Ok(result),
                ServerMessage::Error(error) => return Err(error),
                _ => continue,
            }
        }
    }

    async fn recv_account_view(&mut self) -> AccountView {
        loop {
            match self.recv().await.expect("连接在账号视图前关闭") {
                ServerMessage::AccountView(view) => return view,
                ServerMessage::Error(error) => panic!("账号视图请求失败: {error:?}"),
                _ => continue,
            }
        }
    }

    async fn request_account(&mut self) -> AccountView {
        self.send("RequestAccount", json!({})).await;
        self.recv_account_view().await
    }

    /// 等待认证后下发的成就状态。
    async fn recv_achievement_state(&mut self) -> AchievementState {
        loop {
            match self.recv().await.expect("连接在成就状态前关闭") {
                ServerMessage::AchievementState(state) => return state,
                ServerMessage::Error(error) => panic!("成就状态下发失败: {error:?}"),
                _ => continue,
            }
        }
    }

    /// 请求绑定验证码；成功返回过期时刻。
    async fn request_phone_code(&mut self, phone: &str) -> Result<DateTime<Utc>, ErrorMessage> {
        self.send("RequestPhoneCode", json!({ "phone": phone }))
            .await;
        loop {
            match self.recv().await.expect("连接在验证码结果前关闭") {
                ServerMessage::PhoneCodeSent(sent) => return Ok(sent.expires_at),
                ServerMessage::Error(error) => return Err(error),
                _ => continue,
            }
        }
    }

    async fn bind_phone(&mut self, phone: &str, code: &str) -> Result<AccountView, ErrorMessage> {
        self.send("BindPhone", json!({ "phone": phone, "code": code }))
            .await;
        loop {
            match self.recv().await.expect("连接在绑定结果前关闭") {
                ServerMessage::AccountView(view) => return Ok(view),
                ServerMessage::Error(error) => return Err(error),
                _ => continue,
            }
        }
    }
}

async fn server(verifier: SteamVerifier) -> std::net::SocketAddr {
    common::start_server_with_steam(Arc::new(FixedClock::new(NOW_MS)), verifier).await
}

/// 启动服务并返回可推进的时钟句柄。
async fn server_and_clock(verifier: SteamVerifier) -> (std::net::SocketAddr, FixedClock) {
    let clock = FixedClock::new(NOW_MS);
    let addr = common::start_server_with_steam(Arc::new(clock.clone()), verifier).await;
    (addr, clock)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn steam_login_creates_then_reuses_account() {
    let addr = server(SteamVerifier::stub_ok(STEAM_ID)).await;

    let mut first_client = WsClient::connect(addr).await;
    let first = first_client
        .login("deadbeef", Some("Ada"))
        .await
        .expect("首次 Steam 登录应建号并签发令牌");
    assert_eq!(first.player_code.len(), 6);
    assert!(first.player_code.chars().all(|ch| ch.is_ascii_digit()));

    let mut second_client = WsClient::connect(addr).await;
    let second = second_client
        .login("deadbeef", Some("改名"))
        .await
        .expect("同一 SteamID 再登录应恢复同一账号");
    assert_eq!(second.player_id, first.player_id, "SteamID 是稳定身份");
    assert_eq!(second.player_code, first.player_code);
    assert_ne!(second.token, first.token, "每次登录签发新令牌");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invalid_ticket_is_rejected() {
    let addr = server(SteamVerifier::stub_err(auth_failed_error("票据无效"))).await;
    let mut client = WsClient::connect(addr).await;
    let error = client
        .login("deadbeef", None)
        .await
        .expect_err("无效票据必须拒绝登录");
    assert_eq!(error.code, ErrorCode::SteamAuthFailed);
    assert!(error.retryable, "重新取票后可重试");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unavailable_steam_is_retryable() {
    let addr = server(SteamVerifier::stub_err(unavailable_error("超时"))).await;
    let mut client = WsClient::connect(addr).await;
    let error = client
        .login("deadbeef", None)
        .await
        .expect_err("Steam 不可用时不得建号");
    assert_eq!(error.code, ErrorCode::SteamUnavailable);
    assert!(error.retryable);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disabled_steam_reports_disabled() {
    let addr = server(SteamVerifier::disabled()).await;
    let mut client = WsClient::connect(addr).await;
    let error = client
        .login("deadbeef", None)
        .await
        .expect_err("未配置时必须拒绝");
    assert_eq!(error.code, ErrorCode::SteamDisabled);
    assert!(!error.retryable);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invalid_ticket_format_skips_backend() {
    let verifier = SteamVerifier::stub_ok(STEAM_ID);
    let addr = common::start_server_with_steam(Arc::new(FixedClock::new(NOW_MS)), verifier).await;
    let mut client = WsClient::connect(addr).await;
    let error = client
        .login("not-hex", None)
        .await
        .expect_err("非法票据必须拒绝");
    assert_eq!(error.code, ErrorCode::SteamAuthFailed);
}

/// 令牌续登对 Steam 账号同样生效；这里直接构造请求确认分支可解析。
#[test]
fn steam_auth_request_round_trips() {
    let request = AuthRequest::Variant2 {
        display_name: None,
        steam_ticket: "deadbeef".to_string().try_into().expect("票据合法"),
    };
    let encoded: Value = serde_json::to_value(&request).expect("请求应能序列化");
    assert_eq!(encoded["steamTicket"], "deadbeef");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn steam_login_sends_account_view() {
    let (addr, _clock) = server_and_clock(SteamVerifier::stub_ok(STEAM_ID)).await;
    let mut client = WsClient::connect(addr).await;
    client
        .login("deadbeef", Some("Ada"))
        .await
        .expect("Steam 登录应成功");

    let view = client.recv_account_view().await;
    assert!(view.steam_bound);
    assert!(!view.phone_bound);
    assert_eq!(view.masked_phone, None);

    let refreshed = client.request_account().await;
    assert_eq!(refreshed.player_id, view.player_id);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auth_sends_empty_achievement_state() {
    let addr = server(SteamVerifier::stub_ok(STEAM_ID)).await;
    let mut client = WsClient::connect(addr).await;
    client
        .login("deadbeef", Some("Ada"))
        .await
        .expect("Steam 登录应成功");

    let state = client.recv_achievement_state().await;
    assert!(
        state.unlocked.is_empty(),
        "成就运行时未实现时下发空列表，不阻断登录"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bind_phone_links_steam_and_phone_identity() {
    let (addr, clock) = server_and_clock(SteamVerifier::stub_ok(STEAM_ID)).await;
    let mut client = WsClient::connect(addr).await;
    let auth = client
        .login("deadbeef", Some("Ada"))
        .await
        .expect("Steam 登录应成功");
    client.recv_account_view().await;

    let expires = client
        .request_phone_code("13800000000")
        .await
        .expect("验证码应发送成功");
    assert_eq!(
        expires.timestamp_millis(),
        i64::try_from(NOW_MS + CODE_TTL_MS).expect("时间戳在范围内")
    );
    let view = client
        .bind_phone("13800000000", DEV_PHONE_CODE)
        .await
        .expect("正确验证码应绑定成功");
    assert!(view.steam_bound);
    assert!(view.phone_bound);
    assert_eq!(view.masked_phone.as_deref(), Some("138****0000"));

    let mut phone_client = WsClient::connect(addr).await;
    let phone_auth = phone_client
        .login_phone("13800000000", DEV_PHONE_CODE)
        .await
        .expect("绑定后手机号应能登录同一账号");
    assert_eq!(phone_auth.player_id, auth.player_id);
    assert_eq!(phone_auth.player_code, auth.player_code);

    // Steam 与手机号是同一账号：手机号登录把 Steam 连接顶下线，后续操作走新连接。
    let kicked = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let message = client.recv().await.expect("Steam 连接在顶号提示前关闭");
            if let ServerMessage::Error(error) = message {
                return error;
            }
        }
    })
    .await
    .expect("5 秒内应收到顶号提示");
    assert_eq!(kicked.code, ErrorCode::SessionReplaced);

    // 同号重复绑定幂等；不同手机号被拒绝。
    clock.advance(RESEND_COOLDOWN_MS);
    phone_client
        .request_phone_code("13800000000")
        .await
        .expect("冷却后应能再次发送");
    let again = phone_client
        .bind_phone("13800000000", DEV_PHONE_CODE)
        .await
        .expect("同号重复绑定应幂等");
    assert_eq!(again.player_id, auth.player_id);

    clock.advance(RESEND_COOLDOWN_MS);
    phone_client
        .request_phone_code("13900000000")
        .await
        .expect("新手机号验证码应发送成功");
    let error = phone_client
        .bind_phone("13900000000", DEV_PHONE_CODE)
        .await
        .expect_err("已绑定账号不能换绑");
    assert_eq!(error.code, ErrorCode::PhoneAlreadyBound);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bind_rejects_code_mismatch_and_expiry() {
    let (addr, clock) = server_and_clock(SteamVerifier::stub_ok(STEAM_ID)).await;
    let mut client = WsClient::connect(addr).await;
    client
        .login("deadbeef", None)
        .await
        .expect("Steam 登录应成功");
    client.recv_account_view().await;

    client
        .request_phone_code("13800000000")
        .await
        .expect("验证码应发送成功");
    let error = client
        .bind_phone("13800000000", "000000")
        .await
        .expect_err("错误验证码必须拒绝");
    assert_eq!(error.code, ErrorCode::CodeInvalid);
    clock.advance(CODE_TTL_MS);
    let error = client
        .bind_phone("13800000000", DEV_PHONE_CODE)
        .await
        .expect_err("过期验证码必须拒绝");
    assert_eq!(error.code, ErrorCode::CodeExpired);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_phone_code_is_rate_limited() {
    let (addr, _clock) = server_and_clock(SteamVerifier::stub_ok(STEAM_ID)).await;
    let mut client = WsClient::connect(addr).await;
    client
        .login("deadbeef", None)
        .await
        .expect("Steam 登录应成功");
    client.recv_account_view().await;

    client
        .request_phone_code("13800000000")
        .await
        .expect("首次发送应成功");
    let error = client
        .request_phone_code("13800000000")
        .await
        .expect_err("冷却期内必须拒绝");
    assert_eq!(error.code, ErrorCode::RateLimited);
    assert!(error.retryable);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bind_rejects_phone_owned_by_other_account() {
    let (addr, _clock) = server_and_clock(SteamVerifier::stub_ok(STEAM_ID)).await;

    let mut phone_client = WsClient::connect(addr).await;
    phone_client
        .login_phone("13800000001", DEV_PHONE_CODE)
        .await
        .expect("手机号账号应创建成功");

    let mut steam_client = WsClient::connect(addr).await;
    steam_client
        .login("deadbeef", None)
        .await
        .expect("Steam 登录应成功");
    steam_client.recv_account_view().await;
    steam_client
        .request_phone_code("13800000001")
        .await
        .expect("验证码应发送成功");
    let error = steam_client
        .bind_phone("13800000001", DEV_PHONE_CODE)
        .await
        .expect_err("占用中的手机号必须拒绝");
    assert_eq!(error.code, ErrorCode::PhoneAlreadyBound);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invalid_phone_is_rejected() {
    let (addr, _clock) = server_and_clock(SteamVerifier::stub_ok(STEAM_ID)).await;
    let mut client = WsClient::connect(addr).await;
    client
        .login("deadbeef", None)
        .await
        .expect("Steam 登录应成功");
    client.recv_account_view().await;

    let error = client
        .request_phone_code("138 0000")
        .await
        .expect_err("格式非法必须拒绝");
    assert_eq!(error.code, ErrorCode::PhoneInvalid);
}
