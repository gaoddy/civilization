//! 最小对局引擎：初始化状态、开局与回合开始。

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::data::{CardType, Catalog, Era, Tribe};
use crate::error::EngineError;
use crate::events::GameEvent;
use crate::intent::{Intent, Outcome, Reject};
use crate::recruit;
use crate::rng::{RngStreams, Stream};
use crate::state::{GameState, Phase, PlayerState, Seat};

/// 固定 8 人对局。
const PLAYER_COUNT: Seat = 8;

/// 规则引擎。持有只读卡牌目录，状态全部由调用方传入。
pub struct Engine {
    pub catalog: Catalog,
}

impl Engine {
    /// 以给定目录与种子创建初始状态：8 名玩家、`Lobby` 阶段、`turn = 0`。
    ///
    /// 共享池与启用流派在这里一次性确定，商店留空等待 Task 6 的刷新流程。
    pub fn new(catalog: Catalog, seed: [u8; 32]) -> (Engine, GameState) {
        let mut rng = RngStreams::from_seed(seed);
        let first_seat = rng.below(Stream::Matchmaking, u64::from(PLAYER_COUNT)) as Seat;
        let enabled_tribes = draw_enabled_tribes(&mut rng);
        let pool = build_pool(&catalog);
        let config = &catalog.config;

        let players = (0..PLAYER_COUNT)
            .map(|seat| PlayerState {
                seat,
                hp: config.initial_hp,
                gold: 0,
                era: Era::Prehistoric,
                turns_in_era: 0,
                shop: vec![None; config.shop_slots],
                board: Vec::new(),
                hand: Vec::new(),
                hero: None,
                hero_uses: BTreeMap::new(),
                damage_dealt: 0,
                eliminated: false,
                first_refresh_used: false,
            })
            .collect();

        let previous_opponent = (0..PLAYER_COUNT).map(|seat| (seat, None)).collect();
        let ranking = (0..PLAYER_COUNT).collect();

        let state = GameState {
            turn: 0,
            phase: Phase::Lobby,
            players,
            pool,
            rng,
            first_seat,
            enabled_tribes,
            previous_opponent,
            previous_board: BTreeMap::new(),
            last_winner: None,
            pending: VecDeque::new(),
            ranking,
            next_instance_id: 1,
        };

        (Engine { catalog }, state)
    }

    /// 校验 8 个座位都选了目录中的英雄卡，写入英雄，进入第 1 回合招募阶段。
    ///
    /// 第 1 回合金币与商店都在这里完成；`start_turn` 只负责第 2 回合及以后。
    pub fn start_game(
        &mut self,
        state: &mut GameState,
        heroes: &BTreeMap<Seat, String>,
    ) -> Result<Vec<GameEvent>, EngineError> {
        if state.players.len() != usize::from(PLAYER_COUNT) {
            return Err(EngineError(format!(
                "对局状态需要 {PLAYER_COUNT} 名玩家，实际为 {}",
                state.players.len()
            )));
        }

        let mut seen_heroes = BTreeSet::new();
        for seat in 0..PLAYER_COUNT {
            let card_id = heroes
                .get(&seat)
                .ok_or_else(|| EngineError(format!("缺少座位 {seat} 的英雄选择")))?;
            let card = self
                .catalog
                .cards
                .get(card_id)
                .ok_or_else(|| EngineError(format!("英雄卡 {card_id} 不在卡牌目录中")))?;
            if !card.is_hero {
                return Err(EngineError(format!("卡牌 {card_id} 不是英雄卡")));
            }
            if !seen_heroes.insert(card_id.as_str()) {
                return Err(EngineError(format!("英雄卡 {card_id} 被重复选择")));
            }
        }

        for seat in 0..PLAYER_COUNT {
            if let (Some(card_id), Some(player)) =
                (heroes.get(&seat), state.players.get_mut(usize::from(seat)))
            {
                player.hero = Some(card_id.clone());
            }
        }

        state.turn = 1;
        state.phase = Phase::Recruit;
        let gold = self.first_turn_gold();
        for player in &mut state.players {
            player.gold = gold;
            player.first_refresh_used = false;
        }

        let mut events = vec![
            GameEvent::TurnStarted { turn: 1 },
            GameEvent::PhaseChanged {
                phase: Phase::Recruit,
                turn: 1,
            },
        ];
        for seat in 0..state.players.len() {
            if state.players[seat].eliminated {
                continue;
            }
            events.extend(self.roll_shop(state, seat as Seat));
        }
        Ok(events)
    }

