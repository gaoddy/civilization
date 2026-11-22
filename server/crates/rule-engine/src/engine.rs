//! 最小对局引擎：初始化状态、开局与回合开始。

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::combat;
use crate::data::{CardType, Catalog, Era, Playstyle};
use crate::effects::{EffectCtx, Trigger};
use crate::error::EngineError;
use crate::events::GameEvent;
use crate::intent::{Intent, Outcome, Reject};
use crate::recruit;
use crate::rng::{RngStreams, Stream};
use crate::state::{
    GameState, InstanceId, Phase, PlayerState, Seat, StatSourceLayer, StatSourceOrigin,
    StatSystemSource,
};

/// 固定 8 人对局。
const PLAYER_COUNT: Seat = 8;

/// 规则引擎。持有只读卡牌目录，状态全部由调用方传入。
pub struct Engine {
    pub catalog: Catalog,
}

impl Engine {
    /// 以给定目录、种子与本局玩法创建初始状态：8 名玩家、`Lobby` 阶段、`turn = 0`。
    ///
    /// 玩法由调用方（房间服务或模拟器）通过 [`crate::playstyles::draw`] 抽取后传入；
    /// 共享池在这里一次性确定，商店留空等待首回合的刷新流程。
    ///
    /// # Panics
    ///
    /// `playstyles` 含 `Neutral` 或存在重复时 panic；调用方保证传入 5 个不同玩法。
    pub fn new(
        catalog: Catalog,
        seed: [u8; 32],
        playstyles: [Playstyle; 5],
    ) -> (Engine, GameState) {
        let engine = Engine { catalog };
        let state = engine.initial_state(seed, playstyles);
        (engine, state)
    }

    /// 以只读目录创建引擎句柄。
    ///
    /// 同一目录可连续创建多局初始状态，不需要按局复制 [`Catalog`]；
    /// 批量模拟与同进程多房间共用这一入口。
    pub fn from_catalog(catalog: Catalog) -> Engine {
        Engine { catalog }
    }

    /// 以本引擎的目录创建一局初始状态：8 名玩家、`Lobby` 阶段、`turn = 0`。
    ///
    /// # Panics
    ///
    /// `playstyles` 含 `Neutral` 或存在重复时 panic；调用方保证传入 5 个不同玩法。
    pub fn initial_state(&self, seed: [u8; 32], playstyles: [Playstyle; 5]) -> GameState {
        let mut rng = RngStreams::from_seed(seed);
        let first_seat = rng.below(Stream::Matchmaking, u64::from(PLAYER_COUNT)) as Seat;
        let enabled_playstyles = playstyle_set(playstyles);
        let pool = build_pool(&self.catalog);
        let config = &self.catalog.config;

        let players = (0..PLAYER_COUNT)
            .map(|seat| PlayerState {
                seat,
                hp: config.initial_hp,
                gold: 0,
                era: Era::Prehistoric,
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
                froze_this_turn: false,
                seven_voyages: None,
            })
            .collect();

        let previous_opponent = (0..PLAYER_COUNT).map(|seat| (seat, None)).collect();
        let ranking = Vec::new();

        GameState {
            turn: 0,
            phase: Phase::Lobby,
            players,
            pool,
            rng,
            first_seat,
            enabled_playstyles,
            previous_opponent,
            last_winner: None,
            pairings: Vec::new(),
            pending_settle: BTreeMap::new(),
            last_won: BTreeSet::new(),
            pending: VecDeque::new(),
            ranking,
            next_instance_id: 1,
            auras: BTreeMap::new(),
            aura_sources: BTreeMap::new(),
        }
    }

