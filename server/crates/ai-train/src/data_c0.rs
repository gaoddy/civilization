//! C0 数据：从启发式对局采样棋盘，生成带胜负标签的棋盘对。
//!
//! 采样时点为第 5/10/15/20 回合招募开始，取每个存活座位当时的棋盘；
//! 每份样本再补一份「同费用替换一张」的扰动副本，扩大卡牌覆盖。
//! 标签用 [`rule_engine::combat::simulate`] 逐对模拟，随机流按对种子新建，
//! 不使用对局 `state.rng`。

use std::collections::BTreeSet;

use ai_core::playstyle_mask;
use rule_engine::combat;
use rule_engine::data::{CardType, Catalog};
use rule_engine::engine::Engine;
use rule_engine::intent::Intent;
use rule_engine::playstyles;
use rule_engine::rng::{RngStreams, seed_from_u64};
use rule_engine::state::{GameState, MinionInstance, Seat};
use sim::{HeuristicPolicy, TurnPolicy};

use crate::env::PLAYER_COUNT;
use crate::random_policy::RandomPolicy;

/// 采样回合：招募开始时的棋盘快照。
const SNAPSHOT_TURNS: [u8; 4] = [5, 10, 15, 20];

/// 攻击方实例编号起点。
const ATTACKER_BASE: u64 = 1;
/// 防守方实例编号起点；与攻击方错开，保证一场模拟内编号唯一。
const DEFENDER_BASE: u64 = 1001;

/// 一份棋盘样本：英雄、本局玩法掩码与棋盘随从。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardSample {
    pub hero: String,
    pub playstyles: u16,
    pub board: Vec<MinionInstance>,
}

/// 一对棋盘与胜负标签：`a_wins` 为真表示攻击方 `a` 获胜。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardPair {
    pub a: BoardSample,
    pub b: BoardSample,
    pub a_wins: bool,
}

/// 第 `pair_index` 对的随机种子；训练侧重新模拟同标签时使用同一推导。
pub fn pair_seed(seed: u64, pair_index: usize) -> u64 {
    seed ^ (pair_index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

/// 跑 `games` 局启发式对局，采样棋盘并生成带标签的棋盘对。
///
/// 返回的每一对都已由战斗模拟给出明确胜者；平局对被丢弃。
pub fn generate_pairs(catalog: &Catalog, games: usize, seed: u64) -> Vec<BoardPair> {
    let mut engine = Engine::from_catalog(catalog.clone());
    let mut pool = Vec::new();
    for game in 0..games {
        let game_seed = pair_seed(seed, game);
        let drawn = playstyles::draw(game_seed);
        let playstyle_ids: Vec<String> = drawn.iter().map(|style| style.id().to_string()).collect();
        let mut state = engine.initial_state(seed_from_u64(game_seed), drawn);
        let mut policy = RecordingPolicy::new(playstyle_mask(&playstyle_ids));
        sim::play_game_with(
            &mut engine,
            &mut state,
            game_seed,
            PLAYER_COUNT,
            &mut policy,
        )
        .expect("启发式对局不应失败");

        let mut rng = RandomPolicy::new(game_seed ^ 0xA5A5_5A5A_1234_5678);
        for sample in policy.finish() {
            if let Some(perturbed) = perturb(catalog, &sample, &mut rng) {
                pool.push(perturbed);
            }
            pool.push(sample);
        }
    }

    let mut rng = RandomPolicy::new(seed ^ 0xC0FF_EE00_1234_5678);
    for index in (1..pool.len()).rev() {
        let pick = rng.below((index + 1) as u64) as usize;
        pool.swap(index, pick);
    }

    let mut pairs = Vec::new();
    for chunk in pool.chunks_exact(2) {
        let (a, b) = (&chunk[0], &chunk[1]);
        if a.board.is_empty() || b.board.is_empty() {
            continue;
        }
        let attacker = renumber(&a.board, ATTACKER_BASE);
        let defender = renumber(&b.board, DEFENDER_BASE);
        let mut sim_rng = RngStreams::from_seed(seed_from_u64(pair_seed(seed, pairs.len())));
        let log = combat::simulate(catalog, &attacker, &defender, 0, 1, 0, &mut sim_rng);
        let Some(winner) = log.winner else {
            continue;
        };
        pairs.push(BoardPair {
            a: a.clone(),
            b: b.clone(),
            a_wins: winner == 0,
        });
    }
    pairs
}

/// 把棋盘实例编号重排到互不重叠的区间，供一场独立模拟使用。
fn renumber(board: &[MinionInstance], base: u64) -> Vec<MinionInstance> {
    board
        .iter()
        .enumerate()
        .map(|(index, minion)| {
            let mut minion = minion.clone();
            minion.instance_id = base + index as u64;
            minion
        })
        .collect()
}

/// 同费用替换一张随从，生成扰动样本；没有可替换的卡时返回 `None`。
fn perturb(catalog: &Catalog, sample: &BoardSample, rng: &mut RandomPolicy) -> Option<BoardSample> {
    if sample.board.is_empty() {
        return None;
    }
    let index = rng.below(sample.board.len() as u64) as usize;
    let replaced = &sample.board[index];
    let level = catalog.cards.get(&replaced.card_id)?.level;
    let candidates: Vec<&String> = catalog
        .cards
        .values()
        .filter(|card| {
            card.card_type == CardType::Minion
                && card.in_shop_pool
                && card.level == level
                && card.id != replaced.card_id
        })
        .map(|card| &card.id)
        .collect();
    if candidates.is_empty() {
        return None;
    }
    let pick = candidates[rng.below(candidates.len() as u64) as usize];
    let mut board = sample.board.clone();
    board[index] = MinionInstance::new(catalog, 0, pick, replaced.golden);
    Some(BoardSample {
        hero: sample.hero.clone(),
        playstyles: sample.playstyles,
        board,
    })
}

/// 包住启发式策略，在指定回合的首次决策前记录全部存活座位的棋盘。
struct RecordingPolicy {
    inner: HeuristicPolicy,
    playstyles: u16,
    seen: BTreeSet<u8>,
    samples: Vec<BoardSample>,
}

impl RecordingPolicy {
    fn new(playstyles: u16) -> Self {
        Self {
            inner: HeuristicPolicy,
            playstyles,
            seen: BTreeSet::new(),
            samples: Vec::new(),
        }
    }

    fn finish(self) -> Vec<BoardSample> {
        self.samples
    }
}

impl TurnPolicy for RecordingPolicy {
    fn plan(
        &mut self,
        state: &GameState,
        seat: Seat,
        engine: &Engine,
        blocked: &[Intent],
    ) -> Option<Intent> {
        if SNAPSHOT_TURNS.contains(&state.turn) && self.seen.insert(state.turn) {
            for player in &state.players {
                if player.eliminated || player.board.is_empty() {
                    continue;
                }
                let Some(hero) = player.hero.clone() else {
                    continue;
                };
                self.samples.push(BoardSample {
                    hero,
                    playstyles: self.playstyles,
                    board: player.board.clone(),
                });
            }
        }
        self.inner.plan(state, seat, engine, blocked)
    }
}
