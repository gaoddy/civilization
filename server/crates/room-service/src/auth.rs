//! 会话认证：手机号 + 开发验证码登录，或令牌续登。
//!
//! 会话本身是进程内 `BTreeMap`，无 I/O：重启后需重新登录（同一手机号由
//! [`crate::account::AccountStore`] 恢复同一账号）。`playerId` 与 `token` 都是服务端
//! 生成的不可预测不透明串，不暴露手机号。过期会话在下次被访问或调用
//! [`SessionStore::purge_expired`] 时清除，M1 不跑后台清理任务。

use std::collections::BTreeMap;
use std::fmt;

use chrono::{DateTime, Utc};
use protocol::client::AuthRequest;
use protocol::server::{AuthResult, ErrorMessage};
use uuid::Uuid;

use crate::account::Account;
use crate::clock::Clock;
use crate::error::error_message_with_id;

/// 开发环境固定短信验证码。
pub const DEV_SMS_CODE: &str = "123456";

/// 会话有效期：24 小时。
pub const SESSION_TTL_MS: u64 = 24 * 60 * 60 * 1000;

/// 玩家编号起点；编号为 6 位数字，范围 `100000..1000000`。
pub const PLAYER_CODE_START: u64 = 100_000;

/// 玩家编号上界（不含）。
pub const PLAYER_CODE_END: u64 = 1_000_000;

/// 认证成功后返回的会话。
#[derive(Clone, PartialEq, Eq)]
pub struct Session {
    /// 服务端生成的不透明玩家 id。
    pub player_id: String,
    /// 对外展示与好友搜索使用的 6 位数字编号。
    pub player_code: String,
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
            .field("player_code", &self.player_code)
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
            player_code: self.player_code.clone(),
            player_id: self.player_id.clone(),
            token: self.token.clone(),
        }
    }
}

/// 进程内会话存储。
#[derive(Debug)]
pub struct SessionStore {
    by_player: BTreeMap<String, Session>,
    by_token: BTreeMap<String, String>,
    /// 已分配的玩家编号 → player_id；进程内不复用，避免好友关系指向新玩家。
    codes: BTreeMap<String, String>,
    /// 已创建会话的单调序号，只用于生成 `玩家N`，不因清理而回退。
    session_ordinal: u64,
    /// 下一个待分配的 6 位编号。
    next_code: u64,
}

impl Default for SessionStore {
    fn default() -> Self {
        Self {
            by_player: BTreeMap::new(),
            by_token: BTreeMap::new(),
            codes: BTreeMap::new(),
            session_ordinal: 0,
            next_code: PLAYER_CODE_START,
        }
    }
}

impl SessionStore {
    /// 创建空存储。
    pub fn new() -> Self {
        Self::default()
    }

    /// 分配一个进程内唯一的 6 位玩家编号。
    ///
    /// 编号从 [`PLAYER_CODE_START`] 起单调分配，回绕后跳过已占用编号；已分配的编号
    /// 不因会话过期而回收，避免旧好友关系指向后来复用同一编号的玩家。
    fn allocate_code(&mut self, player_id: &str) -> String {
        for _ in 0..(PLAYER_CODE_END - PLAYER_CODE_START) {
            let code = format!("{:06}", self.next_code);
            self.next_code = if self.next_code + 1 >= PLAYER_CODE_END {
                PLAYER_CODE_START
            } else {
                self.next_code + 1
            };
            if !self.codes.contains_key(&code) {
                self.codes.insert(code.clone(), player_id.to_string());
                return code;
            }
        }
        // 90 万个编号在单进程内耗尽属于不可恢复状态；保留最后一个编号而不是崩溃。
        format!("{:06}", self.next_code)
    }

    /// 认证一次请求。
    ///
    /// 内存模式的完整入口：手机号分支按本进程序号分配身份。接入数据库时由调用方
    /// 先经 `AccountStore` 解析账号，再用 [`SessionStore::authenticate_with`]。
    ///
    /// # Errors
    ///
    /// 返回带稳定错误码的 [`ErrorMessage`]，见 [`SessionStore::authenticate_with`]。
    pub fn authenticate(
        &mut self,
        req: &AuthRequest,
        clock: &dyn Clock,
    ) -> Result<Session, ErrorMessage> {
        self.authenticate_with(req, clock, None)
    }

    /// 认证一次请求，`account` 为调用方已解析的持久化账号。
    ///
    /// - 手机号分支：验证码等于 [`DEV_SMS_CODE`] 才成功；`display_name` 缺省时按本进程
    ///   第几个新会话生成 `玩家N`（从 1 起）。提供了 `account` 时直接签发会话。
    /// - 令牌分支：按令牌查到会话且未过期才成功，否则 `TOKEN_EXPIRED`。
    ///
    /// # Errors
    ///
    /// 验证码错误 `AUTH_FAILED`，令牌无效或过期 `TOKEN_EXPIRED`，两者都不可原样重试。
    pub fn authenticate_with(
        &mut self,
        req: &AuthRequest,
        clock: &dyn Clock,
        account: Option<Account>,
    ) -> Result<Session, ErrorMessage> {
        match req {
            AuthRequest::Variant0 {
                code,
                display_name,
                phone: _,
            } => {
                if code != DEV_SMS_CODE {
                    return Err(invalid_code_error());
                }

                let account = match account {
                    Some(account) => account,
                    None => {
                        self.session_ordinal = self.session_ordinal.saturating_add(1);
                        let display_name = match display_name {
                            Some(name) => String::from(name.clone()),
                            None => format!("玩家{}", self.session_ordinal),
                        };
                        let player_id = format!("p_{}", Uuid::new_v4().simple());
                        Account {
                            player_code: self.allocate_code(&player_id),
                            player_id,
                            display_name,
                        }
                    }
                };

                self.codes
                    .insert(account.player_code.clone(), account.player_id.clone());
                let session = Session {
                    player_id: account.player_id,
                    player_code: account.player_code,
                    token: Uuid::new_v4().simple().to_string(),
                    display_name: account.display_name,
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

/// 验证码错误的下行载荷；入口在解析账号前先用它拒绝非法验证码。
pub(crate) fn invalid_code_error() -> ErrorMessage {
    error_message_with_id(
        "AUTH_FAILED",
        "error.AUTH_FAILED.invalid_code",
        &[],
        "短信验证码错误",
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
