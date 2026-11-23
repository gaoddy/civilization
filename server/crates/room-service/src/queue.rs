//! 自动匹配队列：入队、取消、逐级放宽积分窗口与确定性成局。
//!
//! 队列只保存排队中的会话与积分，不读系统时间，也不直接建房；成局由调用方
//! 拿到 [`MatchQueue::tick`] 返回的 8 个 [`QueueEntry`] 后交给房间服务。
//!
//! 匹配规则（产品 §5）：
//! - 初始窗口 ±150；等待满 30 秒放宽到 ±300，满 60 秒放宽到 ±500，满 90 秒不限；
//! - 优先排除最近 3 局对手；本人等待满 60 秒后放宽该排除；
//! - 选人确定性：以等待最久（再按 `player_id`）的玩家为锚点，候选取窗口内
//!   积分差从最小到大，再按入队时间与 `player_id` 排序，取前 8 人。

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use crate::auth::Session;
use crate::rating::RankedConfig;

/// 新账号初始积分；匹配积分由调用方按账号实际积分传入。
pub use crate::rating::DEFAULT_RATING;

/// 初始匹配窗口（±分）。
pub const INITIAL_RATING_WINDOW: i32 = 150;

/// 等待 30 秒后的窗口（±分）。
pub const RELAXED_RATING_WINDOW: i32 = 300;

/// 等待 60 秒后的窗口（±分）。
pub const WIDE_RATING_WINDOW: i32 = 500;

/// 每次放宽窗口的等待间隔。
pub const WINDOW_RELAX_STEP_MS: u64 = 30_000;

/// 等待满该时长后不再限制积分。
pub const UNLIMITED_AFTER_MS: u64 = 90_000;

/// 等待满该时长后不再排除最近对手。
pub const RECENT_OPPONENT_RELAX_MS: u64 = 60_000;

/// 每名玩家保留的最近对局数，用于排除对手。
pub const RECENT_OPPONENT_MATCHES: usize = 3;

/// 每局座位数。
pub const SEATS_PER_MATCH: usize = 8;

/// 一条排队记录。
#[derive(Debug, Clone)]
pub struct QueueEntry {
    /// 排队玩家的会话；成局后用于建房与分配座位。
    pub session: Session,
    /// 匹配积分；同分玩家按先到先得。
    pub rating: i32,
    /// 入队时刻（Unix epoch 毫秒）；窗口与放宽排除都按它计算。
    pub joined_at_ms: u64,
}

/// 下发给单个玩家的排队状态快照。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueStateData {
    /// 本人已等待时长（毫秒）。
    pub waiting_ms: u64,
    /// 当前队列里的玩家数。
    pub waiting_players: usize,
    /// 当前积分半径；`None` 表示不限积分。
    pub rating_window: Option<i32>,
}

/// 窗口变化通知：只发给仍排队、且窗口与上次下发不同的玩家。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueNotice {
    pub player_id: String,
    pub state: QueueStateData,
}

/// 一次队列 tick 的结果。
#[derive(Debug, Default)]
pub struct QueueTick {
    /// 需要补发 `QueueState` 的玩家。
    pub notices: Vec<QueueNotice>,
    /// 已凑满的 8 人；按座位号（入队时间、`player_id`）排列。
    pub formed: Option<Vec<QueueEntry>>,
}

/// 队列操作失败原因；`code` 映射到协议错误码。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueError {
    /// 玩家已在队列里。
    Already,
    /// 玩家已在房间里，不能排队。
    InRoom,
    /// 玩家不在队列里（取消排队以外的操作）。
    NotJoined,
}

impl QueueError {
    /// 映射到协议错误码。
    pub fn code(self) -> protocol::ErrorCode {
        match self {
            Self::Already => protocol::ErrorCode::QueueAlready,
            Self::InRoom => protocol::ErrorCode::QueueInRoom,
            Self::NotJoined => protocol::ErrorCode::QueueNotJoined,
        }
    }
}

impl std::fmt::Display for QueueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Already => f.write_str("已在匹配队列中"),
            Self::InRoom => f.write_str("已在房间中，不能匹配"),
            Self::NotJoined => f.write_str("不在匹配队列中"),
        }
    }
}

impl std::error::Error for QueueError {}

