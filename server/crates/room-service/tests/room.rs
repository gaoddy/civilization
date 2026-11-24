//! 房间状态机与大堂流程测试。
//!
//! 覆盖建房、加入/离开、准备、开局计划（`prepare_start` / `commit_start`）与
//! `RoomUpdate` 视图。房间码与座位分配均为确定性行为，测试直接断言具体取值。
//! 文末另有真实 WebSocket 的建房、邀请、房主移除/解散用例。

mod common;

use std::collections::{BTreeMap, BTreeSet};

use futures_util::{SinkExt, StreamExt};
use protocol::ErrorCode;
use protocol::client::AuthRequest;
use protocol::server::RoomUpdate;
use protocol::{RoomClosedReason, ServerEnvelope, ServerMessage};
use room_service::auth::{Session, SessionStore};
use room_service::clock::{Clock, FixedClock};
use room_service::room::{ROOM_INVITE_TTL_MS, RoomError, RoomRegistry, SEAT_COUNT, StartPlan};
use rule_engine::data::{Catalog, Category, CategoryKind, HeroCard, Playstyle, hero_in_pool};
use rule_engine::state::Phase;
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message;

const CARDS_JSON: &str = include_str!("../../../data/v0.1/cards.json");
const HEROES_JSON: &str = include_str!("../../../data/v0.1/heroes.json");
const SKILLS_JSON: &str = include_str!("../../../data/v0.1/skills.json");
const CONFIG_JSON: &str = include_str!("../../../data/v0.1/config.json");
const TAXONOMY_JSON: &str = include_str!("../../../data/v0.1/taxonomy.json");

/// 真实卡表；英雄卡用 `adam-smith`，非英雄卡用 `agi`。
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

/// 用固定短信码登录出一个会话。
fn session(store: &mut SessionStore, clock: &FixedClock, name: &str) -> Session {
    let display_name = name.to_string().try_into().expect("展示名合法");
    store
        .authenticate(
            &AuthRequest::Variant0 {
                code: "123456".to_string(),
                display_name: Some(display_name),
                phone: "13800000000".to_string(),
            },
            clock,
        )
        .expect("开发验证码应登录成功")
}

/// 建一个 8 人满座的大厅房间，返回注册表、房间 id、房间码与按座位号排列的会话。
fn lobby_room() -> (RoomRegistry, String, String, Vec<Session>) {
    lobby_room_with(catalog())
}

/// 用指定卡表建满座大厅；候选池相关用例借此控制英雄卡数量与玩法。
fn lobby_room_with(catalog: Catalog) -> (RoomRegistry, String, String, Vec<Session>) {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let mut registry = RoomRegistry::with_seed(0);

    let host = session(&mut store, &clock, "房主");
    let room = registry.create(&host, &catalog);
    let room_id = room.id.clone();
    let code = room.code.clone();

    let mut players = vec![host];
    for index in 1..SEAT_COUNT {
        let player = session(&mut store, &clock, &format!("玩家{index}"));
        registry.join(&code, &player, &catalog).expect("加入成功");
        players.push(player);
    }
    (registry, room_id, code, players)
}

/// 只保留指定英雄，并给每张英雄补全 13 条 `play:`，使其对任意启用玩法都入池。
fn catalog_with_generic_heroes(hero_ids: &[&str]) -> Catalog {
    let mut catalog = catalog();
    let keep_all = hero_ids.is_empty();
    catalog
        .heroes
        .retain(|_, hero| keep_all || hero_ids.contains(&hero.id.as_str()));
    for hero in catalog.heroes.values_mut() {
        make_generic_hero(hero);
    }
    catalog
}

/// 给英雄挂上全部 13 个玩法归属，`hero_in_pool` 对任意本局玩法组合恒为真。
fn make_generic_hero(hero: &mut HeroCard) {
    hero.categories = Playstyle::ALL
        .iter()
        .map(|playstyle| Category {
            kind: CategoryKind::Play,
            value: playstyle.id().to_string(),
        })
        .collect();
}

/// 让大厅里的所有真人准备就绪，但仍停留在 `Lobby`。
fn mark_all_ready(registry: &mut RoomRegistry, room_id: &str, players: &[Session]) {
    for player in players {
        registry
            .ready(room_id, &player.player_id, true)
            .expect("准备成功");
    }
}

/// 用真实卡表为房间构造开局计划。
fn prepare(registry: &RoomRegistry, room_id: &str, host: &str) -> StartPlan {
    registry
        .prepare_start(room_id, host, &catalog())
        .expect("8 人模式应能构造开局计划")
}

/// 在满座大厅里让所有真人准备，但仍停留在 `Lobby`。
fn ready_room() -> (RoomRegistry, String, String, Vec<Session>) {
    let (mut registry, room_id, code, players) = lobby_room();
    mark_all_ready(&mut registry, &room_id, &players);
    (registry, room_id, code, players)
}

/// 已完成 `prepare_start` + `commit_start`、阶段推进到 `Recruit` 的房间。
fn started_room() -> (RoomRegistry, String, String, Vec<Session>) {
    let (mut registry, room_id, code, players) = ready_room();
    let plan = prepare(&registry, &room_id, &players[0].player_id);
    registry
        .commit_start(&room_id, plan)
        .expect("全员就绪后应能提交开局");
    (registry, room_id, code, players)
}

#[test]
fn create_assigns_host_to_seat0() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");

    let mut registry = RoomRegistry::with_seed(0);
    let room = registry.create(&host, &catalog());

    assert_eq!(room.code.len(), 6, "房间码应为 6 位");
    assert!(
        room.code
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()),
        "房间码只含大写字母数字，实际为 {}",
        room.code
    );
    assert!(
        !room.code.chars().any(|c| "0O1I".contains(c)),
        "房间码应排除易混字符 0O1I，实际为 {}",
        room.code
    );
    assert_eq!(room.code, "222222", "同 seed 的房间码应可预期");
    assert_eq!(room.host_id, host.player_id);
    assert_eq!(room.seat_of(&host.player_id), Some(0), "房主坐 0 号座位");
    assert_eq!(room.phase, Phase::Lobby);
    assert!(room.bot_fill, "建房默认开启机器人补位");
    assert!(room.candidates().is_empty(), "等待区不再预先分配英雄候选");
    assert!(!room.slot(0).unwrap().is_bot);

    // 同一 seed 的两个注册表生成同一房间码，保证测试可预期。
    let mut other = RoomRegistry::with_seed(0);
    assert_eq!(other.create(&host, &catalog()).code, room.code);
}

#[test]
fn join_fills_seats_in_order() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");

    let mut registry = RoomRegistry::with_seed(0);
    let code = registry.create(&host, &catalog()).code.clone();

    for expected_seat in 1..SEAT_COUNT {
        let player = session(&mut store, &clock, &format!("玩家{expected_seat}"));
        let room = registry
            .join(&code, &player, &catalog())
            .expect("未满时应能加入");
        assert_eq!(
            room.seat_of(&player.player_id),
            Some(expected_seat as u8),
            "第 {expected_seat} 名玩家应坐 {expected_seat} 号座位"
        );
    }
}

