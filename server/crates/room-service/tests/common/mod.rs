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
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let (addr, router) = room_service::bind("127.0.0.1:0", clock)
        .await
        .expect("绑定端口应当成功");
    tokio::spawn(async move {
        let _ = room_service::serve(addr, router).await;
    });
    wait_for_server(addr).await;
    addr
}
