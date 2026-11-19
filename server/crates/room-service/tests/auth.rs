//! 会话认证、时钟与下行消息序号的外部行为测试。
//!
//! 这些测试只通过公开接口驱动，不依赖内部存储结构。

use protocol::ErrorCode;
use protocol::client::AuthRequest;
use room_service::auth::{DEV_SMS_CODE, SessionStore};
use room_service::clock::{Clock, FixedClock};
use room_service::envelope::SeatChannel;
use serde_json::Map;

/// 构造开发验证码登录请求（`displayName` 缺省）。
fn phone_auth(phone: &str, code: &str) -> AuthRequest {
    AuthRequest::Variant0 {
        code: code.to_string(),
        display_name: None,
        phone: phone.to_string(),
    }
}

/// 构造令牌续登请求。
fn token_auth(token: &str) -> AuthRequest {
    AuthRequest::Variant1 {
        display_name: None,
        token: token.to_string(),
    }
}

const NOW_MS: u64 = 1_700_000_000_000;
const DAY_MS: u64 = 24 * 60 * 60 * 1000;

#[test]
fn auth_accepts_dev_code_and_generates_display_name() {
    let clock = FixedClock::new(NOW_MS);
    let mut store = SessionStore::new();

    let first = store
        .authenticate(&phone_auth("13800000001", DEV_SMS_CODE), &clock)
        .expect("开发验证码应认证成功");
    assert_eq!(first.display_name, "玩家1");

    let second = store
        .authenticate(&phone_auth("13800000002", DEV_SMS_CODE), &clock)
        .expect("第二个开发验证码会话应认证成功");
    assert_eq!(second.display_name, "玩家2");

    assert_ne!(first.player_id, second.player_id, "playerId 必须逐个生成");
    assert_ne!(first.token, second.token, "token 必须逐个生成");
    assert_ne!(
        first.player_code, second.player_code,
        "玩家编号必须逐个生成"
    );
}

#[test]
fn player_code_is_six_digits_and_stable_across_token_auth() {
    let clock = FixedClock::new(NOW_MS);
    let mut store = SessionStore::new();

    let session = store
        .authenticate(&phone_auth("13800000001", DEV_SMS_CODE), &clock)
        .expect("首次认证成功");
    assert_eq!(session.player_code.len(), 6);
    assert!(
        session.player_code.chars().all(|ch| ch.is_ascii_digit()),
        "玩家编号必须是 6 位数字：{}",
        session.player_code
    );
    assert_eq!(session.player_code, "100000", "首个编号从 100000 起");

    let again = store
        .authenticate(&token_auth(&session.token), &clock)
        .expect("有效令牌应认证成功");
    assert_eq!(again.player_code, session.player_code, "续登必须复用编号");
    assert_eq!(session.auth_result().player_code, session.player_code);
}

#[test]
fn auth_rejects_wrong_code() {
    let clock = FixedClock::new(NOW_MS);
    let mut store = SessionStore::new();

    let err = store
        .authenticate(&phone_auth("13800000001", "000000"), &clock)
        .expect_err("错误验证码必须被拒绝");
    assert_eq!(err.code, ErrorCode::AuthFailed);
    assert_eq!(err.code.to_string(), "AUTH_FAILED");
    assert!(!err.retryable, "错误验证码不可原样重试");
}

#[test]
fn auth_token_round_trip() {
    let clock = FixedClock::new(NOW_MS);
    let mut store = SessionStore::new();

    let session = store
        .authenticate(&phone_auth("13800000001", DEV_SMS_CODE), &clock)
        .expect("首次认证成功");

    let again = store
        .authenticate(&token_auth(&session.token), &clock)
        .expect("有效令牌应认证成功");
    assert_eq!(again.player_id, session.player_id);
    assert_eq!(again.token, session.token);
    assert_eq!(again.display_name, "玩家1");

    let result = session.auth_result();
    assert_eq!(result.player_id, session.player_id);
    assert_eq!(result.token, session.token);
    assert_eq!(
        result.expires_at.timestamp_millis(),
        i64::try_from(NOW_MS + DAY_MS).expect("时间戳在 i64 范围内")
    );

    // 时钟推进到过期之后，旧令牌必须被拒绝。
    let later = FixedClock::new(NOW_MS + DAY_MS);
    let err = store
        .authenticate(&token_auth(&session.token), &later)
        .expect_err("过期令牌必须被拒绝");
    assert_eq!(err.code, ErrorCode::TokenExpired);
    assert_eq!(err.code.to_string(), "TOKEN_EXPIRED");
}

#[test]
fn session_debug_redacts_token() {
    let clock = FixedClock::new(NOW_MS);
    let mut store = SessionStore::new();
    let session = store
        .authenticate(&phone_auth("13800000001", DEV_SMS_CODE), &clock)
        .expect("认证成功");

    let debug = format!("{session:?}");
    assert!(
        !debug.contains(&session.token),
        "Debug 输出不得包含明文 token：{debug}"
    );
    assert!(
        debug.contains("<redacted>"),
        "token 字段应标记为 <redacted>：{debug}"
    );
    assert!(
        debug.contains(&session.player_id),
        "非敏感字段仍应出现在 Debug 输出中"
    );
}

