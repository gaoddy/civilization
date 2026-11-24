//! 重连、全量快照与断线保留窗口测试。
//!
//! 大部分用例使用 [`FixedClock`] 与进程内引擎，不启网络；`Reconnect.out` 只绑定不
//! 实际读取，发送端是否被保存由连接层（Task 12）消费，这里只验证命令返回值。
//! 文末另有真实 WebSocket 的「对局结束即销毁房间、结束后重连不补发结果」用例。

mod common;

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use common::{
    RawClient, authenticate, authenticate_with_token, first_candidate, start_and_pick_all,
    wait_all_ready, wait_hero_pick, wait_room_update,
};
use futures_util::{SinkExt, StreamExt};
use protocol::intents::Intent as ProtocolIntent;
use protocol::server::Snapshot;
use protocol::view::PendingChoice;
use protocol::{ServerEnvelope, ServerMessage};
use room_service::clock::{Clock, FixedClock, SystemClock};
use room_service::envelope::SeatChannel;
use room_service::game::{GameTask, RoomCommand, SeatUpdate, UpdateKind};
use room_service::room::PlayerSlot;
use rule_engine::data::{Catalog, Playstyle};
use rule_engine::state::Seat;
use serde_json::{Value, json};
use sim::bot::client::{BotConfig, run};
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;

const HEROES_JSON: &str = include_str!("../../../data/v0.1/heroes.json");
const CARDS_JSON: &str = include_str!("../../../data/v0.1/cards.json");
const SKILLS_JSON: &str = include_str!("../../../data/v0.1/skills.json");
const CONFIG_JSON: &str = include_str!("../../../data/v0.1/config.json");
const TAXONOMY_JSON: &str = include_str!("../../../data/v0.1/taxonomy.json");

/// 8 个座位各用一张不同的英雄卡；下标 1 是达尔文（主动技能「进化」）。
const HEROES: [&str; 8] = [
    "adam-smith",
    "darwin",
    "davinci",
    "edison",
    "einstein",
    "faraday",
    "galileo",
    "knuth",
];

const SEED: u64 = 7;

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

fn heroes() -> BTreeMap<Seat, String> {
    HEROES
        .iter()
        .enumerate()
        .map(|(seat, hero)| (seat as Seat, (*hero).to_string()))
        .collect()
}

fn players() -> Vec<PlayerSlot> {
    (0..8)
        .map(|seat| PlayerSlot {
            player_id: format!("p{seat}"),
            display_name: format!("玩家{seat}"),
            ready: true,
            is_bot: false,
            channel: SeatChannel::new(),
            connected: true,
            disconnected_at_ms: None,
        })
        .collect()
}

fn task() -> (GameTask, FixedClock) {
    let clock = FixedClock::new(1_000_000);
    let clock_arc: Arc<dyn Clock> = Arc::new(clock.clone());
    let task = GameTask::new(
        catalog(),
        SEED,
        [
            Playstyle::Mammal,
            Playstyle::Bird,
            Playstyle::Plant,
            Playstyle::Virus,
            Playstyle::Machine,
        ],
        heroes(),
        players(),
        clock_arc,
        Arc::new(room_service::AchievementCatalog::embedded()),
    )
    .expect("开局应当成功");
    (task, clock)
}

/// 处理一条命令并要求恰好返回一条更新。
fn one_update(task: &mut GameTask, cmd: RoomCommand) -> SeatUpdate {
    let mut updates = task.handle(cmd);
    assert_eq!(updates.len(), 1, "命令应返回恰好一条更新");
    updates.remove(0)
}

/// 把一条快照更新编码成信封并反序列化为协议 `Snapshot`。
fn encode_snapshot(task: &mut GameTask, update: &SeatUpdate) -> (Snapshot, u64) {
    let envelope = task.envelope_for(update).expect("快照应能编码成信封");
    assert_eq!(&*envelope.type_, "Snapshot", "信封类型应为 Snapshot");
    let seq = envelope.seq.get();
    let snapshot: Snapshot =
        serde_json::from_value(Value::Object(envelope.payload)).expect("载荷应是合法 Snapshot");
    (snapshot, seq)
}

/// 构造重连命令；返回命令与接收端（接收端仅保活，测试不读取）。
fn reconnect(
    player: &str,
    last_seq: u64,
) -> (RoomCommand, mpsc::Receiver<protocol::ServerEnvelope>) {
    let (out, rx) = mpsc::channel(8);
    (
        RoomCommand::Reconnect {
            player_id: player.to_string(),
            last_seq,
            out,
        },
        rx,
    )
}

