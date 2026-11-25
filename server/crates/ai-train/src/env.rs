//! 八人对局环境：按座位升序发放决策，阶段推进沿用 `sim` 的顺序。
//!
//! 与 `sim` 的整局驱动不同，这里每个决策单独返回，调用方拿到观测、动作列表与
//! 终局奖励；训练不关心日志、事件哈希与统计。

use std::collections::BTreeMap;

use ai_core::{Action, ActionFeatures, CardIndex, Encoder, History, Observation};
use rule_engine::data::Catalog;
use rule_engine::engine::Engine;
use rule_engine::error::EngineError;
use rule_engine::hash::state_hash;
use rule_engine::legal::legal_actions;
use rule_engine::playstyles;
use rule_engine::rng::seed_from_u64;
use rule_engine::state::{GameState, Phase, Seat};
use rule_engine::view::{placement_of, view};

/// 固定 8 人对局。
pub const PLAYER_COUNT: usize = 8;

/// 一次 `step` 的结果。
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    /// 对局是否结束。
    pub done: bool,
    /// 终局奖励，按座位索引；未结束时全 0。
    pub terminal_reward: [f32; PLAYER_COUNT],
}

impl Step {
    fn empty() -> Self {
        Self {
            done: false,
            terminal_reward: [0.0; PLAYER_COUNT],
        }
    }
}

/// 八人对局环境。
///
/// 支持两种开局：直接给定英雄（[`Env::reset`]），或先走选英雄步骤
/// （[`Env::reset_with_candidates`]，`PickHero` 是每个座位的第一个决策）。
pub struct Env {
    engine: Engine,
    state: GameState,
    history: History,
    encoder: Encoder,
    candidates: BTreeMap<Seat, Vec<String>>,
    started: bool,
}

impl Env {
    pub fn new(catalog: &Catalog) -> Self {
        let engine = Engine::from_catalog(catalog.clone());
        let state = engine.initial_state([0; 32], playstyles::draw(0));
        let encoder = Encoder::new(CardIndex::build(catalog));
        Self {
            engine,
            state,
            history: History::new(),
            encoder,
            candidates: BTreeMap::new(),
            started: true,
        }
    }

    /// 以候选英雄重置：先逐个座位选英雄，全部选完后才发英雄并进入第 1 回合。
    pub fn reset_with_candidates(
        &mut self,
        seed: u64,
        candidates: &BTreeMap<Seat, Vec<String>>,
    ) -> Result<(), EngineError> {
        let playstyles = playstyles::draw(seed);
        self.state = self.engine.initial_state(seed_from_u64(seed), playstyles);
        self.history = History::new();
        self.candidates = candidates.clone();
        self.started = false;
        Ok(())
    }

    /// 本局已选定的英雄；未开始时为对应座位的候选列表。
    pub fn candidates(&self) -> &BTreeMap<Seat, Vec<String>> {
        &self.candidates
    }

    pub fn started(&self) -> bool {
        self.started
    }

    /// 用给定种子与座位英雄重置对局，并进入第 1 回合招募。
    pub fn reset(&mut self, seed: u64, heroes: &BTreeMap<Seat, String>) -> Result<(), EngineError> {
        let playstyles = playstyles::draw(seed);
        self.state = self.engine.initial_state(seed_from_u64(seed), playstyles);
        self.history = History::new();
        self.candidates = BTreeMap::new();
        self.engine.start_game(&mut self.state, heroes)?;
        self.started = true;
        Ok(())
    }

    /// 下一个需要决策的座位；全部结束或对局结束时为 `None`。
    pub fn current_seat(&self) -> Option<Seat> {
        if !self.started {
            return (0..PLAYER_COUNT as Seat)
                .find(|seat| self.state.players[usize::from(*seat)].hero.is_none());
        }
        if self.state.phase != Phase::Recruit {
            return None;
        }
        (0..PLAYER_COUNT as Seat).find(|&seat| {
            let player = &self.state.players[usize::from(seat)];
            !player.eliminated && !player.recruit_done
        })
    }

    /// 指定座位当前全部合法动作，顺序与 `legal_actions` 一致。
    /// 指定座位当前的合法动作及其特征，顺序与 [`Env::legal`] 一致。
    pub fn action_candidates(&self, seat: Seat) -> Vec<(Action, ActionFeatures)> {
        self.legal(seat)
            .into_iter()
            .map(|action| {
                let features = ActionFeatures::from_action(
                    &action,
                    &self.state,
                    seat,
                    self.encoder.index(),
                );
                (action, features)
            })
            .collect()
    }

