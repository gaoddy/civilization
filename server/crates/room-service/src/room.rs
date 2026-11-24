//! 房间状态机与大堂流程。
//!
//! 房间只保存大厅阶段的状态：座位、准备标记与补位开关。对局状态与选英雄由
//! `GameTask` 持有，本模块只负责校验开局条件并交出开局计划。
//!
//! 确定性约定：
//! - 座位从 0 起，0 号座位是房主；`slots` 用 `BTreeMap<Seat, _>` 保持座位顺序。
//! - 房间码由注册表的单调计数器派生，6 位大写字母数字，排除易混字符 `0O1I`。
//! - 本模块不读系统时间；邀请 token 使用进程内随机 UUID，不影响对局确定性。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use protocol::RoomUpdatePlayersItem;
use protocol::server::RoomUpdate;
use rule_engine::data::{Catalog, Playstyle, hero_in_pool};
use rule_engine::state::{Phase, Seat};
use uuid::Uuid;

use crate::auth::Session;
use crate::envelope::SeatChannel;
use crate::game::RECONNECT_GRACE_MS;

/// 每局固定 8 个座位。
pub const SEAT_COUNT: usize = 8;

/// 房间码字母表：去掉 `0`、`O`、`1`、`I`，共 32 个可读字符。
const CODE_ALPHABET: &[u8] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";

/// 房间码长度。
const CODE_LEN: usize = 6;

/// 每名玩家分到的候选英雄数量。
pub const CANDIDATE_COUNT: usize = 4;

/// 房间邀请 token 有效期：60 秒。
pub const ROOM_INVITE_TTL_MS: u64 = 60_000;

/// 房间的一次性邀请 token。
#[derive(Debug, Clone)]
pub struct InviteToken {
    pub room_id: RoomId,
    pub expires_at_ms: u64,
}

/// 新建邀请 token 的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedInvite {
    pub token: String,
    pub room_code: String,
    pub expires_at_ms: u64,
}

/// 房间标识；进程内唯一，不对外暴露给玩家。
pub type RoomId = String;

/// 一个座位上的玩家。
///
/// `channel` 与 `connected` 是连接层状态，随房间一起保存，便于 Task 9/11 复用；
/// 大厅协议 `RoomUpdate` 不包含这两项。
#[derive(Debug, Clone)]
pub struct PlayerSlot {
    pub player_id: String,
    pub display_name: String,
    pub ready: bool,
    /// 是否是开局时补入的机器人；机器人不参与离开、踢人、断线宽限与房主转移。
    pub is_bot: bool,
    /// 该座位的下行信封序号分配器。
    pub channel: SeatChannel,
    /// 连接是否在线；大厅断线与重连由连接层维护。
    pub connected: bool,
    /// 大厅断线时刻（Unix epoch 毫秒）；在线时为 `None`。
    pub disconnected_at_ms: Option<u64>,
}

impl PlayerSlot {
    fn from_session(session: &Session) -> Self {
        Self {
            player_id: session.player_id.clone(),
            display_name: session.display_name.clone(),
            ready: false,
            is_bot: false,
            channel: SeatChannel::new(),
            connected: true,
            disconnected_at_ms: None,
        }
    }

    /// 为开局计划构造一个机器人座位；`n` 是机器人按座位升序的序号（1 起）。
    fn bot(room_id: &str, seat: Seat, n: usize) -> Self {
        Self {
            player_id: format!("bot:{room_id}:{seat}"),
            display_name: format!("机器人 {n}"),
            ready: true,
            is_bot: true,
            channel: SeatChannel::new(),
            connected: true,
            disconnected_at_ms: None,
        }
    }
}

/// 大厅里的一个房间。
///
/// 阶段为 `Lobby` 时成员集合可变；进入 `Recruit` 后成员集合冻结，加入、准备、
/// 开始与离开都会被拒绝。选英雄在 `GameTask` 内部进行，对局中的断线也由
/// `GameTask` 维护。
#[derive(Debug, Clone)]
pub struct Room {
    pub id: RoomId,
    pub code: String,
    pub host_id: String,
    pub phase: Phase,
    /// 本局种子；创建时由房间码派生。
    pub seed: u64,
    /// 本局启用的 5 个玩法；创建房间时由本局种子抽取。
    pub playstyles: [Playstyle; 5],
    /// 是否自动匹配局；房间局为 `false`，匹配局为 `true`，只有匹配局计入天梯。
    pub ranked: bool,
    /// 开始时空位是否用机器人补满；只在 `Lobby` 阶段可由房主修改。
    pub bot_fill: bool,
    /// 座位 → 玩家；只保存已占用座位，键即座位号（0 起）。
    slots: BTreeMap<Seat, PlayerSlot>,
    /// 本局各座位的候选英雄；`prepare_start` 规划、`commit_start` 写入。
    candidates: BTreeMap<Seat, Vec<String>>,
    /// 一次性邀请 token；只在大厅阶段创建，消费或过期即移除。
    invite_tokens: BTreeMap<String, InviteToken>,
}

