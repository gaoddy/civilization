#![forbid(unsafe_code)]

//! 房间服务骨架：时钟注入、会话认证与下行消息序号。
//!
//! 依赖方向为 `room-service → protocol`、`room-service → rule-engine`；本 crate 负责
//! 网络与 I/O 边界，引擎保持纯库、不读时钟。

pub mod auth;
pub mod clock;
pub mod envelope;
pub mod error;
pub mod filter;
pub mod game;
pub mod net;
pub mod room;
pub mod view;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use rule_engine::data::Catalog;
use tokio::sync::Mutex;

const CARDS_JSON: &str = include_str!("../../../data/v0.1/cards.json");
const SKILLS_JSON: &str = include_str!("../../../data/v0.1/skills.json");
const TOKENS_JSON: &str = include_str!("../../../data/v0.1/tokens.json");
const CONFIG_JSON: &str = include_str!("../../../data/v0.1/config.json");

/// 服务共享状态。
///
/// 锁粒度：会话表与房间注册表各自一把 [`tokio::sync::Mutex`]，只在短操作内持有、
/// 不跨网络等待；房间内的权威可变状态由每个房间的写者任务串行持有（见 [`net`]）。
#[derive(Clone)]
pub struct AppState {
    pub sessions: Arc<Mutex<SessionStore>>,
    pub rooms: Arc<Mutex<RoomRegistry>>,
    pub clock: Arc<dyn Clock>,
    pub catalog: Catalog,
    /// 已创建房间 → 房间任务入口（含房间码与对局种子）。
    pub hub: Arc<Mutex<net::RoomHub>>,
}

impl AppState {
    /// 用仓库内嵌的 v0.1 数据装配共享状态。
    ///
    /// # Errors
    ///
    /// 卡表加载失败、或英雄卡不足以给满座每人分配一组候选时返回错误文本。
    pub fn new(clock: Arc<dyn Clock>) -> Result<Self, String> {
        let catalog = Catalog::from_json(CARDS_JSON, SKILLS_JSON, TOKENS_JSON, CONFIG_JSON)
            .map_err(|err| format!("加载卡牌目录失败: {err}"))?;
        ensure_hero_pool(catalog.cards.values().filter(|card| card.is_hero).count())?;
        Ok(Self {
            sessions: Arc::new(Mutex::new(SessionStore::new())),
            rooms: Arc::new(Mutex::new(RoomRegistry::new())),
            clock,
            catalog,
            hub: Arc::new(Mutex::new(net::RoomHub::default())),
        })
    }

    /// 当前存活房间 task 的数量；供测试断言房间回收。
    pub async fn hub_len(&self) -> usize {
        self.hub.lock().await.len()
    }
}

/// 校验英雄卡够满座每人一组候选。
///
/// 每人一段、每段 [`room::CANDIDATE_COUNT`] 张；卡不够时最后几名玩家拿到空候选，
/// 此后既不能选英雄也不能准备，房间开不了局、该页又没有离开入口。装配阶段直接
/// 拒绝启动，比运行期卡住更容易发现。
///
/// # Errors
///
/// 英雄卡数量小于 `SEAT_COUNT * CANDIDATE_COUNT` 时返回错误文本。
fn ensure_hero_pool(hero_count: usize) -> Result<(), String> {
    let required = room::SEAT_COUNT * room::CANDIDATE_COUNT;
    if hero_count < required {
        return Err(format!(
            "英雄卡 {hero_count} 张，少于满座所需的 {required} 张"
        ));
    }
    Ok(())
}

/// 绑定监听地址并构造路由，同时返回共享状态（测试用它观察房间回收）。
///
/// # Errors
///
/// 同 [`bind`]。
pub async fn bind_state(
    listen: &str,
    clock: Arc<dyn Clock>,
) -> Result<(SocketAddr, Router, AppState), String> {
    let state = AppState::new(clock)?;
    let listener = tokio::net::TcpListener::bind(listen)
        .await
        .map_err(|err| format!("监听 {listen} 失败: {err}"))?;
    let addr = listener
        .local_addr()
        .map_err(|err| format!("读取监听地址失败: {err}"))?;
    drop(listener);
    Ok((addr, net::router(state.clone()), state))
}

/// 绑定监听地址并构造路由，便于测试拿到系统分配的真实端口。
///
/// `listen` 写 `127.0.0.1:0` 时由系统分配端口。本函数只探测端口后立即释放监听器，
/// 实际服务由 [`serve`] 重新绑定同一地址完成。
///
/// # Errors
///
/// 地址无法解析、端口绑定失败或卡表加载失败。
pub async fn bind(listen: &str, clock: Arc<dyn Clock>) -> Result<(SocketAddr, Router), String> {
    let (addr, router, _state) = bind_state(listen, clock).await?;
    Ok((addr, router))
}

/// 在指定地址上启动 HTTP/WebSocket 服务，直到进程被关闭。
///
/// # Errors
///
/// 端口绑定失败或服务异常退出。
pub async fn serve(addr: SocketAddr, router: Router) -> Result<(), String> {
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|err| format!("监听 {addr} 失败: {err}"))?;
    axum::serve(listener, router)
        .await
        .map_err(|err| format!("服务退出: {err}"))
}

pub use auth::{DEV_SMS_CODE, SESSION_TTL_MS, Session, SessionStore};
pub use clock::{Clock, FixedClock, SystemClock};
pub use envelope::SeatChannel;
pub use error::error_message;
pub use game::{
    Deduped, GameTask, RECONNECT_GRACE_MS, RoomCommand, SeatUpdate, TIMER_WARNING_SECONDS,
    UpdateKind,
};
pub use room::{PlayerSlot, Room, RoomError, RoomId, RoomRegistry, SEAT_COUNT};
pub use view::SeatInfo;

#[cfg(test)]
mod tests {
    use super::{ensure_hero_pool, room};

    #[test]
    fn hero_pool_must_cover_every_seat() {
        let required = room::SEAT_COUNT * room::CANDIDATE_COUNT;
        assert!(ensure_hero_pool(required).is_ok(), "刚好够分应通过");
        let error = ensure_hero_pool(required - 1).expect_err("缺一张就必须拒绝启动");
        assert!(
            error.contains(&required.to_string()),
            "错误文本应写出所需张数，实际：{error}"
        );
    }
}
