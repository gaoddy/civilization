//! 持久化端到端：账号身份、好友、申请、最近对局与邀请跨重启恢复。
//!
//! 需要 PostgreSQL：设置 `TEST_DATABASE_URL`（或 `DATABASE_URL`）后运行
//! `make db-up && make db-test`；未配置时每个用例打印跳过原因并返回。

mod common;

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use common::{ClientWs, connect_with_retry, database_url, start_server_with_db, test_db};
use futures_util::{SinkExt, StreamExt};
use protocol::{ErrorCode, FriendUpdate, RoomUpdate, ServerEnvelope, ServerMessage};
use room_service::rating::{MatchRecord, RankedConfig, RatingStore};
use room_service::social::{FriendStatus, SocialError, SocialStore};
use room_service::{AccountStore, Clock, FixedClock, SystemClock};
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message;

const NOW: u64 = 1_700_000_000_000;
const DAY: u64 = 24 * 60 * 60 * 1000;

fn skip(name: &str) {
    eprintln!("SKIP {name}: 未设置 TEST_DATABASE_URL / DATABASE_URL（先运行 make db-up）");
}

/// 在测试库里创建三名玩家，返回 `(player_id, player_code)`。
async fn seed_accounts(db: &Arc<room_service::Db>) -> Vec<(String, String)> {
    let mut accounts = AccountStore::load(db.clone()).await.expect("账号应能载入");
    let mut seeded = Vec::new();
    for (index, name) in ["甲", "乙", "丙"].iter().enumerate() {
        let phone = format!("1370000{:04}", index);
        let account = accounts
            .login(&phone, Some(name), NOW)
            .await
            .expect("账号应能创建");
        seeded.push((account.player_id, account.player_code));
    }
    seeded
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn account_identity_survives_restart() {
    let Some(store) = test_db().await else {
        skip("account_identity_survives_restart");
        return;
    };

    let mut accounts = AccountStore::load(store.db.clone())
        .await
        .expect("账号应能载入");
    let account = accounts
        .login("13800000001", Some("甲"), NOW)
        .await
        .expect("首次登录应成功");
    assert_eq!(account.player_code.len(), 6);
    assert!(account.player_code.chars().all(|ch| ch.is_ascii_digit()));

    let again = accounts
        .login("13800000001", Some("改名"), NOW)
        .await
        .expect("同号再登录应成功");
    assert_eq!(again.player_id, account.player_id);
    assert_eq!(again.player_code, account.player_code);
    assert_eq!(again.display_name, "甲", "重新登录不覆盖已存展示名");

    // 模拟重启：新的 AccountStore 从库重建，再按同号登录。
    let mut reloaded = AccountStore::load(store.db.clone())
        .await
        .expect("重启后账号应能载入");
    let restored = reloaded
        .login("13800000001", None, NOW)
        .await
        .expect("重启后同号登录应恢复账号");
    assert_eq!(restored.player_id, account.player_id);
    assert_eq!(restored.player_code, account.player_code);
    assert_eq!(restored.display_name, "甲");

    let other = reloaded
        .login("13800000002", None, NOW)
        .await
        .expect("新手机号应创建新账号");
    assert_ne!(other.player_id, account.player_id);
    assert_ne!(other.player_code, account.player_code);
    assert_eq!(other.display_name, "玩家2", "默认名按持久化序号生成");

    store.cleanup().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn achievement_unlock_survives_restart() {
    let Some(store) = test_db().await else {
        skip("achievement_unlock_survives_restart");
        return;
    };

    let mut accounts = AccountStore::load(store.db.clone())
        .await
        .expect("账号应能载入");
    let account = accounts
        .login("13800000003", Some("成就"), NOW)
        .await
        .expect("首次登录应成功");
    assert!(
        accounts
            .unlocked_achievements(&account.player_id)
            .is_empty()
    );
    assert!(
        accounts
            .unlock_achievement(&account.player_id, "genetics", NOW)
            .await
            .expect("首次解锁应成功")
    );
    assert!(
        !accounts
            .unlock_achievement(&account.player_id, "genetics", NOW)
            .await
            .expect("重复解锁应返回 false")
    );

    let reloaded = AccountStore::load(store.db.clone())
        .await
        .expect("重启后账号与成就应能载入");
    assert_eq!(
        reloaded.unlocked_achievements(&account.player_id),
        vec!["genetics".to_string()]
    );

    store.cleanup().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn steam_identity_survives_restart() {
    let Some(store) = test_db().await else {
        skip("steam_identity_survives_restart");
        return;
    };

    let mut accounts = AccountStore::load(store.db.clone())
        .await
        .expect("账号应能载入");
    let first = accounts
        .login_steam("76561198000000001", Some("Steam 玩家"), NOW)
        .await
        .expect("首次 Steam 登录应建号");
    assert_eq!(first.phone, None);
    assert_eq!(first.steam_id.as_deref(), Some("76561198000000001"));

    let again = accounts
        .login_steam("76561198000000001", Some("改名"), NOW)
        .await
        .expect("同一 SteamID 再登录应恢复同一账号");
    assert_eq!(again.player_id, first.player_id);
    assert_eq!(again.player_code, first.player_code);
    assert_eq!(again.display_name, "Steam 玩家", "重新登录不覆盖已存展示名");

    let mut reloaded = AccountStore::load(store.db.clone())
        .await
        .expect("重启后账号应能载入");
    let restored = reloaded
        .login_steam("76561198000000001", None, NOW)
        .await
        .expect("重启后同 SteamID 登录应恢复账号");
    assert_eq!(restored.player_id, first.player_id);
    assert_eq!(restored.phone, None);

    let view = reloaded.view(&restored.player_id).expect("账号视图应存在");
    assert!(view.steam_bound);
    assert!(!view.phone_bound);
    assert_eq!(view.masked_phone, None);

    store.cleanup().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bind_phone_links_identities_in_database() {
    let Some(store) = test_db().await else {
        skip("bind_phone_links_identities_in_database");
        return;
    };

    let mut accounts = AccountStore::load(store.db.clone())
        .await
        .expect("账号应能载入");
    let steam = accounts
        .login_steam("76561198000000002", Some("Ada"), NOW)
        .await
        .expect("Steam 账号应创建");
    let view = accounts
        .bind_phone(&steam.player_id, "13800000009", NOW)
        .await
        .expect("绑定手机号应成功");
    assert!(view.phone_bound);
    assert_eq!(view.masked_phone.as_deref(), Some("138****0009"));

    let phone_login = accounts
        .login("13800000009", Some("改名"), NOW)
        .await
        .expect("绑定后手机号应恢复同一账号");
    assert_eq!(phone_login.player_id, steam.player_id);

    let idempotent = accounts
        .bind_phone(&steam.player_id, "13800000009", NOW)
        .await
        .expect("同号重复绑定应幂等");
    assert_eq!(idempotent.player_id, steam.player_id);

    let wrong_phone = accounts
        .bind_phone(&steam.player_id, "13800000008", NOW)
        .await
        .expect_err("已绑定账号不能换绑");
    assert_eq!(wrong_phone.code, ErrorCode::PhoneAlreadyBound);

    let other = accounts
        .login("13800000007", None, NOW)
        .await
        .expect("手机号账号应创建");
    let steam_two = accounts
        .login_steam("76561198000000003", None, NOW)
        .await
        .expect("第二个 Steam 账号应创建");
    let occupied = accounts
        .bind_phone(&steam_two.player_id, "13800000007", NOW)
        .await
        .expect_err("占用中的手机号必须拒绝");
    assert_eq!(occupied.code, ErrorCode::PhoneAlreadyBound);

    let phone_only = accounts
        .bind_phone(&other.player_id, "13800000006", NOW)
        .await
        .expect_err("手机号账号不能再绑定手机号");
    assert_eq!(phone_only.code, ErrorCode::InvalidStage);

    store.cleanup().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn social_data_survives_restart() {
    let Some(store) = test_db().await else {
        skip("social_data_survives_restart");
        return;
    };

    let seeded = seed_accounts(&store.db).await;
    let (a_id, a_code) = &seeded[0];
    let (b_id, b_code) = &seeded[1];
    let (c_id, _) = &seeded[2];

    let mut social = SocialStore::load(store.db.clone(), NOW)
        .await
        .expect("社交数据应能载入");
    social.apply(a_id, b_code, NOW).await.expect("申请成功");
    social
        .respond(b_id, a_id, true, NOW)
        .await
        .expect("同意成功");
    social
        .apply(c_id, a_code, NOW)
        .await
        .expect("丙向甲申请成功");
    social
        .record_match(&[a_id.clone(), c_id.clone()], NOW)
        .await
        .expect("记录最近对局成功");
    let invite = social
        .create_invite(a_id, b_id, "room_1", "ABC234", NOW)
        .await
        .expect("邀请成功");

    let mut reloaded = SocialStore::load(store.db.clone(), NOW + 1_000)
        .await
        .expect("重启后社交数据应能载入");
    let snapshot = reloaded.snapshot(a_id, NOW + 1_000, |_| FriendStatus::Offline);
    assert_eq!(snapshot.friends.len(), 1);
    assert_eq!(&snapshot.friends[0].player_id, b_id);
    assert_eq!(&snapshot.incoming[0].player_id, c_id);
    assert_eq!(&snapshot.recent[0].player_id, c_id, "最近对局跨重启保留");

    let snapshot = reloaded.snapshot(b_id, NOW + 1_000, |_| FriendStatus::Offline);
    assert_eq!(snapshot.friends.len(), 1, "好友关系跨重启保留");
    assert_eq!(&snapshot.friends[0].player_id, a_id);
    assert!(snapshot.incoming.is_empty(), "乙没有待处理申请");

    let outcome = reloaded
        .respond_invite(b_id, &invite.id, true, NOW + 1_000)
        .await
        .expect("未过期邀请跨重启仍可响应");
    assert_eq!(outcome_room(&outcome), "ABC234");

    store.cleanup().await;
}

fn outcome_room(outcome: &room_service::social::InviteOutcome) -> String {
    match outcome {
        room_service::social::InviteOutcome::Accepted(invite)
        | room_service::social::InviteOutcome::Declined(invite) => invite.room_code.clone(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expired_records_are_dropped_on_reload() {
    let Some(store) = test_db().await else {
        skip("expired_records_are_dropped_on_reload");
        return;
    };

    let seeded = seed_accounts(&store.db).await;
    let (a_id, a_code) = &seeded[0];
    let (b_id, b_code) = &seeded[1];
    let (c_id, _) = &seeded[2];

    let mut social = SocialStore::load(store.db.clone(), NOW)
        .await
        .expect("社交数据应能载入");
    social.apply(a_id, b_code, NOW).await.expect("申请成功");
    social
        .respond(b_id, a_id, true, NOW)
        .await
        .expect("同意成功");
    // 第三个玩家留下一条待处理申请，用于验证 7 天过期。
    social
        .apply(c_id, a_code, NOW)
        .await
        .expect("丙向甲申请成功");
    let invite = social
        .create_invite(a_id, b_id, "room_1", "ABC234", NOW)
        .await
        .expect("邀请成功");

    let mut reloaded = SocialStore::load(store.db.clone(), NOW + 8 * DAY)
        .await
        .expect("重启后社交数据应能载入");
    let snapshot = reloaded.snapshot(a_id, NOW + 8 * DAY, |_| FriendStatus::Offline);
    assert!(snapshot.incoming.is_empty(), "超过 7 天的申请不再载入");
    assert_eq!(
        reloaded
            .respond_invite(b_id, &invite.id, true, NOW + 8 * DAY)
            .await
            .unwrap_err(),
        SocialError::InviteNotFound,
        "过期邀请在载入时已删除"
    );

    store.cleanup().await;
}

// ---------------------------------------------------------------------------
// WebSocket：重启后重新登录恢复好友，未过期邀请仍可响应
// ---------------------------------------------------------------------------

struct Client {
    ws: ClientWs,
    next_id: u64,
}

impl Client {
    async fn connect(addr: std::net::SocketAddr) -> Self {
        Self {
            ws: connect_with_retry(&format!("ws://{addr}/ws")).await,
            next_id: 0,
        }
    }

    async fn send(&mut self, kind: &str, payload: Value) {
        self.next_id += 1;
        let envelope = json!({
            "v": 1,
            "id": format!("persist-{}", self.next_id),
            "type": kind,
            "ts": "2026-01-01T00:00:00Z",
            "payload": payload,
        });
        self.ws
            .send(Message::Text(envelope.to_string().into()))
            .await
            .expect("发送应当成功");
    }

    async fn recv(&mut self) -> Option<ServerMessage> {
        while let Some(item) = self.ws.next().await {
            match item {
                Ok(Message::Text(text)) => {
                    let envelope: ServerEnvelope =
                        serde_json::from_str(text.as_str()).expect("下行信封应能解析");
                    return Some(envelope.parse().expect("下行载荷应能解析"));
                }
                Ok(Message::Close(_)) => return None,
                Ok(_) => continue,
                Err(_) => return None,
            }
        }
        None
    }

    async fn wait_for<T>(
        &mut self,
        label: &str,
        mut extract: impl FnMut(&ServerMessage) -> Option<T>,
    ) -> T {
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let message = self.recv().await.expect("等待期间连接不应关闭");
                if let ServerMessage::Error(error) = &message {
                    panic!("{label} 收到错误 {}: {}", error.code, error.message);
                }
                if let Some(value) = extract(&message) {
                    return value;
                }
            }
        })
        .await
        .unwrap_or_else(|_| panic!("{label} 10 秒内未到达"))
    }

    async fn wait_error(&mut self, code: ErrorCode) {
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let message = self.recv().await.expect("等待错误期间连接不应关闭");
                if let ServerMessage::Error(error) = message {
                    assert_eq!(error.code, code);
                    return;
                }
            }
        })
        .await
        .unwrap_or_else(|_| panic!("10 秒内未收到错误 {code:?}"))
    }

    async fn login(&mut self, phone: &str, name: &str) -> (String, String) {
        self.send(
            "AuthRequest",
            json!({ "phone": phone, "code": "123456", "displayName": name }),
        )
        .await;
        self.wait_for("认证", |message| match message {
            ServerMessage::AuthResult(result) => {
                Some((result.player_id.clone(), result.player_code.clone()))
            }
            _ => None,
        })
        .await
    }

    async fn wait_friend_update(
        &mut self,
        label: &str,
        predicate: impl Fn(&FriendUpdate) -> bool,
    ) -> FriendUpdate {
        self.wait_for(label, |message| match message {
            ServerMessage::FriendUpdate(update) if predicate(update) => Some(update.clone()),
            _ => None,
        })
        .await
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn websocket_relogin_restores_friends_and_invite() {
    let Some(store) = test_db().await else {
        skip("websocket_relogin_restores_friends_and_invite");
        return;
    };
    let (addr, handle) = start_server_with_db(Arc::new(SystemClock), store.db.clone()).await;

    let mut host = Client::connect(addr).await;
    let (host_id, host_code) = host.login("13911110001", "房主").await;
    let mut guest = Client::connect(addr).await;
    let (guest_id, guest_code) = guest.login("13911110002", "受邀者").await;

    // 建立好友关系。
    host.send("FriendRequest", json!({ "toPlayerCode": guest_code }))
        .await;
    guest
        .wait_friend_update("收到好友申请", |update| !update.incoming.is_empty())
        .await;
    guest
        .send(
            "FriendRespond",
            json!({ "fromPlayerId": host_id.clone(), "accept": true }),
        )
        .await;
    host.wait_friend_update("好友列表", |update| !update.friends.is_empty())
        .await;
    guest
        .wait_friend_update("好友列表", |update| !update.friends.is_empty())
        .await;

    // 建房并邀请；记录邀请 id 供重启后使用。
    host.send("CreateRoom", json!({})).await;
    let room: RoomUpdate = host
        .wait_for("房间更新", |message| match message {
            ServerMessage::RoomUpdate(update) if update.players.len() == 1 => Some(update.clone()),
            _ => None,
        })
        .await;
    assert!(!room.code.is_empty(), "应拿到房间码");
    host.send("InviteToRoom", json!({ "friendId": guest_id.clone() }))
        .await;
    let invite_id = guest
        .wait_for("收到邀请", |message| match message {
            ServerMessage::FriendInvite(invite) => Some(invite.invite_id.clone()),
            _ => None,
        })
        .await;

    // 重启：旧进程状态全部丢弃，新服务同一 schema。
    handle.abort();
    let (addr, handle) = start_server_with_db(Arc::new(SystemClock), store.db.clone()).await;

    let mut host = Client::connect(addr).await;
    let (restored_id, restored_code) = host.login("13911110001", "房主").await;
    assert_eq!(restored_id, host_id, "同号登录恢复同一 playerId");
    assert_eq!(restored_code, host_code, "同号登录恢复同一玩家编号");

    let mut guest = Client::connect(addr).await;
    guest.login("13911110002", "受邀者").await;
    let update = guest
        .wait_friend_update("重启后好友列表", |update| !update.friends.is_empty())
        .await;
    assert_eq!(update.friends[0].player_id, host_id, "好友关系跨重启保留");
    assert_eq!(
        format!("{:?}", update.friends[0].status),
        "Online",
        "好友在线状态重启后按连接重新计算"
    );

    // 邀请已持久化，但房间只在内存：响应应报 ROOM_NOT_FOUND 而不是 INVITE_NOT_FOUND。
    guest
        .send(
            "RespondInvite",
            json!({ "inviteId": invite_id, "accept": true }),
        )
        .await;
    guest.wait_error(ErrorCode::RoomNotFound).await;

    handle.abort();
    store.cleanup().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fixed_clock_keeps_invite_window() {
    // 固定时钟下重启不推进时间，邀请仍然有效。
    let Some(store) = test_db().await else {
        skip("fixed_clock_keeps_invite_window");
        return;
    };
    let clock = Arc::new(FixedClock::new(NOW));
    let seeded = seed_accounts(&store.db).await;
    let (a_id, _) = &seeded[0];
    let (b_id, b_code) = &seeded[1];
    let mut social = SocialStore::load(store.db.clone(), NOW)
        .await
        .expect("社交数据应能载入");
    social.apply(a_id, b_code, NOW).await.expect("申请成功");
    social
        .respond(b_id, a_id, true, NOW)
        .await
        .expect("同意成功");
    let invite = social
        .create_invite(a_id, b_id, "room_1", "ABC234", NOW)
        .await
        .expect("邀请成功");

    let mut reloaded = SocialStore::load(store.db.clone(), clock.now_ms())
        .await
        .expect("重启后社交数据应能载入");
    let outcome = reloaded
        .respond_invite(b_id, &invite.id, true, clock.now_ms() + 59_000)
        .await
        .expect("固定时钟未推进，邀请仍在有效期内");
    assert_eq!(outcome_room(&outcome), "ABC234");
    store.cleanup().await;
}

#[tokio::test]
async fn ping_succeeds_and_migrations_are_repeatable() {
    let Some(url) = database_url() else {
        skip("ping_succeeds_and_migrations_are_repeatable");
        return;
    };
    let first = room_service::Db::connect(&url)
        .await
        .expect("首次连接与迁移应成功");
    first.ping().await.expect("SELECT 1 应成功");
    drop(first);
    room_service::Db::connect(&url)
        .await
        .expect("重复连接与迁移应幂等");
}

/// 天梯结算、排行榜、宗师快照与最近战绩跨重启恢复。
///
/// 段位阈值压到 1000 附近，让一局 8 人同分对局就能覆盖宗师判定；宗师上限
/// 取 3，验证「积分够但名次在快照外」显示大师。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ranked_settlement_survives_restart() {
    const NAME: &str = "ranked_settlement_survives_restart";
    let Some(store) = test_db().await else {
        skip(NAME);
        return;
    };

    let mut accounts = AccountStore::load(store.db.clone())
        .await
        .expect("账号应能载入");
    let mut ids = Vec::new();
    for index in 0..8 {
        let phone = format!("1390000{index:04}");
        let account = accounts
            .login(&phone, Some(&format!("玩家{}", index + 1)), NOW)
            .await
            .expect("账号应能创建");
        ids.push(account.player_id);
    }

    let config = RankedConfig::from_json(
        r#"{
            "masterRankLimit": 3,
            "tiers": [
                {"id": "apprentice", "minRating": 0},
                {"id": "bronze", "minRating": 900},
                {"id": "silver", "minRating": 1000},
                {"id": "gold", "minRating": 1001},
                {"id": "platinum", "minRating": 1002},
                {"id": "diamond", "minRating": 1003},
                {"id": "master", "minRating": 1004},
                {"id": "grandmaster", "minRating": 1005}
            ]
        }"#,
    )
    .expect("测试配置应通过校验");

    let mut ratings = RatingStore::load(store.db.clone())
        .await
        .expect("积分应能载入");
    let start: BTreeMap<String, i32> = ids.iter().map(|id| (id.clone(), 1000)).collect();
    let placements: Vec<(String, u8)> = ids
        .iter()
        .enumerate()
        .map(|(index, id)| (id.clone(), u8::try_from(index + 1).expect("名次应合法")))
        .collect();
    let record = MatchRecord {
        match_id: "m_db",
        room_id: "room_db",
        seed: "seed",
        finished_at_ms: NOW,
    };
    let results = ratings
        .settle_match(&config, &record, &placements, &start)
        .await
        .expect("结算应写入数据库");
    assert_eq!(results[0].rating_delta, 100);
    assert_eq!(results[0].rating_after, 1100);
    assert_eq!(results[7].rating_after, 900);

    ratings
        .ensure_master_snapshot(&config, NOW, &ids)
        .await
        .expect("宗师快照应写入数据库");
    assert_eq!(ratings.tier(&config, &ids[0], 1100), "grandmaster");
    assert_eq!(
        ratings.tier(&config, &ids[3], 1014),
        "master",
        "积分达到宗师线但名次在快照外时显示大师"
    );

    // 模拟重启：积分、排行榜、宗师快照与最近战绩都从库恢复。
    let reloaded = RatingStore::load(store.db.clone())
        .await
        .expect("重启后积分应能载入");
    assert_eq!(reloaded.rating_of(&ids[0]), 1100);
    assert_eq!(reloaded.rating_of(&ids[7]), 900);
    assert_eq!(
        reloaded.summary(&config, &ids[7]).highest_rating,
        1000,
        "首次扣分不能抹掉初始最高积分"
    );
    assert_eq!(reloaded.tier(&config, &ids[0], 1100), "grandmaster");
    let rows = reloaded
        .leaderboard(&ids, 100)
        .await
        .expect("排行榜应能查询");
    assert_eq!(rows.len(), 8);
    assert_eq!(rows[0].player_id, ids[0]);
    assert_eq!(rows[7].player_id, ids[7]);
    let recent = reloaded
        .recent_for(&ids[0])
        .await
        .expect("最近战绩应能查询");
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].placement, 1);
    assert_eq!(recent[0].rating_after, 1100);

    store.cleanup().await;
}