/// 开局计划：`prepare_start` 只读构造，`commit_start` 才是写入点。
#[derive(Debug, Clone)]
pub struct StartPlan {
    pub room_id: RoomId,
    pub seed: u64,
    pub playstyles: [Playstyle; 5],
    pub ranked: bool,
    /// 8 条，下标即座位号；真人座位为注册表副本，机器人座位为新生成。
    pub slots: Vec<PlayerSlot>,
    /// 座位 → 本人候选（8 个座位都有）。
    pub candidates: BTreeMap<Seat, Vec<String>>,
}

impl Room {
    /// 当前所有已占用座位，按座位号升序。
    pub fn slots(&self) -> &BTreeMap<Seat, PlayerSlot> {
        &self.slots
    }

    /// 指定座位上的玩家。
    pub fn slot(&self, seat: Seat) -> Option<&PlayerSlot> {
        self.slots.get(&seat)
    }

    /// 玩家所在座位；不在房间里返回 `None`。
    pub fn seat_of(&self, player_id: &str) -> Option<Seat> {
        self.slots
            .iter()
            .find(|(_, slot)| slot.player_id == player_id)
            .map(|(&seat, _)| seat)
    }

    /// 按座位号升序克隆所有玩家，供对局驱动构造 `GameTask`。
    pub fn players(&self) -> Vec<PlayerSlot> {
        self.slots.values().cloned().collect()
    }

    /// 本局各座位的候选英雄；开局前为空表，`commit_start` 后覆盖 8 个座位。
    pub fn candidates(&self) -> &BTreeMap<Seat, Vec<String>> {
        &self.candidates
    }

    /// 真人座位数；机器人不计入。
    pub fn human_count(&self) -> usize {
        self.slots.values().filter(|slot| !slot.is_bot).count()
    }

    /// 所有真人座位是否都已准备；没有真人时返回 `false`。
    pub fn all_humans_ready(&self) -> bool {
        self.human_count() > 0
            && self
                .slots
                .values()
                .filter(|slot| !slot.is_bot)
                .all(|slot| slot.ready)
    }

    fn slot_mut(&mut self, player_id: &str) -> Option<&mut PlayerSlot> {
        self.slots
            .values_mut()
            .find(|slot| slot.player_id == player_id)
    }

    /// 按 `player_id` 取可变座位槽位；供连接层维护该座位的下行序号。
    pub fn slot_entry_mut(&mut self, player_id: &str) -> Option<&mut PlayerSlot> {
        self.slot_mut(player_id)
    }

    fn is_occupied(&self, player_id: &str) -> bool {
        self.slots.values().any(|slot| slot.player_id == player_id)
    }
}

/// 房间操作失败原因。
///
/// 变体保持稳定；下行错误码经 [`RoomError::code`] 精确映射为
/// `protocol/schema/v1/errors.schema.json` 中的 `ErrorCode`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomError {
    /// 房间码或房间标识不存在。
    NotFound,
    /// 8 个座位已满。
    Full,
    /// 只有房主能开始对局。
    NotHost,
    /// 非 `Lobby` 阶段不允许改变大厅成员或大厅状态。
    WrongPhase,
    /// 人数不足或未全部准备，不能开始。
    RoomNotReady,
    /// 玩家不在房间里。
    NotInRoom,
    /// 操作目标非法，例如房主试图移除自己。
    InvalidTarget,
    /// 其他保留原因；字符串是内部说明，不作为协议错误码。
    Unknown(String),
    /// 邀请 token 不存在或已被消费。
    InviteNotFound,
    /// 邀请 token 已过期。
    InviteExpired,
}

