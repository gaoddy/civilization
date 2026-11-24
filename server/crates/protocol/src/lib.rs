//! 协议 Rust 契约：由 `protocol/schema/v1` 经 typify 生成，信封判别为手写。
//!
//! 唯一来源是 `protocol/schema/v1/*.schema.json`；本 crate 不手写与 schema
//! 不一致的类型。生成物见 `protocol/generated/protocol.rs`，漂移用
//! `make protocol-rs-check` 检出。

// 生成代码整体放行 clippy：typify 会产出可派生但手写的 `Default`、对 Copy 类型
// 调用 `clone` 等，`-D warnings` 下不应据此否定生成物。
mod generated {
    #![allow(clippy::all)]

    include!("../../../../protocol/generated/protocol.rs");
}

pub use generated::*;

/// 上行载荷类型，按信封 `type` 判别。
pub mod client {
    pub use crate::{
        AuthRequest, BindPhone, CombatPlayed, CreateRoom, CreateRoomInvite, DissolveRoom,
        FriendRemove, FriendRequest, FriendRespond, IntentMessage, InviteToRoom, JoinQueue,
        JoinRoom, KickMember, LeaveQueue, LeaveRoom, Ready, Reconnect, RedeemRoomInvite,
        RequestAccount, RequestLeaderboard, RequestPhoneCode, RequestProfile, RequestSnapshot,
        RespondInvite, SelectHeroCard, SetBotFill, StartGame,
    };
}

/// 下行载荷类型，按信封 `type` 判别。
pub mod server {
    pub use crate::{
        AccountView, AchievementState, AchievementUnlocked, AuthResult, CombatResult, ErrorMessage,
        FriendInvite, FriendInviteResult, FriendRequestItem, FriendUpdate, FriendUpdateFriendsItem,
        FriendUpdateFriendsItemStatus, FriendUpdateOutgoingItem, FriendUpdateRecentItem,
        FriendUpdateRecentItemRelation, GameOver, GameStarted, HeroPickStarted, InviteAccepted,
        Leaderboard, LeaderboardEntriesItem, MatchFound, PhaseChanged, PhoneCodeSent,
        PlayerProfile, PlayerProfileRecentItem, QueueState, RoomClosed, RoomInviteCode, RoomUpdate,
        SelfStats, Snapshot, StateUpdate, TimerWarning,
    };
}

/// 玩家可见状态视图。
pub mod view {
    pub use crate::{
        Affordances, BoardAffordance, BoardMinion, CombatLogEntry, GameEvent, HandAffordance,
        HandCard, PendingChoice, Phase, PlayerSummary, PlayerView, ShopAffordance, ShopSlot,
    };
}

/// 玩家意图（内部标签枚举，`#[serde(tag = "kind")]`）。
pub mod intents {
    pub use crate::Intent;
}

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

/// 协议层错误：目前只承载稳定错误码字符串。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolError(pub &'static str);

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for ProtocolError {}

/// 上行信封判别结果。只做 `type` → payload 的分派，不含任何业务规则。
#[derive(Debug, Clone)]
pub enum ClientMessage {
    AuthRequest(AuthRequest),
    CreateRoom(CreateRoom),
    JoinRoom(JoinRoom),
    LeaveRoom(LeaveRoom),
    JoinQueue(JoinQueue),
    LeaveQueue(LeaveQueue),
    KickMember(KickMember),
    DissolveRoom(DissolveRoom),
    Ready(Ready),
    StartGame(StartGame),
    SetBotFill(SetBotFill),
    SelectHeroCard(SelectHeroCard),
    Intent(IntentMessage),
    CombatPlayed(CombatPlayed),
    RequestSnapshot(RequestSnapshot),
    Reconnect(Reconnect),
    FriendRequest(FriendRequest),
    FriendRespond(FriendRespond),
    FriendRemove(FriendRemove),
    InviteToRoom(InviteToRoom),
    RespondInvite(RespondInvite),
    RequestAccount(RequestAccount),
    RequestPhoneCode(RequestPhoneCode),
    BindPhone(BindPhone),
    CreateRoomInvite(CreateRoomInvite),
    RedeemRoomInvite(RedeemRoomInvite),
    RequestLeaderboard(RequestLeaderboard),
    RequestProfile(RequestProfile),
}