/// 进程内匹配队列。
#[derive(Debug)]
pub struct MatchQueue {
    /// 窗口与最近对手排除参数；由服务配置注入。
    config: Arc<RankedConfig>,
    entries: BTreeMap<String, QueueEntry>,
    /// `player_id → 最近若干局的对手集合`，队首为最近一局。
    recent: BTreeMap<String, VecDeque<BTreeSet<String>>>,
    /// 上次已下发的窗口；用于只在窗口变化时补发 `QueueState`。
    reported_window: BTreeMap<String, Option<i32>>,
    /// 是否已有驱动任务在跑，避免每个入队请求各起一个 ticker。
    driver_running: bool,
}

impl Default for MatchQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl MatchQueue {
    /// 空队列，使用默认天梯配置。
    pub fn new() -> Self {
        Self::with_config(Arc::new(RankedConfig::default()))
    }

    /// 空队列，使用注入的匹配窗口与最近对手参数。
    pub fn with_config(config: Arc<RankedConfig>) -> Self {
        Self {
            config,
            entries: BTreeMap::new(),
            recent: BTreeMap::new(),
            reported_window: BTreeMap::new(),
            driver_running: false,
        }
    }

    /// 当前排队人数。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否没有玩家在排队。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 玩家是否在队列里。
    pub fn contains(&self, player_id: &str) -> bool {
        self.entries.contains_key(player_id)
    }

    /// 入队；同一玩家重复入队返回 [`QueueError::Already`]。
    ///
    /// # Errors
    ///
    /// 玩家已在队列里返回 [`QueueError::Already`]；房间态由调用方先判定。
    pub fn join(&mut self, session: Session, rating: i32, now_ms: u64) -> Result<(), QueueError> {
        let player_id = session.player_id.clone();
        if self.entries.contains_key(&player_id) {
            return Err(QueueError::Already);
        }
        let entry = QueueEntry {
            session,
            rating,
            joined_at_ms: now_ms,
        };
        self.reported_window.insert(
            player_id.clone(),
            self.config
                .window_for(now_ms.saturating_sub(entry.joined_at_ms)),
        );
        self.entries.insert(player_id, entry);
        Ok(())
    }

    /// 移出队列；不在队列时幂等，返回是否真的移除。
    pub fn remove(&mut self, player_id: &str) -> bool {
        self.reported_window.remove(player_id);
        self.entries.remove(player_id).is_some()
    }

    /// 当前排队状态；玩家不在队列返回 `None`。
    pub fn state(&self, player_id: &str, now_ms: u64) -> Option<QueueStateData> {
        self.entries.get(player_id).map(|entry| QueueStateData {
            waiting_ms: now_ms.saturating_sub(entry.joined_at_ms),
            waiting_players: self.entries.len(),
            rating_window: self
                .config
                .window_for(now_ms.saturating_sub(entry.joined_at_ms)),
        })
    }

    /// 移除不再有资格排队的玩家（断线、已进房）；返回被移除的 `player_id`。
    pub fn retain(&mut self, mut keep: impl FnMut(&QueueEntry) -> bool) -> Vec<String> {
        let stale: Vec<String> = self
            .entries
            .values()
            .filter(|entry| !keep(entry))
            .map(|entry| entry.session.player_id.clone())
            .collect();
        for player_id in &stale {
            self.remove(player_id);
        }
        stale
    }

    /// 驱动任务是否已在运行；返回 `true` 表示本次调用应启动驱动。
    pub fn begin_driver(&mut self) -> bool {
        if self.driver_running {
            false
        } else {
            self.driver_running = true;
            true
        }
    }

    /// 标记驱动任务退出；队列为空时由驱动调用。
    pub fn end_driver(&mut self) {
        self.driver_running = false;
    }

    /// 推进一次匹配：先尝试成局，再收集窗口变化通知。
    pub fn tick(&mut self, now_ms: u64) -> QueueTick {
        let formed = self.try_form(now_ms);
        let notices = self.collect_window_notices(now_ms);
        QueueTick { notices, formed }
    }