impl RoomError {
    /// 映射到协议错误码。
    ///
    /// 返回生成枚举 [`protocol::ErrorCode`] 而非字符串，映射写错会编译失败。
    /// `Unknown` 没有对应的客户端可纠正语义，统一退化 `INTERNAL_ERROR`。
    pub fn code(&self) -> protocol::ErrorCode {
        use protocol::ErrorCode;
        match self {
            Self::NotFound => ErrorCode::RoomNotFound,
            Self::Full => ErrorCode::RoomFull,
            Self::NotHost => ErrorCode::NotHost,
            Self::WrongPhase => ErrorCode::InvalidStage,
            Self::RoomNotReady => ErrorCode::RoomNotReady,
            Self::NotInRoom => ErrorCode::NotInRoom,
            Self::InvalidTarget => ErrorCode::InvalidChoice,
            Self::InviteNotFound => ErrorCode::InviteNotFound,
            Self::InviteExpired => ErrorCode::InviteExpired,
            Self::Unknown(_) => ErrorCode::InternalError,
        }
    }
}

impl fmt::Display for RoomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => f.write_str("房间不存在"),
            Self::Full => f.write_str("房间已满"),
            Self::NotHost => f.write_str("只有房主可以开始对局"),
            Self::WrongPhase => f.write_str("当前阶段不允许该操作"),
            Self::RoomNotReady => f.write_str("人数不足或尚未全部准备"),
            Self::NotInRoom => f.write_str("玩家不在房间内"),
            Self::InvalidTarget => f.write_str("不能移除自己"),
            Self::InviteNotFound => f.write_str("邀请不存在或已使用"),
            Self::InviteExpired => f.write_str("邀请已过期"),
            Self::Unknown(reason) => f.write_str(reason),
        }
    }
}

impl std::error::Error for RoomError {}

/// 进程内房间注册表。
///
/// `by_code` 与 `rooms` 都是 `BTreeMap`，遍历顺序确定；房间码计数器只增不减，
/// 保证同输入同输出且测试可预期。
#[derive(Debug)]
pub struct RoomRegistry {
    by_code: BTreeMap<String, RoomId>,
    rooms: BTreeMap<RoomId, Room>,
    /// 邀请 token → 房间；房间销毁或 token 消费/过期时同步移除。
    token_rooms: BTreeMap<String, RoomId>,
    next_room_seq: u64,
}

impl RoomRegistry {
    /// 以 0 为计数器起点创建注册表；首个房间码为 `222222`。
    pub fn new() -> Self {
        Self::with_seed(0)
    }

    /// 指定房间码计数器起点，便于测试断言具体房间码。
    pub fn with_seed(seed: u64) -> Self {
        Self {
            by_code: BTreeMap::new(),
            rooms: BTreeMap::new(),
            token_rooms: BTreeMap::new(),
            next_room_seq: seed,
        }
    }

    /// 建房：创建者坐 0 号座位并成为房主，阶段为 `Lobby`。
    ///
    /// 等待区不再预先分配英雄候选；候选在房主开始对局时由 `prepare_start`
    /// 规划，成功后才由 `commit_start` 写入。
    pub fn create(&mut self, host: &Session, _catalog: &Catalog) -> Room {
        let (id, code) = self.allocate_id();

        let mut slots = BTreeMap::new();
        slots.insert(0, PlayerSlot::from_session(host));
        // 本局种子只算一次：玩法抽取与英雄候选分配共用同一个值。
        let seed = seed_for_code(&code);
        let room = Room {
            id: id.clone(),
            code: code.clone(),
            host_id: host.player_id.clone(),
            phase: Phase::Lobby,
            seed,
            // 玩法在房间创建时确定：之后到本局结束保持不变。
            playstyles: rule_engine::playstyles::draw(seed),
            ranked: false,
            bot_fill: true,
            slots,
            candidates: BTreeMap::new(),
            invite_tokens: BTreeMap::new(),
        };
        // 注册表持有权威副本；返回值供调用者立即广播大厅视图。
        self.rooms.insert(id.clone(), room.clone());
        self.by_code.insert(code, id);
        room
    }