#[test]
fn reconnect_returns_full_snapshot() {
    let (mut task, _clock) = task();

    // 旧连接先收到一条状态更新，模拟断线前的最后一条消息。
    let old = one_update(
        &mut task,
        RoomCommand::RequestSnapshot {
            player_id: "p0".to_string(),
        },
    );
    let old_envelope = task.envelope_for(&old).expect("旧消息应能编码");
    let old_seq = old_envelope.seq.get();
    let expected_turn = task.player_view(0).expect("视图应可用").turn;

    let (cmd, _rx) = reconnect("p0", old_seq);
    let update = one_update(&mut task, cmd);
    assert!(matches!(update.kind, UpdateKind::Snapshot));

    let (snapshot, _seq) = encode_snapshot(&mut task, &update);
    assert_eq!(
        snapshot.view.turn, expected_turn,
        "重连快照的回合应与当前一致"
    );
    assert_eq!(snapshot.view.self_.player_id, "p0");
}

#[test]
fn reconnect_with_stale_last_seq_still_gets_snapshot() {
    let (mut task, _clock) = task();

    let (cmd, _rx) = reconnect("p1", 0);
    let update = one_update(&mut task, cmd);
    assert!(
        matches!(update.kind, UpdateKind::Snapshot),
        "过期的 last_seq 也必须回全量快照（M1 不重放历史）"
    );

    let (snapshot, seq) = encode_snapshot(&mut task, &update);
    assert!(seq > 0);
    assert_eq!(snapshot.seq.map(std::num::NonZeroU64::get), Some(seq));
}

#[test]
fn snapshot_seq_continues_seat_counter() {
    let (mut task, _clock) = task();

    // 断线前该座位已经收到两条消息。
    let first = one_update(
        &mut task,
        RoomCommand::RequestSnapshot {
            player_id: "p2".to_string(),
        },
    );
    let first_seq = task.envelope_for(&first).expect("应能编码").seq.get();
    let second = one_update(
        &mut task,
        RoomCommand::RequestSnapshot {
            player_id: "p2".to_string(),
        },
    );
    let second_seq = task.envelope_for(&second).expect("应能编码").seq.get();
    assert!(second_seq > first_seq);

    let (cmd, _rx) = reconnect("p2", second_seq);
    let update = one_update(&mut task, cmd);
    let (snapshot, snapshot_seq) = encode_snapshot(&mut task, &update);

    assert!(
        snapshot_seq > second_seq,
        "快照序号必须大于重连前该座位最后一条消息的序号"
    );
    assert_eq!(
        snapshot.seq.map(std::num::NonZeroU64::get),
        Some(snapshot_seq),
        "载荷里的 seq 必须与信封一致"
    );
    assert_eq!(
        task.channel_seq(2),
        snapshot_seq,
        "快照编码后座位计数器停在该序号"
    );
}

#[test]
fn disconnect_keeps_seat_for_thirty_seconds() {
    let (mut task, clock) = task();

    task.handle(RoomCommand::Disconnect {
        player_id: "p3".to_string(),
    });

    clock.advance(29_000);
    let updates = task.tick();
    assert_eq!(task.seat_of("p3"), Some(3), "宽限期内座位不得释放");
    assert!(task.is_connected(3), "宽限期内仍视为在线");
    assert!(
        updates.iter().all(|update| update.player_id != "p3"),
        "宽限期内不产生下行更新"
    );

    clock.advance(2_000);
    let updates = task.tick();
    assert_eq!(task.seat_of("p3"), Some(3), "超过宽限期座位仍然保留");
    assert!(!task.is_connected(3), "超过 30 秒应标记离线");
    assert!(
        updates.iter().all(|update| update.player_id != "p3"),
        "离线是内部状态，不下发通知"
    );
}

#[test]
fn disconnect_marks_offline_after_grace() {
    let (mut task, clock) = task();

    task.handle(RoomCommand::Disconnect {
        player_id: "p4".to_string(),
    });
    assert!(task.is_connected(4));

    clock.advance(30_000);
    task.tick();
    assert!(!task.is_connected(4), "恰好 30 秒也应离线");
    assert_eq!(task.seat_of("p4"), Some(4), "离线不释放座位");

    // 再次 tick 不应恢复在线，也不产生该座位的更新。
    assert!(task.tick().iter().all(|update| update.player_id != "p4"));
    assert!(!task.is_connected(4));
}

