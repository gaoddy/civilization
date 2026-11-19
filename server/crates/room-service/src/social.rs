//! 社交存储：账号资料缓存、好友关系、申请、最近对局与房间邀请。
//!
//! 数据库为唯一真相源：写操作先落库、成功后再改内存；启动时 [`SocialStore::load`]
//! 从库重建全部状态。未配置数据库时退化为纯内存模式（测试与本地开发）。读取入口
//! 会顺手清理过期申请与邀请，M2 不跑后台清理任务；时间由调用方传入。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

use protocol::ErrorCode;
use uuid::Uuid;

use crate::db::{Db, NewInvite, ms_to_utc, utc_to_ms};

/// 好友上限。
pub const MAX_FRIENDS: usize = 100;
/// 待处理申请上限。
pub const MAX_INCOMING: usize = 20;
/// 申请有效期：7 天。
pub const REQUEST_TTL_MS: u64 = 7 * 24 * 60 * 60 * 1000;
/// 拒绝后的发送冷却窗口：24 小时。
pub const REJECT_WINDOW_MS: u64 = 24 * 60 * 60 * 1000;
/// 冷却窗口内最多发送申请次数。
pub const REJECT_LIMIT: usize = 3;
/// 邀请有效期：60 秒。
pub const INVITE_TTL_MS: u64 = 60 * 1000;
/// 最近对局保留的局数。
pub const RECENT_MATCHES: usize = 20;

/// 账号对外资料。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub player_code: String,
    pub display_name: String,
}

/// 好友在线状态；声明顺序即列表排序顺序。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FriendStatus {
    Online,
    InGame,
    Offline,
}

impl FriendStatus {
    /// 协议取值。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Online => "online",
            Self::InGame => "inGame",
            Self::Offline => "offline",
        }
    }
}

/// 好友列表项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FriendEntry {
    pub player_id: String,
    pub player_code: String,
    pub display_name: String,
    pub status: FriendStatus,
}

/// 待处理申请项（两个方向共用）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestEntry {
    pub player_id: String,
    pub player_code: String,
    pub display_name: String,
    pub sent_at_ms: u64,
}

/// 最近对局玩家与本人的关系。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    None,
    Friend,
    Incoming,
    Outgoing,
}

impl Relation {
    /// 协议取值。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Friend => "friend",
            Self::Incoming => "incoming",
            Self::Outgoing => "outgoing",
        }
    }
}

/// 最近对局列表项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentEntry {
    pub player_id: String,
    pub player_code: String,
    pub display_name: String,
    pub relation: Relation,
}

/// 一次下发的完整社交视图。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SocialSnapshot {
    pub friends: Vec<FriendEntry>,
    pub incoming: Vec<RequestEntry>,
    pub outgoing: Vec<RequestEntry>,
    pub recent: Vec<RecentEntry>,
}

/// 房间邀请。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invite {
    pub id: String,
    pub from_player_id: String,
    pub from_player_code: String,
    pub from_display_name: String,
    pub to_player_id: String,
    pub room_id: String,
    pub room_code: String,
    pub expires_at_ms: u64,
}

/// 邀请响应结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InviteOutcome {
    Accepted(Invite),
    Declined(Invite),
}

/// 社交操作失败原因。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocialError {
    /// 编号对应用户不存在，或申请已不存在。
    PlayerNotFound,
    /// 不能向自己发送申请。
    SelfFriend,
    /// 已经是好友。
    AlreadyFriend,
    /// 任一方向已有待处理申请。
    RequestPending,
    /// 自己或对方好友已满。
    FriendListFull,
    /// 对方待处理申请已满。
    RequestListFull,
    /// 24 小时内发送次数达到上限。
    RequestLimit,
    /// 双方不是好友。
    NotFriends,
    /// 邀请已不存在。
    InviteNotFound,
    /// 邀请已过期。
    InviteExpired,
    /// 数据库写入失败。
    Storage,
}

