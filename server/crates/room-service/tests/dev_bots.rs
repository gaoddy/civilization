//! `sim::bot::dev` 的端到端测试：7 个机器人 + 1 个「开发者」座位打完整局。

use std::sync::Arc;
use std::time::{Duration, Instant};

use room_service::clock::{Clock, SystemClock};
use sim::bot::client::{BotConfig, run as run_bot};
use sim::bot::dev::{DEV_BOTS, DevConfig, run as run_dev};
use tokio::sync::oneshot;

/// 任意一张英雄卡，给「开发者」座位用。
fn any_hero() -> String {
    let catalog = sim::embedded_catalog().expect("内嵌卡表应能加载");
    catalog
        .cards
        .values()
        .find(|card| card.is_hero)
        .map(|card| card.id.clone())
        .expect("卡表应有英雄卡")
}

async fn wait_for_server(addr: std::net::SocketAddr) {
    let url = format!("ws://{addr}/ws");
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut delay = Duration::from_millis(10);
    loop {
        match tokio_tungstenite::connect_async(&url).await {
            Ok((ws, _)) => {
                drop(ws);
                return;
            }
            Err(error) => {
                assert!(Instant::now() < deadline, "2 秒内无法连接 {url}: {error}");
                tokio::time::sleep(delay).await;
                delay = (delay * 2).min(Duration::from_millis(50));
            }
        }
    }
}

async fn start_server() -> std::net::SocketAddr {
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

#[test]
fn dev_bots_leaves_exactly_one_seat_for_the_developer() {
    assert_eq!(DEV_BOTS, usize::from(sim::PLAYER_COUNT) - 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dev_run_finishes_a_game_once_the_developer_joins() {
    let addr = start_server().await;
    let url = format!("ws://{addr}/ws");

    let (code_tx, code_rx) = oneshot::channel();
    let dev = tokio::spawn(run_dev(DevConfig {
        url: url.clone(),
        intent_seed: 11,
        room_code_sink: Some(code_tx),
        run_timeout: Duration::from_secs(60),
    }));

    let room_code = tokio::time::timeout(Duration::from_secs(10), code_rx)
        .await
        .expect("10 秒内应能拿到房间码")
        .expect("建房 bot 不应提前退出");
    assert_eq!(room_code.len(), 6, "房间码应为 6 位");

    // 「开发者」座位：第 8 人，进了房间才开始。
    let human = tokio::spawn(run_bot(BotConfig {
        url,
        room: room_code.clone(),
        name: "human".to_string(),
        hero: any_hero(),
        create: false,
        intent_seed: 99,
        room_code_sink: None,
    }));

    let outcome = dev
        .await
        .expect("dev 任务不应 panic")
        .expect("dev 应打完一局");
    let human_outcome = human
        .await
        .expect("human 任务不应 panic")
        .expect("human 应打完一局");

    assert_eq!(outcome.room_code, room_code);
    assert_eq!(outcome.ranking.len(), 8, "排名应为 8 条");
    assert_eq!(human_outcome.ranking.len(), 8);
    assert!(outcome.turns <= 20, "回合数 {} 超过 20", outcome.turns);
    assert_eq!(
        outcome.ranking, human_outcome.ranking,
        "两个座位应看到同一份排名"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dev_run_gives_up_when_nobody_joins() {
    let addr = start_server().await;
    let error = run_dev(DevConfig {
        url: format!("ws://{addr}/ws"),
        intent_seed: 1,
        room_code_sink: None,
        run_timeout: Duration::from_millis(300),
    })
    .await
    .expect_err("无人进房时应超时报错");
    assert!(
        error.contains("未完成开局"),
        "错误消息应说明超时原因，实际：{error}"
    );
}
