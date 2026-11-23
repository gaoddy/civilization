//! `sim load` 端到端冒烟：真实服务端上的稳态压测与 `LeaveRoom` 清理。
//!
//! 只覆盖 2 局与 1→2 两档 ramp，验证客户端编排、停止信号与房间回收；
//! 容量数字不在测试中断言。

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use room_service::AppState;
use room_service::clock::SystemClock;
use sim::load::{LoadConfig, Scenario};

/// 启动内存存储的真实服务端，并等到 WebSocket 就绪。
async fn start_server() -> (std::net::SocketAddr, AppState) {
    let (addr, router, state) = room_service::bind_state("127.0.0.1:0", Arc::new(SystemClock))
        .await
        .expect("绑定端口应当成功");
    tokio::spawn(async move {
        let _ = room_service::serve(addr, router).await;
    });
    let url = format!("ws://{addr}/ws");
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match tokio_tungstenite::connect_async(&url).await {
            Ok((mut ws, _response)) => {
                let _ = ws.close(None).await;
                break;
            }
            Err(error) => {
                assert!(Instant::now() < deadline, "2 秒内无法连接 {url}: {error}");
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        }
    }
    (addr, state)
}

/// 压测配置；测试不启动子进程、不采样、不写报告。
fn config(url: String, scenario: Scenario, rooms: usize, duration_ms: u64) -> LoadConfig {
    let catalog = sim::embedded_catalog().expect("内嵌卡表应能加载");
    let hero = catalog
        .heroes
        .values()
        .next()
        .expect("卡表应有英雄卡")
        .id
        .clone();
    LoadConfig {
        scenario,
        url: Some(url),
        server_bin: None,
        server_log: PathBuf::from("output/loadtest-test-server.log"),
        rooms,
        rooms_max: rooms,
        step: 1,
        duration_ms,
        step_ms: duration_ms,
        start_timeout_ms: 15_000,
        sample_ms: 500,
        intent_seed: 7,
        hero,
        quiet: true,
        remote_sample: None,
        fresh_server_per_step: false,
        allow_remote_ramp: false,
        max_start_failure_permille: 10,
        max_start_p95_ms: 15_000,
    }
}

/// 等房间注册表清空；停止信号应让对局按弃权结束并销毁房间。
async fn wait_rooms_gone(state: &AppState) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if state.hub_len().await == 0 {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "10 秒内房间未清理，剩余 {}",
            state.hub_len().await
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn steady_two_rooms_start_and_cleanup() {
    let (addr, state) = start_server().await;
    let report = sim::load::run(config(
        format!("ws://{addr}/ws"),
        Scenario::Steady,
        2,
        4_000,
    ))
    .await
    .expect("压测应能完成");
    assert_eq!(report.steps.len(), 1, "steady 应只有一档");
    let step = &report.steps[0];
    assert!(step.games_started >= 2, "应有至少 2 局开局：{step:?}");
    assert_eq!(
        step.start_failures, 0,
        "不应有启动失败：{:?}",
        step.failures
    );
    assert!(step.start_latency_ms.p95.is_some(), "应记录开局时延");
    wait_rooms_gone(&state).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ramp_one_then_two_cleans_each_step() {
    let (addr, state) = start_server().await;
    let mut config = config(format!("ws://{addr}/ws"), Scenario::Ramp, 1, 3_000);
    config.rooms_max = 2;
    config.step = 1;
    config.allow_remote_ramp = true;
    let report = sim::load::run(config).await.expect("压测应能完成");
    assert_eq!(report.steps.len(), 2, "ramp 应有两档");
    assert!(
        report.steps.iter().all(|step| step.status == "healthy"),
        "两档都应健康：{:?}",
        report.steps
    );
    assert!(!report.limit.found, "未达到上限时不应标记 found");
    wait_rooms_gone(&state).await;
}