impl SocialError {
    /// 映射到协议错误码。
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::PlayerNotFound => ErrorCode::FriendNotFound,
            Self::SelfFriend => ErrorCode::FriendSelf,
            Self::AlreadyFriend => ErrorCode::FriendAlready,
            Self::RequestPending => ErrorCode::FriendRequestPending,
            Self::FriendListFull => ErrorCode::FriendListFull,
            Self::RequestListFull => ErrorCode::FriendRequestFull,
            Self::RequestLimit => ErrorCode::FriendRequestLimit,
            Self::NotFriends => ErrorCode::NotFriends,
            Self::InviteNotFound => ErrorCode::InviteNotFound,
            Self::InviteExpired => ErrorCode::InviteExpired,
            Self::Storage => ErrorCode::InternalError,
        }
    }
}

impl fmt::Display for SocialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlayerNotFound => f.write_str("玩家不存在或申请已失效"),
            Self::SelfFriend => f.write_str("不能添加自己为好友"),
            Self::AlreadyFriend => f.write_str("已经是好友"),
            Self::RequestPending => f.write_str("已有待处理的好友申请"),
            Self::FriendListFull => f.write_str("好友已满"),
            Self::RequestListFull => f.write_str("对方申请列表已满"),
            Self::RequestLimit => f.write_str("24 小时内发送申请次数已达上限"),
            Self::NotFriends => f.write_str("对方不是好友"),
            Self::InviteNotFound => f.write_str("邀请不存在"),
            Self::InviteExpired => f.write_str("邀请已过期"),
            Self::Storage => f.write_str("社交数据写入失败"),
        }
    }
}

impl std::error::Error for SocialError {}

/// 社交存储。
#[derive(Debug, Default)]
pub struct SocialStore {
    db: Option<Arc<Db>>,
    profiles: BTreeMap<String, Profile>,
    by_code: BTreeMap<String, String>,
    friends: BTreeMap<String, BTreeSet<String>>,
    /// 收件方 → 申请人 → 发送时刻。
    incoming: BTreeMap<String, BTreeMap<String, u64>>,
    /// 申请人 → 收件方 → 发送时刻。
    outgoing: BTreeMap<String, BTreeMap<String, u64>>,
    /// 申请人 → 收件方 → 最近发送时刻，用于 24 小时次数限制。
    attempts: BTreeMap<String, BTreeMap<String, Vec<u64>>>,
    /// 玩家 → 最近同局玩家，最新在前。
    recent: BTreeMap<String, Vec<String>>,
    invites: BTreeMap<String, Invite>,
}

impl SocialStore {
    /// 创建纯内存存储（测试与未配置数据库的本地开发）。
    pub fn new() -> Self {
        Self::default()
    }

    /// 从数据库重建社交状态；档案来自 `account` 表。
    ///
    /// # Errors
    ///
    /// 查询失败时返回错误文本。
    pub async fn load(db: Arc<Db>, now_ms: u64) -> Result<Self, String> {
        let stored = db
            .load_social(
                ms_to_utc(now_ms.saturating_sub(REQUEST_TTL_MS)),
                ms_to_utc(now_ms.saturating_sub(REJECT_WINDOW_MS)),
                ms_to_utc(now_ms),
            )
            .await
            .map_err(|err| format!("载入社交数据失败: {err}"))?;
        let accounts = db
            .load_accounts()
            .await
            .map_err(|err| format!("载入账号失败: {err}"))?;

        let mut store = Self {
            db: Some(db),
            ..Self::default()
        };
        for account in accounts {
            store.register(
                &account.account_id,
                &account.player_code,
                &account.display_name,
            );
        }
        for friendship in stored.friendships {
            store
                .friends
                .entry(friendship.player_id)
                .or_default()
                .insert(friendship.friend_id);
        }
        for request in stored.requests {
            let sent_at = utc_to_ms(request.sent_at);
            store
                .incoming
                .entry(request.to_account_id.clone())
                .or_default()
                .insert(request.from_account_id.clone(), sent_at);
            store
                .outgoing
                .entry(request.from_account_id)
                .or_default()
                .insert(request.to_account_id, sent_at);
        }
        for attempt in stored.attempts {
            store
                .attempts
                .entry(attempt.from_account_id)
                .or_default()
                .entry(attempt.to_account_id)
                .or_default()
                .push(utc_to_ms(attempt.sent_at));
        }
        for recent in stored.recent {
            let entry = store.recent.entry(recent.player_id).or_default();
            if !entry.contains(&recent.opponent_id) {
                entry.push(recent.opponent_id);
            }
        }
        for invite in stored.invites {
            let (from_player_code, from_display_name) =
                store.profiles.get(&invite.from_account_id).map_or_else(
                    || (String::new(), String::new()),
                    |profile| (profile.player_code.clone(), profile.display_name.clone()),
                );
            store.invites.insert(
                invite.invite_id.clone(),
                Invite {
                    id: invite.invite_id,
                    from_player_id: invite.from_account_id,
                    from_player_code,
                    from_display_name,
                    to_player_id: invite.to_account_id,
                    room_id: invite.room_id,
                    room_code: invite.room_code,
                    expires_at_ms: utc_to_ms(invite.expires_at),
                },
            );
        }
        // 每人的最近对局按库里顺序恢复，最多保留 RECENT_MATCHES 条。
        for entry in store.recent.values_mut() {
            entry.truncate(RECENT_MATCHES);
        }
        Ok(store)
    }