#[test]
fn join_rejects_ninth_player() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");

    let mut registry = RoomRegistry::with_seed(0);
    let room = registry.create(&host, &catalog());
    let room_id = room.id.clone();
    let code = room.code.clone();

    for index in 1..SEAT_COUNT {
        let player = session(&mut store, &clock, &format!("玩家{index}"));
        registry
            .join(&code, &player, &catalog())
            .expect("前 8 人应能加入");
    }

    let ninth = session(&mut store, &clock, "第九人");
    assert_eq!(
        registry
            .join(&code, &ninth, &catalog())
            .expect_err("满座后应拒绝"),
        RoomError::Full
    );
    assert_eq!(
        registry
            .room_update(&room_id)
            .expect("房间仍存在")
            .players
            .len(),
        SEAT_COUNT
    );
}

#[test]
fn join_unknown_code_returns_not_found() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let player = session(&mut store, &clock, "玩家");

    let mut registry = RoomRegistry::with_seed(0);
    assert_eq!(
        registry
            .join("AAAAAA", &player, &catalog())
            .expect_err("不存在的房间码"),
        RoomError::NotFound
    );
}

#[test]
fn join_same_player_twice_is_idempotent() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");

    let mut registry = RoomRegistry::with_seed(0);
    let room = registry.create(&host, &catalog());
    let room_id = room.id.clone();
    let code = room.code.clone();

    registry
        .join(&code, &host, &catalog())
        .expect("同一玩家重复加入应幂等成功");

    assert_eq!(registry.room_update(&room_id).unwrap().players.len(), 1);
    assert_eq!(
        registry.room_update(&room_id).unwrap().players[0].player_id,
        host.player_id
    );
}

#[test]
fn ready_succeeds_without_hero_selection() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");
    let catalog = catalog();

    let mut registry = RoomRegistry::with_seed(0);
    let room = registry.create(&host, &catalog);
    let room_id = room.id.clone();

    registry
        .ready(&room_id, &host.player_id, false)
        .expect("取消准备不要求已选英雄");
    registry
        .ready(&room_id, &host.player_id, true)
        .expect("等待区准备不再要求先选英雄");

    assert!(registry.room(&room_id).unwrap().slot(0).unwrap().ready);
}

#[test]
fn prepare_start_plans_bots_from_lowest_seats() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");
    let mut registry = RoomRegistry::with_seed(0);
    let room = registry.create(&host, &catalog());
    let room_id = room.id.clone();
    registry
        .ready(&room_id, &host.player_id, true)
        .expect("准备成功");

    let plan = prepare(&registry, &room_id, &host.player_id);

    assert_eq!(plan.slots.len(), SEAT_COUNT, "开局计划应覆盖 8 个座位");
    assert!(!plan.slots[0].is_bot, "0 号座位是真人房主");
    for seat in 1..SEAT_COUNT {
        let slot = &plan.slots[seat];
        assert!(slot.is_bot, "座位 {seat} 应由机器人补位");
        assert_eq!(
            slot.player_id,
            format!("bot:{room_id}:{seat}"),
            "机器人 playerId 由房间与座位派生"
        );
        assert_eq!(
            slot.display_name,
            format!("机器人 {seat}"),
            "机器人按座位升序编号"
        );
        assert!(slot.ready && slot.connected, "机器人座位直接就绪并在线");
    }
}

#[test]
fn prepare_start_does_not_mutate_registry() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");
    let mut registry = RoomRegistry::with_seed(0);
    let room = registry.create(&host, &catalog());
    let room_id = room.id.clone();
    registry
        .ready(&room_id, &host.player_id, true)
        .expect("准备成功");

    let _plan = prepare(&registry, &room_id, &host.player_id);

    let room = registry.room(&room_id).expect("房间仍存在");
    assert_eq!(room.slots().len(), 1, "准备阶段不得写入机器人座位");
    assert_eq!(room.phase, Phase::Lobby, "准备阶段不得推进阶段");
    assert!(room.candidates().is_empty(), "准备阶段不得写入候选");
}

#[test]
fn prepare_start_requires_all_humans_ready() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");
    let second = session(&mut store, &clock, "玩家2");
    let mut registry = RoomRegistry::with_seed(0);
    let room = registry.create(&host, &catalog());
    let room_id = room.id.clone();
    registry
        .join(&room.code, &second, &catalog())
        .expect("加入成功");
    registry
        .ready(&room_id, &host.player_id, true)
        .expect("房主准备成功");

    assert_eq!(
        registry
            .prepare_start(&room_id, &host.player_id, &catalog())
            .expect_err("有人未准备不能开局"),
        RoomError::RoomNotReady
    );
    assert_eq!(registry.room(&room_id).unwrap().phase, Phase::Lobby);
}

#[test]
fn prepare_start_with_bot_fill_off_requires_eight_humans() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");
    let second = session(&mut store, &clock, "玩家2");

    let mut registry = RoomRegistry::with_seed(0);
    let room = registry.create(&host, &catalog());
    let room_id = room.id.clone();
    let code = room.code.clone();
    registry.join(&code, &second, &catalog()).expect("加入成功");
    registry
        .set_bot_fill(&room_id, &host.player_id, false)
        .expect("关闭补位成功");
    registry
        .ready(&room_id, &host.player_id, true)
        .expect("准备成功");
    registry
        .ready(&room_id, &second.player_id, true)
        .expect("准备成功");

    assert_eq!(
        registry
            .prepare_start(&room_id, &host.player_id, &catalog())
            .expect_err("补位关闭且真人不足 8 不能开局"),
        RoomError::RoomNotReady
    );
    assert_eq!(
        registry.room(&room_id).unwrap().phase,
        Phase::Lobby,
        "失败时注册表不变"
    );

    for index in 2..SEAT_COUNT {
        let player = session(&mut store, &clock, &format!("玩家{index}"));
        registry.join(&code, &player, &catalog()).expect("加入成功");
        registry
            .ready(&room_id, &player.player_id, true)
            .expect("准备成功");
    }
    let plan = registry
        .prepare_start(&room_id, &host.player_id, &catalog())
        .expect("8 名真人就绪后应能开局");
    assert!(
        plan.slots.iter().all(|slot| !slot.is_bot),
        "补位关闭时不得产生机器人"
    );
}

#[test]
fn prepare_start_requires_host_and_lobby() {
    let (mut registry, room_id, _code, players) = ready_room();

    assert_eq!(
        registry
            .prepare_start(&room_id, &players[1].player_id, &catalog())
            .expect_err("非房主不能开始"),
        RoomError::NotHost
    );
    assert_eq!(
        registry
            .prepare_start(&room_id, &players[1].player_id, &catalog())
            .expect_err("非房主不能开始")
            .code(),
        ErrorCode::NotHost
    );

    let plan = prepare(&registry, &room_id, &players[0].player_id);
    registry
        .commit_start(&room_id, plan)
        .expect("房主提交开局成功");
    assert_eq!(
        registry
            .prepare_start(&room_id, &players[0].player_id, &catalog())
            .expect_err("已离开大厅不能再次准备"),
        RoomError::WrongPhase
    );
}

