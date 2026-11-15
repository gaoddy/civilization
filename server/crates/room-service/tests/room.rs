//! 房间状态机与大堂流程测试。
//!
//! 覆盖建房、加入/离开、英雄选择、准备与开局前置判定，以及 `RoomUpdate` 视图。
//! 房间码与座位分配均为确定性行为，测试直接断言具体取值。

use protocol::ErrorCode;
use protocol::client::AuthRequest;
use room_service::auth::{Session, SessionStore};
use room_service::clock::FixedClock;
use room_service::room::{RoomError, RoomRegistry, SEAT_COUNT};
use rule_engine::data::Catalog;
use rule_engine::state::Phase;

const CARDS_JSON: &str = include_str!("../../../data/v0.1/cards.json");
const SKILLS_JSON: &str = include_str!("../../../data/v0.1/skills.json");
const TOKENS_JSON: &str = include_str!("../../../data/v0.1/tokens.json");
const CONFIG_JSON: &str = include_str!("../../../data/v0.1/config.json");

/// 真实卡表；英雄卡用 `adam-smith`，非英雄卡用 `agi`。
fn catalog() -> Catalog {
    Catalog::from_json(CARDS_JSON, SKILLS_JSON, TOKENS_JSON, CONFIG_JSON).expect("卡表应能加载")
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

/// 8 个座位各用一张不同的英雄卡，便于断言座位与英雄的对应关系。
const HEROES: [&str; SEAT_COUNT] = [
    "adam-smith",
    "darwin",
    "davinci",
    "edison",
    "einstein",
    "faraday",
    "galileo",
    "knuth",
];

/// 建一个 8 人满座的大厅房间，返回注册表、房间 id、房间码与按座位号排列的会话。
fn lobby_room() -> (RoomRegistry, String, String, Vec<Session>) {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let mut registry = RoomRegistry::with_seed(0);

    let host = session(&mut store, &clock, "房主");
    let room = registry.create(&host);
    let room_id = room.id.clone();
    let code = room.code.clone();

    let mut players = vec![host];
    for index in 1..SEAT_COUNT {
        let player = session(&mut store, &clock, &format!("玩家{index}"));
        registry.join(&code, &player).expect("加入成功");
        players.push(player);
    }
    (registry, room_id, code, players)
}

/// 在满座大厅里让所有人选好不同英雄并准备，但仍停留在 `Lobby`。
fn ready_room() -> (RoomRegistry, String, String, Vec<Session>) {
    let (mut registry, room_id, code, players) = lobby_room();
    let catalog = catalog();
    for (seat, player) in players.iter().enumerate() {
        registry
            .select_hero(&room_id, &player.player_id, HEROES[seat], &catalog)
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
    let room = registry.create(&host);

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
    assert_eq!(other.create(&host).code, room.code);
}

#[test]
fn join_fills_seats_in_order() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");

    let mut registry = RoomRegistry::with_seed(0);
    let code = registry.create(&host).code.clone();

    for expected_seat in 1..SEAT_COUNT {
        let player = session(&mut store, &clock, &format!("玩家{expected_seat}"));
        let room = registry.join(&code, &player).expect("未满时应能加入");
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
    let room = registry.create(&host);
    let room_id = room.id.clone();
    let code = room.code.clone();

    for index in 1..SEAT_COUNT {
        let player = session(&mut store, &clock, &format!("玩家{index}"));
        registry.join(&code, &player).expect("前 8 人应能加入");
    }

    let ninth = session(&mut store, &clock, "第九人");
    assert_eq!(
        registry.join(&code, &ninth).expect_err("满座后应拒绝"),
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
            .join("AAAAAA", &player)
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
    let room = registry.create(&host);
    let room_id = room.id.clone();
    let code = room.code.clone();

    registry
        .join(&code, &host)
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
    let room = registry.create(&host);
    let room_id = room.id.clone();
    let code = room.code.clone();
    registry.join(&code, &second).expect("加入成功");

    registry
        .select_hero(&room_id, &host.player_id, "adam-smith", &catalog)
        .expect("房主选英雄成功");
    registry
        .select_hero(&room_id, &second.player_id, "adam-smith", &catalog)
        .expect("重复选择同一张英雄卡必须被允许");

    let room = registry.room(&room_id).expect("房间存在");
    assert_eq!(
        room.slot(0).unwrap().hero_card_id.as_deref(),
        Some("adam-smith")
    );
    assert_eq!(
        room.slot(1).unwrap().hero_card_id.as_deref(),
        Some("adam-smith"),
        "两个座位允许选择同一张英雄卡"
    );
}

#[test]
fn select_hero_rejects_non_hero() {
    let clock = FixedClock::new(1_000_000);
    let mut store = SessionStore::new();
    let host = session(&mut store, &clock, "房主");
    let catalog = catalog();

    let mut registry = RoomRegistry::with_seed(0);
    let room = registry.create(&host);
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
    let room = registry.create(&host);
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
    registry
        .select_hero(&room_id, &host.player_id, "adam-smith", &catalog)
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
    let room = registry.create(&host);
    let room_id = room.id.clone();
    let code = room.code.clone();
    registry.join(&code, &second).expect("加入成功");
    registry
        .select_hero(&room_id, &second.player_id, "adam-smith", &catalog)
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
    let room = registry.create(&host);
    let room_id = room.id.clone();
    registry
        .select_hero(&room_id, &host.player_id, "adam-smith", &catalog)
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
    let room = registry.create(&host);
    let room_id = room.id.clone();
    let code = room.code.clone();

    let mut players = vec![host];
    for index in 1..SEAT_COUNT {
        let player = session(&mut store, &clock, &format!("玩家{index}"));
        registry.join(&code, &player).expect("加入成功");
        players.push(player);
    }

    // 8 人齐、全部选了同一个英雄（重复英雄允许），但只准备 7 人。
    for player in &players {
        registry
            .select_hero(&room_id, &player.player_id, "adam-smith", &catalog)
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
    for seat in 0..SEAT_COUNT as u8 {
        assert_eq!(heroes.get(&seat).map(String::as_str), Some("adam-smith"));
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
    let room = registry.create(&host);
    let room_id = room.id.clone();
    let code = room.code.clone();
    registry.join(&code, &second).expect("加入成功");
    registry
        .select_hero(&room_id, &second.player_id, "adam-smith", &catalog)
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
        Some("adam-smith")
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
    let room = registry.create(&host);
    let room_id = room.id.clone();
    let code = room.code.clone();
    registry.join(&code, &second).expect("玩家2 加入");
    registry.join(&code, &third).expect("玩家3 加入");

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
    let room = registry.join(&code, &fourth).expect("加入空位成功");
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
    let room = registry.create(&host);
    let room_id = room.id.clone();
    let code = room.code.clone();
    registry.join(&code, &second).expect("玩家2 加入");
    registry.join(&code, &third).expect("玩家3 加入");

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
    let room = registry.create(&host);
    let room_id = room.id.clone();
    let code = room.code.clone();

    registry.leave(&room_id, &host.player_id).expect("离开成功");
    assert!(registry.room_update(&room_id).is_none(), "空房间应销毁");

    let newcomer = session(&mut store, &clock, "新玩家");
    assert_eq!(
        registry.join(&code, &newcomer).expect_err("房间码应失效"),
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
    let room = registry.create(&host);
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
    let room = registry.create(&host);
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
    for (seat, _) in players.iter().enumerate() {
        assert_eq!(
            heroes.get(&(seat as u8)).map(String::as_str),
            Some(HEROES[seat]),
            "座位 {seat} 的英雄应与所选一致"
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
        registry.join(&code, &newcomer).expect_err("开局后不能加入"),
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
            .select_hero(&room_id, &players[1].player_id, HEROES[2], &catalog)
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
