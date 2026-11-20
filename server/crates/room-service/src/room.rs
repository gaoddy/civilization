//! 房间状态机与大堂流程。
//!
//! 房间只保存大厅阶段的状态：座位、准备标记、英雄选择。对局状态由 Task 9 的
//! `GameTask` 持有，本模块只负责判定能否开局并交出英雄表。
//!
//! 确定性约定：
//! - 座位从 0 起，0 号座位是房主；`slots` 用 `BTreeMap<Seat, _>` 保持座位顺序。
//! - 房间码由注册表的单调计数器派生，6 位大写字母数字，排除易混字符 `0O1I`。
//! - 本模块不读系统时间，也不持有随机源。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use protocol::RoomUpdatePlayersItem;
use protocol::server::RoomUpdate;
use rule_engine::data::{Catalog, Playstyle, hero_in_pool};
use rule_engine::state::{Phase, Seat};

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
    /// 该座位可选的英雄卡 id；只在发给本人的 `RoomUpdate` 中下发。
    pub hero_candidates: Vec<String>,
    /// 候选取自洗好序列的第几段；座位释放时随之回收，避免候选池被永久占位。
    pub candidate_segment: Option<usize>,
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
            hero_card_id: None,
            hero_candidates: Vec::new(),
            candidate_segment: None,
            channel: SeatChannel::new(),
            connected: true,
            disconnected_at_ms: None,
        }
    }
}

/// 大厅里的一个房间。
///
/// 阶段为 `Lobby` 时成员集合可变；进入 `Recruit` 后成员集合冻结，加入、准备、
/// 选英雄与离开都会被拒绝，对局中的断线由 Task 11 的 `Disconnect` 处理。
#[derive(Debug, Clone)]
pub struct Room {
    pub id: RoomId,
    pub code: String,
    pub host_id: String,
    pub phase: Phase,
    /// 本局种子；创建时由房间码派生，回房重置时由调用方换新。
    pub seed: u64,
    /// 本局启用的 5 个玩法；创建房间时由本局种子抽取，回房重置时随新种子重抽。
    pub playstyles: [Playstyle; 5],
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

    /// 指定玩家本人的候选；玩家不在房间里返回 `None`。
    pub fn candidates_of(&self, player_id: &str) -> Option<&[String]> {
        self.slots
            .values()
            .find(|slot| slot.player_id == player_id)
            .map(|slot| slot.hero_candidates.as_slice())
    }