#[test]
fn commit_start_inserts_bots_and_candidates() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");
    let mut registry = RoomRegistry::with_seed(0);
    let room = registry.create(&host, &catalog());
    let room_id = room.id.clone();
    registry
        .ready(&room_id, &host.player_id, true)
        .expect("准备成功");

    let plan = prepare(&registry, &room_id, &host.player_id);
    registry.commit_start(&room_id, plan).expect("提交开局成功");

    let room = registry.room(&room_id).expect("房间仍存在");
    assert_eq!(room.phase, Phase::Recruit, "提交后推进到招募阶段");
    assert_eq!(room.slots().len(), SEAT_COUNT, "8 个座位全部写入");
    assert_eq!(room.candidates().len(), SEAT_COUNT, "候选覆盖 8 个座位");
    for seat in 0..SEAT_COUNT as u8 {
        assert!(
            room.candidates()
                .get(&seat)
                .is_some_and(|list| !list.is_empty()),
            "座位 {seat} 应有候选"
        );
    }
    assert_eq!(room.slots().values().filter(|slot| slot.is_bot).count(), 7);
}

#[test]
fn commit_start_rejects_unknown_room() {
    let (registry, room_id, _code, players) = ready_room();
    let plan = prepare(&registry, &room_id, &players[0].player_id);
    let mut registry = registry;
    assert_eq!(
        registry
            .commit_start("room_missing", plan)
            .expect_err("房间不存在"),
        RoomError::NotFound
    );
}

#[test]
fn set_bot_fill_requires_host_and_lobby() {
    let (mut registry, room_id, _code, players) = lobby_room();

    assert_eq!(
        registry
            .set_bot_fill(&room_id, &players[1].player_id, false)
            .expect_err("非房主不能修改补位"),
        RoomError::NotHost
    );
    registry
        .set_bot_fill(&room_id, &players[0].player_id, false)
        .expect("房主可以修改补位");
    assert!(!registry.room(&room_id).unwrap().bot_fill);

    let (mut started, started_room_id, _code, started_players) = started_room();
    assert_eq!(
        started
            .set_bot_fill(&started_room_id, &started_players[0].player_id, true)
            .expect_err("开局后不能修改补位"),
        RoomError::WrongPhase
    );
}

#[test]
fn room_update_carries_seat_and_bot_fill() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");
    let second = session(&mut store, &clock, "玩家2");
    let third = session(&mut store, &clock, "玩家3");
    let mut registry = RoomRegistry::with_seed(0);
    let room = registry.create(&host, &catalog());
    let room_id = room.id.clone();
    let code = room.code.clone();
    registry.join(&code, &second, &catalog()).expect("加入成功");
    registry.join(&code, &third, &catalog()).expect("加入成功");

    let update = registry.room_update(&room_id).expect("房间存在");
    assert!(update.bot_fill, "默认开启补位");
    assert_eq!(update.players.len(), 3);
    for (index, item) in update.players.iter().enumerate() {
        assert_eq!(item.seat, i64::try_from(index).expect("座位号可表示"));
        assert!(!item.ready);
    }
    assert!(update.players[0].is_host);

    registry
        .set_bot_fill(&room_id, &host.player_id, false)
        .expect("关闭补位成功");
    let update = registry.room_update(&room_id).expect("房间存在");
    assert!(!update.bot_fill, "视图应反映关闭后的补位开关");

    // 中间座位离开后，剩余玩家按座位号升序排列，而不是按数组位置重排。
    registry
        .leave(&room_id, &second.player_id)
        .expect("中间玩家离开成功");
    let update = registry.room_update(&room_id).expect("房间存在");
    let seats: Vec<i64> = update.players.iter().map(|item| item.seat).collect();
    assert_eq!(seats, vec![0, 2], "座位号保持原值并升序排列");
    assert_eq!(update.players[0].player_id, host.player_id);
    assert_eq!(update.players[1].player_id, third.player_id);
}

#[test]
fn host_transfer_skips_bots() {
    // 机器人只在 `commit_start` 时写入，而删除座位只在大厅允许；这里先构造带机器人的
    // 注册表，再把阶段拨回大厅，专门覆盖 `remove_slot` 跳过机器人座位的分支。
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");
    let second = session(&mut store, &clock, "玩家2");
    let mut registry = RoomRegistry::with_seed(0);
    let room = registry.create(&host, &catalog());
    let room_id = room.id.clone();
    registry
        .join(&room.code, &second, &catalog())
        .expect("加入成功");
    registry
        .ready(&room_id, &host.player_id, true)
        .expect("准备成功");
    registry
        .ready(&room_id, &second.player_id, true)
        .expect("准备成功");
    let plan = prepare(&registry, &room_id, &host.player_id);
    registry.commit_start(&room_id, plan).expect("提交开局成功");
    assert_eq!(
        registry
            .room(&room_id)
            .unwrap()
            .slots()
            .values()
            .filter(|slot| slot.is_bot)
            .count(),
        6,
        "夹具应包含 6 个机器人"
    );
    registry.room_mut(&room_id).expect("房间存在").phase = Phase::Lobby;

    registry
        .leave(&room_id, &host.player_id)
        .expect("房主离开成功");

    let room = registry.room(&room_id).expect("房间仍存在");
    assert_eq!(
        room.host_id, second.player_id,
        "房主应转给座位号最小的真人，而不是机器人"
    );
    assert!(!room.slot(1).expect("真人座位").is_bot);

    // 只剩机器人时，房主离开不应把房主转给机器人，而应直接销毁房间。
    let room = registry.create(&host, &catalog());
    let room_id = room.id.clone();
    registry
        .join(&room.code, &second, &catalog())
        .expect("加入成功");
    registry
        .ready(&room_id, &host.player_id, true)
        .expect("准备成功");
    registry
        .ready(&room_id, &second.player_id, true)
        .expect("准备成功");
    let plan = prepare(&registry, &room_id, &host.player_id);
    registry.commit_start(&room_id, plan).expect("提交开局成功");
    registry.room_mut(&room_id).expect("房间存在").phase = Phase::Lobby;
    registry
        .kick(&room_id, &host.player_id, &second.player_id)
        .expect("移除唯一真人成员");
    registry
        .leave(&room_id, &host.player_id)
        .expect("房主离开成功");
    assert!(
        registry.room(&room_id).is_none(),
        "只剩机器人时房间应销毁，而不是让机器人接任房主"
    );
}

#[test]
fn bot_seats_reject_leave_and_kick() {
    // 同 `host_transfer_skips_bots`：机器人只在开局提交时写入，把阶段拨回大厅以覆盖成员管理。
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");
    let mut registry = RoomRegistry::with_seed(0);
    let room = registry.create(&host, &catalog());
    let room_id = room.id.clone();
    registry
        .ready(&room_id, &host.player_id, true)
        .expect("准备成功");
    let plan = prepare(&registry, &room_id, &host.player_id);
    registry.commit_start(&room_id, plan).expect("提交开局成功");
    registry.room_mut(&room_id).expect("房间存在").phase = Phase::Lobby;

    let bot_id = format!("bot:{room_id}:1");
    assert_eq!(
        registry
            .leave(&room_id, &bot_id)
            .expect_err("机器人不能离开房间"),
        RoomError::NotInRoom
    );
    assert_eq!(
        registry
            .kick(&room_id, &host.player_id, &bot_id)
            .expect_err("机器人不能被移除"),
        RoomError::NotInRoom
    );
    assert!(
        registry.room(&room_id).unwrap().seat_of(&bot_id).is_some(),
        "被拒绝后机器人座位保持不变"
    );
}