    /// 登记或更新玩家资料；续登时编号不变。
    pub fn register(&mut self, player_id: &str, player_code: &str, display_name: &str) {
        if let Some(old) = self.profiles.get(player_id)
            && old.player_code != player_code
        {
            self.by_code.remove(&old.player_code);
        }
        self.by_code
            .insert(player_code.to_string(), player_id.to_string());
        self.profiles.insert(
            player_id.to_string(),
            Profile {
                player_code: player_code.to_string(),
                display_name: display_name.to_string(),
            },
        );
    }

    /// 按玩家编号查资料。
    pub fn profile_by_code(&self, player_code: &str) -> Option<(&str, &Profile)> {
        let player_id = self.by_code.get(player_code)?;
        self.profiles
            .get(player_id)
            .map(|profile| (player_id.as_str(), profile))
    }

    /// 按 `player_id` 查资料。
    pub fn profile(&self, player_id: &str) -> Option<&Profile> {
        self.profiles.get(player_id)
    }

    /// 好友 `player_id` 列表。
    pub fn friends_of(&self, player_id: &str) -> Vec<String> {
        self.friends
            .get(player_id)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// 是否已是好友。
    pub fn is_friend(&self, a: &str, b: &str) -> bool {
        self.friends.get(a).is_some_and(|set| set.contains(b))
    }

    /// 发送好友申请。
    ///
    /// 校验通过后先落库，失败则不改内存。
    ///
    /// # Errors
    ///
    /// 见 [`SocialError`]：编号不存在、加自己、已是好友、已有待处理申请、
    /// 好友或申请列表已满、24 小时发送次数超限、数据库写入失败。
    pub async fn apply(
        &mut self,
        from: &str,
        to_code: &str,
        now_ms: u64,
    ) -> Result<(), SocialError> {
        let to = self
            .by_code
            .get(to_code)
            .cloned()
            .ok_or(SocialError::PlayerNotFound)?;
        if from == to {
            return Err(SocialError::SelfFriend);
        }
        self.prune_requests(from, now_ms);
        self.prune_requests(&to, now_ms);
        if self.is_friend(from, &to) {
            return Err(SocialError::AlreadyFriend);
        }
        // 两个方向、收发两张表都可能是同一请求的镜像，任一方向已有待处理即拒绝。
        let pending = self
            .outgoing
            .get(from)
            .is_some_and(|map| map.contains_key(&to))
            || self
                .outgoing
                .get(&to)
                .is_some_and(|map| map.contains_key(from))
            || self
                .incoming
                .get(from)
                .is_some_and(|map| map.contains_key(&to))
            || self
                .incoming
                .get(&to)
                .is_some_and(|map| map.contains_key(from));
        if pending {
            return Err(SocialError::RequestPending);
        }
        if self.incoming.get(&to).map_or(0, BTreeMap::len) >= MAX_INCOMING {
            return Err(SocialError::RequestListFull);
        }
        if self.friends.get(&to).map_or(0, BTreeSet::len) >= MAX_FRIENDS
            || self.friends.get(from).map_or(0, BTreeSet::len) >= MAX_FRIENDS
        {
            return Err(SocialError::FriendListFull);
        }
        self.prune_attempts(from, &to, now_ms);
        let attempts = self
            .attempts
            .get(from)
            .and_then(|map| map.get(&to))
            .cloned()
            .unwrap_or_default();
        if attempts.len() >= REJECT_LIMIT {
            return Err(SocialError::RequestLimit);
        }
        if let Some(db) = self.db.clone() {
            db.record_request(from, &to, ms_to_utc(now_ms))
                .await
                .map_err(|_| SocialError::Storage)?;
        }
        self.incoming
            .entry(to.clone())
            .or_default()
            .insert(from.to_string(), now_ms);
        self.outgoing
            .entry(from.to_string())
            .or_default()
            .insert(to.clone(), now_ms);
        self.attempts
            .entry(from.to_string())
            .or_default()
            .entry(to)
            .or_default()
            .push(now_ms);
        Ok(())
    }

    /// 处理好友申请。
    ///
    /// 同意后双方立即成为好友；拒绝后保留 24 小时发送次数记录。申请、好友与发送
    /// 记录在同一事务内落库。
    ///
    /// # Errors
    ///
    /// 申请不存在、双方好友列表已满或数据库写入失败时返回 [`SocialError`]。
    pub async fn respond(
        &mut self,
        to: &str,
        from: &str,
        accept: bool,
        now_ms: u64,
    ) -> Result<(), SocialError> {
        self.prune_requests(to, now_ms);
        self.prune_requests(from, now_ms);
        let pending = self
            .incoming
            .get(to)
            .is_some_and(|map| map.contains_key(from));
        if !pending {
            return Err(SocialError::PlayerNotFound);
        }
        if accept
            && (self.friends.get(to).map_or(0, BTreeSet::len) >= MAX_FRIENDS
                || self.friends.get(from).map_or(0, BTreeSet::len) >= MAX_FRIENDS)
        {
            return Err(SocialError::FriendListFull);
        }

        if let Some(db) = self.db.clone() {
            db.respond_request(to, from, accept, ms_to_utc(now_ms))
                .await
                .map_err(|_| SocialError::Storage)?;
        }

        if let Some(map) = self.incoming.get_mut(to) {
            map.remove(from);
        }
        if let Some(map) = self.outgoing.get_mut(from) {
            map.remove(to);
        }
        if accept {
            self.friends
                .entry(to.to_string())
                .or_default()
                .insert(from.to_string());
            self.friends
                .entry(from.to_string())
                .or_default()
                .insert(to.to_string());
            self.clear_attempts(from, to);
            self.clear_attempts(to, from);
        }
        Ok(())
    }

    /// 删除好友，双向立即解除。
    ///
    /// 关系、申请与发送记录在同一事务内落库。
    ///
    /// # Errors
    ///
    /// 双方不是好友或数据库写入失败时返回 [`SocialError`]。
    pub async fn remove(&mut self, a: &str, b: &str) -> Result<(), SocialError> {
        let is_friend = self.friends.get(a).is_some_and(|set| set.contains(b));
        if !is_friend {
            return Err(SocialError::NotFriends);
        }
        if let Some(db) = self.db.clone() {
            db.remove_friendship(a, b)
                .await
                .map_err(|_| SocialError::Storage)?;
        }
        if let Some(set) = self.friends.get_mut(a) {
            set.remove(b);
        }
        if let Some(set) = self.friends.get_mut(b) {
            set.remove(a);
        }
        for (from, to) in [(a, b), (b, a)] {
            if let Some(map) = self.incoming.get_mut(from) {
                map.remove(to);
            }
            if let Some(map) = self.outgoing.get_mut(from) {
                map.remove(to);
            }
            self.clear_attempts(from, to);
        }
        Ok(())
    }

    /// 记录一局对局的参与者，把同局其他玩家并入每人的最近对局列表。
    ///
    /// # Errors
    ///
    /// 数据库写入失败时返回 [`SocialError::Storage`]。
    pub async fn record_match(
        &mut self,
        participants: &[String],
        now_ms: u64,
    ) -> Result<(), SocialError> {
        if let Some(db) = self.db.clone() {
            db.record_match(participants, ms_to_utc(now_ms))
                .await
                .map_err(|_| SocialError::Storage)?;
        }
        for player in participants {
            let entry = self.recent.entry(player.clone()).or_default();
            for other in participants {
                if other == player {
                    continue;
                }
                entry.retain(|id| id != other);
                entry.insert(0, other.clone());
            }
            entry.truncate(RECENT_MATCHES);
        }
        Ok(())
    }

    /// 构造指定玩家的完整社交视图。
    ///
    /// `status_of` 由调用方按连接与房间状态给出，本模块不感知网络。
    pub fn snapshot(
        &mut self,
        player_id: &str,
        now_ms: u64,
        status_of: impl Fn(&str) -> FriendStatus,
    ) -> SocialSnapshot {
        self.prune_requests(player_id, now_ms);
        let mut friends: Vec<FriendEntry> = self
            .friends_of(player_id)
            .into_iter()
            .filter_map(|friend_id| {
                let profile = self.profiles.get(&friend_id)?;
                Some(FriendEntry {
                    player_id: friend_id.clone(),
                    player_code: profile.player_code.clone(),
                    display_name: profile.display_name.clone(),
                    status: status_of(&friend_id),
                })
            })
            .collect();
        friends.sort_by(|a, b| {
            a.status
                .cmp(&b.status)
                .then_with(|| {
                    a.display_name
                        .chars()
                        .next()
                        .cmp(&b.display_name.chars().next())
                })
                .then_with(|| a.player_id.cmp(&b.player_id))
        });

        let incoming = self.request_entries(
            player_id,
            self.incoming.get(player_id).cloned().unwrap_or_default(),
        );
        let outgoing = self.request_entries(
            player_id,
            self.outgoing.get(player_id).cloned().unwrap_or_default(),
        );

        let recent = self
            .recent
            .get(player_id)
            .map(|ids| {
                ids.iter()
                    .filter(|id| id.as_str() != player_id)
                    .filter_map(|id| {
                        let profile = self.profiles.get(id)?;
                        Some(RecentEntry {
                            player_id: id.clone(),
                            player_code: profile.player_code.clone(),
                            display_name: profile.display_name.clone(),
                            relation: self.relation(player_id, id),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        SocialSnapshot {
            friends,
            incoming,
            outgoing,
            recent,
        }
    }

    /// 创建房间邀请；调用方负责校验被邀请人在线且不在其它房间。
    ///
    /// 先落库再进入内存；已过期邀请在写入前清理。
    ///
    /// # Errors
    ///
    /// 双方不是好友、发起人资料缺失或数据库写入失败时返回 [`SocialError`]。
    pub async fn create_invite(
        &mut self,
        from: &str,
        to: &str,
        room_id: &str,
        room_code: &str,
        now_ms: u64,
    ) -> Result<Invite, SocialError> {
        if !self.is_friend(from, to) {
            return Err(SocialError::NotFriends);
        }
        let profile = self
            .profiles
            .get(from)
            .cloned()
            .ok_or(SocialError::PlayerNotFound)?;
        let invite = Invite {
            id: format!("inv_{}", Uuid::new_v4().simple()),
            from_player_id: from.to_string(),
            from_player_code: profile.player_code,
            from_display_name: profile.display_name,
            to_player_id: to.to_string(),
            room_id: room_id.to_string(),
            room_code: room_code.to_string(),
            expires_at_ms: now_ms.saturating_add(INVITE_TTL_MS),
        };
        if let Some(db) = self.db.clone() {
            db.save_invite(&NewInvite {
                invite_id: &invite.id,
                from,
                to,
                room_id,
                room_code,
                expires_at: ms_to_utc(invite.expires_at_ms),
                created_at: ms_to_utc(now_ms),
            })
            .await
            .map_err(|_| SocialError::Storage)?;
        }
        self.purge_invites(now_ms);
        self.invites.insert(invite.id.clone(), invite.clone());
        Ok(invite)
    }

    /// 响应邀请。
    ///
    /// # Errors
    ///
    /// 邀请不存在或不属于该玩家返回 [`SocialError::InviteNotFound`]；已过期返回
    /// [`SocialError::InviteExpired`] 并删除该邀请；数据库写入失败返回
    /// [`SocialError::Storage`]。
    pub async fn respond_invite(
        &mut self,
        to: &str,
        invite_id: &str,
        accept: bool,
        now_ms: u64,
    ) -> Result<InviteOutcome, SocialError> {
        let invite = self
            .invites
            .get(invite_id)
            .cloned()
            .ok_or(SocialError::InviteNotFound)?;
        if invite.to_player_id != to {
            return Err(SocialError::InviteNotFound);
        }
        if let Some(db) = self.db.clone() {
            db.delete_invite(invite_id)
                .await
                .map_err(|_| SocialError::Storage)?;
        }
        self.invites.remove(invite_id);
        if now_ms >= invite.expires_at_ms {
            return Err(SocialError::InviteExpired);
        }
        if accept {
            Ok(InviteOutcome::Accepted(invite))
        } else {
            Ok(InviteOutcome::Declined(invite))
        }
    }

    /// 清理已过期邀请并返回它们，供调用方通知邀请人。
    pub fn purge_invites(&mut self, now_ms: u64) -> Vec<Invite> {
        let expired: Vec<String> = self
            .invites
            .iter()
            .filter(|(_, invite)| now_ms >= invite.expires_at_ms)
            .map(|(id, _)| id.clone())
            .collect();
        expired
            .into_iter()
            .filter_map(|id| self.invites.remove(&id))
            .collect()
    }

    fn request_entries(&self, _owner: &str, map: BTreeMap<String, u64>) -> Vec<RequestEntry> {
        let mut entries: Vec<RequestEntry> = map
            .into_iter()
            .filter_map(|(player_id, sent_at_ms)| {
                let profile = self.profiles.get(&player_id)?;
                Some(RequestEntry {
                    player_id,
                    player_code: profile.player_code.clone(),
                    display_name: profile.display_name.clone(),
                    sent_at_ms,
                })
            })
            .collect();
        entries.sort_by(|a, b| b.sent_at_ms.cmp(&a.sent_at_ms));
        entries
    }

    fn relation(&self, player: &str, other: &str) -> Relation {
        if self.is_friend(player, other) {
            Relation::Friend
        } else if self
            .incoming
            .get(player)
            .is_some_and(|map| map.contains_key(other))
        {
            Relation::Incoming
        } else if self
            .outgoing
            .get(player)
            .is_some_and(|map| map.contains_key(other))
        {
            Relation::Outgoing
        } else {
            Relation::None
        }
    }

    fn prune_requests(&mut self, player: &str, now_ms: u64) {
        if let Some(map) = self.incoming.get_mut(player) {
            map.retain(|_, sent| now_ms.saturating_sub(*sent) < REQUEST_TTL_MS);
        }
        if let Some(map) = self.outgoing.get_mut(player) {
            map.retain(|_, sent| now_ms.saturating_sub(*sent) < REQUEST_TTL_MS);
        }
    }

    fn clear_attempts(&mut self, from: &str, to: &str) {
        if let Some(map) = self.attempts.get_mut(from) {
            map.remove(to);
        }
    }

    /// 把指定对的发送记录裁到 24 小时窗口内。
    fn prune_attempts(&mut self, from: &str, to: &str, now_ms: u64) {
        if let Some(map) = self.attempts.get_mut(from)
            && let Some(list) = map.get_mut(to)
        {
            list.retain(|at| now_ms.saturating_sub(*at) < REJECT_WINDOW_MS);
        }
    }
}
