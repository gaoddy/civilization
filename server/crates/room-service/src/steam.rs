//! Steam 票据校验。
//!
//! 身份以 Steam Web API 的验票结果为准，不信任客户端自报的 SteamID。未配置 publisher
//! key 或 app id 时为 [`SteamVerifier::Disabled`]，Steam 登录返回 `STEAM_DISABLED`，
//! 手机号登录不受影响。

use std::time::{Duration, Instant};

use protocol::server::ErrorMessage;
use serde::Deserialize;

use crate::error::error_message;

/// Steam 验票接口。
const AUTHENTICATE_URL: &str =
    "https://api.steampowered.com/ISteamUserAuth/AuthenticateUserTicket/v1/";
/// 单次验票超时；失败不自动重试，由客户端触发。
const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);
/// 票据长度上限；超出直接判非法，不发起外部请求。
const MAX_TICKET_LEN: usize = 2048;

/// Steam Web API 配置。
#[derive(Debug, Clone)]
pub struct SteamConfig {
    pub api_key: String,
    pub app_id: u64,
}

impl SteamConfig {
    /// 从 `STEAM_WEB_API_KEY` 与 `STEAM_APP_ID` 读取；任一缺失或非法时返回 `None`。
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("STEAM_WEB_API_KEY").ok()?;
        let api_key = api_key.trim().to_string();
        if api_key.is_empty() {
            return None;
        }
        let app_id = std::env::var("STEAM_APP_ID").ok()?;
        let app_id = app_id.trim().parse().ok()?;
        Some(Self { api_key, app_id })
    }
}

/// Steam 票据校验器。
#[derive(Debug, Clone, Default)]
pub enum SteamVerifier {
    /// 调用 Steam Web API 验票。
    Http {
        client: reqwest::Client,
        config: SteamConfig,
    },
    /// 未配置 Steam Web API。
    #[default]
    Disabled,
    /// 测试注入的固定结果；不发起网络请求。
    Stub(Result<String, ErrorMessage>),
}

impl SteamVerifier {
    /// 未配置 Steam 的构造器。
    pub fn disabled() -> Self {
        Self::Disabled
    }

    /// 按环境变量装配；缺配置时为 `Disabled`。
    pub fn from_env() -> Self {
        Self::from_config(SteamConfig::from_env())
    }

    /// 按显式配置装配；`None` 时为 `Disabled`。
    pub fn from_config(config: Option<SteamConfig>) -> Self {
        let Some(config) = config else {
            return Self::Disabled;
        };
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("构建 Steam HTTP 客户端失败");
        Self::Http { client, config }
    }

    /// 测试桩：固定成功返回 `steam_id`。
    pub fn stub_ok(steam_id: &str) -> Self {
        Self::Stub(Ok(steam_id.to_string()))
    }

    /// 测试桩：固定返回错误。
    pub fn stub_err(error: ErrorMessage) -> Self {
        Self::Stub(Err(error))
    }

    /// 当前是否可校验票据。
    pub fn enabled(&self) -> bool {
        !matches!(self, Self::Disabled)
    }

    /// 校验一张票据，成功返回 SteamID。
    ///
    /// # Errors
    ///
    /// 票据格式非法或 Steam 判定无效为 `STEAM_AUTH_FAILED`；超时、网络错误或非 2xx 为
    /// `STEAM_UNAVAILABLE`；未配置为 `STEAM_DISABLED`。
    pub async fn verify(&self, ticket: &str) -> Result<String, ErrorMessage> {
        if !is_valid_ticket(ticket) {
            return Err(auth_failed_error("票据格式非法"));
        }
        match self {
            Self::Disabled => Err(steam_disabled_error()),
            Self::Stub(result) => result.clone(),
            Self::Http { client, config } => verify_http(client, config, ticket).await,
        }
    }
}

/// 票据只允许十六进制串，长度受限；非法输入不发起外部请求。
fn is_valid_ticket(ticket: &str) -> bool {
    !ticket.is_empty()
        && ticket.len() <= MAX_TICKET_LEN
        && ticket.chars().all(|ch| ch.is_ascii_hexdigit())
}