    /// 校验 8 个座位都选了目录中的英雄卡，允许重复；写入英雄，进入第 1 回合招募阶段。
    ///
    /// 第 1 回合金币与商店都在这里完成；`start_turn` 只负责第 2 回合及以后。
    pub fn start_game(
        &self,
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
            if !self.catalog.heroes.contains_key(card_id) {
                return Err(EngineError(format!("英雄卡 {card_id} 不在英雄目录中")));
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
            player.froze_this_turn = false;
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
    pub fn start_turn(&self, state: &mut GameState) -> Result<Vec<GameEvent>, EngineError> {
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
            // 解冻只在冻结的当回合可用；冻结标记本身保留，见 `recruit::roll_player`。
            player.froze_this_turn = false;
            if player.eliminated {
                continue;
            }
            player.first_refresh_used = false;
            player.first_refresh_pending = true;
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
        // 七下西洋在航程结束后的这个时点一次性到账，与当回合正常金币相加；
        // 到账后清空航程，允许同一招募阶段再次启航。
        for player in &mut state.players {
            if player.eliminated {
                continue;
            }
            player.gold = gold;
            if let Some(voyage) = player.seven_voyages.take() {
                if voyage.remaining == 0 {
                    let pending = u8::try_from(voyage.pending).unwrap_or(u8::MAX);
                    player.gold = player.gold.saturating_add(pending);
                } else {
                    player.seven_voyages = Some(voyage);
                }
            }
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
        &self,
        state: &mut GameState,
        seat: Seat,
        intent: Intent,
    ) -> Result<Outcome, Reject> {
        let outcome = recruit::apply(self, state, seat, intent)?;
        self.recompute_auras_for(state, seat);
        Ok(outcome)
    }

    /// 重抽指定玩家的商店，保留冻结槽位；返回 `ShopRolled` 事件。
    pub fn roll_shop(&self, state: &mut GameState, seat: Seat) -> Vec<GameEvent> {
        recruit::roll_player(self, state, seat, true);
        vec![GameEvent::ShopRolled { seat }]
    }

    /// 繁衍：每回合为棋盘随从增加固定层数 + `breed` 层数的生命值。
    ///
    /// 固定层数来自卡牌所引用技能 `keyword` 中的「繁衍 N」（无数字按 1）。
    /// 繁衍只加生命、不加攻击，攻击力为 0 的随从也只获得生命。
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
            minion.record_stat_source(
                StatSourceLayer::Growth,
                StatSourceOrigin::System(StatSystemSource::Breed),
                0,
                amount,
            );
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
            let Some(keyword) = self.catalog.skill(*skill_id).keyword.as_deref() else {
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
    fn apply_contagion(&self, state: &mut GameState, seat: Seat) {
        let Some(player) = state.players.get_mut(usize::from(seat)) else {
            return;
        };
        for minion in &mut player.board {
            if minion.contagion > 0 {
                minion.stats.health.add_permanent(-minion.contagion);
                minion.record_stat_source(
                    StatSourceLayer::Permanent,
                    StatSourceOrigin::System(StatSystemSource::Contagion),
                    0,
                    -minion.contagion,
                );
            }
        }
        self.resolve_recruit_deaths(state, seat);
    }

    /// 招募阶段移除生命值 ≤ 0 的随从，并结算亡语与播种。
    ///
    /// 传染与潜伏爆发共用：两者都在招募回合内造成伤害，击杀必须立即离场，
    /// 否则下一次 `Settle` 清掉临时层后会把死掉的随从复活。
    pub(crate) fn resolve_recruit_deaths(&self, state: &mut GameState, seat: Seat) {
        let dead = {
            let Some(player) = state.players.get_mut(usize::from(seat)) else {
                return;
            };
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
                level_override: None,
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
                    self.catalog.skill(*skill_id).keyword.as_deref() == Some(keyword)
                })
            })
            .unwrap_or(false)
    }

    /// 按站位从左到右触发棋盘随从的 `OnTurnStart`；金色随从由执行器触发两次。
    ///
    /// 英雄卡先结算：无来源触发由执行器解析到英雄池，与首次刷新、战斗开始的
    /// 「英雄被动优先」口径一致。
    fn fire_turn_start(&self, state: &mut GameState, seat: Seat) -> Vec<GameEvent> {
        let Some(player) = state.players.get(usize::from(seat)) else {
            return Vec::new();
        };
        let has_hero = player.hero.is_some();
        let owners: Vec<(InstanceId, bool)> = player
            .board
            .iter()
            .map(|minion| (minion.instance_id, minion.golden))
            .collect();
        let mut events = Vec::new();
        if has_hero {
            events.extend(self.fire(
                state,
                Trigger::OnTurnStart,
                EffectCtx {
                    seat,
                    source: None,
                    target: None,
                    event_target: None,
                    golden: false,
                    level_override: None,
                },
            ));
        }
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
                    level_override: None,
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

/// 把调用方传入的 5 个玩法转为集合；拒绝 `Neutral` 与重复。
fn playstyle_set(playstyles: [Playstyle; 5]) -> BTreeSet<Playstyle> {
    let mut enabled = BTreeSet::new();
    for playstyle in playstyles {
        assert_ne!(
            playstyle,
            Playstyle::Neutral,
            "enabled_playstyles 不能包含 Neutral"
        );
        assert!(
            enabled.insert(playstyle),
            "enabled_playstyles 不能包含重复玩法"
        );
    }
    enabled
}

/// 按 `pool_counts[level - 1]` 为每张进商店卡池的随从填充共享池。
fn build_pool(catalog: &Catalog) -> BTreeMap<String, u16> {
    let mut pool = BTreeMap::new();
    for card in catalog.cards.values() {
        if card.card_type != CardType::Minion || !card.in_shop_pool {
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
