//! 手机号绑定验证码：开发环境固定码、5 分钟过期与 60 秒重发冷却。
//!
//! 验证码只存在进程内，服务重启即失效；手机号登录的短信通道与绑定验证码是两条
//! 独立链路，前者仍是开发固定码。生产短信通道未接入，见 `open-questions.md`。

use std::collections::BTreeMap;

/// 开发环境固定验证码。
pub const DEV_PHONE_CODE: &str = "123456";

/// 验证码有效期：5 分钟。
pub const CODE_TTL_MS: u64 = 5 * 60 * 1000;

/// 同一账号的重发冷却：60 秒。
pub const RESEND_COOLDOWN_MS: u64 = 60 * 1000;

/// 手机号格式：可带前导 `+`，数字 6–15 位。国际短信通道确定前只做结构校验。
pub fn is_valid_phone(phone: &str) -> bool {
    let digits = phone.strip_prefix('+').unwrap_or(phone);
    (6..=15).contains(&digits.len()) && digits.chars().all(|ch| ch.is_ascii_digit())
}

/// 发送验证码失败原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueError {
    /// 距上次发送不足 [`RESEND_COOLDOWN_MS`]。
    Cooldown,
}

/// 校验验证码失败原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckError {
    /// 验证码不匹配或没有待校验记录。
    Invalid,
    /// 验证码已过期。
    Expired,
}

#[derive(Debug, Clone)]
struct PendingCode {
    phone: String,
    code: String,
    sent_at_ms: u64,
    expires_at_ms: u64,
}

/// 按账号保存待校验的验证码。
#[derive(Debug, Default)]
pub struct PhoneCodeStore {
    by_player: BTreeMap<String, PendingCode>,
}

impl PhoneCodeStore {
    /// 创建空存储。
    pub fn new() -> Self {
        Self::default()
    }

    /// 生成或复用开发验证码，返回过期时刻（Unix epoch 毫秒）。
    ///
    /// # Errors
    ///
    /// 同一账号在冷却期内再次请求返回 [`IssueError::Cooldown`]。
    pub fn issue(&mut self, player_id: &str, phone: &str, now_ms: u64) -> Result<u64, IssueError> {
        if let Some(pending) = self.by_player.get(player_id)
            && now_ms < pending.sent_at_ms.saturating_add(RESEND_COOLDOWN_MS)
        {
            return Err(IssueError::Cooldown);
        }
        let expires_at_ms = now_ms.saturating_add(CODE_TTL_MS);
        self.by_player.insert(
            player_id.to_string(),
            PendingCode {
                phone: phone.to_string(),
                code: DEV_PHONE_CODE.to_string(),
                sent_at_ms: now_ms,
                expires_at_ms,
            },
        );
        Ok(expires_at_ms)
    }

    /// 校验并消费验证码；成功后记录被删除，不能重复使用。
    ///
    /// # Errors
    ///
    /// 无记录、手机号不一致或验证码不匹配为 [`CheckError::Invalid`]；过期先删除再返回
    /// [`CheckError::Expired`]。
    pub fn consume(
        &mut self,
        player_id: &str,
        phone: &str,
        code: &str,
        now_ms: u64,
    ) -> Result<(), CheckError> {
        let Some(pending) = self.by_player.get(player_id) else {
            return Err(CheckError::Invalid);
        };
        if pending.phone != phone {
            return Err(CheckError::Invalid);
        }
        if now_ms >= pending.expires_at_ms {
            self.by_player.remove(player_id);
            return Err(CheckError::Expired);
        }
        if pending.code != code {
            return Err(CheckError::Invalid);
        }
        self.by_player.remove(player_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{CheckError, DEV_PHONE_CODE, IssueError, PhoneCodeStore, is_valid_phone};

    const NOW: u64 = 1_700_000_000_000;

    #[test]
    fn phone_format_accepts_e164_like_numbers() {
        assert!(is_valid_phone("13800000000"));
        assert!(is_valid_phone("+14155552671"));
        assert!(!is_valid_phone(""));
        assert!(!is_valid_phone("138 0000"));
        assert!(!is_valid_phone("+12345"));
    }

    #[test]
    fn issue_respects_cooldown() {
        let mut store = PhoneCodeStore::new();
        let expires = store
            .issue("p_1", "13800000000", NOW)
            .expect("首次发送应成功");
        assert_eq!(expires, NOW + 5 * 60 * 1000);
        assert_eq!(
            store.issue("p_1", "13800000000", NOW + 59_999),
            Err(IssueError::Cooldown)
        );
        store
            .issue("p_1", "13800000000", NOW + 60_000)
            .expect("冷却结束后可重发");
    }

    #[test]
    fn consume_checks_phone_code_and_expiry() {
        let mut store = PhoneCodeStore::new();
        store.issue("p_1", "13800000000", NOW).expect("发送应成功");
        assert_eq!(
            store.consume("p_1", "13800000000", "000000", NOW),
            Err(CheckError::Invalid)
        );
        assert_eq!(
            store.consume("p_1", "13900000000", DEV_PHONE_CODE, NOW),
            Err(CheckError::Invalid)
        );
        assert_eq!(
            store.consume("p_1", "13800000000", DEV_PHONE_CODE, NOW + 5 * 60 * 1000),
            Err(CheckError::Expired)
        );
        assert_eq!(
            store.consume("p_1", "13800000000", DEV_PHONE_CODE, NOW),
            Err(CheckError::Invalid),
            "过期记录已清除"
        );
    }

    #[test]
    fn consumed_code_cannot_be_reused() {
        let mut store = PhoneCodeStore::new();
        store.issue("p_1", "13800000000", NOW).expect("发送应成功");
        store
            .consume("p_1", "13800000000", DEV_PHONE_CODE, NOW)
            .expect("正确验证码应通过");
        assert_eq!(
            store.consume("p_1", "13800000000", DEV_PHONE_CODE, NOW),
            Err(CheckError::Invalid)
        );
    }
}
