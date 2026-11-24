//! 端到端测试：真实 WebSocket 服务、8 个机器人完整对局、重连与房间生命周期。
//!
//! 这是 M1a 唯一允许使用真实 [`SystemClock`] 的测试。

mod common;

use std::sync::Arc;
use std::time::Duration;

use common::{
    RawClient, authenticate, authenticate_with_token, connect_with_retry, first_candidate,
    start_and_pick_all, start_server, wait_all_ready, wait_for_server, wait_game_started,
    wait_hero_pick, wait_room_update, wait_room_update_until,
};
use futures_util::{SinkExt, StreamExt};
use protocol::server::GameStarted;
use protocol::{ErrorCode, ServerEnvelope, ServerMessage};
use room_service::clock::{Clock, FixedClock, SystemClock};
use room_service::game::RECONNECT_GRACE_MS;
use serde_json::json;
use sim::bot::client::{BotConfig, run};
use tokio::sync::oneshot;
use tokio_tungstenite::tungstenite::Message;

/// 取仓库内嵌卡表里卡牌 id 升序的前 8 张英雄。
fn hero_cards() -> Vec<String> {
    let catalog = sim::embedded_catalog().expect("内嵌卡表应能加载");
    catalog
        .heroes
        .values()
        .take(8)
        .map(|hero| hero.id.clone())
        .collect()
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

    let outcomes = tokio::time::timeout(Duration::from_secs(600), async move {
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
    .expect("对局应在 600 秒内结束");

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

// ---------------------------------------------------------------------------
// 原始 WS 客户端：用于重连、序号与房间生命周期测试。
// ---------------------------------------------------------------------------

struct GameFixture {
    room_id: String,
    /// `(player_id, token)`，按座位顺序。
    identities: Vec<(String, String)>,
    /// 保持 8 条连接存活；丢弃即断线。
    clients: Vec<RawClient>,
    /// 各客户端按座位顺序收到的 `GameStarted`。
    started: Vec<GameStarted>,
    /// 各客户端按座位顺序选择的英雄。
    picks: Vec<String>,
}

/// 用 8 条原始连接建好房间、全员准备、房主开始、每人选英雄，直到收到 `GameStarted`。
async fn start_eight_raw_clients(addr: std::net::SocketAddr) -> GameFixture {
    let mut clients = Vec::with_capacity(8);
    let mut identities = Vec::with_capacity(8);

    let mut host = RawClient::connect(addr).await;
    let (player_id, token) = authenticate(&mut host, "host").await;
    host.send("CreateRoom", json!({})).await;
    let room = wait_room_update(&mut host).await;
    let room_id = room.room_id.clone();
    let room_code = room.code.clone();
    host.send("Ready", json!({ "ready": true })).await;
    clients.push(host);
    identities.push((player_id, token));

    for index in 1..8 {
        let mut client = RawClient::connect(addr).await;
        let (player_id, token) = authenticate(&mut client, &format!("p{index}")).await;
        client.send("JoinRoom", json!({ "code": room_code })).await;
        let joined = wait_room_update(&mut client).await;
        assert!(
            joined
                .players
                .iter()
                .any(|player| player.player_id == player_id),
            "入房后本人应出现在 RoomUpdate 里"
        );
        client.send("Ready", json!({ "ready": true })).await;
        clients.push(client);
        identities.push((player_id, token));
    }

    // 等注册表确认全员准备后再开始，避免 `StartGame` 撞上 ROOM_NOT_READY。
    wait_all_ready(&mut clients[0], 8).await;
    let (started, picks) = start_and_pick_all(&mut clients).await;

    GameFixture {
        room_id,
        identities,
        clients,
        started,
        picks,
    }
}

/// 1 真人开局：选英雄阶段列出 8 个座位、7 个机器人；房主选完后机器人自动打第 1 回合。
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn one_human_starts_with_seven_bots() {
    let addr = start_server().await;
    let mut host = RawClient::connect(addr).await;
    let (_host_id, _token) = authenticate(&mut host, "solo-host").await;
    host.send("CreateRoom", json!({})).await;
    let room = wait_room_update(&mut host).await;
    assert!(room.bot_fill, "新房间默认开启机器人补位");
    host.send("Ready", json!({ "ready": true })).await;
    wait_all_ready(&mut host, 1).await;
    host.send("StartGame", json!({})).await;

    let pick = wait_hero_pick(&mut host).await;
    assert_eq!(pick.players.len(), 8, "选英雄阶段应列出 8 个座位");
    assert_eq!(
        pick.players.iter().filter(|player| player.is_bot).count(),
        7,
        "1 名真人应补入 7 个机器人"
    );
    assert_eq!(
        pick.players
            .iter()
            .find(|player| player.seat == 0)
            .map(|player| player.is_bot),
        Some(false),
        "房主座位不是机器人"
    );
    assert_eq!(
        pick.candidates.len(),
        room_service::room::CANDIDATE_COUNT,
        "本人候选数量应与注册表一致"
    );
    assert!(!pick.deadline.is_empty(), "截止时刻不应为空");

    let hero = first_candidate(&pick);
    host.send("SelectHeroCard", json!({ "cardId": hero })).await;
    let started = wait_game_started(&mut host).await;
    assert_eq!(started.view.turn.get(), 1, "开局在第 1 回合");

    // 机器人由服务端在开局后立即驱动：GameStarted 之后应收到第 1 回合的视图更新。
    let update = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let (_, message) = host
                .recv_room()
                .await
                .expect("等待机器人招募回合时连接关闭");
            match message {
                ServerMessage::StateUpdate(update) => return update,
                ServerMessage::Error(error) => {
                    panic!("等待机器人回合时收到错误 {}: {}", error.code, error.message)
                }
                _ => {}
            }
        }
    })
    .await
    .expect("10 秒内应收到机器人行动后的视图");
    assert_eq!(update.view.turn.get(), 1, "机器人行动仍应在第 1 回合");
    let bot_recruited = update
        .view
        .players
        .iter()
        .filter(|player| player.player_id.starts_with("bot:"))
        .any(|player| player.recruited);
    assert!(bot_recruited, "至少一个机器人应已结束第 1 回合招募");
    println!("one human + 7 bots: turn={}", update.view.turn);
}