    pub fn legal(&self, seat: Seat) -> Vec<Action> {
        if !self.started {
            return self
                .candidates
                .get(&seat)
                .map(|heroes| {
                    heroes
                        .iter()
                        .map(|hero| Action::PickHero {
                            hero: hero.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default();
        }
        legal_actions(&self.engine, &self.state, seat)
            .iter()
            .filter_map(Action::from_intent)
            .collect()
    }

    pub fn observe(&self, seat: Seat) -> Observation {
        let seat_view = view(&self.engine, &self.state, seat).expect("观测座位应存在");
        self.encoder.encode(&seat_view, &self.history)
    }

    pub fn placement(&self, seat: Seat) -> Option<u8> {
        placement_of(&self.state, seat)
    }

    pub fn turn(&self) -> u8 {
        self.state.turn
    }

    pub fn phase(&self) -> Phase {
        self.state.phase
    }

    pub fn state_hash(&self) -> u64 {
        state_hash(&self.state)
    }

    /// 为当前座位执行一个动作；全员结束招募时自动推进到下一回合。
    pub fn step(&mut self, action: Action) -> Result<Step, EngineError> {
        let seat = self
            .current_seat()
            .ok_or_else(|| EngineError("当前没有待决策座位".to_string()))?;
        if !self.started {
            return self.pick_hero(seat, action);
        }
        let intent = action
            .to_intent()
            .ok_or_else(|| EngineError("该动作在招募阶段没有对应意图".to_string()))?;
        self.engine
            .apply(&mut self.state, seat, intent)
            .map_err(|reject| {
                EngineError(format!(
                    "座位 {seat} 的动作被拒绝: {} {}",
                    reject.code.as_str(),
                    reject.message
                ))
            })?;
        self.advance_if_complete()?;
        Ok(self.result())
    }

    /// 开局选英雄：只接受候选内的 `PickHero`；全员选完后开局。
    fn pick_hero(&mut self, seat: Seat, action: Action) -> Result<Step, EngineError> {
        let Action::PickHero { hero } = action else {
            return Err(EngineError(format!(
                "座位 {seat} 处于选英雄阶段，只接受 PickHero"
            )));
        };
        let allowed = self.candidates.get(&seat).cloned().unwrap_or_default();
        if !allowed.contains(&hero) {
            return Err(EngineError(format!(
                "座位 {seat} 的英雄 {hero} 不在候选内"
            )));
        }
        self.state.players[usize::from(seat)].hero = Some(hero);
        let heroes: BTreeMap<Seat, String> = self
            .state
            .players
            .iter()
            .filter_map(|player| player.hero.clone().map(|hero| (player.seat, hero)))
            .collect();
        if heroes.len() == PLAYER_COUNT {
            self.engine.start_game(&mut self.state, &heroes)?;
            self.started = true;
        }
        Ok(Step::empty())
    }

    fn recruit_complete(&self) -> bool {
        self.state
            .players
            .iter()
            .all(|player| player.eliminated || player.recruit_done)
    }

    fn advance_if_complete(&mut self) -> Result<(), EngineError> {
        // 最后一名玩家 `EndRecruit` 时引擎已把阶段推进到 `Combat`，
        // 所以这里既要看阶段，也要看招募是否真的结束。
        let ready = match self.state.phase {
            Phase::Combat => true,
            Phase::Recruit => self.recruit_complete(),
            _ => false,
        };
        if !ready {
            return Ok(());
        }
        self.engine.finish_recruit(&mut self.state)?;
        self.record_history();
        self.engine.resolve_combat_with(&mut self.state, false)?;
        self.engine.resolve_settle(&mut self.state)?;
        if self.state.phase != Phase::GameOver {
            self.engine.start_turn(&mut self.state)?;
        }
        Ok(())
    }

    /// 记录每个真实对战座位看到的对手棋盘；幽灵战没有可记录的实体棋盘。
    fn record_history(&mut self) {
        let pairings = self.state.pairings.clone();
        for pairing in pairings {
            if pairing.is_ghost() {
                continue;
            }
            let opponent_of_a = self.board_cards(pairing.b);
            self.history.record(pairing.a, &opponent_of_a);
            let opponent_of_b = self.board_cards(pairing.a);
            self.history.record(pairing.b, &opponent_of_b);
        }
    }

    fn board_cards(&self, seat: Seat) -> Vec<i32> {
        self.state.players[usize::from(seat)]
            .board
            .iter()
            .map(|minion| self.encoder.index().card(&minion.card_id))
            .collect()
    }

    fn result(&self) -> Step {
        if self.state.phase != Phase::GameOver {
            return Step::empty();
        }
        let mut terminal_reward = [0.0; PLAYER_COUNT];
        for seat in 0..PLAYER_COUNT as Seat {
            if let Some(place) = self.placement(seat) {
                terminal_reward[usize::from(seat)] = (8.0 - f32::from(place)) / 7.0;
            }
        }
        Step {
            done: true,
            terminal_reward,
        }
    }
}
