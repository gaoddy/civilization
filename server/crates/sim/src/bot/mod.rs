//! 机器人子模块：只读视图决策 [`brain`] 与真实 WebSocket 客户端 [`client`]。

pub mod brain;
pub mod client;

/// 开发环境固定短信验证码；与 `room-service` 的 `DEV_SMS_CODE` 保持一致。
/// `sim` 不能依赖 `room-service`，因此在此重复声明。
pub const DEV_SMS_CODE: &str = "123456";