    /// 以等待最久的玩家为锚点，选出 8 名两两兼容的玩家。
    fn try_form(&mut self, now_ms: u64) -> Option<Vec<QueueEntry>> {
        if self.entries.len() < SEATS_PER_MATCH {
            return None;
        }
        let mut anchors: Vec<QueueEntry> = self.entries.values().cloned().collect();
        anchors.sort_by_key(|entry| (entry.joined_at_ms, entry.session.player_id.clone()));

        for anchor in anchors {
            let mut candidates: Vec<QueueEntry> = self
                .entries
                .values()
                .filter(|entry| entry.session.player_id != anchor.session.player_id)
                .cloned()
                .collect();
            candidates.sort_by_key(|entry| {
                (
                    (i64::from(entry.rating) - i64::from(anchor.rating)).abs(),
                    entry.joined_at_ms,
                    entry.session.player_id.clone(),
                )
            });

            let mut selected = vec![anchor];
            for candidate in candidates {
                if selected.len() == SEATS_PER_MATCH {
                    break;
                }
                if selected
                    .iter()
                    .all(|member| self.pair_eligible(member, &candidate, now_ms))
                {
                    selected.push(candidate);
                }
            }

            if selected.len() == SEATS_PER_MATCH {
                // 座位按入队顺序分配：先到先得，同刻按 player_id。
                selected.sort_by_key(|entry| (entry.joined_at_ms, entry.session.player_id.clone()));
                for entry in &selected {
                    self.entries.remove(&entry.session.player_id);
                    self.reported_window.remove(&entry.session.player_id);
                }
                self.record_recent(&selected);
                return Some(selected);
            }
        }
        None
    }

    /// 记录本局对手，只保留最近 [`RECENT_OPPONENT_MATCHES`] 局。
    fn record_recent(&mut self, players: &[QueueEntry]) {
        for player in players {
            let opponents: BTreeSet<String> = players
                .iter()
                .filter(|other| other.session.player_id != player.session.player_id)
                .map(|other| other.session.player_id.clone())
                .collect();
            let matches = self
                .recent
                .entry(player.session.player_id.clone())
                .or_default();
            matches.push_front(opponents);
            matches.truncate(RECENT_OPPONENT_MATCHES);
        }
    }

    /// 窗口发生变化的排队玩家；每个玩家只在窗口切换时收到一次。
    fn collect_window_notices(&mut self, now_ms: u64) -> Vec<QueueNotice> {
        let waiting_players = self.entries.len();
        let mut notices = Vec::new();
        for (player_id, entry) in &self.entries {
            let window = self
                .config
                .window_for(now_ms.saturating_sub(entry.joined_at_ms));
            let reported = self.reported_window.get(player_id).copied();
            if reported == Some(window) {
                continue;
            }
            self.reported_window.insert(player_id.clone(), window);
            notices.push(QueueNotice {
                player_id: player_id.clone(),
                state: QueueStateData {
                    waiting_ms: now_ms.saturating_sub(entry.joined_at_ms),
                    waiting_players,
                    rating_window: window,
                },
            });
        }
        notices
    }

    /// 两名玩家是否可以在 `now_ms` 组成同一局。
    fn pair_eligible(&self, a: &QueueEntry, b: &QueueEntry, now_ms: u64) -> bool {
        let diff = (i64::from(a.rating) - i64::from(b.rating)).abs();
        let window_ok = match (
            self.config
                .window_for(now_ms.saturating_sub(a.joined_at_ms)),
            self.config
                .window_for(now_ms.saturating_sub(b.joined_at_ms)),
        ) {
            (None, None) => true,
            // 一方不限积分时仍受另一方窗口约束：刚入队的玩家不应被长等待玩家拉进大分差局。
            (None, Some(window)) | (Some(window), None) => diff <= i64::from(window),
            // 两人各自可接受的窗口取更严格的一方。
            (Some(left), Some(right)) => diff <= i64::from(left.min(right)),
        };
        window_ok && !self.recently_played(a, b, now_ms) && !self.recently_played(b, a, now_ms)
    }