/// 8 名真人全员选完后才进入 `GameStarted`，每人开局英雄与本人选择一致。
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn eight_humans_choose_then_start() {
    let addr = start_server().await;
    let fixture = start_eight_raw_clients(addr).await;
    assert_eq!(fixture.started.len(), 8, "8 名真人都应收到 GameStarted");
    for (index, started) in fixture.started.iter().enumerate() {
        assert_eq!(started.view.turn.get(), 1, "开局在第 1 回合");
        assert_eq!(started.view.players.len(), 8, "开局视图应含 8 个座位");
        assert_eq!(
            started.view.self_.hero_card_id.as_deref(),
            Some(fixture.picks[index].as_str()),
            "座位 {index} 的开局英雄应与本人选择一致"
        );
        assert!(
            started
                .view
                .players
                .iter()
                .all(|player| player.hero_card_id.is_some()),
            "全员都应持有开局英雄"
        );
    }
    println!("eight humans picked then started");
}

/// 补位关闭且真人不足 8 人时点开始被拒绝；房间仍可准备与离开。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bot_fill_off_requires_eight_humans() {
    let addr = start_server().await;
    let mut host = RawClient::connect(addr).await;
    let (host_id, _token) = authenticate(&mut host, "nofill-host").await;
    host.send("CreateRoom", json!({})).await;
    let room = wait_room_update(&mut host).await;

    let mut guest = RawClient::connect(addr).await;
    let (guest_id, _guest_token) = authenticate(&mut guest, "nofill-guest").await;
    guest.send("JoinRoom", json!({ "code": room.code })).await;
    let joined = wait_room_update(&mut guest).await;
    assert_eq!(joined.players.len(), 2);

    host.send("Ready", json!({ "ready": true })).await;
    guest.send("Ready", json!({ "ready": true })).await;
    wait_all_ready(&mut host, 2).await;

    host.send("SetBotFill", json!({ "enabled": false })).await;
    let updated = wait_room_update_until(&mut host, |update| {
        update.players.len() == 2 && !update.bot_fill
    })
    .await;
    assert!(!updated.bot_fill, "补位应已关闭");

    host.send("StartGame", json!({})).await;
    let rejected = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let (_, message) = host
                .recv_room()
                .await
                .expect("等待 ROOM_NOT_READY 时连接关闭");
            if let ServerMessage::Error(error) = message {
                return error;
            }
        }
    })
    .await
    .expect("10 秒内应收到开始被拒的错误");
    assert_eq!(
        rejected.code,
        ErrorCode::RoomNotReady,
        "补位关闭且真人不足 8 人应返回 ROOM_NOT_READY"
    );

    // 房间仍在 Lobby：取消准备与离开都应正常生效。
    guest.send("Ready", json!({ "ready": false })).await;
    let unready = wait_room_update_until(&mut host, |update| {
        update.players.len() == 2
            && update
                .players
                .iter()
                .any(|player| player.player_id == guest_id && !player.ready)
    })
    .await;
    assert!(!unready.bot_fill, "取消准备后补位开关保持关闭");

    guest.send("LeaveRoom", json!({})).await;
    let left = wait_room_update_until(&mut host, |update| update.players.len() == 1).await;
    assert_eq!(left.players[0].player_id, host_id, "离开后只剩房主");
    println!("bot fill off rejected start but room stayed usable");
}