#[test]
fn leave_frees_seat() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");
    let second = session(&mut store, &clock, "玩家2");
    let third = session(&mut store, &clock, "玩家3");

    let mut registry = RoomRegistry::with_seed(0);
    let room = registry.create(&host, &catalog());
    let room_id = room.id.clone();
    let code = room.code.clone();
    registry
        .join(&code, &second, &catalog())
        .expect("玩家2 加入");
    registry
        .join(&code, &third, &catalog())
        .expect("玩家3 加入");

    registry
        .leave(&room_id, &second.player_id)
        .expect("离开成功");
    assert_eq!(
        registry.room(&room_id).unwrap().seat_of(&second.player_id),
        None,
        "离开后座位应立即空出"
    );
    assert_eq!(
        registry.room_update(&room_id).unwrap().players.len(),
        2,
        "其余座位不重排"
    );

    // 新玩家占用第一个空位（座位 1），而不是追加到末尾。
    let fourth = session(&mut store, &clock, "玩家4");
    let room = registry
        .join(&code, &fourth, &catalog())
        .expect("加入空位成功");
    assert_eq!(room.seat_of(&fourth.player_id), Some(1));
}

#[test]
fn leave_host_transfers_host_to_lowest_seat() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");
    let second = session(&mut store, &clock, "玩家2");
    let third = session(&mut store, &clock, "玩家3");

    let mut registry = RoomRegistry::with_seed(0);
    let room = registry.create(&host, &catalog());
    let room_id = room.id.clone();
    let code = room.code.clone();
    registry
        .join(&code, &second, &catalog())
        .expect("玩家2 加入");
    registry
        .join(&code, &third, &catalog())
        .expect("玩家3 加入");

    registry.leave(&room_id, &host.player_id).expect("房主离开");

    let room = registry.room(&room_id).expect("房间仍存在");
    assert_eq!(room.host_id, second.player_id, "房主转给最小座位号的玩家");
    assert_eq!(room.seat_of(&second.player_id), Some(1), "座位号不重排");
    assert!(
        registry.room_update(&room_id).unwrap().players[0].is_host,
        "视图里 0 号座位不再是房主，1 号座位是新房主"
    );
}

#[test]
fn leave_last_player_removes_room() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");

    let mut registry = RoomRegistry::with_seed(0);
    let room = registry.create(&host, &catalog());
    let room_id = room.id.clone();
    let code = room.code.clone();

    registry.leave(&room_id, &host.player_id).expect("离开成功");
    assert!(registry.room_update(&room_id).is_none(), "空房间应销毁");

    let newcomer = session(&mut store, &clock, "新玩家");
    assert_eq!(
        registry
            .join(&code, &newcomer, &catalog())
            .expect_err("房间码应失效"),
        RoomError::NotFound
    );
}

#[test]
fn leave_unknown_room_or_player_rejected() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");
    let outsider = session(&mut store, &clock, "路人");

    let mut registry = RoomRegistry::with_seed(0);
    let room = registry.create(&host, &catalog());
    let room_id = room.id.clone();

    assert_eq!(
        registry
            .leave("room_missing", &host.player_id)
            .expect_err("房间不存在"),
        RoomError::NotFound
    );
    assert_eq!(
        registry
            .leave(&room_id, &outsider.player_id)
            .expect_err("不在房间里"),
        RoomError::NotInRoom
    );
    let error = registry
        .leave(&room_id, &outsider.player_id)
        .expect_err("不在房间里");
    assert_eq!(error.code(), ErrorCode::NotInRoom);
}

#[test]
fn ready_unknown_player_rejected() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");
    let outsider = session(&mut store, &clock, "路人");

    let mut registry = RoomRegistry::with_seed(0);
    let room = registry.create(&host, &catalog());
    let room_id = room.id.clone();

    assert_eq!(
        registry
            .ready(&room_id, &outsider.player_id, false)
            .expect_err("不在房间里"),
        RoomError::NotInRoom
    );
}

#[test]
fn prepare_and_commit_advance_to_recruit() {
    let (mut registry, room_id, _code, players) = ready_room();
    assert_eq!(
        registry.room(&room_id).unwrap().phase,
        Phase::Lobby,
        "准备前应仍在大厅"
    );

    let plan = prepare(&registry, &room_id, &players[0].player_id);
    assert_eq!(
        registry.room(&room_id).unwrap().phase,
        Phase::Lobby,
        "prepare_start 只读，不改变阶段"
    );
    registry
        .commit_start(&room_id, plan.clone())
        .expect("8 人齐且全部准备时应能开局");

    assert_eq!(plan.slots.len(), SEAT_COUNT);
    assert_eq!(
        registry.room(&room_id).unwrap().phase,
        Phase::Recruit,
        "提交成功后应推进到 Recruit"
    );
}

#[test]
fn commit_start_twice_rejects_invalid_stage() {
    let (mut registry, room_id, _code, players) = ready_room();
    let plan = prepare(&registry, &room_id, &players[0].player_id);
    registry
        .commit_start(&room_id, plan.clone())
        .expect("首次提交成功");
    let seats_before: Vec<(u8, String)> = registry
        .room(&room_id)
        .expect("房间存在")
        .slots()
        .iter()
        .map(|(&seat, slot)| (seat, slot.player_id.clone()))
        .collect();

    assert_eq!(
        registry
            .commit_start(&room_id, plan)
            .expect_err("同一计划不得重复提交"),
        RoomError::WrongPhase
    );
    let room = registry.room(&room_id).expect("房间存在");
    assert_eq!(room.phase, Phase::Recruit, "重复提交不得回退阶段");
    let seats_after: Vec<(u8, String)> = room
        .slots()
        .iter()
        .map(|(&seat, slot)| (seat, slot.player_id.clone()))
        .collect();
    assert_eq!(seats_after, seats_before, "重复提交不得改变座位");

    assert_eq!(
        registry
            .prepare_start(&room_id, &players[0].player_id, &catalog())
            .expect_err("已开局的房间不能再准备"),
        RoomError::WrongPhase
    );
}

#[test]
fn commit_start_rejects_bot_conflicting_with_lobby_seat() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");
    let second = session(&mut store, &clock, "玩家2");
    let late = session(&mut store, &clock, "迟到玩家");
    let mut registry = RoomRegistry::with_seed(0);
    let room = registry.create(&host, &catalog());
    let room_id = room.id.clone();
    let code = room.code.clone();
    registry.join(&code, &second, &catalog()).expect("加入成功");
    for player in [&host, &second] {
        registry
            .ready(&room_id, &player.player_id, true)
            .expect("准备成功");
    }

    // 计划用机器人补 2..7；提交前又有一名真人加入，占据 2 号座位。
    let plan = prepare(&registry, &room_id, &host.player_id);
    assert!(plan.slots[2].is_bot, "2 号座位在计划中是机器人");
    registry
        .join(&code, &late, &catalog())
        .expect("大厅仍可加入");
    assert_eq!(
        registry
            .room(&room_id)
            .expect("房间存在")
            .seat_of(&late.player_id),
        Some(2)
    );

    assert_eq!(
        registry
            .commit_start(&room_id, plan)
            .expect_err("计划中的机器人座位已被真人占用"),
        RoomError::WrongPhase
    );
    let room = registry.room(&room_id).expect("房间存在");
    assert_eq!(room.phase, Phase::Lobby, "冲突提交不得推进阶段");
    assert_eq!(room.human_count(), 3, "不得写入部分机器人座位");
    assert!(room.candidates().is_empty(), "冲突提交不得写入候选");
}

