//! 最小对局引擎：初始化状态、开局与回合开始。

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::combat;
use crate::data::{CardType, Catalog, Era, Tribe};
use crate::effects::{EffectCtx, Trigger};
use crate::error::EngineError;
use crate::events::GameEvent;
use crate::intent::{Intent, Outcome, Reject};
use crate::recruit;
use crate::rng::{RngStreams, Stream};
use crate::state::{GameState, InstanceId, Phase, PlayerState, Seat};

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
                first_refresh_pending: true,
                recruit_done: false,
            })
            .collect();

        let previous_opponent = (0..PLAYER_COUNT).map(|seat| (seat, None)).collect();
        let ranking = Vec::new();

        let state = GameState {
            turn: 0,
            phase: Phase::Lobby,
            players,
            pool,
            rng,
            first_seat,
            enabled_tribes,
            previous_opponent,
            last_winner: None,
            pairings: Vec::new(),
            pending_settle: BTreeMap::new(),
            last_won: BTreeSet::new(),
            pending: VecDeque::new(),
            ranking,
            next_instance_id: 1,
            auras: BTreeMap::new(),
        };

        (Engine { catalog }, state)
    }

    /// 校验 8 个座位都选了目录中的英雄卡，允许重复；写入英雄，进入第 1 回合招募阶段。
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
            player.first_refresh_pending = true;
            player.recruit_done = false;
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
        self.recompute_auras(state);
        Ok(events)
    }

    /// 开始下一回合：先结算回合开始效果，再发金币并刷新商店。
    ///
    /// `start_game` 已经发过第 1 回合金币，因此本方法不用于第 1 回合。
    /// 结算完成后调用时接受 `Settle`，并发出进入 `Recruit` 的 `PhaseChanged`。
    ///
    /// 回合开始顺序固定（关键词机制 §2.1）：每个未出局玩家按座位升序，
    /// 其棋盘按站位从左到右依次结算繁衍、传染与其他 `OnTurnStart` 效果；
    /// 全部效果结算完毕后，才发放本回合金币并刷新商店。
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
            player.recruit_done = false;
            if player.eliminated {
                continue;
            }
            player.first_refresh_used = false;
            player.first_refresh_pending = true;
            player.turns_in_era = player.turns_in_era.saturating_add(1);
        }

        let mut events = vec![GameEvent::TurnStarted { turn }];
        if previous_phase != Phase::Recruit {
            events.push(GameEvent::PhaseChanged {
                phase: Phase::Recruit,
                turn,
            });
        }

        // (a) 繁衍 → (b) 传染 → (c) 其他回合开始效果；座位升序、站位从左到右。
        let seats: Vec<Seat> = state
            .players
            .iter()
            .filter(|player| !player.eliminated)
            .map(|player| player.seat)
            .collect();
        for seat in seats {
            self.apply_breed(state, seat);
            self.apply_contagion(state, seat);
            events.extend(self.fire_turn_start(state, seat));
        }
        self.recompute_auras(state);

        // 回合开始效果全部结算后，才发金币并刷新商店。
        for player in &mut state.players {
            if player.eliminated {
                continue;
            }
            player.gold = gold;
        }
        for seat in 0..state.players.len() {
            if state.players[seat].eliminated {
                continue;
            }
            events.extend(self.roll_shop(state, seat as Seat));
        }
        self.recompute_auras(state);
        Ok(events)
    }

    /// 以指定座位执行一条意图；拒绝时状态保持不变。
    pub fn apply(
        &mut self,
        state: &mut GameState,
        seat: Seat,
        intent: Intent,
    ) -> Result<Outcome, Reject> {
        let outcome = recruit::apply(self, state, seat, intent)?;
        self.recompute_auras(state);
        Ok(outcome)
    }

    /// 重抽指定玩家的商店，保留冻结槽位；返回 `ShopRolled` 事件。
    pub fn roll_shop(&mut self, state: &mut GameState, seat: Seat) -> Vec<GameEvent> {
        recruit::roll_player(self, state, seat, true);
        vec![GameEvent::ShopRolled { seat }]
    }

    /// 繁衍：每回合为棋盘随从增加固定层数 + `breed` 层数的生命值。
    ///
    /// 固定层数来自卡牌所引用技能 `keyword` 中的「繁衍 N」（无数字按 1）。
    /// 繁衍只加生命、不加攻击；学者同样只获得生命。
    fn apply_breed(&self, state: &mut GameState, seat: Seat) {
        let Some(player) = state.players.get_mut(usize::from(seat)) else {
            return;
        };
        for minion in &mut player.board {
            let layers = self
                .static_breed(&minion.card_id)
                .saturating_add(minion.breed);
            // 金色随从的关键词触发两次；繁衍等价于 N×2。
            let amount = if minion.golden {
                layers.saturating_mul(2)
            } else {
                layers
            };
            minion.stats.health.add_growth(amount);
            // `breed_applied` 记录本实例累计已通过繁衍获得的生命。
            minion.breed_applied = minion.breed_applied.saturating_add(amount);
        }
    }

    fn static_breed(&self, card_id: &str) -> i32 {
        let Some(card) = self.catalog.cards.get(card_id) else {
            return 0;
        };
        let mut total = 0i32;
        for skill_id in &card.skills {
            let Some(keyword) = self
                .catalog
                .skills
                .get(skill_id)
                .and_then(|skill| skill.keyword.as_deref())
            else {
                continue;
            };
            let Some(rest) = keyword.strip_prefix("breed") else {
                continue;
            };
            let rest = rest.trim();
            let layers = if rest.is_empty() {
                1
            } else {
                rest.parse::<i32>().unwrap_or(1)
            };
            total = total.saturating_add(layers);
        }
        total
    }

    /// 传染：每层施加一个永久 -1 生命修正，然后移除生命降至 0 的随从。
    ///
    /// 传染致死的随从在移除后结算亡语，再按「播种」召唤幼苗；复生只作用于战斗，
    /// 不在回合开始生效。
    fn apply_contagion(&mut self, state: &mut GameState, seat: Seat) {
        let dead = {
            let Some(player) = state.players.get_mut(usize::from(seat)) else {
                return;
            };
            for minion in &mut player.board {
                if minion.contagion > 0 {
                    minion.stats.health.add_permanent(-minion.contagion);
                }
            }
            let board = std::mem::take(&mut player.board);
            let mut kept = Vec::with_capacity(board.len());
            let mut dead = Vec::new();
            for minion in board {
                if combat::effective_health(&self.catalog, &minion) > 0 {
                    kept.push(minion);
                } else {
                    dead.push(minion);
                }
            }
            player.board = kept;
            dead
        };

        for minion in dead {
            let ctx = EffectCtx {
                seat,
                source: Some(minion.instance_id),
                target: None,
                event_target: None,
                golden: minion.golden,
                discover_level: None,
            };
            self.fire_death_recruit(state, &minion.card_id, &ctx);
            if self.card_has_keyword(&minion.card_id, "sow") {
                let count = if minion.golden { 2 } else { 1 };
                for _ in 0..count {
                    let _ = self.summon_recruit(state, seat, combat::SEEDLING_TOKEN, None, false);
                }
            }
        }
    }

    /// 卡牌是否通过技能 `keyword` 携带某个常驻关键词。
    pub(crate) fn card_has_keyword(&self, card_id: &str, keyword: &str) -> bool {
        self.catalog
            .cards
            .get(card_id)
            .map(|card| {
                card.skills.iter().any(|skill_id| {
                    self.catalog
                        .skills
                        .get(skill_id)
                        .and_then(|skill| skill.keyword.as_deref())
                        == Some(keyword)
                })
            })
            .unwrap_or(false)
    }

    /// 按站位从左到右触发棋盘随从的 `OnTurnStart`；金色随从由执行器触发两次。
    fn fire_turn_start(&mut self, state: &mut GameState, seat: Seat) -> Vec<GameEvent> {
        let Some(player) = state.players.get(usize::from(seat)) else {
            return Vec::new();
        };
        let owners: Vec<(InstanceId, bool)> = player
            .board
            .iter()
            .map(|minion| (minion.instance_id, minion.golden))
            .collect();
        let mut events = Vec::new();
        for (instance_id, golden) in owners {
            events.extend(self.fire(
                state,
                Trigger::OnTurnStart,
                EffectCtx {
                    seat,
                    source: Some(instance_id),
                    target: None,
                    event_target: None,
                    golden,
                    discover_level: None,
                },
            ));
        }
        events
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