async fn verify_http(
    client: &reqwest::Client,
    config: &SteamConfig,
    ticket: &str,
) -> Result<String, ErrorMessage> {
    let started = Instant::now();
    let response = client
        .get(AUTHENTICATE_URL)
        .query(&[
            ("key", config.api_key.as_str()),
            ("appid", &config.app_id.to_string()),
            ("ticket", ticket),
        ])
        .send()
        .await;

    let body = match response {
        Ok(response) if response.status().is_success() => {
            response.json::<AuthenticateTicketResponse>().await
        }
        Ok(response) => {
            log_failure("http_status", started);
            return Err(unavailable_error(&format!(
                "Steam 验票返回 HTTP {}",
                response.status()
            )));
        }
        Err(error) => {
            log_failure("transport", started);
            return Err(unavailable_error(&format!("Steam 验票请求失败: {error}")));
        }
    };

    let body = match body {
        Ok(body) => body,
        Err(error) => {
            log_failure("decode", started);
            return Err(auth_failed_error(&format!(
                "Steam 验票响应无法解析: {error}"
            )));
        }
    };
    let Some(params) = body.response.params else {
        let code = body.response.error.and_then(|error| error.errorcode);
        log_failure("steam_error", started);
        return Err(auth_failed_error(&format!(
            "Steam 验票未通过，错误码 {code:?}"
        )));
    };
    if params.result != "OK" {
        log_failure("not_ok", started);
        return Err(auth_failed_error("Steam 验票未通过"));
    }
    let Some(steam_id) = params.steamid.filter(|value| !value.is_empty()) else {
        log_failure("missing_steamid", started);
        return Err(auth_failed_error("Steam 验票缺少 steamid"));
    };
    Ok(steam_id)
}

/// 失败日志只记录原因与耗时，不记录票据、key 或响应原文。
fn log_failure(reason: &str, started: Instant) {
    eprintln!(
        "room-service: Steam 验票失败 reason={reason} latency_ms={}",
        started.elapsed().as_millis()
    );
}

#[derive(Debug, Deserialize)]
struct AuthenticateTicketResponse {
    response: AuthenticateTicketBody,
}

#[derive(Debug, Deserialize)]
struct AuthenticateTicketBody {
    #[serde(default)]
    params: Option<AuthenticateTicketParams>,
    #[serde(default)]
    error: Option<AuthenticateTicketError>,
}

#[derive(Debug, Deserialize)]
struct AuthenticateTicketParams {
    result: String,
    #[serde(default)]
    steamid: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AuthenticateTicketError {
    #[serde(default)]
    errorcode: Option<i64>,
}

/// 票据无效或 appid 不匹配；可重试（重新取票）。
pub fn auth_failed_error(message: &str) -> ErrorMessage {
    error_message("STEAM_AUTH_FAILED", message, true)
}

/// Steam 服务超时或不可用；可重试。
pub fn unavailable_error(message: &str) -> ErrorMessage {
    error_message("STEAM_UNAVAILABLE", message, true)
}

/// Steam 登录未配置时的下行错误。
pub fn steam_disabled_error() -> ErrorMessage {
    error_message("STEAM_DISABLED", "Steam 登录未配置", false)
}

#[cfg(test)]
mod tests {
    use super::{SteamVerifier, is_valid_ticket};

    #[test]
    fn ticket_format_accepts_hex_only() {
        assert!(is_valid_ticket("14000000deadbeef"));
        assert!(is_valid_ticket("ABCDEF0123"));
        assert!(!is_valid_ticket(""));
        assert!(!is_valid_ticket("not-hex"));
        assert!(!is_valid_ticket(&"a".repeat(2049)));
    }

    #[tokio::test]
    async fn disabled_verifier_reports_disabled() {
        let error = SteamVerifier::disabled()
            .verify("deadbeef")
            .await
            .expect_err("未配置时必须拒绝");
        assert_eq!(error.code.to_string(), "STEAM_DISABLED");
    }

    #[tokio::test]
    async fn invalid_ticket_does_not_reach_backend() {
        let error = SteamVerifier::stub_ok("76561198000000001")
            .verify("not-hex")
            .await
            .expect_err("非法票据必须直接拒绝");
        assert_eq!(error.code.to_string(), "STEAM_AUTH_FAILED");
    }
}
