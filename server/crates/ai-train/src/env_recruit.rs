//! C1 单座位构筑环境：学习座位走真实招募流程，其余座位退出对局。
//!
//! 与八人环境不同，这里不跑 `resolve_settle`、不结算伤害与淘汰：本座位
//! `EndRecruit` 后直接对采样棋盘模拟一场战斗。其余座位在开局前标记为出局，
//! 商店与牌池只由学习座位消耗，等价于单人池。

use ai_core::{Action, ActionFeatures, CardIndex, Encoder, History, Observation};
use ai_model::{ModelConfig, Weights, board_value};
use rule_engine::combat::{self, CombatLog, CombatSetup};
use rule_engine::data::Catalog;
use rule_engine::engine::Engine;
use rule_engine::error::EngineError;
use rule_engine::legal::legal_actions;
use rule_engine::playstyles;
use rule_engine::rng::{RngStreams, seed_from_u64};
use rule_engine::state::{GameState, MinionInstance, Phase, Seat};

use crate::data_c0::BoardSample;
use crate::data_c0::board_observation;
use crate::env::PLAYER_COUNT;

/// 学习座位固定为 0。
pub const LEARNING_SEAT: Seat = 0;
/// 对手棋盘在战斗里占用的座位号。
pub const OPPONENT_SEAT: Seat = 1;

/// 奖励来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewardMode {
    /// `s(我方棋盘, 我方英雄) − s(对手棋盘, 对手英雄)`，`s` 来自 C0 权重。
    Value,
    /// 直接模拟一场战斗：胜 +1、负 −1、平 0。
    Combat,
}

/// 一场战斗的可比对摘要。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatSummary {
    pub winner: Option<Seat>,
    pub action_count: u16,
    pub attacker_survivors: usize,
    pub defender_survivors: usize,
    pub stalemate: bool,
}

impl CombatSummary {
    fn of(log: &CombatLog) -> Self {
        Self {
            winner: log.winner,
            action_count: log.action_count,
            attacker_survivors: log.attacker_survivors.len(),
            defender_survivors: log.defender_survivors.len(),
            stalemate: log.stalemate,
        }
    }
}

/// 一次 `step` 的结果。
#[derive(Debug, Clone, PartialEq)]
pub struct RecruitStep {
    pub done: bool,
    pub reward: f32,
    /// 仅在 `EndRecruit` 那一步有值。
    pub combat: Option<CombatSummary>,
}

/// 单座位构筑环境。
pub struct RecruitEnv {
    engine: Engine,
    state: GameState,
    encoder: Encoder,
    history: History,
    catalog: Catalog,
    model: ModelConfig,
    weights: Option<Weights>,
    reward_mode: RewardMode,
    opponent: Option<BoardSample>,
    hero: String,
    playstyles: u16,
    episode_seed: u64,
    done: bool,
}

impl RecruitEnv {
    pub fn new(catalog: &Catalog, weights: Option<Weights>, model: ModelConfig) -> Self {
        let engine = Engine::from_catalog(catalog.clone());
        let state = engine.initial_state([0; 32], playstyles::draw(0));
        let reward_mode = if weights.is_some() {
            RewardMode::Value
        } else {
            RewardMode::Combat
        };
        Self {
            engine,
            state,
            encoder: Encoder::new(CardIndex::build(catalog)),
            history: History::new(),
            catalog: catalog.clone(),
            model,
            weights,
            reward_mode,
            opponent: None,
            hero: String::new(),
            playstyles: 0,
            episode_seed: 0,
            done: false,
        }
    }

    pub fn set_reward_mode(&mut self, mode: RewardMode) {
        self.reward_mode = mode;
    }

    /// 重置到第 1 回合招募；其余七个座位在本局内视为已出局。
    pub fn reset(
        &mut self,
        seed: u64,
        hero: &str,
        opponent: &BoardSample,
    ) -> Result<(), EngineError> {
        let drawn = playstyles::draw(seed);
        self.playstyles = ai_core::playstyle_mask(
            &drawn
                .iter()
                .map(|playstyle| playstyle.id().to_string())
                .collect::<Vec<_>>(),
        );
        self.state = self.engine.initial_state(seed_from_u64(seed), drawn);
        for player in self
            .state
            .players
            .iter_mut()
            .skip(usize::from(LEARNING_SEAT) + 1)
        {
            player.eliminated = true;
        }
        let mut heroes = std::collections::BTreeMap::new();
        for seat in 0..PLAYER_COUNT as Seat {
            let name = if seat == LEARNING_SEAT {
                hero
            } else {
                opponent.hero.as_str()
            };
            heroes.insert(seat, name.to_string());
        }
        self.engine.start_game(&mut self.state, &heroes)?;
        self.history = History::new();
        self.hero = hero.to_string();
        self.opponent = Some(opponent.clone());
        self.episode_seed = seed;
        self.done = false;
        Ok(())
    }

