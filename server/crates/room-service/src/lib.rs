#![forbid(unsafe_code)]

//! 房间服务骨架：时钟注入、会话认证与下行消息序号。
//!
//! 依赖方向为 `room-service → protocol`、`room-service → rule-engine`；本 crate 负责
//! 网络与 I/O 边界，引擎保持纯库、不读时钟。

pub mod auth;
pub mod clock;
pub mod envelope;
pub mod error;
pub mod room;

pub use auth::{DEV_SMS_CODE, SESSION_TTL_MS, Session, SessionStore};
pub use clock::{Clock, FixedClock, SystemClock};
pub use envelope::SeatChannel;
pub use error::error_message;
pub use room::{PlayerSlot, Room, RoomError, RoomId, RoomRegistry, SEAT_COUNT};
