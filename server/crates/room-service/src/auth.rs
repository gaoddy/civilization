//! 会话认证：手机号 + 开发验证码登录，或令牌续登。
//!
//! 存储为进程内 `BTreeMap`，确定性、无 I/O；`playerId` 与 `token` 都是服务端生成的
//! 不透明串，不暴露手机号。

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use protocol::client::AuthRequest;
use protocol::server::{AuthResult, ErrorMessage};

use crate::clock::Clock;
use crate::error::error_message;

/// 开发环境固定短信验证码。
pub const DEV_SMS_CODE: &str = "123456";

/// 会话有效期：24 小时。
pub const SESSION_TTL_MS: u64 = 24 * 60 * 60 * 1000;

/// 认证成功后返回的会话。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    /// 服务端生成的不透明玩家 id。
    pub player_id: String,
    /// 续登令牌。
    pub token: String,
    /// 展示名；缺省时为 `玩家N`。
    pub display_name: String,
    /// 过期时刻（Unix epoch 毫秒）。
    expires_at_ms: u64,
}

impl Session {
    /// 会话过期时刻（Unix epoch 毫秒）。
    pub fn expires_at_ms(&self) -> u64 {
        self.expires_at_ms
    }

    /// 构造对应的下行 [`AuthResult`]。
    pub fn auth_result(&self) -> AuthResult {
        AuthResult {
            expires_at: to_utc(self.expires_at_ms),
            player_id: self.player_id.clone(),
            token: self.token.clone(),
        }
    }
}

/// 进程内会话存储。
#[derive(Debug, Default)]
pub struct SessionStore {
    by_player: BTreeMap<String, Session>,
    by_token: BTreeMap<String, String>,
    next_player: u64,
    next_token: u64,
    created_sessions: u64,
}

impl SessionStore {
    /// 创建空存储。
    pub fn new() -> Self {
        Self::default()
    }

    /// 认证一次请求。
    ///
    /// - 手机号分支：验证码等于 [`DEV_SMS_CODE`] 才成功；`displayName` 缺省时按本进程
    ///   第几个新会话生成 `玩家N`（从 1 起）。
    /// - 令牌分支：按令牌查到会话且未过期才成功，否则 `TOKEN_EXPIRED`。
    ///
    /// # Errors
    ///
    /// 返回带稳定错误码的 [`ErrorMessage`]：验证码错误 `AUTH_FAILED`，
    /// 令牌无效或过期 `TOKEN_EXPIRED`。两者都不可原样重试。
    pub fn authenticate(
        &mut self,
        req: &AuthRequest,
        clock: &dyn Clock,
    ) -> Result<Session, ErrorMessage> {
        match req {
            AuthRequest::Variant0 {
                code,
                display_name,
                phone: _,
            } => {
                if code != DEV_SMS_CODE {
                    return Err(error_message("AUTH_FAILED", "短信验证码错误", false));
                }

                self.created_sessions = self.created_sessions.saturating_add(1);
                let display_name = match display_name {
                    Some(name) => String::from(name.clone()),
                    None => format!("玩家{}", self.created_sessions),
                };

                self.next_player = self.next_player.saturating_add(1);
                self.next_token = self.next_token.saturating_add(1);
                let session = Session {
                    player_id: format!("p_{}", self.next_player),
                    token: format!("tok_{}", self.next_token),
                    display_name,
                    expires_at_ms: clock.now_ms().saturating_add(SESSION_TTL_MS),
                };

                self.by_token
                    .insert(session.token.clone(), session.player_id.clone());
                self.by_player
                    .insert(session.player_id.clone(), session.clone());
                Ok(session)
            }
            AuthRequest::Variant1 { token, .. } => {
                let player_id = self
                    .by_token
                    .get(token)
                    .cloned()
                    .ok_or_else(token_expired)?;
                let session = self
                    .by_player
                    .get(&player_id)
                    .cloned()
                    .ok_or_else(token_expired)?;
                if clock.now_ms() >= session.expires_at_ms {
                    return Err(token_expired());
                }
                Ok(session)
            }
        }
    }

    /// 已创建的会话总数（测试与指标用）。
    pub fn created_sessions(&self) -> u64 {
        self.created_sessions
    }
}

fn token_expired() -> ErrorMessage {
    error_message("TOKEN_EXPIRED", "令牌无效或已过期", false)
}

/// Unix epoch 毫秒转 UTC 时间；超出可表示范围时退化为 epoch 起点。
fn to_utc(ms: u64) -> DateTime<Utc> {
    i64::try_from(ms)
        .ok()
        .and_then(DateTime::<Utc>::from_timestamp_millis)
        .unwrap_or(DateTime::UNIX_EPOCH)
}