/// 下行信封判别结果。只做 `type` → payload 的分派，不含任何业务规则。
#[derive(Debug, Clone)]
pub enum ServerMessage {
    AuthResult(AuthResult),
    RoomUpdate(RoomUpdate),
    RoomClosed(RoomClosed),
    QueueState(QueueState),
    MatchFound(MatchFound),
    HeroPickStarted(HeroPickStarted),
    GameStarted(GameStarted),
    Snapshot(Snapshot),
    StateUpdate(StateUpdate),
    PhaseChanged(PhaseChanged),
    TimerWarning(TimerWarning),
    CombatResult(CombatResult),
    GameOver(GameOver),
    Error(ErrorMessage),
    FriendUpdate(FriendUpdate),
    FriendInvite(FriendInvite),
    FriendInviteResult(FriendInviteResult),
    InviteAccepted(InviteAccepted),
    AccountView(AccountView),
    PhoneCodeSent(PhoneCodeSent),
    RoomInviteCode(RoomInviteCode),
    AchievementState(AchievementState),
    AchievementUnlocked(AchievementUnlocked),
    Leaderboard(Leaderboard),
    PlayerProfile(PlayerProfile),
}

/// 已知 s2c 消息类型名，与 `messages.json#/directions/s2c` 一致。
const SERVER_MESSAGE_TYPES: [&str; 25] = [
    "AuthResult",
    "RoomUpdate",
    "RoomClosed",
    "QueueState",
    "MatchFound",
    "HeroPickStarted",
    "GameStarted",
    "Snapshot",
    "StateUpdate",
    "PhaseChanged",
    "TimerWarning",
    "CombatResult",
    "GameOver",
    "Error",
    "FriendUpdate",
    "FriendInvite",
    "FriendInviteResult",
    "InviteAccepted",
    "AccountView",
    "PhoneCodeSent",
    "RoomInviteCode",
    "AchievementState",
    "AchievementUnlocked",
    "Leaderboard",
    "PlayerProfile",
];

/// 从 payload 值反序列化；失败统一为 `BAD_PAYLOAD`。
fn parse_payload<T: DeserializeOwned>(payload: Value) -> Result<T, ProtocolError> {
    serde_json::from_value(payload).map_err(|_| ProtocolError("BAD_PAYLOAD"))
}

impl ClientEnvelope {
    /// 按 `type` 把 `payload` 反序列化为具体载荷。未知 `type` 返回 `UNKNOWN_TYPE`，
    /// payload 与类型不匹配返回 `BAD_PAYLOAD`。
    pub fn parse(&self) -> Result<ClientMessage, ProtocolError> {
        let payload = || Value::Object(self.payload.clone());
        match self.type_.as_str() {
            "AuthRequest" => parse_payload(payload()).map(ClientMessage::AuthRequest),
            "CreateRoom" => parse_payload(payload()).map(ClientMessage::CreateRoom),
            "JoinRoom" => parse_payload(payload()).map(ClientMessage::JoinRoom),
            "LeaveRoom" => parse_payload(payload()).map(ClientMessage::LeaveRoom),
            "JoinQueue" => parse_payload(payload()).map(ClientMessage::JoinQueue),
            "LeaveQueue" => parse_payload(payload()).map(ClientMessage::LeaveQueue),
            "KickMember" => parse_payload(payload()).map(ClientMessage::KickMember),
            "DissolveRoom" => parse_payload(payload()).map(ClientMessage::DissolveRoom),
            "Ready" => parse_payload(payload()).map(ClientMessage::Ready),
            "StartGame" => parse_payload(payload()).map(ClientMessage::StartGame),
            "SetBotFill" => parse_payload(payload()).map(ClientMessage::SetBotFill),
            "SelectHeroCard" => parse_payload(payload()).map(ClientMessage::SelectHeroCard),
            "Intent" => parse_payload(payload()).map(ClientMessage::Intent),
            "CombatPlayed" => parse_payload(payload()).map(ClientMessage::CombatPlayed),
            "RequestSnapshot" => parse_payload(payload()).map(ClientMessage::RequestSnapshot),
            "Reconnect" => parse_payload(payload()).map(ClientMessage::Reconnect),
            "FriendRequest" => parse_payload(payload()).map(ClientMessage::FriendRequest),
            "FriendRespond" => parse_payload(payload()).map(ClientMessage::FriendRespond),
            "FriendRemove" => parse_payload(payload()).map(ClientMessage::FriendRemove),
            "InviteToRoom" => parse_payload(payload()).map(ClientMessage::InviteToRoom),
            "RespondInvite" => parse_payload(payload()).map(ClientMessage::RespondInvite),
            "RequestAccount" => parse_payload(payload()).map(ClientMessage::RequestAccount),
            "RequestPhoneCode" => parse_payload(payload()).map(ClientMessage::RequestPhoneCode),
            "BindPhone" => parse_payload(payload()).map(ClientMessage::BindPhone),
            "CreateRoomInvite" => parse_payload(payload()).map(ClientMessage::CreateRoomInvite),
            "RedeemRoomInvite" => parse_payload(payload()).map(ClientMessage::RedeemRoomInvite),
            "RequestLeaderboard" => parse_payload(payload()).map(ClientMessage::RequestLeaderboard),
            "RequestProfile" => parse_payload(payload()).map(ClientMessage::RequestProfile),
            _ => Err(ProtocolError("UNKNOWN_TYPE")),
        }
    }
}