#[test]
fn fixed_clock_is_controllable() {
    let clock = FixedClock::new(1_000);
    assert_eq!(clock.now_ms(), 1_000);
    let before = clock.now_rfc3339();
    assert_eq!(
        before,
        chrono::DateTime::<chrono::Utc>::from_timestamp_millis(1_000)
            .expect("合法时间戳")
            .to_rfc3339()
    );

    clock.set(2_000);
    assert_eq!(clock.now_ms(), 2_000);
    clock.advance(500);
    assert_eq!(clock.now_ms(), 2_500);
    assert_ne!(clock.now_rfc3339(), before, "RFC3339 应与 now_ms 同步变化");
    assert_eq!(
        clock.now_rfc3339(),
        chrono::DateTime::<chrono::Utc>::from_timestamp_millis(2_500)
            .expect("合法时间戳")
            .to_rfc3339()
    );
}

#[test]
fn seat_channel_seq_starts_at_one_and_increments() {
    let mut channel = SeatChannel::new();
    let payload = || Map::new();

    for expected in 1..=3u64 {
        let envelope = channel
            .bind("Error", payload())
            .expect("已知下行类型应构造成功");
        assert_eq!(envelope.seq.get(), expected);
    }

    // 失败的 bind 同样消耗一个序号（每次 bind 先自增），序号不回退。
    assert!(channel.bind("NotAKnownType", payload()).is_err());
    let envelope = channel
        .bind("Error", payload())
        .expect("失败后仍可继续 bind");
    assert_eq!(envelope.seq.get(), 5);
}

#[test]
fn tokens_are_not_guessable_sequence() {
    let clock = FixedClock::new(NOW_MS);
    let mut store = SessionStore::new();

    let first = store
        .authenticate(&phone_auth("13800000001", DEV_SMS_CODE), &clock)
        .expect("首次认证成功");
    let second = store
        .authenticate(&phone_auth("13800000002", DEV_SMS_CODE), &clock)
        .expect("第二次认证成功");

    assert!(
        first.token.len() >= 32,
        "token 长度应至少 32：{}",
        first.token
    );
    assert!(
        second.token.len() >= 32,
        "token 长度应至少 32：{}",
        second.token
    );
    assert_ne!(first.token, second.token, "token 必须互不相同");
    assert!(!first.token.starts_with("tok_"));
    assert!(!second.token.starts_with("tok_"));
}

#[test]
fn player_id_does_not_contain_phone() {
    let clock = FixedClock::new(NOW_MS);
    let mut store = SessionStore::new();
    let phone = "13800000000";

    let session = store
        .authenticate(&phone_auth(phone, DEV_SMS_CODE), &clock)
        .expect("认证成功");
    assert!(
        !session.player_id.contains(phone),
        "playerId 不得包含手机号：{}",
        session.player_id
    );
}

#[test]
fn expired_token_is_removed_from_store() {
    let clock = FixedClock::new(NOW_MS);
    let mut store = SessionStore::new();

    let session = store
        .authenticate(&phone_auth("13800000001", DEV_SMS_CODE), &clock)
        .expect("首次认证成功");
    assert_eq!(store.created_sessions(), 1);

    clock.advance(DAY_MS);
    let err = store
        .authenticate(&token_auth(&session.token), &clock)
        .expect_err("过期令牌必须被拒绝");
    assert_eq!(err.code, ErrorCode::TokenExpired);

    // 访问过期令牌时已把它从两表移除，清理函数找不到任何条目。
    assert_eq!(store.created_sessions(), 0);
    assert_eq!(store.purge_expired(&clock), 0);
}

#[test]
fn purge_expired_removes_only_expired() {
    let clock = FixedClock::new(NOW_MS);
    let mut store = SessionStore::new();

    let first = store
        .authenticate(&phone_auth("13800000001", DEV_SMS_CODE), &clock)
        .expect("首个会话认证成功");

    // 推进到首个会话过期；第二个会话在此时创建，尚未过期。
    clock.advance(DAY_MS);
    let second = store
        .authenticate(&phone_auth("13800000002", DEV_SMS_CODE), &clock)
        .expect("第二个会话认证成功");
    assert_eq!(store.created_sessions(), 2);

    assert_eq!(store.purge_expired(&clock), 1);
    assert_eq!(store.created_sessions(), 1);

    let again = store
        .authenticate(&token_auth(&second.token), &clock)
        .expect("未过期令牌仍可认证");
    assert_eq!(again.player_id, second.player_id);

    let err = store
        .authenticate(&token_auth(&first.token), &clock)
        .expect_err("已清理的过期令牌必须被拒绝");
    assert_eq!(err.code, ErrorCode::TokenExpired);
}