    /// 纯查询：8 个座位全部占用、全部准备且全部选好英雄时返回 `座位 → 英雄卡`。
    ///
    /// 只读，不改变阶段；非 `Lobby` 阶段成员集合已冻结，一律返回 `None`。
    /// 实际开局由 [`RoomRegistry::try_start`] 或 Task 9 驱动执行。
    pub fn start_ready(&self) -> Option<BTreeMap<Seat, String>> {
        if self.phase != Phase::Lobby {
            return None;
        }
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

    /// 按 `player_id` 取可变座位槽位；供连接层维护该座位的下行序号。
    pub fn slot_entry_mut(&mut self, player_id: &str) -> Option<&mut PlayerSlot> {
        self.slot_mut(player_id)
    }

    fn is_occupied(&self, player_id: &str) -> bool {
        self.slots.values().any(|slot| slot.player_id == player_id)
    }

    /// 按 [`Room::seed`] 洗出的英雄序列给一名玩家切出一组候选。
    ///
    /// 候选只取 [`card_in_pool`] 通过的英雄；池为空时回退到全部英雄卡。
    /// 池大小 ≥ 满座总需求时沿用分段、房间内互不重复；池不足时按
    /// `(segment * 4 + i) % len` 取 `min(4, len)` 张，允许跨玩家重复，
    /// 且池小于 4 时每人取整个池并至少 1 张。
    ///
    /// 洗牌与切段都是确定性的：同一房间、同一批英雄卡表、同一进房顺序，
    /// 得到同一组候选。
    fn assign_candidates(&mut self, player_id: &str, catalog: &Catalog) {
        if self
            .candidates_of(player_id)
            .is_some_and(|candidates| !candidates.is_empty())
        {
            return;
        }
        let enabled: BTreeSet<Playstyle> = self.playstyles.iter().copied().collect();
        let mut heroes: Vec<String> = catalog
            .heroes
            .values()
            .filter(|hero| hero_in_pool(hero, &enabled))
            .map(|hero| hero.id.clone())
            .collect();
        // 产品兜底：本局玩法过滤后一张英雄都没有时，回退到全部英雄卡。
        if heroes.is_empty() {
            heroes = catalog
                .heroes
                .values()
                .map(|hero| hero.id.clone())
                .collect();
        }
        heroes.sort();
        shuffle(&mut heroes, self.seed);
        if heroes.is_empty() {
            return;
        }

        // 段号取当前座位里最小的未用段：座位释放后段号自动空出，候选池不会被永久占满。
        let used: Vec<usize> = self
            .slots
            .values()
            .filter_map(|slot| slot.candidate_segment)
            .collect();
        let segment = (0..)
            .find(|index| !used.contains(index))
            .unwrap_or(used.len());
        let start = segment * CANDIDATE_COUNT;

        let picked = if heroes.len() >= SEAT_COUNT * CANDIDATE_COUNT {
            // 池足够满座分段互不重复。
            if start >= heroes.len() {
                return;
            }
            let end = (start + CANDIDATE_COUNT).min(heroes.len());
            heroes[start..end].to_vec()
        } else {
            // 池不足：允许跨玩家重复；每人最多 4 张，池小于 4 时取整个池。
            let take = CANDIDATE_COUNT.min(heroes.len());
            (0..take)
                .map(|offset| heroes[(start + offset) % heroes.len()].clone())
                .collect()
        };
        if let Some(slot) = self.slot_mut(player_id) {
            slot.hero_candidates = picked;
            slot.candidate_segment = Some(segment);
        }
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
    /// 准备前必须已选英雄。
    HeroNotSelected,
    /// 人数不足、未全部准备或未全部选英雄，不能开始。
    RoomNotReady,
    /// 卡不存在或不是英雄卡。
    NotAHeroCard,
    /// 玩家不在房间里。
    NotInRoom,
    /// 操作目标非法，例如房主试图移除自己。
    InvalidTarget,
    /// 所选英雄不在本人的候选里。
    NotCandidate,
    /// 其他保留原因；字符串是内部说明，不作为协议错误码。
    Unknown(String),
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
            Self::HeroNotSelected => ErrorCode::HeroNotSelected,
            Self::RoomNotReady => ErrorCode::RoomNotReady,
            Self::NotAHeroCard => ErrorCode::InvalidChoice,
            Self::NotInRoom => ErrorCode::NotInRoom,
            Self::InvalidTarget => ErrorCode::InvalidChoice,
            Self::NotCandidate => ErrorCode::InvalidChoice,
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
            Self::HeroNotSelected => f.write_str("请先选择英雄"),
            Self::RoomNotReady => f.write_str("人数不足或尚未全部准备"),
            Self::NotAHeroCard => f.write_str("不是可选的英雄卡"),
            Self::NotInRoom => f.write_str("玩家不在房间内"),
            Self::InvalidTarget => f.write_str("不能移除自己"),
            Self::NotCandidate => f.write_str("不在可选英雄范围内"),
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
    ///
    /// 建房时立即给房主切出一组候选英雄，使房主不等其他人也能看到自己的候选。
    pub fn create(&mut self, host: &Session, catalog: &Catalog) -> Room {
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
        // 本局种子只算一次：玩法抽取与英雄候选分配共用同一个值。
        let seed = seed_for_code(&code);
        let mut room = Room {
            id: id.clone(),
            code: code.clone(),
            host_id: host.player_id.clone(),
            phase: Phase::Lobby,
            seed,
            // 玩法在房间创建时确定：必须先于英雄候选分配，之后到本局结束保持不变。
            playstyles: rule_engine::playstyles::draw(seed),
            slots,
        };
        room.assign_candidates(&host.player_id, catalog);
        // 注册表持有权威副本；返回值供调用者立即广播大厅视图。
        self.rooms.insert(id.clone(), room.clone());
        self.by_code.insert(code, id);
        room
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
        catalog: &Catalog,
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
        room.assign_candidates(&player.player_id, catalog);
        Ok(room)
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

        let next_host = room
            .slots
            .iter()
            .next()
            .map(|(_, slot)| slot.player_id.clone());
        match next_host {
            Some(next_host) => room.host_id = next_host,
            None => {
                let code = room.code.clone();
                self.rooms.remove(room_id);
                self.by_code.remove(&code);
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
        self.remove_slot(room_id, seat);
        Ok(())
    }

    /// 房主移除指定成员；座位立即空出且编号不重排，不触发房主转移。
    ///
    /// 只有 `Lobby` 阶段允许移除；房主不能移除自己。
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

    /// 设置准备状态；准备前必须先选英雄。
    ///
    /// 非 `Lobby` 阶段成员集合已冻结，返回 [`RoomError::WrongPhase`]。
    pub fn ready(&mut self, room_id: &str, player_id: &str, ready: bool) -> Result<(), RoomError> {
        let room = self.rooms.get_mut(room_id).ok_or(RoomError::NotFound)?;
        if room.phase != Phase::Lobby {
            return Err(RoomError::WrongPhase);
        }
        let slot = room.slot_mut(player_id).ok_or(RoomError::NotInRoom)?;
        if ready && slot.hero_card_id.is_none() {
            return Err(RoomError::HeroNotSelected);
        }
        slot.ready = ready;
        Ok(())
    }

    /// 选择英雄。卡必须存在于英雄池，且在本人的候选里；确认后不可改选。
    ///
    /// 非 `Lobby` 阶段成员集合已冻结，返回 [`RoomError::WrongPhase`]。
    pub fn select_hero(
        &mut self,
        room_id: &str,
        player_id: &str,
        card_id: &str,
        catalog: &Catalog,
    ) -> Result<(), RoomError> {
        let room = self.rooms.get_mut(room_id).ok_or(RoomError::NotFound)?;
        if room.phase != Phase::Lobby {
            return Err(RoomError::WrongPhase);
        }
        if !room.is_occupied(player_id) {
            return Err(RoomError::NotInRoom);
        }
        if !catalog.heroes.contains_key(card_id) {
            return Err(RoomError::NotAHeroCard);
        }
        if room.slot_mut(player_id).is_some_and(|slot| slot.ready) {
            return Err(RoomError::WrongPhase);
        }
        if !room
            .candidates_of(player_id)
            .is_some_and(|candidates| candidates.iter().any(|id| id == card_id))
        {
            return Err(RoomError::NotCandidate);
        }

        let slot = room.slot_mut(player_id).expect("上面已确认玩家在房间内");
        slot.hero_card_id = Some(card_id.to_string());
        Ok(())
    }

    /// 指定玩家的候选英雄；房间或玩家不存在返回 `None`。
    pub fn candidates_of(&self, room_id: &str, player_id: &str) -> Option<Vec<String>> {
        self.rooms
            .get(room_id)?
            .candidates_of(player_id)
            .map(<[_]>::to_vec)
    }

    /// 标记座位离线但保留座位与候选，用于大厅断线宽限。
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

    /// 标记座位重新在线并清除断线时刻；座位与候选保持不变。
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
                slot.disconnected_at_ms
                    .is_some_and(|at| now_ms.saturating_sub(at) >= RECONNECT_GRACE_MS)
            })
            .map(|slot| slot.player_id.clone())
            .collect();
        victims
            .into_iter()
            .filter(|player_id| self.leave(room_id, player_id).is_ok())
            .collect()
    }

    /// 纯查询：房间是否满足开局条件；满足时返回 `座位 → 英雄卡`。
    ///
    /// 房间不存在或不满足条件都返回 `None`，不改变任何状态。
    pub fn start_ready(&self, room_id: &str) -> Option<BTreeMap<Seat, String>> {
        self.rooms.get(room_id)?.start_ready()
    }

    /// 房主发起开局：校验房主、阶段、人数与准备状态，成功后把阶段推进到 `Recruit`。
    ///
    /// 与纯查询 [`Self::start_ready`] 分开，便于轮询与命令处理各自调用。
    ///
    /// 需要「先构造对局、成功后再提交阶段」的调用方（例如 `room-service::net`）
    /// 应改用 [`Self::start_heroes`] + [`Self::commit_start`]：本方法在构造对局
    /// 之前就推进阶段，构造失败会把房间卡在 `Recruit`。
    pub fn try_start(
        &mut self,
        room_id: &str,
        player_id: &str,
    ) -> Result<BTreeMap<Seat, String>, RoomError> {
        let heroes = self.start_heroes(room_id, player_id)?;
        self.commit_start(room_id)?;
        Ok(heroes)
    }

    /// 开局校验的第一步：只校验房主、阶段、人数与准备状态并返回英雄表，
    /// **不改变房间阶段**，也不消耗任何资源。
    ///
    /// 与 [`Self::commit_start`] 配合使用：调用方先构造对局驱动，成功后再提交阶段，
    /// 避免构造成本失败时房间已离开 `Lobby`。
    pub fn start_heroes(
        &self,
        room_id: &str,
        player_id: &str,
    ) -> Result<BTreeMap<Seat, String>, RoomError> {
        let room = self.rooms.get(room_id).ok_or(RoomError::NotFound)?;
        if room.host_id != player_id {
            return Err(RoomError::NotHost);
        }
        if room.phase != Phase::Lobby {
            return Err(RoomError::WrongPhase);
        }
        room.start_ready().ok_or(RoomError::RoomNotReady)
    }

    /// 开局校验的第二步：把已通过校验的房间从 `Lobby` 推进到 `Recruit`。
    ///
    /// 重复提交返回 [`RoomError::WrongPhase`]，不会回退阶段。
    pub fn commit_start(&mut self, room_id: &str) -> Result<(), RoomError> {
        let room = self.rooms.get_mut(room_id).ok_or(RoomError::NotFound)?;
        if room.phase != Phase::Lobby {
            return Err(RoomError::WrongPhase);
        }
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
                    hero_candidates: None,
                    is_host: slot.player_id == room.host_id,
                    player_id: slot.player_id.clone(),
                    ready: slot.ready,
                })
                .collect(),
            room_id: room.id.clone(),
        })
    }

    /// 构造只给 `viewer` 的 `RoomUpdate`：只有该玩家那一项带 `hero_candidates`。
    ///
    /// 广播必须逐个座位序列化，否则候选会随共用 payload 泄露给其他玩家。
    pub fn room_update_for(&self, room_id: &str, viewer: &str) -> Option<RoomUpdate> {
        let mut update = self.room_update(room_id)?;
        for item in &mut update.players {
            item.hero_candidates = if item.player_id == viewer {
                self.rooms
                    .get(room_id)?
                    .candidates_of(viewer)
                    .map(<[_]>::to_vec)
            } else {
                None
            };
        }
        Some(update)
    }
}

impl Default for RoomRegistry {
    fn default() -> Self {
        Self::new()
    }
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
