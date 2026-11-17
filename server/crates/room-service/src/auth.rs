//! 会话认证：手机号 + 开发验证码登录，或令牌续登。
//!
//! 存储为进程内 `BTreeMap`，无 I/O；`playerId` 与 `token` 都是服务端生成的不可预测
//! 不透明串，不暴露手机号。过期会话在下次被访问或调用 [`SessionStore::purge_expired`]
//! 时清除，M1 不跑后台清理任务。

use std::collections::BTreeMap;
use std::fmt;

use chrono::{DateTime, Utc};
use protocol::client::AuthRequest;
use protocol::server::{AuthResult, ErrorMessage};
use uuid::Uuid;

use crate::clock::Clock;
use crate::error::error_message_with_id;

/// 开发环境固定短信验证码。
pub const DEV_SMS_CODE: &str = "123456";

/// 会话有效期：24 小时。
pub const SESSION_TTL_MS: u64 = 24 * 60 * 60 * 1000;

/// 认证成功后返回的会话。
#[derive(Clone, PartialEq, Eq)]
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

/// 手写 `Debug`：`token` 是 24 小时有效的明文续登凭证，任何 `{:?}` 都不得泄漏。
impl fmt::Debug for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Session")
            .field("player_id", &self.player_id)
            .field("token", &"<redacted>")
            .field("display_name", &self.display_name)
            .field("expires_at_ms", &self.expires_at_ms)
            .finish()
    }
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
    /// 已创建会话的单调序号，只用于生成 `玩家N`，不因清理而回退。
    session_ordinal: u64,
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
                    return Err(error_message_with_id(
                        "AUTH_FAILED",
                        "error.AUTH_FAILED.invalid_code",
                        &[],
                        "短信验证码错误",
                        false,
                    ));
                }

                self.session_ordinal = self.session_ordinal.saturating_add(1);
                let display_name = match display_name {
                    Some(name) => String::from(name.clone()),
                    None => format!("玩家{}", self.session_ordinal),
                };

                let session = Session {
                    player_id: format!("p_{}", Uuid::new_v4().simple()),
                    token: Uuid::new_v4().simple().to_string(),
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
                let Some(player_id) = self.by_token.get(token).cloned() else {
                    return Err(token_expired());
                };
                let Some(session) = self.by_player.get(&player_id).cloned() else {
                    // 孤立令牌：会话已不在，顺手清掉。
                    self.by_token.remove(token);
                    return Err(token_expired());
                };
                if clock.now_ms() >= session.expires_at_ms {
                    // 过期会话在被访问时即从两张表移除，避免无界增长。
                    self.by_player.remove(&player_id);
                    self.by_token.remove(token);
                    return Err(token_expired());
                }
                Ok(session)
            }
        }
    }

    /// 清除所有已过期会话，返回清除的会话条数。
    ///
    /// 先收集过期令牌再逐个移除，避免边遍历边修改表。
    pub fn purge_expired(&mut self, clock: &dyn Clock) -> usize {
        let now = clock.now_ms();
        let expired: Vec<String> = self
            .by_token
            .iter()
            .filter(|(_, player_id)| {
                self.by_player
                    .get(*player_id)
                    .is_none_or(|session| now >= session.expires_at_ms)
            })
            .map(|(token, _)| token.clone())
            .collect();

        for token in &expired {
            if let Some(player_id) = self.by_token.remove(token) {
                self.by_player.remove(&player_id);
            }
        }
        expired.len()
    }

    /// 当前活跃会话数（测试与指标用）。
    pub fn created_sessions(&self) -> u64 {
        self.by_player.len() as u64
    }
}

fn token_expired() -> ErrorMessage {
    error_message_with_id(
        "TOKEN_EXPIRED",
        "error.TOKEN_EXPIRED.invalid_token",
        &[],
        "令牌无效或已过期",
        false,
    )
}

/// Unix epoch 毫秒转 UTC 时间；超出可表示范围时退化为 epoch 起点。
fn to_utc(ms: u64) -> DateTime<Utc> {
    i64::try_from(ms)
        .ok()
        .and_then(DateTime::<Utc>::from_timestamp_millis)
        .unwrap_or(DateTime::UNIX_EPOCH)
}