#[test]
fn bind_sender_registers_first_connection() {
    let (mut task, _clock) = task();

    let (out, _rx) = mpsc::channel(4);
    assert!(task.bind_sender("p6", out), "已知玩家应绑定成功");
    assert!(task.sender(6).is_some(), "首次连接也应持有发送端");

    let (unknown, _unknown_rx) = mpsc::channel(4);
    assert!(
        !task.bind_sender("nobody", unknown),
        "未知玩家绑定应返回 false"
    );
    let bound = (0..8).filter(|seat| task.sender(*seat).is_some()).count();
    assert_eq!(bound, 1, "只有已知玩家的座位持有发送端");
}

#[test]
fn request_snapshot_returns_full_view() {
    let (mut task, _clock) = task();

    let update = one_update(
        &mut task,
        RoomCommand::RequestSnapshot {
            player_id: "p5".to_string(),
        },
    );
    assert!(matches!(update.kind, UpdateKind::Snapshot));

    let (snapshot, _seq) = encode_snapshot(&mut task, &update);
    let expected = task.state_update(5).expect("状态应可用").view;
    assert_eq!(
        serde_json::to_value(&snapshot.view).expect("视图应可序列化"),
        serde_json::to_value(&expected).expect("视图应可序列化"),
        "快照视图应与当前状态更新视图完全一致"
    );
    assert!(snapshot.seq.is_some(), "快照必须带当前序号");
}

#[test]
fn snapshot_includes_pending_choice() {
    let (mut task, _clock) = task();

    // 座位 1 是达尔文，主动技能「进化」花费 2 金币（初始 3 金币），
    // 使用后立刻产生一条发现待选。
    let used = task.handle(RoomCommand::Intent {
        player_id: "p1".to_string(),
        client_intent_id: "hero-power-1".to_string(),
        intent: ProtocolIntent::UseHeroPower {
            skill_id: "evolution-discovery".to_string(),
            target: None,
        },
    });
    assert!(
        used.iter()
            .all(|update| !matches!(&update.kind, UpdateKind::Error(_))),
        "使用英雄技能不应报错"
    );

    let update = one_update(
        &mut task,
        RoomCommand::RequestSnapshot {
            player_id: "p1".to_string(),
        },
    );
    let (snapshot, _seq) = encode_snapshot(&mut task, &update);

    assert!(
        !snapshot.view.self_.pending_choices.is_empty(),
        "快照必须带上未消费的待选"
    );
    assert!(
        snapshot
            .view
            .self_
            .pending_choices
            .iter()
            .any(|choice| matches!(choice, PendingChoice::Discover { .. })),
        "待选类型应为发现"
    );
}

#[test]
fn reconnect_snapshot_matches_current_state_after_actions() {
    let (mut task, clock) = task();

    // 断线前座位 0 买入一张牌，状态与开局不同。
    let bought = task.handle(RoomCommand::Intent {
        player_id: "p0".to_string(),
        client_intent_id: "buy-before-reconnect".to_string(),
        intent: ProtocolIntent::BuyMinion { slot: 0 },
    });
    assert!(
        bought
            .iter()
            .all(|update| !matches!(&update.kind, UpdateKind::Error(_))),
        "买入不应报错"
    );

    task.handle(RoomCommand::Disconnect {
        player_id: "p0".to_string(),
    });

    // 断线期间其他座位继续操作，全局状态推进；重连必须回到当前状态而不是断线前状态。
    let while_offline = task.handle(RoomCommand::Intent {
        player_id: "p1".to_string(),
        client_intent_id: "buy-while-p0-offline".to_string(),
        intent: ProtocolIntent::BuyMinion { slot: 0 },
    });
    assert!(
        while_offline
            .iter()
            .all(|update| !matches!(&update.kind, UpdateKind::Error(_))),
        "断线期间其他座位的操作不应报错"
    );
    clock.advance(10_000);

    let (cmd, _rx) = reconnect("p0", 0);
    let update = one_update(&mut task, cmd);
    let (snapshot, _seq) = encode_snapshot(&mut task, &update);
    let current = task.state_update(0).expect("当前状态应可用");

    assert_eq!(
        serde_json::to_value(&snapshot.view).expect("快照视图应可序列化"),
        serde_json::to_value(&current.view).expect("当前视图应可序列化"),
        "重连快照必须与断线重连后的当前服务端视图完全一致"
    );
    assert_eq!(
        snapshot.view.self_.hand.len(),
        1,
        "断线前的买入应保留在重连快照里"
    );
    assert_eq!(snapshot.view.self_.gold, 0, "金币应与当前状态一致");
}

