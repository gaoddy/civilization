//! 房间状态机与大堂流程测试。
//!
//! 覆盖建房、加入/离开、英雄选择、准备与开局前置判定，以及 `RoomUpdate` 视图。
//! 房间码与座位分配均为确定性行为，测试直接断言具体取值。
//! 文末另有真实 WebSocket 的房主移除/解散用例。

mod common;

use std::collections::BTreeSet;

use futures_util::{SinkExt, StreamExt};
use protocol::ErrorCode;
use protocol::client::AuthRequest;
use protocol::server::RoomUpdate;
use protocol::{RoomClosedReason, ServerEnvelope, ServerMessage};
use room_service::auth::{Session, SessionStore};
use room_service::clock::FixedClock;
use room_service::room::{RoomError, RoomRegistry, SEAT_COUNT};
use rule_engine::data::{Card, Catalog, Playstyle, Specialty, SpecialtyKind, card_in_pool};
use rule_engine::state::Phase;
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message;

const CARDS_JSON: &str = include_str!("../../../data/v0.1/cards.json");
const SKILLS_JSON: &str = include_str!("../../../data/v0.1/skills.json");
const TOKENS_JSON: &str = include_str!("../../../data/v0.1/tokens.json");
const CONFIG_JSON: &str = include_str!("../../../data/v0.1/config.json");
const TAXONOMY_JSON: &str = include_str!("../../../data/v0.1/taxonomy.json");