#[test]
fn non_lobby_rejects_join_ready_and_leave() {
    let (mut registry, room_id, code, players) = started_room();
    let catalog = catalog();

    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let newcomer = session(&mut store, &clock, "迟到玩家");

    // 满座 + 已开局：阶段检查必须先于 ROOM_FULL。
    assert_eq!(
        registry
            .join(&code, &newcomer, &catalog)
            .expect_err("开局后不能加入"),
        RoomError::WrongPhase
    );
    assert_eq!(
        registry
            .ready(&room_id, &players[1].player_id, false)
            .expect_err("开局后不能改准备"),
        RoomError::WrongPhase
    );
    assert_eq!(
        registry
            .leave(&room_id, &players[1].player_id)
            .expect_err("开局后不能走大厅离开"),
        RoomError::WrongPhase
    );

    let room = registry.room(&room_id).expect("房间仍存在");
    assert_eq!(room.players().len(), SEAT_COUNT, "成员集合应保持不变");
    assert_eq!(
        room.seat_of(&players[1].player_id),
        Some(1),
        "座位不应被释放或重排"
    );
    assert_eq!(
        registry.room_update(&room_id).unwrap().players.len(),
        SEAT_COUNT
    );
    assert_eq!(room.phase, Phase::Recruit);
}

#[test]
fn candidates_are_limited_to_enabled_playstyles() {
    let (registry, room_id, _code, players) = ready_room();
    let room = registry.room(&room_id).expect("房间存在");
    let enabled: BTreeSet<Playstyle> = room.playstyles.iter().copied().collect();
    assert_eq!(enabled.len(), 5, "本局应启用 5 个不同玩法");

    let plan = prepare(&registry, &room_id, &players[0].player_id);
    let catalog = catalog();
    for seat in 0..SEAT_COUNT as u8 {
        for id in plan.candidates.get(&seat).expect("每个座位都应有候选") {
            let hero = catalog.heroes.get(id).expect("候选必须是卡表中的英雄");
            assert!(
                hero_in_pool(hero, &enabled),
                "候选 {id}（categories={:?}）不在本局启用的玩法内",
                hero.categories
            );
        }
    }
}

#[test]
fn every_playstyle_yields_non_empty_candidates() {
    // 13 个玩法都要至少在某个房间的启用集合中出现一次，且该房间的候选非空。
    // 真实英雄数据中真菌、细菌、原生生物没有直接归属的英雄；任意 5 个玩法
    // 的组合仍至少包含一个有英雄的玩法，因此候选始终非空。
    let catalog = catalog();
    let all: BTreeSet<Playstyle> = Playstyle::ALL.into_iter().collect();
    let mut seen = BTreeSet::new();
    for seed in 0..500 {
        let clock = FixedClock::new(1_000_000);
        let mut store = SessionStore::new();
        let mut registry = RoomRegistry::with_seed(seed);
        let host = session(&mut store, &clock, "房主");
        let room = registry.create(&host, &catalog);
        registry
            .ready(&room.id, &host.player_id, true)
            .expect("准备成功");
        let plan = registry
            .prepare_start(&room.id, &host.player_id, &catalog)
            .expect("单真人应能开局");
        for seat in 0..SEAT_COUNT as u8 {
            let candidates = plan.candidates.get(&seat).expect("每个座位都有候选");
            assert!(
                !candidates.is_empty(),
                "seed {seed}、玩法 {:?} 的座位 {seat} 候选不应为空",
                room.playstyles
            );
            let enabled: BTreeSet<Playstyle> = room.playstyles.iter().copied().collect();
            for id in candidates {
                let hero = catalog.heroes.get(id).expect("候选必须是真实英雄卡");
                assert!(
                    hero_in_pool(hero, &enabled),
                    "候选 {id} 必须命中本局启用玩法"
                );
            }
        }
        seen.extend(room.playstyles);
        if seen.len() == all.len() {
            break;
        }
    }
    assert_eq!(seen, all, "遍历房间种子应覆盖全部 13 个玩法");
}

#[test]
fn each_seat_gets_four_distinct_candidates_when_pool_is_large() {
    // 32 张英雄全部带 13 条 play:，候选池恰好 32 张，走满座分段。
    let catalog = catalog_with_generic_heroes(&[]);
    let (mut registry, room_id, _code, players) = lobby_room_with(catalog.clone());
    mark_all_ready(&mut registry, &room_id, &players);
    let plan = registry
        .prepare_start(&room_id, &players[0].player_id, &catalog)
        .expect("应能构造开局计划");

    let mut seen = BTreeSet::new();
    for seat in 0..SEAT_COUNT as u8 {
        let candidates = plan.candidates.get(&seat).expect("每个座位都应有候选");
        assert_eq!(candidates.len(), 4, "座位 {seat} 应取 4 张");
        for id in candidates {
            assert!(seen.insert(id.clone()), "池足够大时候选不得重复");
        }
    }
    assert_eq!(seen.len(), SEAT_COUNT * 4, "8 人 × 4 张应互不重复");
}

#[test]
fn narrow_pool_allows_duplicates_across_seats() {
    // 池只有 4 张（< 32）：每座位仍是 4 张，但允许跨座位重复。
    let heroes = ["adam-smith", "beethoven", "darwin", "davinci"];
    let catalog = catalog_with_generic_heroes(&heroes);
    let (mut registry, room_id, _code, players) = lobby_room_with(catalog.clone());
    mark_all_ready(&mut registry, &room_id, &players);
    let plan = registry
        .prepare_start(&room_id, &players[0].player_id, &catalog)
        .expect("应能构造开局计划");
    let expected: BTreeSet<String> = heroes.iter().map(|id| (*id).to_string()).collect();

    let mut seen = BTreeSet::new();
    let mut duplicated = false;
    for seat in 0..SEAT_COUNT as u8 {
        let candidates = plan.candidates.get(&seat).expect("每个座位都应有候选");
        assert_eq!(candidates.len(), 4, "每座位仍取 4 张");
        assert_eq!(
            candidates.iter().cloned().collect::<BTreeSet<_>>(),
            expected,
            "池恰好 4 张时每座位取整个池"
        );
        for id in candidates {
            if !seen.insert(id.clone()) {
                duplicated = true;
            }
        }
    }
    assert!(duplicated, "池不足满座需求时应允许跨座位重复");
}

#[test]
fn pool_smaller_than_candidate_count_takes_whole_pool() {
    // 池只有 2 张（< 4）：每个座位取整个池，至少 1 张。
    let heroes = ["mendel", "lisshi"];
    let catalog = catalog_with_generic_heroes(&heroes);
    let (mut registry, room_id, _code, players) = lobby_room_with(catalog.clone());
    mark_all_ready(&mut registry, &room_id, &players);
    let plan = registry
        .prepare_start(&room_id, &players[0].player_id, &catalog)
        .expect("应能构造开局计划");
    let expected: BTreeSet<String> = heroes.iter().map(|id| (*id).to_string()).collect();

    for seat in 0..SEAT_COUNT as u8 {
        let candidates = plan
            .candidates
            .get(&seat)
            .expect("每个座位都至少拿到 1 张候选");
        assert_eq!(candidates.len(), heroes.len(), "池小于候选上限时取整个池");
        assert!(!candidates.is_empty(), "候选至少 1 张");
        assert_eq!(
            candidates.iter().cloned().collect::<BTreeSet<_>>(),
            expected,
            "候选应覆盖整个小池"
        );
    }
}