#[test]
fn combat_reconnect_resends_combat_result() {
    let (mut task, _clock) = task();

    // 全员结束招募进入演示阶段。
    for seat in 0..8 {
        task.handle(RoomCommand::Intent {
            player_id: format!("p{seat}"),
            client_intent_id: format!("end-{seat}"),
            intent: ProtocolIntent::EndRecruit,
        });
    }
    assert_eq!(
        task.player_view(0).expect("视图应可用").phase,
        protocol::view::Phase::Combat,
        "全员结束后应停在演示阶段"
    );

    let (cmd, _rx) = reconnect("p0", 0);
    let updates = task.handle(cmd);
    assert_eq!(updates.len(), 2, "重连应回快照与本场战报各一条");
    assert!(
        matches!(updates[0].kind, UpdateKind::Snapshot),
        "第一条应为全量快照"
    );
    assert!(
        matches!(updates[1].kind, UpdateKind::CombatResult(_)),
        "第二条应为该座位本场 CombatResult"
    );
    assert_eq!(updates[0].player_id, "p0");
    assert_eq!(updates[1].player_id, "p0");

    // 演示阶段重连的快照同样必须与当前服务端视图一致。
    let (snapshot, _seq) = encode_snapshot(&mut task, &updates[0]);
    let current = task.state_update(0).expect("当前状态应可用");
    assert_eq!(
        serde_json::to_value(&snapshot.view).expect("快照视图应可序列化"),
        serde_json::to_value(&current.view).expect("当前视图应可序列化"),
        "演示阶段重连快照必须与当前视图一致"
    );
}

// ---------------------------------------------------------------------------
// 真实 WebSocket：对局结束即销毁房间，结束后重连不补发结果。
// ---------------------------------------------------------------------------

/// 测试用最小 WebSocket 客户端。
struct WsClient {
    ws: common::ClientWs,
    next_id: u64,
}

impl WsClient {
    async fn connect(addr: std::net::SocketAddr) -> Self {
        let ws = common::connect_with_retry(&format!("ws://{addr}/ws")).await;
        Self { ws, next_id: 0 }
    }

    async fn send(&mut self, kind: &str, payload: Value) {
        self.next_id += 1;
        let envelope = json!({
            "v": 1,
            "id": format!("reconnect_ws_{}", self.next_id),
            "type": kind,
            "ts": "2026-01-01T00:00:00Z",
            "payload": payload,
        });
        self.ws
            .send(Message::Text(envelope.to_string().into()))
            .await
            .expect("发送应当成功");
    }

    async fn recv_room(&mut self) -> ServerMessage {
        loop {
            let envelope: ServerEnvelope = match self.ws.next().await {
                Some(Ok(Message::Text(text))) => {
                    serde_json::from_str(text.as_str()).expect("下行信封应能解析")
                }
                Some(Ok(Message::Close(_))) | None => panic!("连接在断言前关闭"),
                Some(Ok(_)) => continue,
                Some(Err(error)) => panic!("WebSocket 读取失败: {error}"),
            };
            let message = envelope.parse().expect("下行载荷应能解析");
            if matches!(
                message,
                ServerMessage::FriendUpdate(_)
                    | ServerMessage::AccountView(_)
                    | ServerMessage::AchievementState(_)
                    | ServerMessage::AchievementUnlocked(_)
            ) {
                continue;
            }
            return message;
        }
    }
}

fn phone_for(name: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in name.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("139{:08}", hash % 100_000_000)
}

async fn authenticate_ws(client: &mut WsClient, name: &str) -> String {
    client
        .send(
            "AuthRequest",
            json!({ "phone": phone_for(name), "code": "123456", "displayName": name }),
        )
        .await;
    loop {
        match client.recv_room().await {
            ServerMessage::AuthResult(result) => return result.player_id,
            ServerMessage::Error(error) => panic!("认证失败: {} {}", error.code, error.message),
            _ => {}
        }
    }
}

