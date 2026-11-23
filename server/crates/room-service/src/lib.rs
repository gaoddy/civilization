#![forbid(unsafe_code)]

//! 房间服务骨架：时钟注入、会话认证与下行消息序号。
//!
//! 依赖方向为 `room-service → protocol`、`room-service → rule-engine`；本 crate 负责
//! 网络与 I/O 边界，引擎保持纯库、不读时钟。

pub mod account;
pub mod auth;
pub mod clock;
pub mod db;
pub mod envelope;
pub mod error;
pub mod filter;
pub mod game;
pub mod net;
pub mod phone_code;
pub mod room;
pub mod social;
pub mod steam;
pub mod view;

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use rule_engine::data::{Catalog, CategoryKind, Playstyle};
use tokio::sync::Mutex;

const CARDS_JSON: &str = include_str!("../../../data/v0.1/cards.json");
const HEROES_JSON: &str = include_str!("../../../data/v0.1/heroes.json");
const SKILLS_JSON: &str = include_str!("../../../data/v0.1/skills.json");
const CONFIG_JSON: &str = include_str!("../../../data/v0.1/config.json");
const TAXONOMY_JSON: &str = include_str!("../../../data/v0.1/taxonomy.json");
const ACHIEVEMENTS_JSON: &str = include_str!("../../../data/v0.2/achievements.json");

/// 成就定义目录：成就 id → Steamworks 成就 API 名。
///
/// 判定与奖励由规则引擎的成就运行时负责；本目录只承担 Steam 侧映射，
/// 供房间服务在账号首次解锁时下发。
#[derive(Clone, Debug, Default)]
pub struct AchievementCatalog {
    by_id: BTreeMap<String, String>,
}

/// 生成数据的单条成就；只反序列化映射需要的字段。
#[derive(Debug, serde::Deserialize)]
struct AchievementRecord {
    id: String,
    steam_api_name: String,
}

impl AchievementCatalog {
    /// 从生成 JSON 建立映射，拒绝空 id、空 API 名与重复 id。
    ///
    /// # Errors
    ///
    /// JSON 无法解析、字段为空或 id 重复时返回错误文本。
    pub fn from_json(raw: &str) -> Result<Self, String> {
        let records: Vec<AchievementRecord> =
            serde_json::from_str(raw).map_err(|err| format!("解析成就数据失败: {err}"))?;
        let mut by_id = BTreeMap::new();
        for record in records {
            if record.id.is_empty() || record.steam_api_name.is_empty() {
                return Err("成就数据的 id 与 steam_api_name 不得为空".to_string());
            }
            if by_id
                .insert(record.id.clone(), record.steam_api_name)
                .is_some()
            {
                return Err(format!("成就 id 重复: {}", record.id));
            }
        }
        if by_id.is_empty() {
            return Err("成就数据为空".to_string());
        }
        Ok(Self { by_id })
    }

    /// 用仓库内嵌的生成数据建立映射；数据非法属于打包错误，直接 panic。
    #[must_use]
    pub fn embedded() -> Self {
        Self::from_json(ACHIEVEMENTS_JSON).expect("内嵌成就数据应合法")
    }

    /// 查询成就对应的 Steamworks API 名。
    pub fn steam_api_name(&self, achievement_id: &str) -> Option<&str> {
        self.by_id.get(achievement_id).map(String::as_str)
    }