#[test]
fn empty_filtered_pool_falls_back_to_all_heroes() {
    // 所有英雄都没有 `play:` 归属：`hero_in_pool` 不命中，过滤后池为空。
    let mut catalog = catalog();
    for hero in catalog.heroes.values_mut() {
        hero.categories.clear();
    }
    let all_heroes: BTreeSet<String> = catalog.heroes.keys().cloned().collect();

    let (mut registry, room_id, _code, players) = lobby_room_with(catalog.clone());
    mark_all_ready(&mut registry, &room_id, &players);
    let plan = registry
        .prepare_start(&room_id, &players[0].player_id, &catalog)
        .expect("应能构造开局计划");
    let mut seen = BTreeSet::new();
    for seat in 0..SEAT_COUNT as u8 {
        let candidates = plan
            .candidates
            .get(&seat)
            .expect("回退后每个座位都应有候选");
        assert_eq!(candidates.len(), 4, "回退到全部英雄后每座位仍取 4 张");
        for id in candidates {
            assert!(all_heroes.contains(id), "回退候选必须来自全部英雄卡");
            assert!(seen.insert(id.clone()), "回退到 32 张时应互不重复");
        }
    }
    assert_eq!(seen, all_heroes, "回退应覆盖全部英雄卡");
}

#[test]
fn same_seed_yields_same_candidates() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");

    let mut first = RoomRegistry::with_seed(0);
    let room = first.create(&host, &catalog());
    first
        .ready(&room.id, &host.player_id, true)
        .expect("准备成功");
    let first_plan = prepare(&first, &room.id, &host.player_id);

    let mut second = RoomRegistry::with_seed(0);
    let room = second.create(&host, &catalog());
    second
        .ready(&room.id, &host.player_id, true)
        .expect("准备成功");
    let second_plan = prepare(&second, &room.id, &host.player_id);

    let expected: BTreeMap<u8, Vec<String>> = (0..SEAT_COUNT as u8)
        .map(|seat| {
            (
                seat,
                first_plan
                    .candidates
                    .get(&seat)
                    .cloned()
                    .unwrap_or_default(),
            )
        })
        .collect();
    let actual: BTreeMap<u8, Vec<String>> = (0..SEAT_COUNT as u8)
        .map(|seat| {
            (
                seat,
                second_plan
                    .candidates
                    .get(&seat)
                    .cloned()
                    .unwrap_or_default(),
            )
        })
        .collect();
    assert_eq!(actual, expected, "同 seed 的候选分配必须恒定");
}

#[test]
fn detach_marks_disconnected_and_keeps_seat() {
    let (mut registry, room_id, _code, players) = lobby_room();
    let player = &players[1].player_id;

    registry
        .mark_disconnected(&room_id, player, 1_000_000)
        .expect("标记断线成功");

    let room = registry.room(&room_id).expect("房间存在");
    assert_eq!(room.seat_of(player), Some(1), "宽限期内座位必须保留");
    assert!(!room.slot(1).unwrap().connected, "座位应标记为离线");
    assert!(
        room.candidates().is_empty(),
        "等待区不持有候选，断线自然不影响候选"
    );
}

#[test]
fn reconnect_within_grace_keeps_seat_and_playstyles() {
    let (mut registry, room_id, _code, players) = lobby_room();
    let player = players[1].player_id.clone();
    let before_playstyles = registry.room(&room_id).expect("房间存在").playstyles;

    registry
        .mark_disconnected(&room_id, &player, 1_000_000)
        .expect("标记断线成功");
    // 宽限期内：未超时，座位不被回收。
    assert!(
        registry
            .tick_disconnects(&room_id, 1_000_000 + 29_000)
            .is_empty()
    );

    registry
        .mark_reconnected(&room_id, &player)
        .expect("重连成功");
    let room = registry.room(&room_id).expect("房间存在");
    assert_eq!(room.seat_of(&player), Some(1), "重连应回到原座位");
    assert!(room.slot(1).unwrap().connected, "重连后应标记为在线");
    assert_eq!(
        room.playstyles, before_playstyles,
        "重连不得重新抽取本局玩法"
    );
}

#[test]
fn grace_expiry_releases_seat() {
    let (mut registry, room_id, _code, players) = lobby_room();
    let player = players[1].player_id.clone();

    registry
        .mark_disconnected(&room_id, &player, 1_000_000)
        .expect("标记断线成功");
    assert!(
        registry
            .tick_disconnects(&room_id, 1_000_000 + 29_000)
            .is_empty()
    );

    let released = registry.tick_disconnects(&room_id, 1_000_000 + 30_000);
    assert_eq!(released, vec![player.clone()], "超时座位应被释放");
    assert_eq!(
        registry
            .room(&room_id)
            .and_then(|room| room.seat_of(&player)),
        None,
        "宽限超时后座位应释放"
    );
}

#[test]
fn disconnect_grace_skips_bots() {
    // 机器人只在 `commit_start` 时写入；把阶段拨回大厅以覆盖 `tick_disconnects`
    // 的机器人过滤分支（生产路径中机器人只存在于 `Recruit` 之后）。
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");
    let second = session(&mut store, &clock, "玩家2");
    let mut registry = RoomRegistry::with_seed(0);
    let room = registry.create(&host, &catalog());
    let room_id = room.id.clone();
    registry
        .join(&room.code, &second, &catalog())
        .expect("加入成功");
    for player in [&host, &second] {
        registry
            .ready(&room_id, &player.player_id, true)
            .expect("准备成功");
    }
    let plan = prepare(&registry, &room_id, &host.player_id);
    registry.commit_start(&room_id, plan).expect("提交开局成功");
    registry.room_mut(&room_id).expect("房间存在").phase = Phase::Lobby;

    let bot = registry
        .room(&room_id)
        .expect("房间存在")
        .slot(2)
        .expect("机器人座位")
        .player_id
        .clone();
    registry
        .mark_disconnected(&room_id, &bot, 1_000_000)
        .expect("标记机器人断线成功");
    registry
        .mark_disconnected(&room_id, &second.player_id, 1_000_000)
        .expect("标记真人断线成功");

    let released = registry.tick_disconnects(&room_id, 1_000_000 + 30_000);
    assert_eq!(
        released,
        vec![second.player_id.clone()],
        "宽限超时只释放真人座位"
    );
    let room = registry.room(&room_id).expect("房间仍存在");
    assert_eq!(room.seat_of(&bot), Some(2), "机器人不参与断线宽限");
    assert_eq!(room.seat_of(&second.player_id), None, "真人座位应释放");
}

#[test]
fn host_kicks_member() {
    let (mut registry, room_id, code, players) = lobby_room();
    let target = players[2].player_id.clone();

    registry
        .kick(&room_id, &players[0].player_id, &target)
        .expect("房主移除成员成功");

    let room = registry.room(&room_id).expect("房间仍存在");
    assert_eq!(room.players().len(), SEAT_COUNT - 1, "成员数应减 1");
    assert_eq!(room.code, code, "房间码不变");
    assert_eq!(room.host_id, players[0].player_id, "房主不变");
    assert_eq!(room.seat_of(&target), None, "被移除者的座位应释放");
    assert!(
        registry.room_of_player(&target).is_none(),
        "按玩家查询应返回 None"
    );
    assert_eq!(
        room.seat_of(&players[1].player_id),
        Some(1),
        "其余座位不重排"
    );
    assert_eq!(
        room.seat_of(&players[3].player_id),
        Some(3),
        "被移除座位之后的座位号也不重排"
    );
}