fn hero_cards() -> Vec<String> {
    let catalog = sim::embedded_catalog().expect("内嵌卡表应能加载");
    catalog
        .heroes
        .values()
        .take(8)
        .map(|hero| hero.id.clone())
        .collect()
}

/// 跑完一局 8 bot 对局并返回房间码。
async fn play_one_game(addr: std::net::SocketAddr) -> String {
    let url = format!("ws://{addr}/ws");
    let heroes = hero_cards();
    let (code_tx, code_rx) = oneshot::channel();
    let host = tokio::spawn(run(BotConfig {
        url: url.clone(),
        room: String::new(),
        name: "reconnect-bot0".to_string(),
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
            name: format!("reconnect-bot{index}"),
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
    room_code
}

/// 对局结束即销毁房间：结束后重连旧房间返回错误，既没有大厅视图也没有缓存 `GameOver`。
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reconnect_after_game_over_finds_room_destroyed() {
    let (addr, router, state) =
        room_service::bind_state("127.0.0.1:0", Arc::new(SystemClock) as Arc<dyn Clock>)
            .await
            .expect("绑定端口应当成功");
    tokio::spawn(async move {
        let _ = room_service::serve(addr, router).await;
    });
    common::wait_for_server(addr).await;

    let _room_code = play_one_game(addr).await;

    // 同一手机号恢复账号，等待注册表里不再残留房间。
    let mut probe = WsClient::connect(addr).await;
    let host_id = authenticate_ws(&mut probe, "reconnect-bot0").await;
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let gone = {
                let rooms = state.rooms.lock().await;
                rooms.room_of_player(&host_id).is_none() && rooms.is_empty()
            };
            if gone {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("对局结束后房间应从注册表删除");

    // 测试服务器只创建过一个房间，其 id 为 `room_0`。
    probe
        .send("Reconnect", json!({ "roomId": "room_0", "lastSeq": 0 }))
        .await;
    let error = match probe.recv_room().await {
        ServerMessage::Error(error) => error,
        other => panic!("重连已销毁的房间应只返回错误，实际 {other:?}"),
    };
    assert!(
        matches!(
            error.code,
            protocol::ErrorCode::RoomNotFound | protocol::ErrorCode::NotInRoom
        ),
        "重连旧房间应视为房间不存在或不在房间，实际 {:?}",
        error.code
    );
    println!("reconnect after game over rejected as {:?}", error.code);
}

// ---------------------------------------------------------------------------
// 真实 WebSocket：选英雄阶段重连与开局后重连。
// ---------------------------------------------------------------------------

struct RawRoom {
    room_id: String,
    /// `(player_id, token)`，按座位顺序。
    identities: Vec<(String, String)>,
    clients: Vec<RawClient>,
}

/// 8 名真人建房、入房并全部准备；尚未 `StartGame`。
async fn join_eight_ready(addr: std::net::SocketAddr, prefix: &str) -> RawRoom {
    let mut clients = Vec::with_capacity(8);
    let mut identities = Vec::with_capacity(8);

    let mut host = RawClient::connect(addr).await;
    let (player_id, token) = authenticate(&mut host, &format!("{prefix}0")).await;
    host.send("CreateRoom", json!({})).await;
    let room = wait_room_update(&mut host).await;
    let room_id = room.room_id.clone();
    let room_code = room.code.clone();
    host.send("Ready", json!({ "ready": true })).await;
    clients.push(host);
    identities.push((player_id, token));

    for index in 1..8 {
        let mut client = RawClient::connect(addr).await;
        let (player_id, token) = authenticate(&mut client, &format!("{prefix}{index}")).await;
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

    wait_all_ready(&mut clients[0], 8).await;
    RawRoom {
        room_id,
        identities,
        clients,
    }
}

/// 选英雄阶段重连必须回 `HeroPickStarted`，本人已选状态与候选保持一致。
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reconnect_during_pick_returns_hero_pick() {
    let addr = common::start_server().await;
    let mut room = join_eight_ready(addr, "pick-recon-").await;

    room.clients[0].send("StartGame", json!({})).await;
    let mut picks = Vec::with_capacity(8);
    for client in room.clients.iter_mut() {
        picks.push(wait_hero_pick(client).await);
    }

    // 只让房主先选：保持选英雄阶段未完成，避免重连落进 Playing。
    let host_hero = first_candidate(&picks[0]);
    room.clients[0]
        .send("SelectHeroCard", json!({ "cardId": host_hero }))
        .await;

    let (target_id, target_token) = room.identities[7].clone();
    let expected_candidates = picks[7].candidates.clone();
    drop(room.clients.pop().expect("应有第 8 条连接"));

    let mut back = RawClient::connect(addr).await;
    let back_id = authenticate_with_token(&mut back, &target_token).await;
    assert_eq!(back_id, target_id, "重连应复用同一 player_id");
    back.send("Reconnect", json!({ "roomId": room.room_id, "lastSeq": 0 }))
        .await;

    let pick = wait_hero_pick(&mut back).await;
    assert_eq!(pick.players.len(), 8, "重连视图应含 8 个座位");
    let own = pick
        .players
        .iter()
        .find(|player| player.seat == 7)
        .expect("应能按座位找到本人");
    assert!(!own.picked, "未选英雄的座位重连后 picked 仍为 false");
    assert!(!own.is_bot, "真人座位不是机器人");
    let host = pick
        .players
        .iter()
        .find(|player| player.seat == 0)
        .expect("应能找到房主座位");
    assert!(host.picked, "已选英雄的座位在重连视图里 picked 为 true");
    assert_eq!(
        pick.candidates, expected_candidates,
        "重连应取回同一组本人候选"
    );
    assert!(!pick.deadline.is_empty(), "重连视图应带截止时刻");
    assert_eq!(
        pick.deadline, picks[7].deadline,
        "重连不应重置选英雄截止时刻"
    );
    println!("reconnect during pick returned HeroPickStarted");
}

/// 进入 `Playing` 后重连仍走 `Snapshot`，不再回 `HeroPickStarted`。
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reconnect_after_start_returns_snapshot() {
    let addr = common::start_server().await;
    let mut room = join_eight_ready(addr, "started-recon-").await;

    let (started, _picks) = start_and_pick_all(&mut room.clients).await;
    assert_eq!(started.len(), 8, "8 名真人选完英雄后都应开局");

    let (target_id, target_token) = room.identities[7].clone();
    drop(room.clients.pop().expect("应有第 8 条连接"));

    let mut back = RawClient::connect(addr).await;
    let back_id = authenticate_with_token(&mut back, &target_token).await;
    assert_eq!(back_id, target_id, "重连应复用同一 player_id");
    back.send("Reconnect", json!({ "roomId": room.room_id, "lastSeq": 0 }))
        .await;

    let snapshot = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let (_, message) = back.recv_room().await.expect("等待 Snapshot 时连接关闭");
            match message {
                ServerMessage::Snapshot(snapshot) => return snapshot,
                ServerMessage::HeroPickStarted(_) => {
                    panic!("选英雄已结束，重连不应再回 HeroPickStarted")
                }
                ServerMessage::Error(error) => {
                    panic!("重连收到错误 {}: {}", error.code, error.message)
                }
                _ => {}
            }
        }
    })
    .await
    .expect("10 秒内应收到 Snapshot");

    assert_eq!(snapshot.view.self_.player_id, target_id, "快照应属于本人");
    assert_eq!(snapshot.view.room_id, room.room_id, "快照应带上房间 id");
    assert_eq!(snapshot.view.turn.get(), 1, "选完英雄后应进入第 1 回合");
    println!("reconnect after start returned Snapshot");
}

// ---------------------------------------------------------------------------
// 单客户端登录：顶号接管对局。
// ---------------------------------------------------------------------------

/// 等本人快照：接管进入进行中对局只靠新登录，不额外发 `Reconnect`。
async fn wait_takeover_snapshot(client: &mut RawClient) -> Snapshot {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let (_, message) = client.recv_room().await.expect("等待 Snapshot 时连接关闭");
            match message {
                ServerMessage::Snapshot(snapshot) => return snapshot,
                ServerMessage::HeroPickStarted(_) => {
                    panic!("对局已进入 Playing，接管不应回 HeroPickStarted")
                }
                ServerMessage::Error(error) => {
                    panic!("接管收到错误 {}: {}", error.code, error.message)
                }
                _ => {}
            }
        }
    })
    .await
    .expect("10 秒内应收到 Snapshot")
}