    /// 自动匹配建房：跳过大厅准备流程，8 名玩家直接满座并推进到 `Recruit`。
    ///
    /// 座位按传入顺序分配，`host_id` 记为第一人；不产生机器人，`bot_fill = false`。
    /// 候选由 [`allocate_candidates`] 按本局种子写入；英雄由对局内的选英雄阶段确定，
    /// 这里不再自动选择。匹配局不暴露房主操作（无邀请、无踢人、无解散）。
    ///
    /// # Errors
    ///
    /// 人数不是 8 时返回 [`RoomError::RoomNotReady`]，注册表不留下半成品房间。
    pub fn create_match(
        &mut self,
        players: &[Session],
        catalog: &Catalog,
    ) -> Result<Room, RoomError> {
        if players.len() != SEAT_COUNT {
            return Err(RoomError::RoomNotReady);
        }
        let (id, code) = self.allocate_id();
        let seed = seed_for_code(&code);
        let playstyles = rule_engine::playstyles::draw(seed);
        let mut slots = BTreeMap::new();
        for (seat, player) in players.iter().enumerate() {
            let mut slot = PlayerSlot::from_session(player);
            // 匹配局没有大厅确认步骤：座位直接视为已准备。
            slot.ready = true;
            slots.insert(seat as Seat, slot);
        }
        let room = Room {
            id: id.clone(),
            code: code.clone(),
            host_id: players[0].player_id.clone(),
            phase: Phase::Recruit,
            seed,
            playstyles,
            ranked: true,
            bot_fill: false,
            slots,
            candidates: allocate_candidates(seed, &playstyles, catalog),
            invite_tokens: BTreeMap::new(),
        };
        self.rooms.insert(id.clone(), room.clone());
        self.by_code.insert(code, id);
        Ok(room)
    }

    /// 分配下一个房间标识与房间码；同一进程内单调且不复用。
    fn allocate_id(&mut self) -> (RoomId, String) {
        loop {
            let sequence = self.next_room_seq;
            self.next_room_seq = self.next_room_seq.saturating_add(1);
            let id = format!("room_{sequence}");
            let code = encode_code(sequence);
            if !self.rooms.contains_key(&id) && !self.by_code.contains_key(&code) {
                break (id, code);
            }
        }
    }

    /// 按玩家查所在房间与座位；不在任何房间返回 `None`。
    pub fn room_of_player(&self, player_id: &str) -> Option<(&Room, Seat)> {
        self.rooms
            .values()
            .find_map(|room| room.seat_of(player_id).map(|seat| (room, seat)))
    }

    /// 按房间码加入，坐第一个空位（座位号从 0 起）。
    ///
    /// 同一玩家重复加入同一房间是幂等的：返回已有房间，不占新座位。
    /// 非 `Lobby` 阶段成员集合已冻结，先于满座判断返回 [`RoomError::WrongPhase`]。
    pub fn join(
        &mut self,
        code: &str,
        player: &Session,
        _catalog: &Catalog,
    ) -> Result<&mut Room, RoomError> {
        let room_id = self.by_code.get(code).cloned().ok_or(RoomError::NotFound)?;
        let room = self.rooms.get_mut(&room_id).expect("索引与房间表一致");

        if room.phase != Phase::Lobby {
            return Err(RoomError::WrongPhase);
        }

        if room.is_occupied(&player.player_id) {
            return Ok(room);
        }

        let seat = (0..SEAT_COUNT as Seat)
            .find(|seat| !room.slots.contains_key(seat))
            .ok_or(RoomError::Full)?;
        room.slots.insert(seat, PlayerSlot::from_session(player));
        Ok(room)
    }

    /// 为玩家所在大厅房间创建一次性邀请 token。
    ///
    /// # Errors
    ///
    /// 不在房间为 [`RoomError::NotInRoom`]；房间已离开大厅阶段为
    /// [`RoomError::WrongPhase`]。
    pub fn create_invite(
        &mut self,
        player_id: &str,
        now_ms: u64,
    ) -> Result<CreatedInvite, RoomError> {
        let room_id = self
            .room_of_player(player_id)
            .map(|(room, _seat)| room.id.clone())
            .ok_or(RoomError::NotInRoom)?;
        let room = self.rooms.get_mut(&room_id).expect("索引与房间表一致");
        if room.phase != Phase::Lobby {
            return Err(RoomError::WrongPhase);
        }
        let token = Uuid::new_v4().simple().to_string();
        let expires_at_ms = now_ms.saturating_add(ROOM_INVITE_TTL_MS);
        room.invite_tokens.insert(
            token.clone(),
            InviteToken {
                room_id: room_id.clone(),
                expires_at_ms,
            },
        );
        self.token_rooms.insert(token.clone(), room_id);
        Ok(CreatedInvite {
            token,
            room_code: room.code.clone(),
            expires_at_ms,
        })
    }

