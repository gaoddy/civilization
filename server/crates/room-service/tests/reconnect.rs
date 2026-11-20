//! 重连、全量快照与断线保留窗口测试。
//!
//! 大部分用例使用 [`FixedClock`] 与进程内引擎，不启网络；`Reconnect.out` 只绑定不
//! 实际读取，发送端是否被保存由连接层（Task 12）消费，这里只验证命令返回值。
//! 文末另有真实 WebSocket 的「对局结束即销毁房间、结束后重连不补发结果」用例。

mod common;

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

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
            hero_card_id: Some(HEROES[seat].to_string()),
            channel: SeatChannel::new(),
            hero_candidates: Vec::new(),
            candidate_segment: None,
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
            if matches!(message, ServerMessage::FriendUpdate(_)) {
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
