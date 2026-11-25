//! C2 完整对局环境：在 [`Env`] 上加入英雄选择前置步骤与联盟座位采样。
//!
//! 英雄选择是每个座位开局的第一步：全部座位选完后才发英雄、进入第 1 回合招募。

use std::collections::BTreeMap;

use ai_core::{Action, ActionFeatures, Observation};
use rule_engine::data::Catalog;
use rule_engine::error::EngineError;
use rule_engine::state::{Phase, Seat};

use crate::env::{Env, Step};
use crate::league::{League, PolicyId};

/// 八人完整对局。
pub struct FullEnv {
    env: Env,
    league: League,
}

impl FullEnv {
    pub fn new(catalog: &Catalog, league: League) -> Self {
        Self {
            env: Env::new(catalog),
            league,
        }
    }

    /// 按对局种子与候选表开局；每个座位先选英雄。
    pub fn reset(
        &mut self,
        seed: u64,
        candidates: &BTreeMap<Seat, Vec<String>>,
    ) -> Result<(), EngineError> {
        self.env.reset_with_candidates(seed, candidates)
    }

    /// 为当前座位执行一个动作。
    pub fn step(&mut self, action: Action) -> Result<Step, EngineError> {
        self.env.step(action)
    }

    pub fn observe(&self, seat: Seat) -> Observation {
        self.env.observe(seat)
    }

    pub fn legal(&self, seat: Seat) -> Vec<Action> {
        self.env.legal(seat)
    }

    pub fn action_candidates(&self, seat: Seat) -> Vec<(Action, ActionFeatures)> {
        self.env.action_candidates(seat)
    }

    /// 只读对局状态，供启发式锚点直接下棋（评测用）。
    pub fn state(&self) -> &rule_engine::state::GameState {
        self.env.state()
    }

    /// 只读引擎，供启发式锚点读取卡表与规则。
    pub fn engine(&self) -> &rule_engine::engine::Engine {
        self.env.engine()
    }

    pub fn current_seat(&self) -> Option<Seat> {
        self.env.current_seat()
    }

    /// 英雄是否已全部选定、对局是否已开始。
    pub fn started(&self) -> bool {
        self.env.started()
    }

    pub fn turn(&self) -> u8 {
        self.env.turn()
    }

    /// 该座位的最终名次；对局未结束时为 `None`。
    pub fn placement(&self, seat: Seat) -> Option<u8> {
        self.env.placement(seat)
    }

    pub fn is_done(&self) -> bool {
        self.env.phase() == Phase::GameOver
    }

    pub fn league(&self) -> &League {
        &self.league
    }

    pub fn league_mut(&mut self) -> &mut League {
        &mut self.league
    }

    /// 给该座位抽一个对手身份。
    pub fn sample_policy(&mut self, seat: Seat) -> PolicyId {
        self.league.sample(seat)
    }
}
