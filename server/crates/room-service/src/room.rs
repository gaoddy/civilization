//! 房间状态机与大堂流程。
//!
//! 房间只保存大厅阶段的状态：座位、准备标记、英雄选择。对局状态由 Task 9 的
//! `GameTask` 持有，本模块只负责判定能否开局并交出英雄表。
//!
//! 确定性约定：
//! - 座位从 0 起，0 号座位是房主；`slots` 用 `BTreeMap<Seat, _>` 保持座位顺序。
//! - 房间码由注册表的单调计数器派生，6 位大写字母数字，排除易混字符 `0O1I`。
//! - 本模块不读系统时间，也不持有随机源。

use std::collections::BTreeMap;
use std::fmt;

use protocol::RoomUpdatePlayersItem;
use protocol::server::RoomUpdate;
use rule_engine::data::Catalog;
use rule_engine::state::{Phase, Seat};

use crate::auth::Session;
use crate::envelope::SeatChannel;

/// 每局固定 8 个座位。
pub const SEAT_COUNT: usize = 8;

/// 房间码字母表：去掉 `0`、`O`、`1`、`I`，共 32 个可读字符。
const CODE_ALPHABET: &[u8] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";

/// 房间码长度。
const CODE_LEN: usize = 6;

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
    pub hero_card_id: Option<String>,
    /// 该座位的下行信封序号分配器。
    pub channel: SeatChannel,
    /// 连接是否在线；断线重连由 Task 11 维护。
    pub connected: bool,
}

impl PlayerSlot {
    fn from_session(session: &Session) -> Self {
        Self {
            player_id: session.player_id.clone(),
            display_name: session.display_name.clone(),
            ready: false,
            hero_card_id: None,
            channel: SeatChannel::new(),
            connected: true,
        }
    }
}

/// 大厅里的一个房间。
#[derive(Debug, Clone)]
pub struct Room {
    pub id: RoomId,
    pub code: String,
    pub host_id: String,
    pub phase: Phase,
    /// 座位 → 玩家；只保存已占用座位，键即座位号（0 起）。
    slots: BTreeMap<Seat, PlayerSlot>,
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

    /// 纯查询：8 个座位全部占用、全部准备且全部选好英雄时返回 `座位 → 英雄卡`。
    ///
    /// 只读，不改变阶段；实际开局由 [`RoomRegistry::try_start`] 或 Task 9 驱动执行。
    pub fn start_ready(&self) -> Option<BTreeMap<Seat, String>> {
        if self.slots.len() != SEAT_COUNT {
            return None;
        }
        let mut heroes = BTreeMap::new();
        for (&seat, slot) in &self.slots {
            if !slot.ready {
                return None;
            }
            let hero = slot.hero_card_id.as_ref()?;
            heroes.insert(seat, hero.clone());
        }
        Some(heroes)
    }

    fn slot_mut(&mut self, player_id: &str) -> Option<&mut PlayerSlot> {
        self.slots
            .values_mut()
            .find(|slot| slot.player_id == player_id)
    }

    fn is_occupied(&self, player_id: &str) -> bool {
        self.slots.values().any(|slot| slot.player_id == player_id)
    }
}

/// 房间操作失败原因。
///
/// 变体保持稳定；下行错误码经 [`RoomError::code`] 映射为
/// `protocol/schema/v1/errors.schema.json` 中的已知取值。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomError {
    /// 房间码或房间标识不存在。
    NotFound,
    /// 8 个座位已满。
    Full,
    /// 只有房主能开始对局。
    NotHost,
    /// 其他拒绝原因；字符串是内部语义标签，不一定等于协议错误码。
    Unknown(String),
}

impl RoomError {
    /// 映射到协议错误码。
    ///
    /// schema 中不存在 `NOT_HOST`、`HERO_NOT_SELECTED`、`ROOM_NOT_READY`，
    /// 这三类统一取语义最接近的 `INVALID_STAGE`（当前状态/角色不允许该操作）；
    /// 非英雄卡取 `INVALID_CHOICE`；不在房间取现有的 `NOT_IN_ROOM`。
    pub fn code(&self) -> &str {
        match self {
            Self::NotFound => "ROOM_NOT_FOUND",
            Self::Full => "ROOM_FULL",
            Self::NotHost => "INVALID_STAGE",
            Self::Unknown(reason) => match reason.as_str() {
                "NOT_IN_ROOM" => "NOT_IN_ROOM",
                "NOT_A_HERO_CARD" => "INVALID_CHOICE",
                _ => "INVALID_STAGE",
            },
        }
    }
}