    /// 消费邀请 token 并入房；token 只能用一次。
    ///
    /// 与 [`RoomRegistry::join`] 共用座位规则：大厅阶段、有空位；同一玩家重复加入
    /// 幂等。token 缺失或过期分别返回 [`RoomError::InviteNotFound`] /
    /// [`RoomError::InviteExpired`]，房间状态不变。
    ///
    /// # Errors
    ///
    /// 见上；房间已开局为 [`RoomError::WrongPhase`]，满座为 [`RoomError::Full`]。
    pub fn redeem_invite(
        &mut self,
        token: &str,
        player: &Session,
        _catalog: &Catalog,
        now_ms: u64,
    ) -> Result<&mut Room, RoomError> {
        let Some(room_id) = self.token_rooms.get(token).cloned() else {
            return Err(RoomError::InviteNotFound);
        };
        let Some(invite) = self
            .rooms
            .get(&room_id)
            .and_then(|room| room.invite_tokens.get(token))
            .cloned()
        else {
            self.token_rooms.remove(token);
            return Err(RoomError::InviteNotFound);
        };
        if now_ms >= invite.expires_at_ms {
            self.remove_token(token);
            return Err(RoomError::InviteExpired);
        }
        // 消费即删除；后续阶段、容量与座位判定失败也不恢复 token。
        self.remove_token(token);

        let room = self.rooms.get_mut(&room_id).expect("索引与房间表一致");
        if room.phase != Phase::Lobby {
            return Err(RoomError::WrongPhase);
        }
        if room.is_occupied(&player.player_id) {
            return Ok(room);
        }
        let seat = (0..SEAT_COUNT as Seat)
            .find(|seat| !room.slots.contains_key(seat))
            .ok_or(RoomError::Full)?;
        room.slots.insert(seat, PlayerSlot::from_session(player));
        Ok(room)
    }

    /// 从房间与索引中同时删除邀请 token。
    fn remove_token(&mut self, token: &str) {
        if let Some(room_id) = self.token_rooms.remove(token)
            && let Some(room) = self.rooms.get_mut(&room_id)
        {
            room.invite_tokens.remove(token);
        }
    }

    /// 移除 `seat` 上的座位，并维护房主与房间生命周期。
    ///
    /// 房主座位被移除时，房主转给座位号最小的剩余成员；最后一人移除后
    /// 同时删除 `rooms` 与 `by_code`。调用方负责阶段与权限校验。
    fn remove_slot(&mut self, room_id: &str, seat: Seat) {
        let Some(room) = self.rooms.get_mut(room_id) else {
            return;
        };
        let Some(removed) = room.slots.remove(&seat) else {
            return;
        };
        if room.host_id != removed.player_id {
            return;
        }

        // 机器人不能接任房主：只剩机器人时房间直接销毁。
        let next_host = room
            .slots
            .iter()
            .find(|(_, slot)| !slot.is_bot)
            .map(|(_, slot)| slot.player_id.clone());
        match next_host {
            Some(next_host) => room.host_id = next_host,
            None => {
                let code = room.code.clone();
                let tokens: Vec<String> = room.invite_tokens.keys().cloned().collect();
                self.rooms.remove(room_id);
                self.by_code.remove(&code);
                for token in tokens {
                    self.token_rooms.remove(&token);
                }
            }
        }
    }

    /// 离开房间。座位立即空出且编号不重排；之后加入的玩家占用最小空位。
    ///
    /// 房主离开时，房主转给座位号最小的剩余玩家；最后一名玩家离开后房间销毁。
    /// 只有 `Lobby` 阶段允许离开；开局后的断线由 Task 11 的 `Disconnect` 处理，
    /// 不走本方法，避免房间成员与对局座位发散。
    pub fn leave(&mut self, room_id: &str, player_id: &str) -> Result<(), RoomError> {
        let room = self.rooms.get(room_id).ok_or(RoomError::NotFound)?;
        if room.phase != Phase::Lobby {
            return Err(RoomError::WrongPhase);
        }
        let seat = room.seat_of(player_id).ok_or(RoomError::NotInRoom)?;
        if room.slot(seat).is_some_and(|slot| slot.is_bot) {
            return Err(RoomError::NotInRoom);
        }
        self.remove_slot(room_id, seat);
        Ok(())
    }

    /// 房主移除指定成员；座位立即空出且编号不重排，不触发房主转移。
    ///
    /// 只有 `Lobby` 阶段允许移除；房主不能移除自己，机器人座位不可移除。
    pub fn kick(&mut self, room_id: &str, actor: &str, target: &str) -> Result<(), RoomError> {
        let room = self.rooms.get(room_id).ok_or(RoomError::NotFound)?;
        if room.phase != Phase::Lobby {
            return Err(RoomError::WrongPhase);
        }
        if room.host_id != actor {
            return Err(RoomError::NotHost);
        }
        if target == actor {
            return Err(RoomError::InvalidTarget);
        }
        let seat = room.seat_of(target).ok_or(RoomError::NotInRoom)?;
        if room.slot(seat).is_some_and(|slot| slot.is_bot) {
            return Err(RoomError::NotInRoom);
        }
        self.remove_slot(room_id, seat);
        Ok(())
    }