/// 同账号新登录直接接管进行中的对局：拿到本人快照，其他玩家看不到托管。
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn second_login_takes_over_started_game() {
    let addr = common::start_server().await;
    let mut room = join_eight_ready(addr, "solo-game-").await;
    let (started, _picks) = start_and_pick_all(&mut room.clients).await;
    assert_eq!(started.len(), 8, "8 名真人选完英雄后都应开局");

    let (target_id, _target_token) = room.identities[7].clone();
    let mut next = RawClient::connect(addr).await;
    let (next_id, _next_token) = authenticate(&mut next, "solo-game-7").await;
    assert_eq!(next_id, target_id, "同一手机号应恢复同一账号");

    let snapshot = wait_takeover_snapshot(&mut next).await;
    assert_eq!(snapshot.view.self_.player_id, target_id, "快照应属于本人");
    assert_eq!(snapshot.view.room_id, room.room_id, "快照应带上房间 id");
    let own = snapshot
        .view
        .players
        .iter()
        .find(|player| player.player_id == target_id)
        .expect("快照应包含本人座位");
    assert!(!own.managed, "接管后本人座位不应处于托管");
    assert!(
        snapshot
            .view
            .players
            .iter()
            .all(|player| player.player_id == target_id || !player.managed),
        "其他玩家不应因顶号进入托管"
    );

    // 旧连接收到顶号提示。
    let error = match room.clients[7].recv_room().await {
        Some((_, ServerMessage::Error(error))) => error,
        other => panic!("旧连接应收到 SESSION_REPLACED，实际 {other:?}"),
    };
    assert_eq!(error.code, protocol::ErrorCode::SessionReplaced);

    // 接管后的连接已绑定下发通道，可继续请求快照。
    next.send("RequestSnapshot", json!({})).await;
    let again = wait_takeover_snapshot(&mut next).await;
    assert_eq!(again.view.room_id, room.room_id);
}