impl fmt::Display for RoomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => f.write_str("房间不存在"),
            Self::Full => f.write_str("房间已满"),
            Self::NotHost => f.write_str("只有房主可以开始对局"),
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
            next_room_seq: seed,
        }
    }

    /// 建房：创建者坐 0 号座位并成为房主，阶段为 `Lobby`。
    pub fn create(&mut self, host: &Session) -> Room {
        let (id, code) = loop {
            let sequence = self.next_room_seq;
            self.next_room_seq = self.next_room_seq.saturating_add(1);
            let id = format!("room_{sequence}");
            let code = encode_code(sequence);
            if !self.rooms.contains_key(&id) && !self.by_code.contains_key(&code) {
                break (id, code);
            }
        };

        let mut slots = BTreeMap::new();
        slots.insert(0, PlayerSlot::from_session(host));
        let room = Room {
            id: id.clone(),
            code: code.clone(),
            host_id: host.player_id.clone(),
            phase: Phase::Lobby,
            slots,
        };
        // 注册表持有权威副本；返回值供调用者立即广播大厅视图。
        self.rooms.insert(id.clone(), room.clone());
        self.by_code.insert(code, id);
        room
    }

    /// 按房间码加入，坐第一个空位（座位号从 0 起）。
    ///
    /// 同一玩家重复加入同一房间是幂等的：返回已有房间，不占新座位。
    pub fn join(&mut self, code: &str, player: &Session) -> Result<&mut Room, RoomError> {
        let room_id = self.by_code.get(code).cloned().ok_or(RoomError::NotFound)?;
        let room = self.rooms.get_mut(&room_id).expect("索引与房间表一致");

        if room.is_occupied(&player.player_id) {
            return Ok(room);
        }

        let seat = (0..SEAT_COUNT as Seat)
            .find(|seat| !room.slots.contains_key(seat))
            .ok_or(RoomError::Full)?;
        room.slots.insert(seat, PlayerSlot::from_session(player));
        Ok(room)
    }

    /// 离开房间。座位立即空出且编号不重排；之后加入的玩家占用最小空位。
    ///
    /// 房主离开时，房主转给座位号最小的剩余玩家；最后一名玩家离开后房间销毁。
    pub fn leave(&mut self, room_id: &str, player_id: &str) -> Result<(), RoomError> {
        let Some(room) = self.rooms.get_mut(room_id) else {
            return Err(RoomError::NotFound);
        };
        let Some(seat) = room.seat_of(player_id) else {
            return Err(RoomError::Unknown("NOT_IN_ROOM".to_string()));
        };
        room.slots.remove(&seat);

        if room.host_id != player_id {
            return Ok(());
        }

        let next_host = room
            .slots
            .iter()
            .next()
            .map(|(_, slot)| slot.player_id.clone());
        match next_host {
            Some(next_host) => {
                room.host_id = next_host;
                Ok(())
            }
            None => {
                let code = room.code.clone();
                self.rooms.remove(room_id);
                self.by_code.remove(&code);
                Ok(())
            }
        }
    }

    /// 设置准备状态；准备前必须先选英雄。
    pub fn ready(&mut self, room_id: &str, player_id: &str, ready: bool) -> Result<(), RoomError> {
        let room = self.rooms.get_mut(room_id).ok_or(RoomError::NotFound)?;
        let slot = room
            .slot_mut(player_id)
            .ok_or_else(|| RoomError::Unknown("NOT_IN_ROOM".to_string()))?;
        if ready && slot.hero_card_id.is_none() {
            return Err(RoomError::Unknown("HERO_NOT_SELECTED".to_string()));
        }
        slot.ready = ready;
        Ok(())
    }

    /// 选择英雄。卡必须存在且 `is_hero`；允许与其他座位重复选择同一张卡。
    pub fn select_hero(
        &mut self,
        room_id: &str,
        player_id: &str,
        card_id: &str,
        catalog: &Catalog,
    ) -> Result<(), RoomError> {
        let room = self.rooms.get_mut(room_id).ok_or(RoomError::NotFound)?;
        if !room.is_occupied(player_id) {
            return Err(RoomError::Unknown("NOT_IN_ROOM".to_string()));
        }
        if !catalog.cards.get(card_id).is_some_and(|card| card.is_hero) {
            return Err(RoomError::Unknown("NOT_A_HERO_CARD".to_string()));
        }

        let slot = room.slot_mut(player_id).expect("上面已确认玩家在房间内");
        slot.hero_card_id = Some(card_id.to_string());
        Ok(())
    }

    /// 纯查询：房间是否满足开局条件；满足时返回 `座位 → 英雄卡`。
    ///
    /// 房间不存在或不满足条件都返回 `None`，不改变任何状态。
    pub fn start_ready(&self, room_id: &str) -> Option<BTreeMap<Seat, String>> {
        self.rooms.get(room_id)?.start_ready()
    }

    /// 房主发起开局：校验房主、人数与准备状态，成功后把阶段推进到 `Recruit`。
    ///
    /// 与纯查询 [`Self::start_ready`] 分开，便于轮询与命令处理各自调用。
    pub fn try_start(
        &mut self,
        room_id: &str,
        player_id: &str,
    ) -> Result<BTreeMap<Seat, String>, RoomError> {
        let room = self.rooms.get_mut(room_id).ok_or(RoomError::NotFound)?;
        if room.host_id != player_id {
            return Err(RoomError::NotHost);
        }
        if room.phase != Phase::Lobby {
            return Err(RoomError::Unknown("INVALID_STAGE".to_string()));
        }
        let heroes = room
            .start_ready()
            .ok_or_else(|| RoomError::Unknown("ROOM_NOT_READY".to_string()))?;
        room.phase = Phase::Recruit;
        Ok(heroes)
    }

    /// 只读查询房间。
    pub fn room(&self, room_id: &str) -> Option<&Room> {
        self.rooms.get(room_id)
    }

    /// 构造大厅下行视图；玩家按座位号升序排列，房间不存在返回 `None`。
    pub fn room_update(&self, room_id: &str) -> Option<RoomUpdate> {
        let room = self.rooms.get(room_id)?;
        Some(RoomUpdate {
            code: room.code.clone(),
            host_id: room.host_id.clone(),
            phase: protocol_phase(room.phase),
            players: room
                .slots
                .values()
                .map(|slot| RoomUpdatePlayersItem {
                    display_name: slot.display_name.clone(),
                    hero_card_id: slot.hero_card_id.clone(),
                    is_host: slot.player_id == room.host_id,
                    player_id: slot.player_id.clone(),
                    ready: slot.ready,
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