    /// 房主解散房间：从注册表与房间码索引删除，返回全部成员 playerId（座位升序）。
    ///
    /// 只有 `Lobby` 阶段允许解散。
    pub fn dissolve(&mut self, room_id: &str, actor: &str) -> Result<Vec<String>, RoomError> {
        let room = self.rooms.get(room_id).ok_or(RoomError::NotFound)?;
        if room.phase != Phase::Lobby {
            return Err(RoomError::WrongPhase);
        }
        if room.host_id != actor {
            return Err(RoomError::NotHost);
        }
        let members = room
            .slots
            .values()
            .map(|slot| slot.player_id.clone())
            .collect();
        self.remove(room_id);
        Ok(members)
    }

    /// 设置准备状态；等待区不再要求先选英雄。
    ///
    /// 非 `Lobby` 阶段成员集合已冻结，返回 [`RoomError::WrongPhase`]。
    pub fn ready(&mut self, room_id: &str, player_id: &str, ready: bool) -> Result<(), RoomError> {
        let room = self.rooms.get_mut(room_id).ok_or(RoomError::NotFound)?;
        if room.phase != Phase::Lobby {
            return Err(RoomError::WrongPhase);
        }
        let slot = room.slot_mut(player_id).ok_or(RoomError::NotInRoom)?;
        slot.ready = ready;
        Ok(())
    }

    /// 标记座位离线但保留座位，用于大厅断线宽限。
    pub fn mark_disconnected(
        &mut self,
        room_id: &str,
        player_id: &str,
        now_ms: u64,
    ) -> Result<(), RoomError> {
        let room = self.rooms.get_mut(room_id).ok_or(RoomError::NotFound)?;
        let slot = room.slot_mut(player_id).ok_or(RoomError::NotInRoom)?;
        slot.connected = false;
        slot.disconnected_at_ms = Some(now_ms);
        Ok(())
    }

    /// 标记座位重新在线并清除断线时刻；只改在线标记，座位、候选与阶段不变。
    pub fn mark_reconnected(&mut self, room_id: &str, player_id: &str) -> Result<(), RoomError> {
        let room = self.rooms.get_mut(room_id).ok_or(RoomError::NotFound)?;
        let slot = room.slot_mut(player_id).ok_or(RoomError::NotInRoom)?;
        slot.connected = true;
        slot.disconnected_at_ms = None;
        Ok(())
    }

    /// 释放指定房间里断线超过 [`RECONNECT_GRACE_MS`] 的座位，返回被释放的 player_id。
    ///
    /// 只处理一个大厅房间：房间 task 各管自己，避免某个房间的 tick 释放别人房间的
    /// 座位却无法通知对方广播。对局中的断线由 `GameTask` 自己维护，不走这里。
    pub fn tick_disconnects(&mut self, room_id: &str, now_ms: u64) -> Vec<String> {
        let Some(room) = self.rooms.get(room_id) else {
            return Vec::new();
        };
        if room.phase != Phase::Lobby {
            return Vec::new();
        }
        let victims: Vec<String> = room
            .slots
            .values()
            .filter(|slot| {
                !slot.is_bot
                    && slot
                        .disconnected_at_ms
                        .is_some_and(|at| now_ms.saturating_sub(at) >= RECONNECT_GRACE_MS)
            })
            .map(|slot| slot.player_id.clone())
            .collect();
        victims
            .into_iter()
            .filter(|player_id| self.leave(room_id, player_id).is_ok())
            .collect()
    }

    /// 房主切换空位是否用机器人补满；只在 `Lobby` 阶段、只有房主能修改。
    pub fn set_bot_fill(
        &mut self,
        room_id: &str,
        actor: &str,
        enabled: bool,
    ) -> Result<(), RoomError> {
        let room = self.rooms.get_mut(room_id).ok_or(RoomError::NotFound)?;
        if room.host_id != actor {
            return Err(RoomError::NotHost);
        }
        if room.phase != Phase::Lobby {
            return Err(RoomError::WrongPhase);
        }
        room.bot_fill = enabled;
        Ok(())
    }

