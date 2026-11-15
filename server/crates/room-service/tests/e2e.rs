//! 端到端测试：真实 WebSocket 服务 + 8 个机器人打完一局。
//!
//! 这是 M1a 唯一允许使用真实 [`SystemClock`] 的测试。

use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use protocol::{ErrorCode, ServerEnvelope, ServerMessage};
use room_service::clock::{Clock, SystemClock};
use serde_json::json;
use sim::bot::client::{BotConfig, run};
use tokio::sync::oneshot;
use tokio_tungstenite::tungstenite::Message;

/// 取仓库内嵌卡表里卡牌 id 升序的前 8 张英雄。
fn hero_cards() -> Vec<String> {
    let catalog = sim::embedded_catalog().expect("内嵌卡表应能加载");
    catalog
        .cards
        .values()
        .filter(|card| card.is_hero)
        .take(8)
        .map(|card| card.id.clone())
        .collect()
}

async fn start_server() -> std::net::SocketAddr {
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let (addr, router) = room_service::bind("127.0.0.1:0", clock)
        .await
        .expect("绑定端口应当成功");
    tokio::spawn(async move {
        let _ = room_service::serve(addr, router).await;
    });
    addr
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn eight_bots_finish_a_game() {
    let addr = start_server().await;
    let url = format!("ws://{addr}/ws");
    let heroes = hero_cards();
    assert_eq!(heroes.len(), 8, "卡表中应有至少 8 张英雄卡");

    let (code_tx, code_rx) = oneshot::channel();
    let host = tokio::spawn(run(BotConfig {
        url: url.clone(),
        room: String::new(),
        name: "bot0".to_string(),
        hero: heroes[0].clone(),
        create: true,
        intent_seed: 11,
        room_code_sink: Some(code_tx),
    }));

    let room_code = tokio::time::timeout(Duration::from_secs(10), code_rx)
        .await
        .expect("10 秒内应能拿到房间码")
        .expect("建房 bot 不应提前退出");
    assert_eq!(room_code.len(), 6, "房间码应为 6 位");
    println!("ROOM_CODE={room_code}");

    let mut handles = vec![host];
    for (index, hero) in heroes.iter().enumerate().skip(1) {
        let index = u64::try_from(index).unwrap_or(0);
        handles.push(tokio::spawn(run(BotConfig {
            url: url.clone(),
            room: room_code.clone(),
            name: format!("bot{index}"),
            hero: hero.clone(),
            create: false,
            intent_seed: index + 11,
            room_code_sink: None,
        })));
    }

    let outcomes = tokio::time::timeout(Duration::from_secs(120), async move {
        let mut outcomes = Vec::with_capacity(handles.len());
        for (index, handle) in handles.into_iter().enumerate() {
            let outcome = handle
                .await
                .unwrap_or_else(|err| panic!("bot{index} 任务 panic: {err}"))
                .unwrap_or_else(|err| panic!("bot{index} 失败: {err}"));
            outcomes.push(outcome);
        }
        outcomes
    })
    .await
    .expect("对局应在 120 秒内结束");

    // 每个 bot 都应拿到完整的 8 条排名；建房 bot 的排名作为权威断言。
    for (index, outcome) in outcomes.iter().enumerate() {
        assert_eq!(
            outcome.ranking.len(),
            8,
            "bot{index} 的排名应为 8 条，实际 {}",
            outcome.ranking.len()
        );
        assert!(
            outcome.turns <= 20,
            "bot{index} 结束时回合数 {} 超过 20",
            outcome.turns
        );
        println!(
            "bot{index} turns={} ranking={:?}",
            outcome.turns, outcome.ranking
        );
    }

    let mut places: Vec<u8> = outcomes[0]
        .ranking
        .iter()
        .map(|(_, place)| *place)
        .collect();
    places.sort_unstable();
    assert_eq!(places, (1..=8).collect::<Vec<u8>>(), "名次应覆盖 1..=8");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rejects_envelope_with_unknown_version() {
    let addr = start_server().await;
    let (mut ws, _response) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws"))
        .await
        .expect("应能连上服务端");

    let envelope = json!({
        "v": 2,
        "id": "bad-version",
        "type": "CreateRoom",
        "ts": "2026-01-01T00:00:00Z",
        "payload": {},
    });
    ws.send(Message::Text(envelope.to_string().into()))
        .await
        .expect("应能发送失败用例信封");

    let next = tokio::time::timeout(Duration::from_secs(5), ws.next())
        .await
        .expect("服务端应返回错误或关闭连接");
    match next {
        Some(Ok(Message::Text(text))) => {
            let envelope: ServerEnvelope =
                serde_json::from_str(text.as_str()).expect("下行信封应能解析");
            match envelope.parse().expect("下行载荷应能解析") {
                ServerMessage::Error(error) => {
                    assert_eq!(error.code, ErrorCode::BadPayload);
                }
                other => panic!("期望 Error，实际 {other:?}"),
            }
        }
        Some(Ok(Message::Close(_))) | None => {
            // 直接关闭连接同样满足「不按 v1 语义执行」。
        }
        other => panic!("期望 Error 或关闭，实际 {other:?}"),
    }
}