impl ServerEnvelope {
    /// 构造一个下行信封。`seq` 必须从 1 起（协议要求单调递增）；`id` 由进程内
    /// 计数器生成，保证本进程内唯一。
    ///
    /// # Errors
    ///
    /// - `seq == 0`（协议要求从 1 起）返回 `BAD_PAYLOAD`。
    /// - `kind` 不是 25 种已知 s2c 类型之一返回 `UNKNOWN_TYPE`。
    pub fn new(
        kind: &str,
        seq: u64,
        payload: serde_json::Map<String, serde_json::Value>,
    ) -> Result<ServerEnvelope, ProtocolError> {
        if !SERVER_MESSAGE_TYPES.contains(&kind) {
            return Err(ProtocolError("UNKNOWN_TYPE"));
        }
        let seq = std::num::NonZeroU64::new(seq).ok_or(ProtocolError("BAD_PAYLOAD"))?;

        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        Ok(Self {
            id: ServerEnvelopeId::try_from(format!("srv_{id}")).expect("进程内计数生成的 id 非空"),
            payload,
            player_id: None,
            reply_to: None,
            room_id: None,
            seq,
            ts: chrono::Utc::now(),
            type_: ServerEnvelopeType::try_from(kind.to_string()).expect("已校验的 s2c 类型名非空"),
            v: Value::from(1),
        })
    }

    /// 把可序列化的载荷编码成 JSON 对象后构造信封。
    ///
    /// # Errors
    ///
    /// 错误语义同 [`ServerEnvelope::new`]；序列化失败或载荷不是 JSON 对象时
    /// 返回 `BAD_PAYLOAD`。
    pub fn encode(kind: &str, seq: u64, payload: &impl Serialize) -> Result<Self, ProtocolError> {
        let payload = serde_json::to_value(payload).map_err(|_| ProtocolError("BAD_PAYLOAD"))?;
        let map = match payload {
            Value::Object(map) => map,
            _ => return Err(ProtocolError("BAD_PAYLOAD")),
        };
        Self::new(kind, seq, map)
    }