/// 旧连接先被强制断开再登录：接管仍成功，座位不因迟到的 `Detach` 被释放或托管。
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn takeover_after_forced_disconnect_keeps_seat() {
    let addr = common::start_server().await;
    let mut room = join_eight_ready(addr, "solo-drop-").await;
    let (started, _picks) = start_and_pick_all(&mut room.clients).await;
    assert_eq!(started.len(), 8);

    let (target_id, _target_token) = room.identities[7].clone();
    // 模拟网络断开：旧 socket 直接丢弃，服务端随后处理 Detach 与断线宽限。
    drop(room.clients.pop().expect("应有第 8 条连接"));

    let mut next = RawClient::connect(addr).await;
    let (next_id, _next_token) = authenticate(&mut next, "solo-drop-7").await;
    assert_eq!(next_id, target_id);

    let snapshot = wait_takeover_snapshot(&mut next).await;
    assert_eq!(snapshot.view.self_.player_id, target_id, "快照应属于本人");
    let own = snapshot
        .view
        .players
        .iter()
        .find(|player| player.player_id == target_id)
        .expect("快照应包含本人座位");
    assert!(!own.managed, "接管后座位不应处于托管");
}

/// 已弃权的座位不接管：同账号新登录停在大厅，不收到房间或对局状态。
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn second_login_skips_forfeited_seat() {
    let addr = common::start_server().await;
    let mut room = join_eight_ready(addr, "solo-forfeit-").await;
    let (started, _picks) = start_and_pick_all(&mut room.clients).await;
    assert_eq!(started.len(), 8);

    // 对局中离开即弃权：座位保留到本局结束，但不再接收任何推送。
    room.clients[7].send("LeaveRoom", json!({})).await;
    let (target_id, _target_token) = room.identities[7].clone();
    drop(room.clients.pop().expect("应有第 8 条连接"));

    let mut next = RawClient::connect(addr).await;
    let (next_id, _next_token) = authenticate(&mut next, "solo-forfeit-7").await;
    assert_eq!(next_id, target_id);

    let quiet = tokio::time::timeout(Duration::from_millis(500), async {
        loop {
            let (_, message) = next.recv_room().await.expect("等待接管结果时连接关闭");
            match message {
                ServerMessage::RoomUpdate(update) => {
                    panic!("弃权座位不应接管房间，实际收到房间 {}", update.room_id)
                }
                ServerMessage::Snapshot(_) | ServerMessage::HeroPickStarted(_) => {
                    panic!("弃权座位不应接管对局")
                }
                _ => {}
            }
        }
    })
    .await;
    assert!(quiet.is_err(), "新登录应停在大厅，不收到房间或对局状态");
}
