//! 光环与条件光环的重算。
//!
//! 光环是静态派生效果：每次区域变化后从零全量重算，结果只写入
//! [`GameState::auras`]，不改变随从的持久字段。战斗快照在结算层临时折叠。
//!
//! 规则口径见 `docs/versions/v0.1/product/rules/keywords-and-mechanics.md`
//! §3.2 与 §10。遍历顺序固定：座位升序、棋盘点顺序；不消耗随机流。

use std::collections::BTreeMap;

use crate::combat;
use crate::effects::{Compare, Effect, EffectCtx, Op, Pick, Selector, Side, Trigger};
use crate::engine::Engine;
use crate::state::{AuraEffect, GameState, InstanceId, PlayerState, Seat};

impl Engine {
    /// 全量重算所有未出局玩家的光环派生值。
    ///
    /// 来源为各玩家棋盘上的每个随从，以及该玩家英雄卡；效果只作用于来源
    /// **同一玩家**的棋盘。缺失卡牌、空棋盘与空条件都退化为无光环。
    pub fn recompute_auras(&mut self, state: &mut GameState) {
        let mut computed: BTreeMap<InstanceId, AuraEffect> = BTreeMap::new();

        let mut seats: Vec<Seat> = state
            .players
            .iter()
            .filter(|player| !player.eliminated)
            .map(|player| player.seat)
            .collect();
        seats.sort_unstable();

        for seat in seats {
            let Some(player) = state.players.iter().find(|player| player.seat == seat) else {
                continue;
            };
            computed.extend(self.player_auras(state, player));
        }

        state.auras = computed;
    }

    /// 某名玩家棋盘与其英雄卡产生的全部光环派生值。
    ///
    /// 不读取也不写入 `state.auras`，因此对已出局玩家的最终阵容同样可用
    /// （幽灵战按出局者自己的光环来源折叠一次）。实例编号全局唯一，直接合并。
    pub fn player_auras(
        &self,
        state: &GameState,
        player: &PlayerState,
    ) -> BTreeMap<InstanceId, AuraEffect> {
        let mut computed = BTreeMap::new();
        for (source, effects) in self.aura_sources(player) {
            for effect in effects {
                if !self.aura_active(state, player, &effect, source) {
                    continue;
                }
                for target in aura_targets(self, state, &effect, player, source) {
                    accumulate(&effect, computed.entry(target).or_default());
                }
            }
        }
        computed
    }

    /// 收集某玩家所有光环来源：棋盘点顺序的随从，然后是英雄卡。
    fn aura_sources(&self, player: &PlayerState) -> Vec<(Option<InstanceId>, Vec<Effect>)> {
        let mut sources: Vec<(Option<InstanceId>, String)> = player
            .board
            .iter()
            .map(|minion| (Some(minion.instance_id), minion.card_id.clone()))
            .collect();
        if let Some(hero) = &player.hero {
            sources.push((None, hero.clone()));
        }

        let mut result = Vec::with_capacity(sources.len());
        for (source, card_id) in sources {
            let Some(card) = self.catalog.cards.get(&card_id) else {
                continue;
            };
            let mut effects = Vec::new();
            for skill_id in &card.skills {
                let Some(skill) = self.catalog.skills.get(skill_id) else {
                    continue;
                };
                effects.extend(
                    skill
                        .effects
                        .iter()
                        .filter(|effect| {
                            matches!(effect.trigger, Trigger::Aura | Trigger::ConditionalAura)
                        })
                        .cloned(),
                );
            }
            if !effects.is_empty() {
                result.push((source, effects));
            }
        }
        result
    }

    /// 光环是否处于激活状态：`Aura` 恒生效；`ConditionalAura` 只在条件成立时生效。
    fn aura_active(
        &self,
        state: &GameState,
        player: &PlayerState,
        effect: &Effect,
        source: Option<InstanceId>,
    ) -> bool {
        match effect.trigger {
            Trigger::Aura => true,
            Trigger::ConditionalAura => effect.params.condition.as_ref().is_some_and(|condition| {
                self.condition_holds_on_board(state, player, condition, source)
            }),
            _ => false,
        }
    }

    /// 条件只统计来源自己玩家的棋盘；招募阶段的口径下敌方阵营恒为空。
    fn condition_holds_on_board(
        &self,
        state: &GameState,
        player: &PlayerState,
        condition: &crate::effects::Condition,
        source: Option<InstanceId>,
    ) -> bool {
        if matches!(condition.side, Side::Enemy) {
            return compare_count(0, condition.compare, condition.count);
        }
        let ctx = aura_ctx(player.seat, source);
        let count = player
            .board
            .iter()
            .filter(|minion| {
                self.filter_matches(state, &condition.filter, minion.instance_id, &ctx)
            })
            .count();
        compare_count(count as i32, condition.compare, condition.count)
    }
}