#[test]
fn kick_rejects_invalid_actor_target_and_phase() {
    let (mut registry, room_id, _code, players) = lobby_room();

    let error = registry
        .kick(&room_id, &players[1].player_id, &players[2].player_id)
        .expect_err("非房主不能移除成员");
    assert_eq!(error, RoomError::NotHost);
    assert_eq!(error.code(), ErrorCode::NotHost);

    let error = registry
        .kick(&room_id, &players[0].player_id, &players[0].player_id)
        .expect_err("房主不能移除自己");
    assert_eq!(error, RoomError::InvalidTarget);
    assert_eq!(error.code(), ErrorCode::InvalidChoice);
    assert_eq!(error.to_string(), "不能移除自己");

    let mut store = SessionStore::new();
    let outsider = session(&mut store, &FixedClock::new(1_000_000), "路人");
    assert_eq!(
        registry
            .kick(&room_id, &players[0].player_id, &outsider.player_id)
            .expect_err("目标不在房间"),
        RoomError::NotInRoom
    );

    let (mut started, started_room_id, _started_code, started_players) = started_room();
    assert_eq!(
        started
            .kick(
                &started_room_id,
                &started_players[0].player_id,
                &started_players[1].player_id,
            )
            .expect_err("开局后不能移除"),
        RoomError::WrongPhase
    );
}

#[test]
fn host_dissolves_room() {
    let (mut registry, room_id, code, players) = lobby_room();
    let expected: Vec<String> = players
        .iter()
        .map(|player| player.player_id.clone())
        .collect();

    let members = registry
        .dissolve(&room_id, &players[0].player_id)
        .expect("房主解散成功");

    assert_eq!(members, expected, "应返回全部成员，按座位升序");
    assert!(registry.room(&room_id).is_none(), "注册表中房间应删除");

    let mut store = SessionStore::new();
    let newcomer = session(&mut store, &FixedClock::new(1_000_000), "新玩家");
    assert_eq!(
        registry
            .join(&code, &newcomer, &catalog())
            .expect_err("解散后房间码应失效"),
        RoomError::NotFound
    );
}

#[test]
fn non_host_dissolve_rejected() {
    let (mut registry, room_id, _code, players) = lobby_room();

    let error = registry
        .dissolve(&room_id, &players[1].player_id)
        .expect_err("非房主不能解散房间");
    assert_eq!(error, RoomError::NotHost);
    assert_eq!(error.code(), ErrorCode::NotHost);
    assert!(registry.room(&room_id).is_some(), "拒绝解散后房间应保留");
}

// ---------------------------------------------------------------------------
// 真实 WebSocket：建房、邀请、房主移除与解散。
// ---------------------------------------------------------------------------

/// 测试用最小 WebSocket 客户端；只解析下行信封与消息类型。
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
            "id": format!("room_ws_{}", self.next_id),
            "type": kind,
            "ts": "2026-01-01T00:00:00Z",
            "payload": payload,
        });
        self.ws
            .send(Message::Text(envelope.to_string().into()))
            .await
            .expect("发送应当成功");
    }

    /// 取下一条下行消息；跳过认证与在线状态变化产生的 `FriendUpdate`。
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

    /// 等到一条 `RoomUpdate`。
    async fn recv_room_update(&mut self) -> RoomUpdate {
        loop {
            if let ServerMessage::RoomUpdate(update) = self.recv_room().await {
                return update;
            }
        }
    }

    /// 等到一条 `RoomClosed`。
    async fn recv_room_closed(&mut self) -> RoomClosedReason {
        loop {
            if let ServerMessage::RoomClosed(closed) = self.recv_room().await {
                return closed.reason;
            }
        }
    }

    /// 等到一条 `Error`，返回协议错误码。
    async fn recv_error(&mut self) -> ErrorCode {
        loop {
            if let ServerMessage::Error(error) = self.recv_room().await {
                return error.code;
            }
        }
    }
}

/// 由名字派生稳定且互不相同的手机号：同一手机号会恢复同一账号。
fn phone_for(name: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in name.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("139{:08}", hash % 100_000_000)
}

/// 认证并返回 `playerId`；token 只用于同一测试内可能的重连。
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

/// 建房并返回 `(房间码, 房主 playerId)`。
async fn create_room_ws(client: &mut WsClient, name: &str) -> (String, String) {
    let player_id = authenticate_ws(client, name).await;
    client.send("CreateRoom", json!({})).await;
    let room = client.recv_room_update().await;
    (room.code, player_id)
}