    /// 成就数量。
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// 是否没有成就定义。
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

/// 服务共享状态。
///
/// 锁粒度：会话、账号、房间注册表与社交存储各自一把 [`tokio::sync::Mutex`]。配置
/// 数据库时，账号与社交写操作会在持锁期间等待数据库；持有时间受单次事务限制，
/// 不跨网络往返叠加。房间内的权威可变状态由每个房间的写者任务串行持有（见 [`net`]）。
#[derive(Clone)]
pub struct AppState {
    pub sessions: Arc<Mutex<SessionStore>>,
    /// 账号身份：数据库写穿与内存缓存；同一手机号恢复同一账号。
    pub accounts: Arc<Mutex<account::AccountStore>>,
    pub rooms: Arc<Mutex<RoomRegistry>>,
    pub clock: Arc<dyn Clock>,
    pub catalog: Catalog,
    /// 已创建房间 → 房间任务入口（含房间码与对局种子）。
    pub hub: Arc<Mutex<net::RoomHub>>,
    /// 好友、申请、最近对局与邀请；配置数据库时以库为准。
    pub social: Arc<Mutex<social::SocialStore>>,
    /// 在线连接登记；好友推送与控制消息按 `player_id` 定位连接。
    pub connections: Arc<Mutex<net::ConnectionRegistry>>,
    /// Steam 票据校验；未配置时为 `Disabled`，手机号登录不受影响。
    pub steam: Arc<steam::SteamVerifier>,
    /// 成就 id → Steam API 名映射；随仓库数据内嵌，启动时加载。
    pub achievements: Arc<AchievementCatalog>,
    /// 手机号绑定验证码；进程内，重启失效。
    pub phone_codes: Arc<Mutex<phone_code::PhoneCodeStore>>,
    /// 持久化数据库；未配置 `DATABASE_URL` 时为 `None`，就绪探针直接通过。
    pub db: Option<Arc<db::Db>>,
}

impl AppState {
    /// 用仓库内嵌的 v0.1 数据装配共享状态；无数据库，身份与社交数据只存在内存。
    ///
    /// # Errors
    ///
    /// 卡表加载失败、或英雄卡为空 / 分类字段不合法时返回错误文本。
    pub fn new(clock: Arc<dyn Clock>) -> Result<Self, String> {
        Ok(Self {
            sessions: Arc::new(Mutex::new(SessionStore::new())),
            accounts: Arc::new(Mutex::new(account::AccountStore::new())),
            rooms: Arc::new(Mutex::new(RoomRegistry::new())),
            clock,
            catalog: load_catalog()?,
            hub: Arc::new(Mutex::new(net::RoomHub::default())),
            social: Arc::new(Mutex::new(social::SocialStore::new())),
            connections: Arc::new(Mutex::new(net::ConnectionRegistry::default())),
            steam: Arc::new(steam::SteamVerifier::disabled()),
            achievements: Arc::new(load_achievements()?),
            phone_codes: Arc::new(Mutex::new(phone_code::PhoneCodeStore::new())),
            db: None,
        })
    }

    /// 装配共享状态并从数据库重建账号与社交数据。
    ///
    /// 数据库是唯一真相源：账号、好友、申请、最近对局与邀请都在启动时载入。
    ///
    /// # Errors
    ///
    /// 卡表加载失败或数据库读取失败时返回错误文本。
    pub async fn with_database(clock: Arc<dyn Clock>, db: Arc<db::Db>) -> Result<Self, String> {
        let catalog = load_catalog()?;
        let accounts = account::AccountStore::load(db.clone()).await?;
        let social = social::SocialStore::load(db.clone(), clock.now_ms()).await?;
        Ok(Self {
            sessions: Arc::new(Mutex::new(SessionStore::new())),
            accounts: Arc::new(Mutex::new(accounts)),
            rooms: Arc::new(Mutex::new(RoomRegistry::new())),
            clock,
            catalog,
            hub: Arc::new(Mutex::new(net::RoomHub::default())),
            social: Arc::new(Mutex::new(social)),
            connections: Arc::new(Mutex::new(net::ConnectionRegistry::default())),
            steam: Arc::new(steam::SteamVerifier::disabled()),
            achievements: Arc::new(load_achievements()?),
            phone_codes: Arc::new(Mutex::new(phone_code::PhoneCodeStore::new())),
            db: Some(db),
        })
    }

    /// 当前存活房间 task 的数量；供测试断言房间回收。
    pub async fn hub_len(&self) -> usize {
        self.hub.lock().await.len()
    }

