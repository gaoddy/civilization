#![allow(dead_code)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use protocol::server::{GameStarted, HeroPickStarted, RoomUpdate};
use protocol::{ServerEnvelope, ServerMessage};
use room_service::clock::{Clock, SystemClock};
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

pub type ClientWs = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// 真实 WebSocket 测试客户端：只负责收发与解析，不含业务断言。
pub struct RawClient {
    pub ws: ClientWs,
    next_id: u64,
}

impl RawClient {
    pub async fn connect(addr: std::net::SocketAddr) -> Self {
        let ws = connect_with_retry(&format!("ws://{addr}/ws")).await;
        Self { ws, next_id: 0 }
    }

    pub async fn send(&mut self, kind: &str, payload: Value) {
        self.next_id += 1;
        let envelope = json!({
            "v": 1,
            "id": format!("raw_{}", self.next_id),
            "type": kind,
            "ts": "2026-01-01T00:00:00Z",
            "payload": payload,
        });
        self.ws
            .send(Message::Text(envelope.to_string().into()))
            .await
            .expect("发送应当成功");
    }

    /// 取下一条下行消息与其 `seq`；连接关闭时返回 `None`。
    pub async fn recv(&mut self) -> Option<(u64, ServerMessage)> {
        while let Some(item) = self.ws.next().await {
            match item {
                Ok(Message::Text(text)) => {
                    let envelope: ServerEnvelope =
                        serde_json::from_str(text.as_str()).expect("下行信封应能解析");
                    let seq = envelope.seq.get();
                    let message = envelope.parse().expect("下行载荷应能解析");
                    return Some((seq, message));
                }
                Ok(Message::Close(_)) => return None,
                Ok(_) => continue,
                Err(_) => return None,
            }
        }
        None
    }

    /// 取下一条业务消息；认证与在线状态等社交推送不进入房间断言。
    pub async fn recv_room(&mut self) -> Option<(u64, ServerMessage)> {
        loop {
            let (seq, message) = self.recv().await?;
            if matches!(
                message,
                ServerMessage::FriendUpdate(_)
                    | ServerMessage::AccountView(_)
                    | ServerMessage::AchievementState(_)
                    | ServerMessage::AchievementUnlocked(_)
            ) {
                continue;
            }
            return Some((seq, message));
        }
    }
}

/// 由名字派生稳定且互不相同的手机号：同一手机号会恢复同一账号，测试客户端
/// 必须各自拥有独立身份。
pub fn phone_for(name: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in name.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("139{:08}", hash % 100_000_000)
}

/// 注册并登录，返回 `(player_id, token)`。
pub async fn authenticate(client: &mut RawClient, name: &str) -> (String, String) {
    client
        .send(
            "AuthRequest",
            json!({ "phone": phone_for(name), "code": "123456", "displayName": name }),
        )
        .await;
    loop {
        let (_, message) = client.recv().await.expect("认证期间连接不应关闭");
        match message {
            ServerMessage::AuthResult(result) => return (result.player_id, result.token),
            ServerMessage::Error(error) => panic!("认证失败: {} {}", error.code, error.message),
            _ => {}
        }
    }
}

/// 用已有 token 续登，返回 `player_id`。
pub async fn authenticate_with_token(client: &mut RawClient, token: &str) -> String {
    client
        .send(
            "AuthRequest",
            json!({ "token": token, "displayName": "reconnect" }),
        )
        .await;
    loop {
        let (_, message) = client.recv().await.expect("认证期间连接不应关闭");
        match message {
            ServerMessage::AuthResult(result) => return result.player_id,
            ServerMessage::Error(error) => {
                panic!("token 认证失败: {} {}", error.code, error.message)
            }
            _ => {}
        }
    }
}

/// 等一条 `RoomUpdate`；期间的其他下行消息忽略。
pub async fn wait_room_update(client: &mut RawClient) -> RoomUpdate {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let (_, message) = client
                .recv_room()
                .await
                .expect("等待 RoomUpdate 时连接关闭");
            if let ServerMessage::RoomUpdate(update) = message {
                return update;
            }
        }
    })
    .await
    .expect("10 秒内应收到 RoomUpdate")
}

/// 等一条满足条件的 `RoomUpdate`。
pub async fn wait_room_update_until(
    client: &mut RawClient,
    predicate: impl Fn(&RoomUpdate) -> bool,
) -> RoomUpdate {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let update = wait_room_update(client).await;
            if predicate(&update) {
                return update;
            }
        }
    })
    .await
    .expect("10 秒内应收到满足条件的 RoomUpdate")
}

/// 等全员准备：`count` 名成员的 `ready` 都为 true。
pub async fn wait_all_ready(client: &mut RawClient, count: usize) -> RoomUpdate {
    wait_room_update_until(client, |update| {
        update.players.len() == count && update.players.iter().all(|player| player.ready)
    })
    .await
}

/// 等选英雄阶段下发；若 `GameStarted` 先到则说明消息顺序错误。
pub async fn wait_hero_pick(client: &mut RawClient) -> HeroPickStarted {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let (_, message) = client
                .recv_room()
                .await
                .expect("等待 HeroPickStarted 时连接关闭");
            match message {
                ServerMessage::HeroPickStarted(pick) => return pick,
                ServerMessage::GameStarted(_) => {
                    panic!("GameStarted 先于 HeroPickStarted 到达，消息顺序错误")
                }
                ServerMessage::Error(error) => {
                    panic!(
                        "等待 HeroPickStarted 时收到错误 {}: {}",
                        error.code, error.message
                    )
                }
                _ => {}
            }
        }
    })
    .await
    .expect("10 秒内应收到 HeroPickStarted")
}