    /// 构造开局计划：校验房主、阶段、真人准备与补位条件，**不修改注册表**。
    ///
    /// 机器人按座位号从小到大补入空位，序号从 1 起连续编号；候选按本局种子在
    /// 房间副本上分配。调用方应先据此构造对局驱动，成功后再调用
    /// [`Self::commit_start`] 写入。
    ///
    /// # Errors
    ///
    /// 房间不存在 [`RoomError::NotFound`]；非房主 [`RoomError::NotHost`]；
    /// 非 `Lobby` [`RoomError::WrongPhase`]；没有真人、真人未全部准备，
    /// 或补位关闭但真人不足 8 时返回 [`RoomError::RoomNotReady`]。
    pub fn prepare_start(
        &self,
        room_id: &str,
        actor: &str,
        catalog: &Catalog,
    ) -> Result<StartPlan, RoomError> {
        let room = self.rooms.get(room_id).ok_or(RoomError::NotFound)?;
        if room.host_id != actor {
            return Err(RoomError::NotHost);
        }
        if room.phase != Phase::Lobby {
            return Err(RoomError::WrongPhase);
        }
        let humans = room.human_count();
        if humans == 0 || !room.all_humans_ready() {
            return Err(RoomError::RoomNotReady);
        }
        if !room.bot_fill && humans != SEAT_COUNT {
            return Err(RoomError::RoomNotReady);
        }

        let mut slots = Vec::with_capacity(SEAT_COUNT);
        let mut bot_index = 0;
        for seat in 0..SEAT_COUNT as Seat {
            match room.slots.get(&seat) {
                Some(slot) => slots.push(slot.clone()),
                None => {
                    bot_index += 1;
                    slots.push(PlayerSlot::bot(room_id, seat, bot_index));
                }
            }
        }

        let candidates = if room.candidates.len() == SEAT_COUNT {
            room.candidates.clone()
        } else {
            allocate_candidates(room.seed, &room.playstyles, catalog)
        };

        Ok(StartPlan {
            room_id: room.id.clone(),
            seed: room.seed,
            playstyles: room.playstyles,
            ranked: room.ranked,
            slots,
            candidates,
        })
    }

    /// 提交开局计划：写入机器人座位与候选，并把阶段推进到 `Recruit`。
    ///
    /// 计划中的机器人座位若已被占用，返回 [`RoomError::WrongPhase`] 且不写入任何
    /// 状态；重复提交同样被阶段校验拒绝。
    pub fn commit_start(&mut self, room_id: &str, plan: StartPlan) -> Result<(), RoomError> {
        let room = self.rooms.get_mut(room_id).ok_or(RoomError::NotFound)?;
        if room.phase != Phase::Lobby {
            return Err(RoomError::WrongPhase);
        }
        if plan
            .slots
            .iter()
            .enumerate()
            .any(|(seat, slot)| slot.is_bot && room.slots.contains_key(&(seat as Seat)))
        {
            return Err(RoomError::WrongPhase);
        }

        for (seat, slot) in plan.slots.into_iter().enumerate() {
            if slot.is_bot {
                room.slots.insert(seat as Seat, slot);
            }
        }
        room.candidates = plan.candidates;
        room.phase = Phase::Recruit;
        Ok(())
    }

    /// 删除房间及其房间码索引；房间已经不存在时返回 `None`。
    ///
    /// 对局结束时由房间 task 调用：注册表是房间码回收的唯一入口，不能绕过它直接
    /// 改内部表。只按 `room_id` 定位，`by_code` 用房间自身记录的房间码同步移除。
    pub fn remove(&mut self, room_id: &str) -> Option<Room> {
        let room = self.rooms.remove(room_id)?;
        self.by_code.remove(&room.code);
        for token in room.invite_tokens.keys() {
            self.token_rooms.remove(token);
        }
        Some(room)
    }

    /// 当前存活房间数；供测试断言回收。
    pub fn len(&self) -> usize {
        self.rooms.len()
    }

    /// 是否没有任何房间。
    pub fn is_empty(&self) -> bool {
        self.rooms.is_empty()
    }

    /// 只读查询房间。
    pub fn room(&self, room_id: &str) -> Option<&Room> {
        self.rooms.get(room_id)
    }

    /// 可变查询房间；供房间任务维护大厅成员的下行序号。
    pub fn room_mut(&mut self, room_id: &str) -> Option<&mut Room> {
        self.rooms.get_mut(room_id)
    }