    /// 装配 Steam 校验器；未调用时为 `Disabled`。
    pub fn with_steam(mut self, verifier: steam::SteamVerifier) -> Self {
        self.steam = Arc::new(verifier);
        self
    }
}

/// 加载并校验仓库内嵌卡表。
fn load_catalog() -> Result<Catalog, String> {
    let catalog = Catalog::from_json(
        CARDS_JSON,
        HEROES_JSON,
        SKILLS_JSON,
        CONFIG_JSON,
        TAXONOMY_JSON,
    )
    .map_err(|err| format!("加载卡牌目录失败: {err}"))?;
    ensure_hero_pool(&catalog)?;
    Ok(catalog)
}

/// 加载仓库内嵌成就映射。
///
/// # Errors
///
/// 成就数据无法解析、为空或 id 重复时返回错误文本。
fn load_achievements() -> Result<AchievementCatalog, String> {
    AchievementCatalog::from_json(ACHIEVEMENTS_JSON)
}

/// 校验英雄卡池：至少 1 张英雄，且每张英雄的关联分类合法。
///
/// 候选池允许小于满座需求（池不足时跨玩家重复），因此不要求英雄达到
/// `SEAT_COUNT * CANDIDATE_COUNT` 张；只要至少有一张可用英雄，房间就能开局。
/// 英雄卡不持有 `taxon`；`categories` 至少 1 条 `play:`，玩法取值必须合法，
/// `prey:` / `host:` 节点必须在分类树中。
///
/// # Errors
///
/// 英雄卡为空，或任意英雄的分类条目不合法时返回错误文本。
fn ensure_hero_pool(catalog: &Catalog) -> Result<(), String> {
    let heroes: Vec<&rule_engine::data::HeroCard> = catalog.heroes.values().collect();
    if heroes.is_empty() {
        return Err("卡表没有英雄卡".to_string());
    }
    for hero in heroes {
        let mut has_play = false;
        for category in &hero.categories {
            match category.kind {
                CategoryKind::Play => {
                    has_play = true;
                    if Playstyle::from_id(&category.value).is_none() {
                        return Err(format!(
                            "英雄卡 {} 的 categories play:{} 不是合法玩法",
                            hero.id, category.value
                        ));
                    }
                }
                CategoryKind::Prey | CategoryKind::Host => {
                    if !catalog.taxonomy.contains(&category.value) {
                        return Err(format!(
                            "英雄卡 {} 的 categories 节点 {} 不在分类树中",
                            hero.id, category.value
                        ));
                    }
                }
            }
        }
        if !has_play {
            return Err(format!(
                "英雄卡 {} 的 categories 至少要有 1 条 play:",
                hero.id
            ));
        }
    }
    Ok(())
}

/// 绑定监听地址并构造路由，Steam 校验器可指定；`db` 为 `None` 时使用内存存储。
///
/// # Errors
///
/// 同 [`bind`]；数据库读取失败时同样返回错误。
pub async fn bind_with_steam(
    listen: &str,
    clock: Arc<dyn Clock>,
    db: Option<Arc<db::Db>>,
    verifier: steam::SteamVerifier,
) -> Result<(SocketAddr, Router), String> {
    let state = match db {
        Some(db) => AppState::with_database(clock, db).await?,
        None => AppState::new(clock)?,
    };
    let (addr, router) = bind_router(listen, state.with_steam(verifier)).await?;
    Ok((addr, router))
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
    let (addr, router) = bind_router(listen, state.clone()).await?;
    Ok((addr, router, state))
}

/// 绑定监听地址并构造路由，同时返回共享状态；社交数据从数据库载入。
///
/// # Errors
///
/// 同 [`bind`]，或数据库读取失败。
pub async fn bind_state_with_db(
    listen: &str,
    clock: Arc<dyn Clock>,
    db: Arc<db::Db>,
) -> Result<(SocketAddr, Router, AppState), String> {
    let state = AppState::with_database(clock, db).await?;
    let (addr, router) = bind_router(listen, state.clone()).await?;
    Ok((addr, router, state))
}

async fn bind_router(listen: &str, state: AppState) -> Result<(SocketAddr, Router), String> {
    let listener = tokio::net::TcpListener::bind(listen)
        .await
        .map_err(|err| format!("监听 {listen} 失败: {err}"))?;
    let addr = listener
        .local_addr()
        .map_err(|err| format!("读取监听地址失败: {err}"))?;
    drop(listener);
    Ok((addr, net::router(state)))
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

/// 绑定监听地址并构造路由，社交数据从数据库载入。
///
/// # Errors
///
/// 同 [`bind`]，或数据库读取失败。
pub async fn bind_with_db(
    listen: &str,
    clock: Arc<dyn Clock>,
    db: Arc<db::Db>,
) -> Result<(SocketAddr, Router), String> {
    let (addr, router, _state) = bind_state_with_db(listen, clock, db).await?;
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

pub use account::{Account, AccountStore};
pub use auth::{
    DEV_SMS_CODE, PLAYER_CODE_END, PLAYER_CODE_START, SESSION_TTL_MS, Session, SessionStore,
};
pub use clock::{Clock, FixedClock, SystemClock};
pub use db::Db;
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
    use super::{
        AchievementCatalog, CARDS_JSON, CONFIG_JSON, HEROES_JSON, SKILLS_JSON, TAXONOMY_JSON,
        ensure_hero_pool, load_achievements,
    };
    use rule_engine::data::{Catalog, Category, CategoryKind};

    fn catalog() -> Catalog {
        Catalog::from_json(
            CARDS_JSON,
            HEROES_JSON,
            SKILLS_JSON,
            CONFIG_JSON,
            TAXONOMY_JSON,
        )
        .expect("卡表应能加载")
    }

    #[test]
    fn hero_pool_requires_at_least_one_hero() {
        let mut catalog = catalog();
        catalog.heroes.clear();
        let error = ensure_hero_pool(&catalog).expect_err("没有英雄卡必须拒绝启动");
        assert!(error.contains("没有英雄卡"), "实际：{error}");
    }

    #[test]
    fn hero_pool_rejects_invalid_play_category() {
        let mut catalog = catalog();
        let hero = catalog.heroes.values_mut().next().expect("应有英雄卡");
        hero.categories = vec![Category {
            kind: CategoryKind::Play,
            value: "not-a-playstyle".to_string(),
        }];
        let error = ensure_hero_pool(&catalog).expect_err("非法玩法关联必须拒绝启动");
        assert!(error.contains("不是合法玩法"), "实际：{error}");
    }

    #[test]
    fn hero_pool_accepts_loaded_catalog() {
        assert!(
            ensure_hero_pool(&catalog()).is_ok(),
            "仓库内嵌卡表应通过英雄校验"
        );
    }

    #[test]
    fn achievement_catalog_maps_repo_data() {
        let achievements = load_achievements().expect("仓库内嵌成就数据应能加载");
        assert_eq!(achievements.len(), 6, "v0.2 应有 6 个成就");
        assert_eq!(
            achievements.steam_api_name("genetics"),
            Some("GENETICS"),
            "成就 id 应映射到 Steam API 名"
        );
        assert_eq!(achievements.steam_api_name("unknown"), None);
    }

    #[test]
    fn achievement_catalog_rejects_duplicate_id() {
        let raw = r#"[
            {"id": "a", "steam_api_name": "AAA"},
            {"id": "a", "steam_api_name": "BBB"}
        ]"#;
        let error = AchievementCatalog::from_json(raw).expect_err("重复 id 必须拒绝");
        assert!(error.contains("重复"), "实际：{error}");
    }

    #[test]
    fn achievement_catalog_rejects_empty_mapping() {
        let error = AchievementCatalog::from_json("[]").expect_err("空映射必须拒绝");
        assert!(error.contains("为空"), "实际：{error}");
    }
}