    /// 开始下一回合：回合数先自增，再按金币曲线为存活玩家发金币。
    ///
    /// `start_game` 已经发过第 1 回合金币，因此本方法不用于第 1 回合。
    pub fn start_turn(&mut self, state: &mut GameState) -> Result<Vec<GameEvent>, EngineError> {
        let turn = state
            .turn
            .checked_add(1)
            .ok_or_else(|| EngineError("回合数溢出".into()))?;
        let gold = self.gold_for_turn(turn)?;

        let previous_phase = state.phase;
        state.turn = turn;
        state.phase = Phase::Recruit;
        for player in &mut state.players {
            player.hero_uses.clear();
            if player.eliminated {
                continue;
            }
            player.gold = gold;
            player.first_refresh_used = false;
            player.turns_in_era = player.turns_in_era.saturating_add(1);
        }

        let mut events = vec![GameEvent::TurnStarted { turn }];
        if previous_phase != Phase::Recruit {
            events.push(GameEvent::PhaseChanged {
                phase: Phase::Recruit,
                turn,
            });
        }
        for seat in 0..state.players.len() {
            if state.players[seat].eliminated {
                continue;
            }
            events.extend(self.roll_shop(state, seat as Seat));
        }
        Ok(events)
    }

    /// 以指定座位执行一条意图；拒绝时状态保持不变。
    pub fn apply(
        &mut self,
        state: &mut GameState,
        seat: Seat,
        intent: Intent,
    ) -> Result<Outcome, Reject> {
        recruit::apply(self, state, seat, intent)
    }

    /// 重抽指定玩家的商店，保留冻结槽位；返回 `ShopRolled` 事件。
    pub fn roll_shop(&mut self, state: &mut GameState, seat: Seat) -> Vec<GameEvent> {
        recruit::roll_player(self, state, seat, true);
        vec![GameEvent::ShopRolled { seat }]
    }

    fn first_turn_gold(&self) -> u8 {
        self.catalog.config.gold_curve.first().copied().unwrap_or(0)
    }

    fn gold_for_turn(&self, turn: u8) -> Result<u8, EngineError> {
        let index = usize::from(turn.saturating_sub(1));
        self.catalog
            .config
            .gold_curve
            .get(index)
            .copied()
            .ok_or_else(|| EngineError(format!("回合 {turn} 超出金币曲线范围")))
    }
}

/// 学者固定启用，其余五个流派（不含通用）按 `Tribe` 顺序排序后无放回抽四个。
fn draw_enabled_tribes(rng: &mut RngStreams) -> BTreeSet<Tribe> {
    let candidates = [
        Tribe::Human,
        Tribe::Machine,
        Tribe::Biology,
        Tribe::Plant,
        Tribe::Virus,
    ];
    let mut remaining = candidates.to_vec();
    let mut enabled = BTreeSet::new();
    enabled.insert(Tribe::Scholar);
    for _ in 0..4 {
        let len = remaining.len() as u64;
        let index = rng.below(Stream::Matchmaking, len) as usize;
        if let Some(tribe) = remaining.get(index).copied() {
            remaining.remove(index);
            enabled.insert(tribe);
        }
    }
    enabled
}

/// 按 `pool_counts[level - 1]` 为每张非 token 随从填充共享池。
fn build_pool(catalog: &Catalog) -> BTreeMap<String, u16> {
    let mut pool = BTreeMap::new();
    for card in catalog.cards.values() {
        if card.card_type != CardType::Minion {
            continue;
        }
        if catalog.tokens.contains_key(&card.id) {
            continue;
        }
        let Some(level_index) = card.level.checked_sub(1).map(usize::from) else {
            continue;
        };
        if let Some(count) = catalog.config.pool_counts.get(level_index) {
            pool.insert(card.id.clone(), *count);
        }
    }
    pool
}