/// 等 `GameStarted`；返回其开局视图。
pub async fn wait_game_started(client: &mut RawClient) -> GameStarted {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let (_, message) = client
                .recv_room()
                .await
                .expect("等待 GameStarted 时连接关闭");
            match message {
                ServerMessage::GameStarted(started) => return started,
                ServerMessage::Error(error) => {
                    panic!(
                        "等待 GameStarted 时收到错误 {}: {}",
                        error.code, error.message
                    )
                }
                _ => {}
            }
        }
    })
    .await
    .expect("10 秒内应收到 GameStarted")
}

/// 取本人候选的第一张；候选由服务端在选英雄阶段下发。
pub fn first_candidate(pick: &HeroPickStarted) -> String {
    pick.candidates
        .first()
        .cloned()
        .expect("选英雄阶段应下发本人候选")
}

/// 8 人房里的标准开局流程：全员已准备 → 房主 `StartGame` → 每人选英雄 →
/// 等 `GameStarted`。返回按客户端顺序对齐的开局视图与所选英雄。
pub async fn start_and_pick_all(clients: &mut [RawClient]) -> (Vec<GameStarted>, Vec<String>) {
    assert_eq!(clients.len(), room_service::SEAT_COUNT, "开局需要 8 名真人");
    clients[0].send("StartGame", json!({})).await;

    let mut picks = Vec::with_capacity(clients.len());
    for client in clients.iter_mut() {
        let pick = wait_hero_pick(client).await;
        let hero = first_candidate(&pick);
        client
            .send("SelectHeroCard", json!({ "cardId": hero.clone() }))
            .await;
        picks.push(hero);
    }

    let mut started = Vec::with_capacity(clients.len());
    for client in clients.iter_mut() {
        started.push(wait_game_started(client).await);
    }
    (started, picks)
}

/// 建立 WS 连接；服务端 `bind` 探测端口后立刻释放，`serve` 再重新绑定，两者之间
/// 存在短暂空窗。默认并行跑多个用例时先到者可能撞上 `ConnectionRefused`，因此
/// 测试侧统一在这里重试，不改变生产签名。
pub async fn connect_with_retry(url: &str) -> ClientWs {
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut delay = Duration::from_millis(10);
    loop {
        match tokio_tungstenite::connect_async(url).await {
            Ok((ws, _response)) => return ws,
            Err(error) => {
                if Instant::now() >= deadline {
                    panic!("2 秒内无法连接 {url}: {error}");
                }
                tokio::time::sleep(delay).await;
                delay = (delay * 2).min(Duration::from_millis(50));
            }
        }
    }
}

/// 等 `serve` 真正绑定端口：用一次成功握手作为就绪信号，随后丢弃探针连接。
pub async fn wait_for_server(addr: std::net::SocketAddr) {
    let url = format!("ws://{addr}/ws");
    drop(connect_with_retry(&url).await);
}

/// 启动一个真实 WebSocket 服务并等到端口就绪。
pub async fn start_server() -> std::net::SocketAddr {
    start_server_with_clock(Arc::new(SystemClock)).await
}

/// 用指定时钟启动服务；调用方保留时钟句柄即可推进时间（邀请过期等用例）。
pub async fn start_server_with_clock(clock: Arc<dyn Clock>) -> std::net::SocketAddr {
    let (addr, router) = room_service::bind("127.0.0.1:0", clock)
        .await
        .expect("绑定端口应当成功");
    tokio::spawn(async move {
        let _ = room_service::serve(addr, router).await;
    });
    wait_for_server(addr).await;
    addr
}

/// 用指定 Steam 校验器启动服务；内存存储，适合验票与登录用例。
pub async fn start_server_with_steam(
    clock: Arc<dyn Clock>,
    verifier: room_service::steam::SteamVerifier,
) -> std::net::SocketAddr {
    let (addr, router) = room_service::bind_with_steam("127.0.0.1:0", clock, None, verifier)
        .await
        .expect("绑定端口应当成功");
    tokio::spawn(async move {
        let _ = room_service::serve(addr, router).await;
    });
    wait_for_server(addr).await;
    addr
}

/// 数据库连接串：优先 `TEST_DATABASE_URL`，其次 `DATABASE_URL`。
pub fn database_url() -> Option<String> {
    std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .ok()
        .filter(|url| !url.trim().is_empty())
}

/// 测试专用数据库：独立 schema，结束时由 [`TestDb::cleanup`] 删除。
pub struct TestDb {
    pub db: Arc<room_service::Db>,
    pub schema: String,
    url: String,
}

impl TestDb {
    /// 删除测试 schema；失败只打印，不影响测试结论。
    pub async fn cleanup(self) {
        if let Err(error) = room_service::db::drop_schema(&self.url, &self.schema).await {
            eprintln!("清理测试 schema 失败: {error}");
        }
    }
}

/// 建立独立 schema 的测试库；未配置数据库时返回 `None`。
pub async fn test_db() -> Option<TestDb> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);

    let url = database_url()?;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let schema = format!(
        "test_{}_{nanos}_{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let db = room_service::Db::connect_in_schema(&url, &schema)
        .await
        .expect("测试库应能连接并迁移");
    Some(TestDb {
        db: Arc::new(db),
        schema,
        url,
    })
}

/// 用指定时钟与数据库启动服务，返回地址与服务任务句柄。
pub async fn start_server_with_db(
    clock: Arc<dyn Clock>,
    db: Arc<room_service::Db>,
) -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
    let (addr, router) = room_service::bind_with_db("127.0.0.1:0", clock, db)
        .await
        .expect("绑定端口应当成功");
    let handle = tokio::spawn(async move {
        let _ = room_service::serve(addr, router).await;
    });
    wait_for_server(addr).await;
    (addr, handle)
}
