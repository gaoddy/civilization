#![allow(dead_code)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use room_service::clock::{Clock, SystemClock};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

pub type ClientWs = WebSocketStream<MaybeTlsStream<TcpStream>>;

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
