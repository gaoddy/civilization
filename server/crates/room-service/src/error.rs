//! 错误码到下行 [`ErrorMessage`] 的统一构造。
//!
//! 错误码取值只来自 `protocol/schema/v1/errors.schema.json`，只增不改。

use std::str::FromStr;

use protocol::ErrorCode;
use protocol::server::ErrorMessage;

/// 用稳定错误码构造下行错误载荷。
///
/// `code` 必须是协议错误码；未知取值退化为 `INTERNAL_ERROR`，避免服务端因调用错误
/// 而 panic，同时保留原始 `message` 便于排查。`message` 只面向日志，不作为 UI 文案；
/// 面向玩家的文案由默认 `messageId = "error.<CODE>"` 决定。
pub fn error_message(code: &str, message: &str, retryable: bool) -> ErrorMessage {
    let code = ErrorCode::from_str(code).unwrap_or(ErrorCode::InternalError);
    error_message_with_id(
        &code.to_string(),
        &format!("error.{code}"),
        &[],
        message,
        retryable,
    )
}

/// 用显式文本 id 构造下行错误载荷。
///
/// `message_params` 为空时序列化会省略该字段。
/// `message` 仍然只面向日志。
pub fn error_message_with_id(
    code: &str,
    message_id: &str,
    message_params: &[&str],
    message: &str,
    retryable: bool,
) -> ErrorMessage {
    let code = ErrorCode::from_str(code).unwrap_or(ErrorCode::InternalError);
    ErrorMessage {
        client_intent_id: None,
        code,
        message_id: message_id.to_string(),
        message_params: message_params
            .iter()
            .map(|param| (*param).to_string())
            .collect(),
        message: message.to_string(),
        retryable,
    }
}