    /// 玩家是否在最近 [`RECENT_OPPONENT_MATCHES`] 局里与对手同局；
    /// 本人等待满配置的放宽时长后不再排除。
    fn recently_played(&self, player: &QueueEntry, other: &QueueEntry, now_ms: u64) -> bool {
        if now_ms.saturating_sub(player.joined_at_ms) >= self.config.recent_opponent_relax_ms {
            return false;
        }
        self.recent
            .get(&player.session.player_id)
            .is_some_and(|matches| {
                matches
                    .iter()
                    .any(|opponents| opponents.contains(&other.session.player_id))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::SessionStore;
    use crate::clock::FixedClock;
    use protocol::client::AuthRequest;

    fn sessions(count: usize) -> Vec<Session> {
        let clock = FixedClock::new(1_000_000);
        let mut store = SessionStore::new();
        (0..count)
            .map(|index| {
                let display_name = format!("玩家{index}").try_into().expect("展示名合法");
                store
                    .authenticate(
                        &AuthRequest::Variant0 {
                            code: "123456".to_string(),
                            display_name: Some(display_name),
                            phone: "13800000000".to_string(),
                        },
                        &clock,
                    )
                    .expect("开发验证码应登录成功")
            })
            .collect()
    }

    fn queue_with(entries: &[Session], rating: i32, joined_at_ms: u64) -> MatchQueue {
        let mut queue = MatchQueue::new();
        for session in entries {
            queue
                .join(session.clone(), rating, joined_at_ms)
                .expect("首次入队应成功");
        }
        queue
    }

    fn ids(entries: &[QueueEntry]) -> Vec<String> {
        entries
            .iter()
            .map(|entry| entry.session.player_id.clone())
            .collect()
    }

    #[test]
    fn window_relaxes_at_thirty_sixty_ninety_seconds() {
        let config = RankedConfig::default();
        assert_eq!(config.window_for(0), Some(150));
        assert_eq!(config.window_for(29_999), Some(150));
        assert_eq!(config.window_for(30_000), Some(300));
        assert_eq!(config.window_for(59_999), Some(300));
        assert_eq!(config.window_for(60_000), Some(500));
        assert_eq!(config.window_for(89_999), Some(500));
        assert_eq!(config.window_for(90_000), None);
    }

    #[test]
    fn eight_players_form_a_deterministic_seat_order() {
        let sessions = sessions(9);
        let mut first = MatchQueue::new();
        for session in sessions.iter().take(8) {
            first
                .join(session.clone(), DEFAULT_RATING, 0)
                .expect("入队成功");
        }
        let formed = first.tick(0).formed.expect("同分 8 人应立即成局");
        assert_eq!(formed.len(), SEATS_PER_MATCH);
        let mut expected: Vec<String> = sessions[..8]
            .iter()
            .map(|session| session.player_id.clone())
            .collect();
        // 同一时刻入队时，座位按 player_id 排序。
        expected.sort();
        assert_eq!(ids(&formed), expected);
        assert!(first.is_empty(), "成局后 8 人应全部移出队列");

        let mut second = MatchQueue::new();
        for session in sessions.iter().take(8) {
            second
                .join(session.clone(), DEFAULT_RATING, 0)
                .expect("入队成功");
        }
        let again = second.tick(0).formed.expect("同输入应再成局");
        assert_eq!(ids(&formed), ids(&again), "同一输入应得到同一座位顺序");
    }

    #[test]
    fn ninth_player_stays_queued() {
        let sessions = sessions(9);
        let mut queue = queue_with(&sessions, DEFAULT_RATING, 0);
        let formed = queue.tick(0).formed.expect("8 人可成局");
        assert_eq!(formed.len(), SEATS_PER_MATCH);
        assert_eq!(queue.len(), 1);
        let formed_ids: BTreeSet<String> = ids(&formed).into_iter().collect();
        let remaining = queue.entries.keys().next().expect("应剩 1 名玩家").clone();
        assert!(
            !formed_ids.contains(&remaining),
            "留在队列的必须是未成局的玩家"
        );
    }

    #[test]
    fn rating_gap_waits_for_window_relaxation() {
        let sessions = sessions(8);
        let mut queue = MatchQueue::new();
        for (index, session) in sessions.iter().enumerate() {
            let rating = if index == 0 { 1000 } else { 1160 };
            queue.join(session.clone(), rating, 0).expect("入队成功");
        }
        assert!(
            queue.tick(0).formed.is_none(),
            "160 分差超出 ±150 初始窗口，不能成局"
        );
        assert!(
            queue.tick(29_999).formed.is_none(),
            "窗口按整 30 秒切换，29.999 秒仍是 ±150"
        );
        assert!(
            queue.tick(30_000).formed.is_some(),
            "等待 30 秒后 ±300 应容纳 160 分差"
        );
    }

    #[test]
    fn unlimited_window_after_ninety_seconds() {
        let sessions = sessions(8);
        let mut queue = MatchQueue::new();
        for session in &sessions {
            queue.join(session.clone(), 1000, 0).expect("入队成功");
        }
        queue.remove(&sessions[7].player_id);
        queue
            .join(sessions[7].clone(), 2000, 0)
            .expect("重新入队成功");
        assert!(queue.tick(0).formed.is_none(), "1000 分差不能立即成局");
        assert!(queue.tick(60_000).formed.is_none(), "±500 仍不够");
        assert!(queue.tick(90_000).formed.is_some(), "90 秒后不限积分应成局");
    }

    #[test]
    fn stricter_window_still_applies_when_other_player_is_unlimited() {
        let sessions = sessions(8);
        let mut queue = MatchQueue::new();
        for session in &sessions[..7] {
            queue.join(session.clone(), 1000, 0).expect("入队成功");
        }
        queue
            .join(sessions[7].clone(), 2000, 90_000)
            .expect("入队成功");

        assert!(
            queue.tick(90_000).formed.is_none(),
            "新入队玩家只有 ±150，不能被不限积分的等待者拉入"
        );
        assert!(
            queue.tick(179_999).formed.is_none(),
            "新玩家未满 90 秒仍受自身窗口约束"
        );
        assert!(
            queue.tick(180_000).formed.is_some(),
            "新玩家等待满 90 秒后不限积分，可以成局"
        );
    }

    #[test]
    fn recent_opponents_are_excluded_then_relaxed() {
        let sessions = sessions(8);
        let mut queue = queue_with(&sessions, DEFAULT_RATING, 0);
        assert!(queue.tick(0).formed.is_some(), "首局应成局");

        // 同一批人在 1 秒后重新排队：互相都是最近对手，不能立刻成局。
        for session in &sessions {
            queue
                .join(session.clone(), DEFAULT_RATING, 1_000)
                .expect("重新入队成功");
        }
        assert!(
            queue.tick(1_000).formed.is_none(),
            "等待不足 60 秒时应排除最近对手"
        );
        assert!(queue.tick(60_999).formed.is_none(), "60 秒前仍排除最近对手");
        assert!(
            queue.tick(61_000).formed.is_some(),
            "等待满 60 秒后放宽对手排除"
        );
    }

    #[test]
    fn recent_history_keeps_only_three_matches() {
        let sessions = sessions(8);
        let mut queue = queue_with(&sessions, DEFAULT_RATING, 0);
        let mut joined = 0_u64;
        for round in 0..4 {
            let formed_at = joined + RECENT_OPPONENT_RELAX_MS;
            assert!(
                queue.tick(formed_at).formed.is_some(),
                "第 {round} 局应成局"
            );
            joined = formed_at + 1;
            for session in &sessions {
                queue
                    .join(session.clone(), DEFAULT_RATING, joined)
                    .expect("重新入队成功");
            }
        }
        let history = &queue.recent[&sessions[0].player_id];
        assert_eq!(history.len(), RECENT_OPPONENT_MATCHES);
    }

    #[test]
    fn leave_is_idempotent_and_state_disappears() {
        let sessions = sessions(1);
        let mut queue = queue_with(&sessions, DEFAULT_RATING, 0);
        assert!(queue.state(&sessions[0].player_id, 5_000).is_some());
        assert!(queue.remove(&sessions[0].player_id));
        assert!(!queue.remove(&sessions[0].player_id));
        assert!(queue.state(&sessions[0].player_id, 5_000).is_none());
    }

    #[test]
    fn duplicate_join_is_rejected() {
        let sessions = sessions(1);
        let mut queue = queue_with(&sessions, DEFAULT_RATING, 0);
        assert_eq!(
            queue.join(sessions[0].clone(), DEFAULT_RATING, 1_000),
            Err(QueueError::Already)
        );
    }

    #[test]
    fn retain_drops_stale_players() {
        let sessions = sessions(3);
        let mut queue = queue_with(&sessions, DEFAULT_RATING, 0);
        let removed = queue.retain(|entry| entry.session.player_id != sessions[1].player_id);
        assert_eq!(removed, vec![sessions[1].player_id.clone()]);
        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn notices_fire_only_when_window_changes() {
        let sessions = sessions(1);
        let mut queue = queue_with(&sessions, DEFAULT_RATING, 0);
        assert!(queue.tick(1_000).notices.is_empty(), "窗口未变不补发");
        let notices = queue.tick(30_000).notices;
        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0].state.rating_window, Some(300));
        assert!(queue.tick(31_000).notices.is_empty(), "同一窗口只发一次");
    }
}