    pub fn observe(&self) -> Observation {
        let seat_view = rules_view(&self.engine, &self.state);
        self.encoder.encode(&seat_view, &self.history)
    }

    /// 学习座位当前全部合法动作。
    pub fn legal(&self) -> Vec<Action> {
        legal_actions(&self.engine, &self.state, LEARNING_SEAT)
            .iter()
            .filter_map(Action::from_intent)
            .collect()
    }

    /// 当前合法动作与各自的候选特征；训练与评测共用同一份打分输入。
    pub fn action_candidates(&self) -> Vec<(Action, ActionFeatures)> {
        self.legal()
            .into_iter()
            .map(|action| {
                let features = ActionFeatures::from_action(
                    &action,
                    &self.state,
                    LEARNING_SEAT,
                    self.encoder.index(),
                );
                (action, features)
            })
            .collect()
    }

    pub fn step(&mut self, action: Action) -> Result<RecruitStep, EngineError> {
        if self.done {
            return Err(EngineError("对局已结束".to_string()));
        }
        let is_end = matches!(action, Action::End);
        let intent = action
            .to_intent()
            .ok_or_else(|| EngineError("该动作在招募阶段没有对应意图".to_string()))?;
        self.engine
            .apply(&mut self.state, LEARNING_SEAT, intent)
            .map_err(|reject| {
                EngineError(format!(
                    "学习座位动作被拒绝: {} {}",
                    reject.code.as_str(),
                    reject.message
                ))
            })?;
        if !is_end {
            return Ok(RecruitStep {
                done: false,
                reward: 0.0,
                combat: None,
            });
        }
        let log = self.fight();
        let reward = self.reward(&log);
        let turn = self.state.turn;
        if turn >= self.catalog.config.turn_limit {
            self.done = true;
            self.state.phase = Phase::GameOver;
        } else {
            self.engine.start_turn(&mut self.state)?;
        }
        Ok(RecruitStep {
            done: self.done,
            reward,
            combat: Some(CombatSummary::of(&log)),
        })
    }

    /// 学习座位血量。
    pub fn hp(&self) -> i32 {
        self.state.players[usize::from(LEARNING_SEAT)].hp
    }

    pub fn turn(&self) -> u8 {
        self.state.turn
    }

    pub fn phase(&self) -> Phase {
        self.state.phase
    }

    pub fn is_done(&self) -> bool {
        self.done
    }

    pub fn state(&self) -> &GameState {
        &self.state
    }

    fn fight(&self) -> CombatLog {
        let turn = self.state.turn;
        let mut rng = RngStreams::from_seed(seed_from_u64(
            self.episode_seed.wrapping_add(u64::from(turn)),
        ));
        let attacker: Vec<MinionInstance> =
            self.state.players[usize::from(LEARNING_SEAT)].board.clone();
        let defender: Vec<MinionInstance> = self
            .opponent
            .as_ref()
            .map(|opponent| opponent.board.clone())
            .unwrap_or_default();
        let setup = CombatSetup {
            attacker_hero: Some(&self.hero),
            defender_hero: self
                .opponent
                .as_ref()
                .map(|opponent| opponent.hero.as_str()),
            attacker_hand: self.state.players[usize::from(LEARNING_SEAT)].hand.len(),
            hand_slots: self.catalog.config.hand_slots,
            playstyles: Some(&self.state.enabled_playstyles),
            ..CombatSetup::default()
        };
        combat::simulate_with(
            &self.engine,
            &attacker,
            &defender,
            LEARNING_SEAT,
            OPPONENT_SEAT,
            LEARNING_SEAT,
            &mut rng,
            setup,
        )
    }

    fn reward(&self, log: &CombatLog) -> f32 {
        match self.reward_mode {
            RewardMode::Value => match (&self.weights, &self.opponent) {
                (Some(weights), Some(opponent)) => {
                    let own = self.own_sample();
                    let own_score = board_value(
                        weights,
                        &self.model,
                        &board_observation(&own, self.encoder.index()),
                    )
                    .unwrap_or(0.0);
                    let opponent_score = board_value(
                        weights,
                        &self.model,
                        &board_observation(opponent, self.encoder.index()),
                    )
                    .unwrap_or(0.0);
                    own_score - opponent_score
                }
                _ => 0.0,
            },
            RewardMode::Combat => match log.winner {
                Some(seat) if seat == LEARNING_SEAT => 1.0,
                Some(_) => -1.0,
                None => 0.0,
            },
        }
    }

    /// 学习座位当前棋盘，编码成与 C0 样本同形的结构。
    fn own_sample(&self) -> BoardSample {
        BoardSample {
            hero: self.hero.clone(),
            playstyles: self.playstyles,
            board: self.state.players[usize::from(LEARNING_SEAT)].board.clone(),
        }
    }
}

fn rules_view<'a>(engine: &'a Engine, state: &'a GameState) -> rule_engine::view::SeatView {
    rule_engine::view::view(engine, state, LEARNING_SEAT).expect("学习座位应存在")
}