/// 真实卡表；英雄卡用 `adam-smith`，非英雄卡用 `agi`。
fn catalog() -> Catalog {
    Catalog::from_json(
        CARDS_JSON,
        SKILLS_JSON,
        TOKENS_JSON,
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

/// 玩家候选里的第一张，用于在「只能从本人候选中选」的规则下完成选英雄。
fn first_candidate(registry: &RoomRegistry, room_id: &str, player_id: &str) -> String {
    registry
        .candidates_of(room_id, player_id)
        .and_then(|list| list.into_iter().next())
        .unwrap_or_else(|| panic!("{player_id} 应有候选"))
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

/// 只保留指定英雄、并清空玩法关联；中立且无 `play:` 的英雄始终在候选池内。
fn catalog_with_generic_heroes(hero_ids: &[&str]) -> Catalog {
    let mut catalog = catalog();
    let keep_all = hero_ids.is_empty();
    catalog
        .cards
        .retain(|_, card| !card.is_hero || keep_all || hero_ids.contains(&card.id.as_str()));
    for card in catalog.cards.values_mut().filter(|card| card.is_hero) {
        make_generic_hero(card);
    }
    catalog
}

/// 把英雄改成「中立且无玩法关联」，`card_in_pool` 恒为真。
fn make_generic_hero(card: &mut Card) {
    card.playstyle = Playstyle::Neutral;
    card.specialty.clear();
}

/// 在满座大厅里让所有人选好不同英雄并准备，但仍停留在 `Lobby`。
fn ready_room() -> (RoomRegistry, String, String, Vec<Session>) {
    let (mut registry, room_id, code, players) = lobby_room();
    let catalog = catalog();
    for player in &players {
        let hero = first_candidate(&registry, &room_id, &player.player_id);
        registry
            .select_hero(&room_id, &player.player_id, &hero, &catalog)
            .expect("选英雄成功");
        registry
            .ready(&room_id, &player.player_id, true)
            .expect("准备成功");
    }
    (registry, room_id, code, players)
}

/// 已完成 `try_start`、阶段推进到 `Recruit` 的房间。
fn started_room() -> (RoomRegistry, String, String, Vec<Session>) {
    let (mut registry, room_id, code, players) = ready_room();
    registry
        .try_start(&room_id, &players[0].player_id)
        .expect("全员就绪后应能开局");
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
fn select_hero_accepts_duplicate_card() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");
    let second = session(&mut store, &clock, "玩家2");
    let catalog = catalog();

    let mut registry = RoomRegistry::with_seed(0);
    let room = registry.create(&host, &catalog);
    let room_id = room.id.clone();
    let code = room.code.clone();
    registry.join(&code, &second, &catalog).expect("加入成功");

    let host_hero = first_candidate(&registry, &room_id, &host.player_id);
    let second_hero = first_candidate(&registry, &room_id, &second.player_id);
    registry
        .select_hero(&room_id, &host.player_id, &host_hero, &catalog)
        .expect("房主选英雄成功");
    registry
        .select_hero(&room_id, &second.player_id, &second_hero, &catalog)
        .expect("第二名玩家从自己的候选中选英雄成功");

    let room = registry.room(&room_id).expect("房间存在");
    assert_eq!(
        room.slot(0).unwrap().hero_card_id.as_deref(),
        Some(host_hero.as_str())
    );
    assert_eq!(
        room.slot(1).unwrap().hero_card_id.as_deref(),
        Some(second_hero.as_str())
    );
}

#[test]
fn select_hero_rejects_non_hero() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");
    let catalog = catalog();

    let mut registry = RoomRegistry::with_seed(0);
    let room = registry.create(&host, &catalog);
    let room_id = room.id.clone();

    let error = registry
        .select_hero(&room_id, &host.player_id, "agi", &catalog)
        .expect_err("非英雄卡不能作为英雄");
    assert_eq!(error, RoomError::NotAHeroCard);
    assert_eq!(error.code(), ErrorCode::InvalidChoice);
    assert_eq!(
        registry
            .select_hero(&room_id, &host.player_id, "no-such-card", &catalog)
            .expect_err("不存在的卡不能作为英雄"),
        RoomError::NotAHeroCard
    );
}

#[test]
fn ready_without_hero_rejected() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");
    let catalog = catalog();

    let mut registry = RoomRegistry::with_seed(0);
    let room = registry.create(&host, &catalog);
    let room_id = room.id.clone();

    // 取消准备始终允许。
    registry
        .ready(&room_id, &host.player_id, false)
        .expect("取消准备不要求已选英雄");

    let error = registry
        .ready(&room_id, &host.player_id, true)
        .expect_err("未选英雄就准备应被拒绝");
    assert_eq!(error, RoomError::HeroNotSelected);
    assert_eq!(error.code(), ErrorCode::HeroNotSelected);

    // 选完英雄后即可准备。
    let hero = first_candidate(&registry, &room_id, &host.player_id);
    registry
        .select_hero(&room_id, &host.player_id, &hero, &catalog)
        .expect("选英雄成功");
    registry
        .ready(&room_id, &host.player_id, true)
        .expect("已选英雄后应能准备");
}

#[test]
fn try_start_by_non_host_rejected() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");
    let second = session(&mut store, &clock, "玩家2");
    let catalog = catalog();

    let mut registry = RoomRegistry::with_seed(0);
    let room = registry.create(&host, &catalog);
    let room_id = room.id.clone();
    let code = room.code.clone();
    registry.join(&code, &second, &catalog).expect("加入成功");
    let hero = first_candidate(&registry, &room_id, &second.player_id);
    registry
        .select_hero(&room_id, &second.player_id, &hero, &catalog)
        .expect("选英雄成功");

    let error = registry
        .try_start(&room_id, &second.player_id)
        .expect_err("非房主不能开始");
    assert_eq!(error, RoomError::NotHost);
    assert_eq!(error.code(), ErrorCode::NotHost);
}

#[test]
fn try_start_with_missing_players_rejected() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");
    let catalog = catalog();

    let mut registry = RoomRegistry::with_seed(0);
    let room = registry.create(&host, &catalog);
    let room_id = room.id.clone();
    let hero = first_candidate(&registry, &room_id, &host.player_id);
    registry
        .select_hero(&room_id, &host.player_id, &hero, &catalog)
        .expect("选英雄成功");
    registry
        .ready(&room_id, &host.player_id, true)
        .expect("准备成功");

    let error = registry
        .try_start(&room_id, &host.player_id)
        .expect_err("人数不足不能开始");
    assert_eq!(error, RoomError::RoomNotReady);
    assert_eq!(error.code(), ErrorCode::RoomNotReady);
    assert_eq!(
        registry.room(&room_id).unwrap().phase,
        Phase::Lobby,
        "开始失败不得改变阶段"
    );
}