/// 创建房间邀请 token，返回 `(token, 房间码)`。
async fn create_invite_ws(client: &mut WsClient) -> (String, String) {
    client.send("CreateRoomInvite", json!({})).await;
    loop {
        if let ServerMessage::RoomInviteCode(invite) = client.recv_room().await {
            return (invite.token, invite.room_code);
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invite_token_joins_room_once() {
    let addr = common::start_server().await;
    let mut host = WsClient::connect(addr).await;
    let (code, _host_id) = create_room_ws(&mut host, "invite-host").await;

    let (token, invite_code) = create_invite_ws(&mut host).await;
    assert_eq!(invite_code, code, "token 应指向当前房间");

    let mut guest = WsClient::connect(addr).await;
    authenticate_ws(&mut guest, "invite-guest").await;
    guest
        .send("RedeemRoomInvite", json!({ "token": token }))
        .await;
    let joined = guest.recv_room_update().await;
    assert_eq!(joined.players.len(), 2, "被邀请者应入房");
    assert_eq!(joined.code, code);

    // token 一次性：其他玩家再消费返回 INVITE_NOT_FOUND。
    let mut third = WsClient::connect(addr).await;
    authenticate_ws(&mut third, "invite-third").await;
    third
        .send("RedeemRoomInvite", json!({ "token": token }))
        .await;
    assert_eq!(third.recv_error().await, ErrorCode::InviteNotFound);

    // 已在房间的连接再消费先命中 ALREADY_IN_ROOM。
    host.send("RedeemRoomInvite", json!({ "token": token }))
        .await;
    assert_eq!(host.recv_error().await, ErrorCode::AlreadyInRoom);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn join_room_twice_reports_already_in_room() {
    let addr = common::start_server().await;
    let mut host = WsClient::connect(addr).await;
    let (code, _host_id) = create_room_ws(&mut host, "dup-host").await;

    let mut guest = WsClient::connect(addr).await;
    authenticate_ws(&mut guest, "dup-guest").await;
    guest.send("JoinRoom", json!({ "code": code })).await;
    guest.recv_room_update().await;
    guest.send("JoinRoom", json!({ "code": code })).await;
    assert_eq!(guest.recv_error().await, ErrorCode::AlreadyInRoom);
}

#[test]
fn invite_token_expires_and_requires_lobby() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let mut registry = RoomRegistry::with_seed(0);
    let host = session(&mut store, &clock, "invite-expiry-host");
    let room = registry.create(&host, &catalog());
    let room_id = room.id.clone();

    let invite = registry
        .create_invite(&host.player_id, clock.now_ms())
        .expect("大厅内应能创建邀请");
    let guest = session(&mut store, &clock, "invite-expiry-guest");
    let expired = registry
        .redeem_invite(
            &invite.token,
            &guest,
            &catalog(),
            clock.now_ms() + ROOM_INVITE_TTL_MS,
        )
        .expect_err("过期 token 必须拒绝");
    assert!(matches!(expired, RoomError::InviteExpired));

    let fresh = registry
        .create_invite(&host.player_id, clock.now_ms())
        .expect("大厅内应能创建邀请");
    registry
        .ready(&room_id, &host.player_id, true)
        .expect("准备成功");
    let plan = registry
        .prepare_start(&room_id, &host.player_id, &catalog())
        .expect("单真人应能开局");
    registry
        .commit_start(&room_id, plan)
        .expect("应能进入开局阶段");
    let wrong_phase = registry
        .redeem_invite(&fresh.token, &guest, &catalog(), clock.now_ms())
        .expect_err("开局后 token 不能入房");
    assert!(matches!(wrong_phase, RoomError::WrongPhase));
    assert!(
        registry
            .create_invite(&host.player_id, clock.now_ms())
            .is_err(),
        "开局后不能再创建邀请"
    );

    let outsider = session(&mut store, &clock, "invite-outsider");
    let not_in_room = registry
        .create_invite(&outsider.player_id, clock.now_ms())
        .expect_err("不在房间不能创建邀请");
    assert!(matches!(not_in_room, RoomError::NotInRoom));
}

#[test]
fn invite_token_requires_free_seat() {
    let clock = FixedClock::new(2_000_000);
    let mut store = SessionStore::new();
    let mut registry = RoomRegistry::with_seed(0);
    let host = session(&mut store, &clock, "invite-full-host");
    let room = registry.create(&host, &catalog());
    for index in 1..SEAT_COUNT {
        let member = session(&mut store, &clock, &format!("invite-full-{index}"));
        registry
            .join(&room.code, &member, &catalog())
            .expect("应能补满座位");
    }
    let invite = registry
        .create_invite(&host.player_id, clock.now_ms())
        .expect("大厅内应能创建邀请");
    let extra = session(&mut store, &clock, "invite-full-extra");
    let full = registry
        .redeem_invite(&invite.token, &extra, &catalog(), clock.now_ms())
        .expect_err("满座必须拒绝");
    assert!(matches!(full, RoomError::Full));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn host_kicks_member_and_non_host_is_rejected_over_real_ws() {
    let addr = common::start_server().await;
    let mut host = WsClient::connect(addr).await;
    let (code, host_id) = create_room_ws(&mut host, "ws-kick-host").await;

    let mut guest = WsClient::connect(addr).await;
    let guest_id = authenticate_ws(&mut guest, "ws-kick-guest").await;
    guest.send("JoinRoom", json!({ "code": code })).await;
    let joined = guest.recv_room_update().await;
    assert_eq!(joined.players.len(), 2, "成员应在房间里");

    // 非房主移除成员必须被拒绝。
    guest
        .send("KickMember", json!({ "targetPlayerId": host_id }))
        .await;
    assert_eq!(
        guest.recv_error().await,
        ErrorCode::NotHost,
        "非房主应收到 NOT_HOST"
    );

    // 房主移除成员：目标收到 RoomClosed(kicked)，其余玩家收到不含目标的 RoomUpdate。
    host.send("KickMember", json!({ "targetPlayerId": guest_id }))
        .await;
    assert_eq!(
        guest.recv_room_closed().await,
        RoomClosedReason::Kicked,
        "被移除者应收到 kicked"
    );
    let after = host.recv_room_update().await;
    assert_eq!(after.players.len(), 1, "移除后只剩房主");
    assert_eq!(after.players[0].player_id, host_id);
    assert!(after.players[0].is_host, "房主身份不变");
    println!("kick ok: players={}", after.players.len());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn host_dissolve_notifies_members_and_invalidates_code_over_real_ws() {
    let addr = common::start_server().await;
    let mut host = WsClient::connect(addr).await;
    let (code, _host_id) = create_room_ws(&mut host, "ws-dissolve-host").await;

    let mut guest = WsClient::connect(addr).await;
    let _guest_id = authenticate_ws(&mut guest, "ws-dissolve-guest").await;
    guest.send("JoinRoom", json!({ "code": code })).await;
    let joined = guest.recv_room_update().await;
    assert_eq!(joined.players.len(), 2);

    host.send("DissolveRoom", json!({})).await;
    assert_eq!(
        host.recv_room_closed().await,
        RoomClosedReason::Dissolved,
        "房主应收到 dissolved"
    );
    assert_eq!(
        guest.recv_room_closed().await,
        RoomClosedReason::Dissolved,
        "成员应收到 dissolved"
    );

    // 房间码必须已失效。
    let mut late = WsClient::connect(addr).await;
    let _ = authenticate_ws(&mut late, "ws-dissolve-late").await;
    late.send("JoinRoom", json!({ "code": code })).await;
    assert_eq!(
        late.recv_error().await,
        ErrorCode::RoomNotFound,
        "解散后旧房间码应返回 ROOM_NOT_FOUND"
    );
    println!("dissolve ok: code={code} invalidated");
}

#[test]
fn create_room_is_not_ranked() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");
    let mut registry = RoomRegistry::with_seed(0);
    let room = registry.create(&host, &catalog());
    assert!(!room.ranked, "普通房间局不计入天梯");
    assert_eq!(room.phase, Phase::Lobby, "房间局仍从大厅开始");
}

#[test]
fn create_match_fills_eight_seats_and_advances_to_recruit() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let players: Vec<Session> = (0..SEAT_COUNT)
        .map(|index| session(&mut store, &clock, &format!("匹配{index}")))
        .collect();

    let mut registry = RoomRegistry::with_seed(0);
    let room = registry
        .create_match(&players, &catalog())
        .expect("8 名玩家应能直接建立匹配房");

    assert!(room.ranked, "匹配局必须标记 ranked");
    assert_eq!(room.phase, Phase::Recruit, "匹配房跳过大厅直接进入招募");
    assert_eq!(room.slots().len(), SEAT_COUNT, "匹配房应满座");
    assert_eq!(room.host_id, players[0].player_id, "房主记为第一人");
    assert!(!room.bot_fill, "匹配房全部真人，不开启补位");
    assert_eq!(room.candidates().len(), SEAT_COUNT, "匹配房应写入候选");
    assert!(
        registry.room(&room.id).is_some_and(|stored| stored.ranked),
        "注册表持有同一匹配房"
    );

    for (index, player) in players.iter().enumerate() {
        let seat = room
            .seat_of(&player.player_id)
            .unwrap_or_else(|| panic!("{} 应占用一个座位", player.player_id));
        assert_eq!(usize::from(seat), index, "座位按入队顺序分配");
        let slot = room.slot(seat).expect("座位上的玩家");
        assert!(slot.ready, "匹配局座位视为已准备");
        assert!(!slot.is_bot, "匹配房没有机器人");
        assert!(
            room.candidates()
                .get(&seat)
                .is_some_and(|list| !list.is_empty()),
            "座位 {seat} 应写入候选"
        );
    }
}