    /// 按 `type` 把 `payload` 反序列化为具体载荷。错误语义同 [`ClientEnvelope::parse`]。
    pub fn parse(&self) -> Result<ServerMessage, ProtocolError> {
        let payload = || Value::Object(self.payload.clone());
        match self.type_.as_str() {
            "AuthResult" => parse_payload(payload()).map(ServerMessage::AuthResult),
            "RoomUpdate" => parse_payload(payload()).map(ServerMessage::RoomUpdate),
            "RoomClosed" => parse_payload(payload()).map(ServerMessage::RoomClosed),
            "QueueState" => parse_payload(payload()).map(ServerMessage::QueueState),
            "MatchFound" => parse_payload(payload()).map(ServerMessage::MatchFound),
            "HeroPickStarted" => parse_payload(payload()).map(ServerMessage::HeroPickStarted),
            "GameStarted" => parse_payload(payload()).map(ServerMessage::GameStarted),
            "Snapshot" => parse_payload(payload()).map(ServerMessage::Snapshot),
            "StateUpdate" => parse_payload(payload()).map(ServerMessage::StateUpdate),
            "PhaseChanged" => parse_payload(payload()).map(ServerMessage::PhaseChanged),
            "TimerWarning" => parse_payload(payload()).map(ServerMessage::TimerWarning),
            "CombatResult" => parse_payload(payload()).map(ServerMessage::CombatResult),
            "GameOver" => parse_payload(payload()).map(ServerMessage::GameOver),
            "Error" => parse_payload(payload()).map(ServerMessage::Error),
            "FriendUpdate" => parse_payload(payload()).map(ServerMessage::FriendUpdate),
            "FriendInvite" => parse_payload(payload()).map(ServerMessage::FriendInvite),
            "FriendInviteResult" => parse_payload(payload()).map(ServerMessage::FriendInviteResult),
            "InviteAccepted" => parse_payload(payload()).map(ServerMessage::InviteAccepted),
            "AccountView" => parse_payload(payload()).map(ServerMessage::AccountView),
            "PhoneCodeSent" => parse_payload(payload()).map(ServerMessage::PhoneCodeSent),
            "RoomInviteCode" => parse_payload(payload()).map(ServerMessage::RoomInviteCode),
            "AchievementState" => parse_payload(payload()).map(ServerMessage::AchievementState),
            "AchievementUnlocked" => {
                parse_payload(payload()).map(ServerMessage::AchievementUnlocked)
            }
            "Leaderboard" => parse_payload(payload()).map(ServerMessage::Leaderboard),
            "PlayerProfile" => parse_payload(payload()).map(ServerMessage::PlayerProfile),
            _ => Err(ProtocolError("UNKNOWN_TYPE")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ClientEnvelope, ClientMessage, ServerEnvelope, ServerMessage};

    fn client_envelope(kind: &str, payload: serde_json::Value) -> ClientEnvelope {
        serde_json::from_value(serde_json::json!({
            "v": 1,
            "id": "m_test",
            "type": kind,
            "ts": "2026-11-15T10:00:00Z",
            "payload": payload,
        }))
        .expect("构造测试信封")
    }

    #[test]
    fn new_message_module_paths_exist() {
        let _: crate::client::StartGame = crate::client::StartGame {};
        let _: crate::client::SetBotFill = crate::client::SetBotFill { enabled: true };
        let _: crate::server::HeroPickStarted = crate::server::HeroPickStarted {
            deadline: "2026-11-15T10:00:30Z".to_string(),
            candidates: Vec::new(),
            players: Vec::new(),
        };
    }

    #[test]
    fn client_envelope_parses_new_messages() {
        assert!(matches!(
            client_envelope("StartGame", serde_json::json!({})).parse(),
            Ok(ClientMessage::StartGame(_))
        ));
        match client_envelope("SetBotFill", serde_json::json!({ "enabled": true })).parse() {
            Ok(ClientMessage::SetBotFill(body)) => assert!(body.enabled),
            other => panic!("未按 SetBotFill 分派: {other:?}"),
        }
    }

    #[test]
    fn server_envelope_dispatches_hero_pick_started() {
        let payload = serde_json::json!({
            "deadline": "2026-11-15T10:00:30Z",
            "candidates": ["darwin"],
            "players": [{
                "playerId": "p1",
                "seat": 0,
                "displayName": "Ada",
                "isBot": false,
                "picked": false,
            }],
        });
        let envelope = ServerEnvelope::new(
            "HeroPickStarted",
            1,
            payload.as_object().expect("对象载荷").clone(),
        )
        .expect("HeroPickStarted 是已知 s2c 类型");
        match envelope.parse() {
            Ok(ServerMessage::HeroPickStarted(body)) => {
                assert_eq!(body.candidates, vec!["darwin".to_string()]);
                assert_eq!(body.players.len(), 1);
            }
            other => panic!("未按 HeroPickStarted 分派: {other:?}"),
        }
    }
}