#[test]
fn start_ready_requires_all_ready() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");
    let catalog = catalog();

    let mut registry = RoomRegistry::with_seed(0);
    let room = registry.create(&host, &catalog);
    let room_id = room.id.clone();
    let code = room.code.clone();

    let mut players = vec![host];
    for index in 1..SEAT_COUNT {
        let player = session(&mut store, &clock, &format!("玩家{index}"));
        registry.join(&code, &player, &catalog).expect("加入成功");
        players.push(player);
    }

    // 8 人齐、各自从本人候选里选好英雄，但只准备 7 人。
    for player in &players {
        let hero = first_candidate(&registry, &room_id, &player.player_id);
        registry
            .select_hero(&room_id, &player.player_id, &hero, &catalog)
            .expect("选英雄成功");
    }
    for player in players.iter().take(SEAT_COUNT - 1) {
        registry
            .ready(&room_id, &player.player_id, true)
            .expect("准备成功");
    }
    assert!(
        registry.start_ready(&room_id).is_none(),
        "仍有人未准备时不可开局"
    );

    registry
        .ready(&room_id, &players[SEAT_COUNT - 1].player_id, true)
        .expect("最后一人准备成功");
    let heroes = registry.start_ready(&room_id).expect("全员就绪后应可开局");
    assert_eq!(heroes.len(), SEAT_COUNT);
    for (seat, player) in players.iter().enumerate() {
        let expected = registry
            .candidates_of(&room_id, &player.player_id)
            .and_then(|list| list.into_iter().next())
            .expect("候选存在");
        assert_eq!(
            heroes.get(&(seat as u8)),
            Some(&expected),
            "座位 {seat} 的英雄应取自本人候选"
        );
    }

    // `start_ready` 是纯查询，不改变阶段。
    assert_eq!(
        registry.room(&room_id).unwrap().phase,
        Phase::Lobby,
        "start_ready 不应改变房间状态"
    );
}