    /// 构造大厅下行视图：真人玩家按座位号升序排列，带 `seat` 与 `botFill`。
    ///
    /// 所有观看者共享同一载荷；等待区不再下发英雄字段。机器人座位只在开局后
    /// 存在，房间已离开大厅，因此这里不列出机器人。
    pub fn room_update(&self, room_id: &str) -> Option<RoomUpdate> {
        let room = self.rooms.get(room_id)?;
        Some(RoomUpdate {
            bot_fill: room.bot_fill,
            code: room.code.clone(),
            host_id: room.host_id.clone(),
            phase: protocol_phase(room.phase),
            players: room
                .slots
                .iter()
                .filter(|(_, slot)| !slot.is_bot)
                .map(|(&seat, slot)| RoomUpdatePlayersItem {
                    display_name: slot.display_name.clone(),
                    is_host: slot.player_id == room.host_id,
                    player_id: slot.player_id.clone(),
                    ready: slot.ready,
                    seat: i64::from(seat),
                })
                .collect(),
            room_id: room.id.clone(),
        })
    }
}

impl Default for RoomRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 按房间种子为 8 个座位分配候选英雄。
///
/// 候选只取 [`hero_in_pool`] 通过的英雄；池为空时回退到全部英雄卡。洗牌与切段
/// 都是确定性的，段号即座位号：
/// - 池 ≥ 满座总需求时每座位 4 张、房间内互不重复；
/// - 池不足时按 `(seat * 4 + i) % len` 取 `min(4, len)` 张，允许跨座位重复；
/// - 池小于 4 时每座位取整个池并至少 1 张。
fn allocate_candidates(
    seed: u64,
    playstyles: &[Playstyle; 5],
    catalog: &Catalog,
) -> BTreeMap<Seat, Vec<String>> {
    let enabled: BTreeSet<Playstyle> = playstyles.iter().copied().collect();
    let mut heroes: Vec<String> = catalog
        .heroes
        .values()
        .filter(|hero| hero_in_pool(hero, &enabled))
        .map(|hero| hero.id.clone())
        .collect();
    // 产品兜底：本局玩法过滤后一张英雄都没有时，回退到全部英雄卡。
    if heroes.is_empty() {
        heroes = catalog.heroes.keys().cloned().collect();
    }
    heroes.sort();
    shuffle(&mut heroes, seed);

    let mut candidates = BTreeMap::new();
    for seat in 0..SEAT_COUNT as Seat {
        let picked = if heroes.is_empty() {
            Vec::new()
        } else if heroes.len() >= SEAT_COUNT * CANDIDATE_COUNT {
            let start = usize::from(seat) * CANDIDATE_COUNT;
            let end = (start + CANDIDATE_COUNT).min(heroes.len());
            heroes
                .get(start..end)
                .map(<[_]>::to_vec)
                .unwrap_or_default()
        } else {
            let take = CANDIDATE_COUNT.min(heroes.len());
            (0..take)
                .map(|offset| {
                    heroes[(usize::from(seat) * CANDIDATE_COUNT + offset) % heroes.len()].clone()
                })
                .collect()
        };
        candidates.insert(seat, picked);
    }
    candidates
}

/// 房间码派生的确定性种子（FNV-1a 64 位）。
///
/// 房间码在同一进程内唯一且确定，因此「同房间码 → 同种子 → 同候选分组」成立，
/// 且不引入随机源或新依赖。
pub fn seed_for_code(code: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in code.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// 用 `seed` 做确定性 Fisher–Yates 洗牌。
///
/// 只用整数线性同余推进种子，不引入随机源；同种子同输入得到同顺序。
fn shuffle(items: &mut [String], seed: u64) {
    let mut state = seed | 1;
    for index in (1..items.len()).rev() {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let swap_with = (state >> 33) as usize % (index + 1);
        items.swap(index, swap_with);
    }
}

/// 把房间码计数器编码成固定 6 位、排除易混字符的大写串。
fn encode_code(mut value: u64) -> String {
    let radix = CODE_ALPHABET.len() as u64;
    let mut chars = [CODE_ALPHABET[0]; CODE_LEN];
    for slot in chars.iter_mut().rev() {
        *slot = CODE_ALPHABET[(value % radix) as usize];
        value /= radix;
    }
    String::from_utf8(chars.to_vec()).expect("房间码字母表只含 ASCII")
}

/// 引擎阶段转协议阶段；两边变体一一对应。
fn protocol_phase(phase: Phase) -> protocol::view::Phase {
    match phase {
        Phase::Lobby => protocol::view::Phase::Lobby,
        Phase::Recruit => protocol::view::Phase::Recruit,
        Phase::Combat => protocol::view::Phase::Combat,
        Phase::Settle => protocol::view::Phase::Settle,
        Phase::GameOver => protocol::view::Phase::GameOver,
    }
}