fn compare_count(count: i32, compare: Compare, expected: i32) -> bool {
    match compare {
        Compare::Ge => count >= expected,
        Compare::Le => count <= expected,
        Compare::Eq => count == expected,
        Compare::Gt => count > expected,
        Compare::Lt => count < expected,
    }
}

/// 把一段光环效果解析为来源玩家棋盘上的目标集合。
fn aura_targets(
    engine: &Engine,
    state: &GameState,
    effect: &Effect,
    player: &PlayerState,
    source: Option<InstanceId>,
) -> Vec<InstanceId> {
    let ctx = aura_ctx(player.seat, source);
    match &effect.selector {
        Selector::SelfCard => source
            .filter(|id| player.board.iter().any(|minion| minion.instance_id == *id))
            .map(|id| vec![id])
            .unwrap_or_default(),
        Selector::Target => Vec::new(),
        Selector::Side {
            side,
            pick,
            filters,
        } => {
            // 光环只作用于来源自己的棋盘：敌方阵营没有合法目标。
            if matches!(side, Side::Enemy) {
                return Vec::new();
            }
            let pool: Vec<InstanceId> = player
                .board
                .iter()
                .filter(|minion| {
                    filters.iter().all(|filter| {
                        engine.filter_matches(state, filter, minion.instance_id, &ctx)
                    })
                })
                .map(|minion| minion.instance_id)
                .collect();
            pick_targets(engine, state, pool, *pick)
        }
    }
}

/// 选取语义与 `exec::apply_pick` 对齐。
///
/// `TopAttack` 取攻击力最高者（并列时按棋盘顺序取前者）；没有显式目标的
/// `Single`、以及需要随机流的 `Random` 在光环求值中不产生目标（光环不消耗随机流）。
fn pick_targets(
    engine: &Engine,
    state: &GameState,
    pool: Vec<InstanceId>,
    pick: Pick,
) -> Vec<InstanceId> {
    match pick {
        Pick::All => pool,
        Pick::Single | Pick::Random => Vec::new(),
        Pick::Leftmost => pool.first().copied().into_iter().collect(),
        Pick::LeftmostAdjacent => pool.iter().take(2).copied().collect(),
        Pick::TopAttack => {
            let mut best: Option<(InstanceId, i32)> = None;
            for id in pool {
                let Some(attack) = board_attack(engine, state, id) else {
                    continue;
                };
                if best.is_none_or(|(_, current)| attack > current) {
                    best = Some((id, attack));
                }
            }
            best.map(|(id, _)| vec![id]).unwrap_or_default()
        }
        // 相邻需要来源实例的棋盘下标，由相邻机制任务在光环求值中接入。
        Pick::Adjacent => Vec::new(),
        // 最低攻击与 `apply_pick` 同口径：攻击力最低，并列取棋盘最左侧。
        Pick::LowestAttack => {
            let mut best: Option<(InstanceId, i32)> = None;
            for id in pool {
                let Some(attack) = board_attack(engine, state, id) else {
                    continue;
                };
                if best.is_none_or(|(_, current)| attack < current) {
                    best = Some((id, attack));
                }
            }
            best.map(|(id, _)| vec![id]).unwrap_or_default()
        }
    }
}

fn board_attack(engine: &Engine, state: &GameState, id: InstanceId) -> Option<i32> {
    state
        .players
        .iter()
        .flat_map(|player| player.board.iter())
        .find(|minion| minion.instance_id == id)
        .map(|minion| combat::effective_attack(&engine.catalog, minion))
}

fn aura_ctx(seat: Seat, source: Option<InstanceId>) -> EffectCtx {
    EffectCtx {
        seat,
        source,
        target: None,
        event_target: None,
        golden: false,
        discover_level: None,
    }
}

/// 把一段光环效果累加到目标派生值上；关键词幂等，数值相加。
fn accumulate(effect: &Effect, target: &mut AuraEffect) {
    match effect.op {
        Op::ModifyStats => {
            target.attack = target
                .attack
                .saturating_add(effect.params.attack.unwrap_or(0));
            target.health = target
                .health
                .saturating_add(effect.params.health.unwrap_or(0));
        }
        Op::GrantKeyword => {
            let Some(keyword) = &effect.params.keyword else {
                return;
            };
            target.keywords.insert(keyword.keyword.clone());
        }
        // 解析层已限制光环只能使用上述两个原语；其余按无效果处理。
        _ => return,
    }
    target.floor_one = true;
}