#[test]
fn room_update_lists_players_and_host() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");
    let second = session(&mut store, &clock, "玩家2");
    let catalog = catalog();

    let mut registry = RoomRegistry::with_seed(0);
    let room = registry.create(&host, &catalog);
    let room_id = room.id.clone();
    let code = room.code.clone();
    registry.join(&code, &second, &catalog).expect("加入成功");
    let hero = first_candidate(&registry, &room_id, &second.player_id);
    registry
        .select_hero(&room_id, &second.player_id, &hero, &catalog)
        .expect("选英雄成功");

    let update = registry.room_update(&room_id).expect("房间存在");
    assert_eq!(update.room_id, room_id);
    assert_eq!(update.code, code);
    assert_eq!(update.host_id, host.player_id);
    assert_eq!(update.phase, protocol::view::Phase::Lobby);
    assert_eq!(update.players.len(), 2);

    assert_eq!(update.players[0].player_id, host.player_id);
    assert_eq!(update.players[0].display_name, "房主");
    assert!(update.players[0].is_host);
    assert!(!update.players[0].ready);
    assert_eq!(update.players[0].hero_card_id, None);

    assert_eq!(update.players[1].player_id, second.player_id);
    assert_eq!(update.players[1].display_name, "玩家2");
    assert!(!update.players[1].is_host);
    assert_eq!(
        update.players[1].hero_card_id.as_deref(),
        Some(hero.as_str())
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
fn try_start_success_advances_to_recruit() {
    let (mut registry, room_id, _code, players) = ready_room();
    assert_eq!(
        registry.room(&room_id).unwrap().phase,
        Phase::Lobby,
        "try_start 前应仍在大厅"
    );

    let heroes = registry
        .try_start(&room_id, &players[0].player_id)
        .expect("8 人齐且全部准备时应能开局");

    assert_eq!(heroes.len(), SEAT_COUNT);
    for (seat, player) in players.iter().enumerate() {
        let selected = registry
            .room(&room_id)
            .and_then(|room| {
                room.slot(seat as u8)
                    .and_then(|slot| slot.hero_card_id.clone())
            })
            .unwrap_or_else(|| panic!("座位 {seat} 已选英雄"));
        assert_eq!(
            heroes.get(&(seat as u8)),
            Some(&selected),
            "座位 {seat} 的英雄应与所选一致"
        );
        assert!(
            registry
                .candidates_of(&room_id, &player.player_id)
                .is_some_and(|list| list.contains(&selected)),
            "座位 {seat} 的英雄必须来自本人候选"
        );
    }
    assert_eq!(
        registry.room(&room_id).unwrap().phase,
        Phase::Recruit,
        "开局成功后应推进到 Recruit"
    );
}

#[test]
fn try_start_twice_rejects_invalid_stage() {
    let (mut registry, room_id, _code, players) = started_room();

    let error = registry
        .try_start(&room_id, &players[0].player_id)
        .expect_err("已开局的房间不能再次开始");
    assert_eq!(error, RoomError::WrongPhase);
    assert_eq!(error.code(), ErrorCode::InvalidStage);
    assert_eq!(
        registry.room(&room_id).unwrap().phase,
        Phase::Recruit,
        "重复开始不得回退或重复推进阶段"
    );
}

#[test]
fn non_lobby_rejects_join_ready_select_and_leave() {
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
    let other_hero = registry
        .candidates_of(&room_id, &players[1].player_id)
        .and_then(|list| list.into_iter().last())
        .expect("候选存在");
    assert_eq!(
        registry
            .select_hero(&room_id, &players[1].player_id, &other_hero, &catalog)
            .expect_err("开局后不能改英雄"),
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
fn start_ready_is_false_after_game_started() {
    let (registry, room_id, _code, _players) = started_room();

    assert!(
        registry.start_ready(&room_id).is_none(),
        "开局后注册表查询不应再报告可以开始"
    );
    assert!(
        registry.room(&room_id).unwrap().start_ready().is_none(),
        "Room::start_ready 在非 Lobby 阶段应返回 None"
    );
}

#[test]
fn candidates_are_limited_to_enabled_playstyles() {
    let (registry, room_id, _code, players) = lobby_room();
    let room = registry.room(&room_id).expect("房间存在");
    let enabled: BTreeSet<Playstyle> = room.playstyles.iter().copied().collect();
    assert_eq!(enabled.len(), 5, "本局应启用 5 个不同玩法");

    let catalog = catalog();
    for player in &players {
        for id in registry
            .candidates_of(&room_id, &player.player_id)
            .expect("每个座位都应有候选")
        {
            let card = catalog.cards.get(&id).expect("候选必须是卡表中的英雄");
            assert!(card.is_hero, "候选 {id} 必须是英雄卡");
            assert!(
                card_in_pool(card, &enabled),
                "候选 {id}（playstyle={:?}）不在本局启用的玩法内",
                card.playstyle
            );
        }
    }
}

#[test]
fn each_seat_gets_four_distinct_candidates_when_pool_is_large() {
    // 32 张英雄全部中立且无 play:，候选池恰好 32 张，走满座分段。
    let (registry, room_id, _code, players) = lobby_room_with(catalog_with_generic_heroes(&[]));
    let mut seen = BTreeSet::new();
    for player in &players {
        let candidates = registry
            .candidates_of(&room_id, &player.player_id)
            .expect("每个座位都应有候选");
        assert_eq!(candidates.len(), 4, "{}", player.player_id);
        for id in candidates {
            assert!(seen.insert(id), "池足够大时候选不得重复");
        }
    }
    assert_eq!(seen.len(), SEAT_COUNT * 4, "8 人 × 4 张应互不重复");
}

#[test]
fn narrow_pool_allows_duplicates_across_seats() {
    // 池只有 4 张（< 32）：每人仍是 4 张，但允许跨玩家重复。
    let heroes = ["adam-smith", "beethoven", "darwin", "davinci"];
    let (registry, room_id, _code, players) = lobby_room_with(catalog_with_generic_heroes(&heroes));
    let expected: BTreeSet<String> = heroes.iter().map(|id| (*id).to_string()).collect();

    let mut seen = BTreeSet::new();
    let mut duplicated = false;
    for player in &players {
        let candidates = registry
            .candidates_of(&room_id, &player.player_id)
            .expect("每个座位都应有候选");
        assert_eq!(candidates.len(), 4, "每人仍取 4 张");
        assert_eq!(
            candidates.iter().cloned().collect::<BTreeSet<_>>(),
            expected,
            "池恰好 4 张时每人取整个池"
        );
        for id in candidates {
            if !seen.insert(id) {
                duplicated = true;
            }
        }
    }
    assert!(duplicated, "池不足满座需求时应允许跨玩家重复");
}

#[test]
fn pool_smaller_than_candidate_count_takes_whole_pool() {
    // 池只有 2 张（< 4）：每人取整个池，至少 1 张。
    let heroes = ["mendel", "lisshi"];
    let (registry, room_id, _code, players) = lobby_room_with(catalog_with_generic_heroes(&heroes));
    let expected: BTreeSet<String> = heroes.iter().map(|id| (*id).to_string()).collect();

    for player in &players {
        let candidates = registry
            .candidates_of(&room_id, &player.player_id)
            .expect("每个座位都至少拿到 1 张候选");
        assert_eq!(candidates.len(), heroes.len(), "池小于人数上限时取整个池");
        assert!(!candidates.is_empty(), "候选至少 1 张");
        assert_eq!(
            candidates.into_iter().collect::<BTreeSet<_>>(),
            expected,
            "候选应覆盖整个小池"
        );
    }
}

#[test]
fn empty_filtered_pool_falls_back_to_all_heroes() {
    // 所有英雄都带 `play:neutral`：`card_in_pool` 不命中，过滤后池为空。
    let mut catalog = catalog();
    for card in catalog.cards.values_mut().filter(|card| card.is_hero) {
        card.playstyle = Playstyle::Neutral;
        card.specialty = vec![Specialty {
            kind: SpecialtyKind::Play,
            value: "neutral".to_string(),
        }];
    }
    let all_heroes: BTreeSet<String> = catalog
        .cards
        .values()
        .filter(|card| card.is_hero)
        .map(|card| card.id.clone())
        .collect();

    let (registry, room_id, _code, players) = lobby_room_with(catalog);
    let mut seen = BTreeSet::new();
    for player in &players {
        let candidates = registry
            .candidates_of(&room_id, &player.player_id)
            .expect("回退后每人都应有候选");
        assert_eq!(candidates.len(), 4, "回退到全部英雄后每人仍取 4 张");
        for id in candidates {
            assert!(all_heroes.contains(&id), "回退候选必须来自全部英雄卡");
            assert!(seen.insert(id), "回退到 32 张时应互不重复");
        }
    }
    assert_eq!(seen, all_heroes, "回退应覆盖全部英雄卡");
}

#[test]
fn released_seat_candidates_come_back_to_the_pool() {
    let (mut registry, room_id, _code, players) = lobby_room();
    let catalog = catalog();
    let leaving = players[1].player_id.clone();
    let freed = registry.candidates_of(&room_id, &leaving).expect("有候选");

    registry.leave(&room_id, &leaving).expect("离开成功");
    let newcomer = session(
        &mut SessionStore::new(),
        &FixedClock::new(1_000_000),
        "新玩家",
    );
    let code = registry.room(&room_id).expect("房间存在").code.clone();
    registry
        .join(&code, &newcomer, &catalog)
        .expect("加入空位成功");

    let reclaimed = registry
        .candidates_of(&room_id, &newcomer.player_id)
        .expect("新玩家应有候选");
    assert_eq!(reclaimed.len(), 4, "空出的段应完整交还给下一名玩家");
    assert_eq!(reclaimed, freed, "回收的段应与离开玩家原本的候选一致");
}

#[test]
fn candidates_are_only_visible_to_their_owner() {
    let (registry, room_id, _code, players) = ready_room();
    let viewer = &players[0].player_id;
    let update = registry
        .room_update_for(&room_id, viewer)
        .expect("房间存在");

    let own = update
        .players
        .iter()
        .find(|item| &item.player_id == viewer)
        .expect("本人应在列表里");
    assert_eq!(
        own.hero_candidates.as_ref().map(Vec::len),
        Some(4),
        "本人应收到 4 张候选"
    );

    for item in update
        .players
        .iter()
        .filter(|item| &item.player_id != viewer)
    {
        assert_eq!(
            item.hero_candidates, None,
            "其他玩家的候选不得下发：{}",
            item.player_id
        );
    }
}

#[test]
fn select_hero_outside_candidates_is_rejected() {
    let (mut registry, room_id, _code, players) = lobby_room();
    let catalog = catalog();
    let player = &players[0].player_id;

    let candidates = registry.candidates_of(&room_id, player).expect("有候选");
    let outside = catalog
        .cards
        .values()
        .find(|card| card.is_hero && !candidates.contains(&card.id))
        .map(|card| card.id.clone())
        .expect("存在不在候选里的英雄卡");

    let error = registry
        .select_hero(&room_id, player, &outside, &catalog)
        .expect_err("候选外的英雄必须被拒绝");
    assert_eq!(error, RoomError::NotCandidate);
    assert_eq!(error.code(), ErrorCode::InvalidChoice);
    assert_eq!(
        registry
            .room(&room_id)
            .and_then(|room| room.slot(0).and_then(|slot| slot.hero_card_id.clone())),
        None,
        "被拒绝的选择不得写入座位"
    );
}

#[test]
fn select_hero_after_ready_is_rejected() {
    let (mut registry, room_id, _code, players) = ready_room();
    let catalog = catalog();
    let player = &players[0].player_id;

    let candidates = registry.candidates_of(&room_id, player).expect("有候选");
    let others: Vec<String> = candidates.iter().skip(1).cloned().collect();
    let error = registry
        .select_hero(&room_id, player, &others[0], &catalog)
        .expect_err("确认后不可改选");
    assert_eq!(error, RoomError::WrongPhase);
    assert_eq!(
        registry
            .room(&room_id)
            .and_then(|room| room.slot(0).and_then(|slot| slot.hero_card_id.clone()))
            .as_deref(),
        Some(candidates[0].as_str()),
        "已确认的英雄不得改变"
    );
}

#[test]
fn detach_marks_disconnected_and_keeps_seat() {
    let (mut registry, room_id, _code, players) = lobby_room();
    let player = &players[1].player_id;
    let before = registry.candidates_of(&room_id, player).expect("有候选");

    registry
        .mark_disconnected(&room_id, player, 1_000_000)
        .expect("标记断线成功");

    let room = registry.room(&room_id).expect("房间存在");
    assert_eq!(room.seat_of(player), Some(1), "宽限期内座位必须保留");
    assert!(!room.slot(1).unwrap().connected, "座位应标记为离线");
    assert_eq!(
        registry.candidates_of(&room_id, player),
        Some(before),
        "断线不得重抽候选"
    );
}

#[test]
fn reconnect_within_grace_restores_same_candidates() {
    let (mut registry, room_id, _code, players) = lobby_room();
    let player = players[1].player_id.clone();
    let before = registry.candidates_of(&room_id, &player).expect("有候选");
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
        registry.candidates_of(&room_id, &player),
        Some(before),
        "重连必须取回同一组候选"
    );
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

#[test]
fn reset_for_next_game_reopens_lobby_with_new_seed() {
    let (mut registry, room_id, _code, players) = started_room();
    let catalog = catalog();
    let old_playstyles = registry.room(&room_id).expect("房间存在").playstyles;
    let old_seed = registry.room(&room_id).expect("房间存在").seed;
    let old_candidates: Vec<Vec<String>> = players
        .iter()
        .map(|player| {
            registry
                .candidates_of(&room_id, &player.player_id)
                .expect("有候选")
        })
        .collect();

    // 找一个能改变玩法抽取的种子，保证断言确定成立而不是依赖概率。
    let new_seed = (1..=u64::MAX)
        .map(|step| old_seed.wrapping_add(step))
        .find(|&seed| rule_engine::playstyles::draw(seed) != old_playstyles)
        .expect("总能找到改变玩法抽取的种子");

    let kept = registry
        .reset_for_next_game(&room_id, new_seed, &catalog, &[])
        .expect("回房重置成功");
    assert!(kept, "仍有人留下时应返回 Ok(true)");

    let room = registry.room(&room_id).expect("房间存在");
    assert_eq!(room.phase, Phase::Lobby, "阶段应回到 Lobby");
    assert_eq!(room.seed, new_seed, "应换用新种子");
    assert_eq!(
        room.playstyles,
        rule_engine::playstyles::draw(new_seed),
        "玩法应按新种子重抽"
    );
    assert_ne!(room.playstyles, old_playstyles, "新种子应改变玩法");
    assert!(
        registry.start_ready(&room_id).is_none(),
        "重置后无人准备，不可直接开局"
    );

    for seat in 0..players.len() {
        let slot = room.slot(seat as u8).expect("座位存在");
        assert!(!slot.ready, "座位 {seat} 应取消准备");
        assert_eq!(slot.hero_card_id, None, "座位 {seat} 应清空英雄");
        assert_eq!(
            slot.candidate_segment,
            Some(seat),
            "段号应按座位顺序重新分配"
        );
    }

    let new_candidates: Vec<Vec<String>> = players
        .iter()
        .map(|player| {
            registry
                .candidates_of(&room_id, &player.player_id)
                .expect("每座位都应有候选")
        })
        .collect();
    for (seat, candidates) in new_candidates.iter().enumerate() {
        assert!(!candidates.is_empty(), "座位 {seat} 候选不得为空");
    }
    assert!(
        new_candidates
            .iter()
            .zip(old_candidates.iter())
            .any(|(new, old)| new != old),
        "换种子后至少一组候选应变化"
    );
}

#[test]
fn reset_removes_departed_and_transfers_host() {
    let (mut registry, room_id, _code, players) = started_room();
    let catalog = catalog();
    let departed_host = players[0].player_id.clone();
    let new_seed = registry
        .room(&room_id)
        .expect("房间存在")
        .seed
        .wrapping_add(11);

    let kept = registry
        .reset_for_next_game(
            &room_id,
            new_seed,
            &catalog,
            std::slice::from_ref(&departed_host),
        )
        .expect("回房重置成功");
    assert!(kept, "仍有成员时房间应保留");

    let room = registry.room(&room_id).expect("房间仍存在");
    assert_eq!(
        room.host_id, players[1].player_id,
        "房主应转给座位号最小的剩余成员"
    );
    assert_eq!(
        room.seat_of(&players[1].player_id),
        Some(1),
        "房主迁移不重排座位"
    );
    assert_eq!(room.seat_of(&departed_host), None, "被移除者不再在房间内");
    assert!(
        registry.room_of_player(&departed_host).is_none(),
        "按玩家查询应返回 None"
    );
    assert_eq!(room.players().len(), SEAT_COUNT - 1, "成员数应减 1");
}

#[test]
fn reset_with_all_departed_removes_room() {
    let (mut registry, room_id, code, players) = started_room();
    let catalog = catalog();
    let departed: Vec<String> = players
        .iter()
        .map(|player| player.player_id.clone())
        .collect();

    let kept = registry
        .reset_for_next_game(&room_id, 42, &catalog, &departed)
        .expect("回房重置成功");
    assert!(!kept, "全员离开时应返回 Ok(false)");
    assert!(registry.room(&room_id).is_none(), "空房间应删除");

    let mut store = SessionStore::new();
    let newcomer = session(&mut store, &FixedClock::new(1_000_000), "新玩家");
    assert_eq!(
        registry
            .join(&code, &newcomer, &catalog)
            .expect_err("房间码应失效"),
        RoomError::NotFound
    );
}

#[test]
fn error_codes_exist_in_schema() {
    let cases = [
        (RoomError::NotFound, "ROOM_NOT_FOUND"),
        (RoomError::Full, "ROOM_FULL"),
        (RoomError::NotHost, "NOT_HOST"),
        (RoomError::WrongPhase, "INVALID_STAGE"),
        (RoomError::HeroNotSelected, "HERO_NOT_SELECTED"),
        (RoomError::RoomNotReady, "ROOM_NOT_READY"),
        (RoomError::NotAHeroCard, "INVALID_CHOICE"),
        (RoomError::NotInRoom, "NOT_IN_ROOM"),
        (RoomError::NotCandidate, "INVALID_CHOICE"),
        (RoomError::InvalidTarget, "INVALID_CHOICE"),
        (RoomError::Unknown("保留原因".to_string()), "INTERNAL_ERROR"),
    ];

    for (error, expected) in cases {
        let code = error.code();
        let json = serde_json::to_value(code).expect("ErrorCode 应可序列化");
        assert_eq!(json.as_str(), Some(expected), "{error:?} 的协议码不符");
        let round_trip: ErrorCode =
            serde_json::from_value(json).expect("序列化出的码必须是 schema 中的 ErrorCode");
        assert_eq!(round_trip, code);
    }
}

// ---------------------------------------------------------------------------
// 真实 WebSocket：房主移除、非房主拒绝与解散。
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