/// `RoomUpdate` 的座位从 0 连续排列，补位开关可读。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn room_update_has_seat_and_bot_fill() {
    let addr = start_server().await;
    let mut host = RawClient::connect(addr).await;
    let _ = authenticate(&mut host, "seat-host2").await;
    host.send("CreateRoom", json!({})).await;
    let room = wait_room_update(&mut host).await;
    assert!(room.bot_fill, "房主应能读到补位开关");
    assert_eq!(room.players.len(), 1);
    assert_eq!(room.players[0].seat, 0, "房主固定在 0 号座位");

    for index in 1..=2 {
        let mut client = RawClient::connect(addr).await;
        let _ = authenticate(&mut client, &format!("seat-{index}")).await;
        client.send("JoinRoom", json!({ "code": room.code })).await;
        let _ = wait_room_update(&mut client).await;
    }

    let update = wait_room_update_until(&mut host, |update| update.players.len() == 3).await;
    assert!(update.bot_fill, "补位开关在大厅里保持可读");
    for (index, player) in update.players.iter().enumerate() {
        assert_eq!(
            player.seat, index as i64,
            "第 {index} 位成员的座位号应为 {index}"
        );
    }
    println!("seat/botFill: players={}", update.players.len());
}

/// 选英雄阶段的消息顺序：`HeroPickStarted` 一定先于 `GameStarted`，且开局视图可解析。
///
/// `wait_hero_pick` 在 `GameStarted` 先到时直接 panic，因此本用例通过即证明顺序正确。
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn hero_pick_message_order() {
    let addr = start_server().await;
    let fixture = start_eight_raw_clients(addr).await;
    assert_eq!(fixture.started.len(), 8, "8 名真人都应先选英雄再开局");
    for (index, started) in fixture.started.iter().enumerate() {
        assert_eq!(
            started.view.phase,
            protocol::view::Phase::Recruit,
            "GameStarted.view 应可解析且处于招募阶段"
        );
        assert_eq!(
            started.view.room_id, fixture.room_id,
            "开局视图应带上房间 id"
        );
        assert_eq!(
            started.view.players[index].hero_card_id.as_deref(),
            Some(fixture.picks[index].as_str()),
            "座位 {index} 的英雄应与选择一致"
        );
    }
    println!("hero pick precedes game started for all 8 seats");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reconnect_with_token_returns_snapshot() {
    let addr = start_server().await;
    let fixture = start_eight_raw_clients(addr).await;
    // 保持 8 条原始连接存活到断言结束。
    let _keep_alive = &fixture.clients;
    let (host_player_id, host_token) = fixture.identities[0].clone();

    let mut reconnected = RawClient::connect(addr).await;
    let player_id = authenticate_with_token(&mut reconnected, &host_token).await;
    assert_eq!(player_id, host_player_id, "token 续登应拿到同一 player_id");

    reconnected
        .send(
            "Reconnect",
            json!({ "roomId": fixture.room_id, "lastSeq": 0 }),
        )
        .await;

    let (_, message) = tokio::time::timeout(Duration::from_secs(5), reconnected.recv_room())
        .await
        .expect("重连应在 5 秒内返回快照")
        .expect("重连连接不应关闭");
    match message {
        ServerMessage::Snapshot(snapshot) => {
            assert_eq!(snapshot.view.room_id, fixture.room_id, "快照应带上 roomId");
            assert!(!snapshot.view.room_id.is_empty(), "roomId 不应为空串");
            assert_eq!(snapshot.view.turn.get(), 1, "尚无人行动，回合应为 1");
            println!(
                "reconnect snapshot: room_id={} turn={} phase={:?}",
                snapshot.view.room_id, snapshot.view.turn, snapshot.view.phase
            );
        }
        other => panic!("期望 Snapshot，实际 {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn joined_connection_errors_use_room_seq() {
    let addr = start_server().await;
    let mut client = RawClient::connect(addr).await;
    let _ = authenticate(&mut client, "seq").await;
    client.send("CreateRoom", json!({})).await;
    let (last_seq, message) = client.recv_room().await.expect("应收到 RoomUpdate");
    assert!(
        matches!(message, ServerMessage::RoomUpdate(_)),
        "实际 {message:?}"
    );

    // 入房后再次建房：错误必须走房间的下行通道，序号继续递增。
    client.send("CreateRoom", json!({})).await;
    let (error_seq, message) = tokio::time::timeout(Duration::from_secs(5), client.recv_room())
        .await
        .expect("应收到 Error")
        .expect("连接不应关闭");
    match message {
        ServerMessage::Error(error) => assert_eq!(error.code, ErrorCode::InvalidStage),
        other => panic!("期望 Error，实际 {other:?}"),
    }
    assert!(
        error_seq > last_seq,
        "入房后错误 seq={error_seq} 应大于最后一条 seq={last_seq}"
    );
    println!("room seq: last={last_seq} error={error_seq}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn leave_room_then_error_keeps_seq_monotonic() {
    let addr = start_server().await;
    let mut client = RawClient::connect(addr).await;
    let _ = authenticate(&mut client, "leaver-seq").await;
    client.send("CreateRoom", json!({})).await;
    let (first_seq, message) = client.recv_room().await.expect("应收到 RoomUpdate");
    assert!(
        matches!(message, ServerMessage::RoomUpdate(_)),
        "实际 {message:?}"
    );

    // 再收一条大厅消息，把该会话的序号推进到多条之后。
    client.send("RequestSnapshot", json!({})).await;
    let (last_seq, message) = client.recv_room().await.expect("应收到 RoomUpdate");
    assert!(
        matches!(message, ServerMessage::RoomUpdate(_)),
        "实际 {message:?}"
    );
    assert!(last_seq > first_seq, "大厅消息序号应递增");

    // 离开房间后连接层改走 direct 通道；新的 Error 必须接在最后一条房间消息之后。
    client.send("LeaveRoom", json!({})).await;
    client.send("Ready", json!({ "ready": true })).await;
    let (error_seq, message) = tokio::time::timeout(Duration::from_secs(5), client.recv_room())
        .await
        .expect("应收到 Error")
        .expect("连接不应关闭");
    match message {
        ServerMessage::Error(error) => assert_eq!(error.code, ErrorCode::NotInRoom),
        other => panic!("期望 Error，实际 {other:?}"),
    }
    assert!(
        error_seq > last_seq,
        "离房后错误 seq={error_seq} 应严格大于离房前最后一条 seq={last_seq}"
    );
    println!("leave seq: last={last_seq} error={error_seq}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stale_disconnect_does_not_release_reconnected_seat() {
    let addr = start_server().await;
    let mut old = RawClient::connect(addr).await;
    let (player_id, token) = authenticate(&mut old, "reconn-seat").await;
    old.send("CreateRoom", json!({})).await;
    let room = wait_room_update(&mut old).await;

    // 新连接用 token 重连同一房间、同一座位：房间 task 登记新代次。
    let mut new = RawClient::connect(addr).await;
    let new_player_id = authenticate_with_token(&mut new, &token).await;
    assert_eq!(new_player_id, player_id, "重连应复用同一 player_id");
    new.send("Reconnect", json!({ "roomId": room.room_id, "lastSeq": 0 }))
        .await;
    let reconnected = wait_room_update(&mut new).await;
    assert_eq!(reconnected.players.len(), 1);

    // 旧连接迟到关闭不得释放新连接的座位。
    drop(old);
    tokio::time::sleep(Duration::from_millis(300)).await;

    new.send("RequestSnapshot", json!({})).await;
    let after = wait_room_update(&mut new).await;
    assert_eq!(
        after.players.len(),
        1,
        "旧连接断开不应释放新连接已重新占用的座位"
    );
    println!(
        "reconnected seat still online: players={}",
        after.players.len()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lobby_room_is_retired_after_last_player_leaves() {
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let (addr, router, state) = room_service::bind_state("127.0.0.1:0", clock)
        .await
        .expect("绑定端口应当成功");
    tokio::spawn(async move {
        let _ = room_service::serve(addr, router).await;
    });
    wait_for_server(addr).await;

    let mut client = RawClient::connect(addr).await;
    let _ = authenticate(&mut client, "leaver").await;
    client.send("CreateRoom", json!({})).await;
    let _ = wait_room_update(&mut client).await;
    client.send("LeaveRoom", json!({})).await;
    drop(client);

    let retired = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if state.hub_len().await == 0 {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;
    assert!(retired.is_ok(), "最后一人离开后房间 task 应被回收");
    println!("hub_len after leave = {}", state.hub_len().await);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn finished_game_room_is_removed_from_registry() {
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let (addr, router, state) = room_service::bind_state("127.0.0.1:0", clock)
        .await
        .expect("绑定端口应当成功");
    tokio::spawn(async move {
        let _ = room_service::serve(addr, router).await;
    });
    wait_for_server(addr).await;

    let url = format!("ws://{addr}/ws");
    let heroes = hero_cards();
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
    tokio::time::timeout(Duration::from_secs(600), async move {
        for handle in handles {
            handle
                .await
                .expect("bot 任务不应 panic")
                .expect("bot 应正常结束");
        }
    })
    .await
    .expect("对局应在 600 秒内结束");

    // 对局结束即销毁房间：注册表与 hub 都应立即清理，不等断线宽限期。
    let retired = tokio::time::timeout(Duration::from_secs(45), async {
        loop {
            let hub = state.hub_len().await;
            let rooms = state.rooms.lock().await.len();
            if hub == 0 && rooms == 0 {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;
    assert!(
        retired.is_ok(),
        "对局结束且连接全断后，房间应从 hub 与注册表消失"
    );
    println!("registry rooms after game over = 0");

    // 旧房间码必须回收为 ROOM_NOT_FOUND，而不是 WrongPhase/INVALID_STAGE。
    let mut late = RawClient::connect(addr).await;
    let _ = authenticate(&mut late, "late-joiner").await;
    late.send("JoinRoom", json!({ "code": room_code })).await;
    let (_, message) = tokio::time::timeout(Duration::from_secs(5), late.recv_room())
        .await
        .expect("加入旧房间码应收到响应")
        .expect("连接不应关闭");
    match message {
        ServerMessage::Error(error) => assert_eq!(
            error.code,
            ErrorCode::RoomNotFound,
            "旧房间码应返回 ROOM_NOT_FOUND"
        ),
        other => panic!("期望 Error，实际 {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lobby_disconnect_keeps_seat_within_grace() {
    let addr = start_server().await;
    let mut host = RawClient::connect(addr).await;
    let _ = authenticate(&mut host, "seat-host").await;
    host.send("CreateRoom", json!({})).await;
    let room = wait_room_update(&mut host).await;
    assert_eq!(room.players.len(), 1);

    let mut guest = RawClient::connect(addr).await;
    let (guest_id, guest_token) = authenticate(&mut guest, "seat-guest").await;
    guest.send("JoinRoom", json!({ "code": room.code })).await;
    let joined = wait_room_update(&mut guest).await;
    assert_eq!(joined.players.len(), 2);
    let guest_seat = joined
        .players
        .iter()
        .find(|player| player.player_id == guest_id)
        .map(|player| player.seat)
        .expect("入房玩家应有座位");
    assert_eq!(guest_seat, 1, "第二人应拿到 1 号座位");

    // 大厅断线保留座位一个宽限期（`RECONNECT_GRACE_MS`），不再立即释放。
    drop(guest);
    tokio::time::sleep(Duration::from_millis(300)).await;
    host.send("RequestSnapshot", json!({})).await;
    let after = wait_room_update(&mut host).await;
    assert_eq!(after.players.len(), 2, "宽限期内座位必须保留");

    // 宽限期内用 token 重连：回到同一座位，座位号不变。
    let mut back = RawClient::connect(addr).await;
    let back_id = authenticate_with_token(&mut back, &guest_token).await;
    assert_eq!(back_id, guest_id, "重连应复用同一 player_id");
    back.send("Reconnect", json!({ "roomId": room.room_id, "lastSeq": 0 }))
        .await;
    let restored = wait_room_update(&mut back).await;
    assert_eq!(restored.players.len(), 2, "重连应回到同一房间");
    let restored_seat = restored
        .players
        .iter()
        .find(|player| player.player_id == guest_id)
        .map(|player| player.seat)
        .expect("重连后仍应占同一座位");
    assert_eq!(restored_seat, guest_seat, "重连必须回到同一座位号");
    println!("lobby seat kept through grace and reconnect");
}

/// 对局中依次退出：其余座位继续，首个退出者名次为 8，仅剩 1 人时立即结束。
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn forfeit_mid_game_ranks_departed_and_finishes_at_one_left() {
    let addr = start_server().await;
    let mut fixture = start_eight_raw_clients(addr).await;
    let player_ids: Vec<String> = fixture
        .identities
        .iter()
        .map(|(player_id, _)| player_id.clone())
        .collect();

    // 第一个退出者走真实 LeaveRoom：对局应继续，其余座位仍收到状态更新。
    fixture.clients[7].send("LeaveRoom", json!({})).await;
    let continued = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match fixture.clients[0]
                .recv_room()
                .await
                .expect("等待其余座位更新时连接关闭")
            {
                (_, ServerMessage::StateUpdate(_)) => return true,
                (_, ServerMessage::GameOver(_)) => return false,
                _ => {}
            }
        }
    })
    .await
    .expect("一人退出后其余座位应继续对局");
    assert!(continued, "首个退出不应立即结束对局");

    // 其余 6 人依次退出，只剩 0 号座位时立即结束。
    for index in (1..7).rev() {
        fixture.clients[index].send("LeaveRoom", json!({})).await;
    }
    let game_over = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let (_, ServerMessage::GameOver(game_over)) = fixture.clients[0]
                .recv_room()
                .await
                .expect("等待 GameOver 时连接关闭")
            {
                return game_over;
            }
        }
    })
    .await
    .expect("仅剩 1 人时应立即结束对局");

    assert_eq!(game_over.ranking.len(), 8, "排名应包含全部 8 个座位");
    assert!(
        game_over.rating_delta.is_none()
            && game_over.rating_after.is_none()
            && game_over.tier_before.is_none()
            && game_over.tier_after.is_none(),
        "房间局不计分，天梯字段必须为 null"
    );
    let place_of = |player_id: &str| {
        game_over
            .ranking
            .iter()
            .find(|item| item.player_id == player_id)
            .map(|item| item.place.get())
            .unwrap_or_else(|| panic!("{player_id} 应在排名里"))
    };
    assert_eq!(place_of(&player_ids[7]), 8, "首个退出者名次应为 8");
    assert_eq!(place_of(&player_ids[0]), 1, "最后留下者名次应为 1");

    let mut places: Vec<u64> = game_over
        .ranking
        .iter()
        .map(|item| item.place.get())
        .collect();
    places.sort_unstable();
    assert_eq!(places, (1..=8).collect::<Vec<u64>>(), "名次应覆盖 1..=8");
    println!("forfeit ranking: departed={:?}", place_of(&player_ids[7]));
}

/// 对局中的断线也要写回注册表：对局结束回房后，大厅 tick 才能按宽限期释放
/// 该座位；重连回来则清除标记，避免回房时被误释放。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mid_game_disconnect_is_recorded_in_registry() {
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let (addr, router, state) = room_service::bind_state("127.0.0.1:0", clock)
        .await
        .expect("绑定端口应当成功");
    tokio::spawn(async move {
        let _ = room_service::serve(addr, router).await;
    });
    wait_for_server(addr).await;

    let mut fixture = start_eight_raw_clients(addr).await;
    let target = fixture.identities[7].0.clone();

    // 直接断开第 8 条连接，不发送 LeaveRoom：服务端应按断线处理并写回注册表。
    drop(fixture.clients.pop().expect("应有第 8 条连接"));

    let marked = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let disconnected = {
                let rooms = state.rooms.lock().await;
                rooms
                    .room_of_player(&target)
                    .and_then(|(room, seat)| {
                        room.slot(seat)
                            .map(|slot| slot.disconnected_at_ms.is_some())
                    })
                    .unwrap_or(false)
            };
            if disconnected {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;
    assert!(marked.is_ok(), "对局中断线应写回注册表断线时刻");

    let token = fixture.identities[7].1.clone();
    let mut reconnected = RawClient::connect(addr).await;
    authenticate_with_token(&mut reconnected, &token).await;
    reconnected
        .send(
            "Reconnect",
            json!({ "roomId": fixture.room_id, "lastSeq": 0 }),
        )
        .await;
    let _ = tokio::time::timeout(Duration::from_secs(5), reconnected.recv_room())
        .await
        .expect("重连应在 5 秒内收到消息");

    let cleared = {
        let rooms = state.rooms.lock().await;
        rooms
            .room_of_player(&target)
            .and_then(|(room, seat)| {
                room.slot(seat)
                    .map(|slot| slot.disconnected_at_ms.is_none())
            })
            .unwrap_or(false)
    };
    assert!(cleared, "重连应清除注册表断线标记");
}

/// 整桌断线且无人发送 `LeaveRoom`：宽限期到期按弃权结算，房间销毁，账号能重新建房。
///
/// 修复前这里没有回收路径：对局只能靠阶段截止磨满 20 回合（实测 2–37 分钟），
/// 期间这 8 个账号再建房一律 `ALREADY_IN_ROOM`。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn abandoned_game_is_reclaimed_after_grace() {
    let clock = Arc::new(FixedClock::new(1_000_000));
    let (addr, router, state) = room_service::bind_state("127.0.0.1:0", clock.clone())
        .await
        .expect("绑定端口应当成功");
    tokio::spawn(async move {
        let _ = room_service::serve(addr, router).await;
    });
    wait_for_server(addr).await;

    let fixture = start_eight_raw_clients(addr).await;
    let room_id = fixture.room_id.clone();

    // 整桌断线：丢弃全部连接，不发送 `LeaveRoom`。
    drop(fixture.clients);

    // 等 8 个座位都写回断线时刻，否则无法区分「还没收到断线」与「没有回收」。
    let marked = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let disconnected = {
                let rooms = state.rooms.lock().await;
                rooms.room(&room_id).map_or(0, |room| {
                    (0..8u8)
                        .filter(|seat| {
                            room.slot(*seat)
                                .is_some_and(|slot| slot.disconnected_at_ms.is_some())
                        })
                        .count()
                })
            };
            if disconnected == 8 {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    assert!(marked.is_ok(), "8 条连接断开后应全部写回断线时刻");

    // 宽限期内账号仍占着房间。
    let mut probe = RawClient::connect(addr).await;
    let _ = authenticate(&mut probe, "host").await;
    probe.send("CreateRoom", json!({})).await;
    let within_grace = tokio::time::timeout(Duration::from_secs(5), probe.recv_room())
        .await
        .expect("宽限期内应收到回应")
        .expect("连接不应关闭")
        .1;
    assert!(
        matches!(&within_grace, ServerMessage::Error(error) if error.code == ErrorCode::AlreadyInRoom),
        "宽限期内重新建房应仍是 ALREADY_IN_ROOM"
    );

    // 宽限期到期：整桌无人在线，房间 task 应结束对局并销毁房间。
    clock.advance(RECONNECT_GRACE_MS);
    let reclaimed = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let gone = {
                let rooms = state.rooms.lock().await;
                rooms.room(&room_id).is_none()
            };
            if gone {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    assert!(
        reclaimed.is_ok(),
        "宽限期到期后整桌断线的房间应被回收，不能一直占到第 20 回合"
    );

    // 账号已释放：同一手机号重新建房应成功。
    probe.send("CreateRoom", json!({})).await;
    let created = tokio::time::timeout(Duration::from_secs(5), probe.recv_room())
        .await
        .expect("重新建房应收到回应")
        .expect("连接不应关闭")
        .1;
    assert!(
        matches!(created, ServerMessage::RoomUpdate(_)),
        "回收后应能重新建房，实际收到错误"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rejects_envelope_with_unknown_version() {
    let addr = start_server().await;
    let mut ws = connect_with_retry(&format!("ws://{addr}/ws")).await;

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
        .expect("服务端应先返回错误")
        .expect("服务端不应无响应");
    let text = match next {
        Ok(Message::Text(text)) => text.as_str().to_string(),
        other => panic!("期望先收到 Error 文本，实际 {other:?}"),
    };
    let envelope: ServerEnvelope = serde_json::from_str(&text).expect("下行信封应能解析");
    match envelope.parse().expect("下行载荷应能解析") {
        ServerMessage::Error(error) => {
            assert!(
                matches!(error.code, ErrorCode::BadPayload | ErrorCode::UnknownType),
                "期望 BAD_PAYLOAD/UNKNOWN_TYPE，实际 {}",
                error.code
            );
        }
        other => panic!("期望 Error，实际 {other:?}"),
    }

    // 收到错误之后才允许关闭。
    let closed = tokio::time::timeout(Duration::from_secs(2), ws.next()).await;
    match closed {
        Ok(Some(Ok(Message::Close(_)))) | Ok(None) | Ok(Some(Err(_))) => {}
        Ok(Some(Ok(other))) => panic!("错误后应关闭连接，实际收到 {other:?}"),
        Err(_) => panic!("错误后连接未在 2 秒内关闭"),
    }
}
