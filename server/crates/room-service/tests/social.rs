//! 社交存储的外部行为测试：申请、好友、最近对局与邀请规则。

use room_service::social::{
    FriendStatus, InviteOutcome, MAX_FRIENDS, MAX_INCOMING, REJECT_LIMIT, REQUEST_TTL_MS,
    SocialError, SocialStore,
};

const NOW: u64 = 1_700_000_000_000;
const HOUR: u64 = 60 * 60 * 1000;
const DAY: u64 = 24 * HOUR;

fn store_with(users: &[(&str, &str, &str)]) -> SocialStore {
    let mut store = SocialStore::new();
    for (player_id, player_code, display_name) in users {
        store.register(player_id, player_code, display_name);
    }
    store
}

fn pair() -> SocialStore {
    store_with(&[("p_a", "100001", "阿甲"), ("p_b", "100002", "阿乙")])
}

#[test]
fn player_code_lookup_returns_profile() {
    let store = pair();
    let (player_id, profile) = store.profile_by_code("100002").expect("编号应能查到");
    assert_eq!(player_id, "p_b");
    assert_eq!(profile.display_name, "阿乙");
    assert!(store.profile_by_code("999999").is_none());
}

#[tokio::test]
async fn apply_and_accept_creates_mutual_friendship() {
    let mut store = pair();
    store.apply("p_a", "100002", NOW).await.expect("申请应成功");

    let snapshot = store.snapshot("p_b", NOW, |_| FriendStatus::Offline);
    assert_eq!(snapshot.incoming.len(), 1);
    assert_eq!(snapshot.incoming[0].player_id, "p_a");

    store
        .respond("p_b", "p_a", true, NOW)
        .await
        .expect("同意应成功");
    assert!(store.is_friend("p_a", "p_b"));
    assert!(store.is_friend("p_b", "p_a"));

    let snapshot = store.snapshot("p_a", NOW, |_| FriendStatus::Online);
    assert_eq!(snapshot.friends.len(), 1);
    assert_eq!(snapshot.friends[0].status, FriendStatus::Online);
    assert_eq!(snapshot.friends[0].display_name, "阿乙");
}

#[tokio::test]
async fn relationships_reject_invalid_requests() {
    let mut store = pair();
    assert_eq!(
        store.apply("p_a", "100001", NOW).await.unwrap_err(),
        SocialError::SelfFriend
    );
    assert_eq!(
        store.apply("p_a", "999999", NOW).await.unwrap_err(),
        SocialError::PlayerNotFound
    );

    store
        .apply("p_a", "100002", NOW)
        .await
        .expect("首次申请成功");
    assert_eq!(
        store.apply("p_a", "100002", NOW).await.unwrap_err(),
        SocialError::RequestPending
    );
    assert_eq!(
        store.apply("p_b", "100001", NOW).await.unwrap_err(),
        SocialError::RequestPending
    );

    store
        .respond("p_b", "p_a", true, NOW)
        .await
        .expect("同意成功");
    assert_eq!(
        store.apply("p_a", "100002", NOW).await.unwrap_err(),
        SocialError::AlreadyFriend
    );
}

#[tokio::test]
async fn rejected_requests_are_limited_to_three_per_day() {
    let mut store = pair();
    for attempt in 0..REJECT_LIMIT {
        store
            .apply("p_a", "100002", NOW + attempt as u64)
            .await
            .expect("未达上限前仍可发送");
        store
            .respond("p_b", "p_a", false, NOW + attempt as u64)
            .await
            .expect("拒绝应成功");
    }
    assert_eq!(
        store.apply("p_a", "100002", NOW + HOUR).await.unwrap_err(),
        SocialError::RequestLimit
    );
    // 24 小时后再发送，窗口已滑过。
    assert!(store.apply("p_a", "100002", NOW + DAY + 1).await.is_ok());
}

#[tokio::test]
async fn requests_expire_after_seven_days() {
    let mut store = pair();
    store.apply("p_a", "100002", NOW).await.expect("申请应成功");
    let snapshot = store.snapshot("p_b", NOW + REQUEST_TTL_MS, |_| FriendStatus::Offline);
    assert!(snapshot.incoming.is_empty(), "过期申请不再下发");
    assert_eq!(
        store
            .respond("p_b", "p_a", true, NOW + REQUEST_TTL_MS)
            .await
            .unwrap_err(),
        SocialError::PlayerNotFound
    );
}

#[tokio::test]
async fn friend_list_is_capped() {
    let mut users: Vec<(String, String, String)> = vec![(
        "p_target".to_string(),
        "200000".to_string(),
        "目标".to_string(),
    )];
    for index in 0..=MAX_FRIENDS {
        users.push((
            format!("p_{index}"),
            format!("{:06}", 200001 + index),
            format!("玩家{index}"),
        ));
    }
    let refs: Vec<(&str, &str, &str)> = users
        .iter()
        .map(|(id, code, name)| (id.as_str(), code.as_str(), name.as_str()))
        .collect();
    let mut store = store_with(&refs);

    for index in 0..MAX_FRIENDS {
        let from = format!("p_{index}");
        store.apply(&from, "200000", NOW).await.expect("申请应成功");
        store
            .respond("p_target", &from, true, NOW)
            .await
            .expect("同意应成功");
    }
    assert_eq!(store.friends_of("p_target").len(), MAX_FRIENDS);
    assert_eq!(
        store
            .apply(&format!("p_{MAX_FRIENDS}"), "200000", NOW)
            .await
            .unwrap_err(),
        SocialError::FriendListFull
    );
}

#[tokio::test]
async fn incoming_request_list_is_capped() {
    let mut users: Vec<(String, String, String)> = Vec::new();
    users.push((
        "p_target".to_string(),
        "300000".to_string(),
        "目标".to_string(),
    ));
    for index in 0..=MAX_INCOMING {
        users.push((
            format!("p_{index}"),
            format!("{:06}", 300001 + index),
            format!("申请人{index}"),
        ));
    }
    let refs: Vec<(&str, &str, &str)> = users
        .iter()
        .map(|(id, code, name)| (id.as_str(), code.as_str(), name.as_str()))
        .collect();
    let mut store = store_with(&refs);

    for index in 0..MAX_INCOMING {
        store
            .apply(&format!("p_{index}"), "300000", NOW)
            .await
            .expect("申请应成功");
    }
    assert_eq!(
        store
            .apply(&format!("p_{MAX_INCOMING}"), "300000", NOW)
            .await
            .unwrap_err(),
        SocialError::RequestListFull
    );
}

#[tokio::test]
async fn removing_friend_is_mutual() {
    let mut store = pair();
    store.apply("p_a", "100002", NOW).await.expect("申请成功");
    store
        .respond("p_b", "p_a", true, NOW)
        .await
        .expect("同意成功");

    store.remove("p_a", "p_b").await.expect("删除应成功");
    assert!(!store.is_friend("p_a", "p_b"));
    assert!(!store.is_friend("p_b", "p_a"));
    assert_eq!(
        store.remove("p_a", "p_b").await.unwrap_err(),
        SocialError::NotFriends
    );
}

#[tokio::test]
async fn recent_matches_are_deduplicated_and_newest_first() {
    let mut store = store_with(&[
        ("p_a", "100001", "甲"),
        ("p_b", "100002", "乙"),
        ("p_c", "100003", "丙"),
    ]);
    store
        .record_match(&["p_a".into(), "p_b".into(), "p_c".into()], NOW)
        .await
        .expect("记录应成功");
    store
        .record_match(&["p_a".into(), "p_b".into()], NOW + 1)
        .await
        .expect("记录应成功");

    let snapshot = store.snapshot("p_a", NOW, |_| FriendStatus::Offline);
    let ids: Vec<&str> = snapshot
        .recent
        .iter()
        .map(|entry| entry.player_id.as_str())
        .collect();
    assert_eq!(ids, vec!["p_b", "p_c"], "最近一局在前且去重");
}

#[tokio::test]
async fn snapshot_orders_by_status_then_name() {
    let mut store = store_with(&[
        ("p_me", "100000", "我"),
        ("p_off", "100001", "阿离"),
        ("p_game", "100002", "阿中"),
        ("p_on_b", "100003", "阿乙"),
        ("p_on_a", "100004", "阿甲"),
    ]);
    for friend in ["p_off", "p_game", "p_on_b", "p_on_a"] {
        store.apply(friend, "100000", NOW).await.expect("申请成功");
        store
            .respond("p_me", friend, true, NOW)
            .await
            .expect("同意成功");
    }
    let snapshot = store.snapshot("p_me", NOW, |id| match id {
        "p_on_a" | "p_on_b" => FriendStatus::Online,
        "p_game" => FriendStatus::InGame,
        _ => FriendStatus::Offline,
    });
    let order: Vec<&str> = snapshot
        .friends
        .iter()
        .map(|entry| entry.player_id.as_str())
        .collect();
    assert_eq!(order, vec!["p_on_a", "p_on_b", "p_game", "p_off"]);
}

#[tokio::test]
async fn invite_requires_friendship_and_expires() {
    let mut store = pair();
    assert_eq!(
        store
            .create_invite("p_a", "p_b", "room_1", "ABC234", NOW)
            .await
            .unwrap_err(),
        SocialError::NotFriends
    );

    store.apply("p_a", "100002", NOW).await.expect("申请成功");
    store
        .respond("p_b", "p_a", true, NOW)
        .await
        .expect("同意成功");

    let invite = store
        .create_invite("p_a", "p_b", "room_1", "ABC234", NOW)
        .await
        .expect("邀请应成功");
    assert_eq!(invite.room_code, "ABC234");
    assert_eq!(invite.expires_at_ms, NOW + 60_000);

    let outcome = store
        .respond_invite("p_b", &invite.id, true, NOW + 1_000)
        .await
        .expect("接受应成功");
    assert!(matches!(outcome, InviteOutcome::Accepted(_)));

    let invite = store
        .create_invite("p_a", "p_b", "room_1", "ABC234", NOW)
        .await
        .expect("邀请应成功");
    assert_eq!(
        store
            .respond_invite("p_b", &invite.id, true, NOW + 60_000)
            .await
            .unwrap_err(),
        SocialError::InviteExpired
    );
}

#[tokio::test]
async fn invite_rejects_non_recipient_and_decline() {
    let mut store = pair();
    store.apply("p_a", "100002", NOW).await.expect("申请成功");
    store
        .respond("p_b", "p_a", true, NOW)
        .await
        .expect("同意成功");
    let invite = store
        .create_invite("p_a", "p_b", "room_1", "ABC234", NOW)
        .await
        .expect("邀请应成功");

    assert_eq!(
        store
            .respond_invite("p_a", &invite.id, true, NOW)
            .await
            .unwrap_err(),
        SocialError::InviteNotFound
    );
    let outcome = store
        .respond_invite("p_b", &invite.id, false, NOW)
        .await
        .expect("拒绝应成功");
    assert!(matches!(outcome, InviteOutcome::Declined(_)));
    assert_eq!(
        store
            .respond_invite("p_b", &invite.id, true, NOW)
            .await
            .unwrap_err(),
        SocialError::InviteNotFound
    );
}
