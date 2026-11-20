//! 光环与条件光环的重算。
//!
//! 光环是静态派生效果：每次区域变化后从零全量重算，结果只写入
//! [`GameState::auras`]，不改变随从的持久字段。战斗快照在结算层临时折叠。
//!
//! 规则口径见 `docs/versions/v0.1/product/rules/keywords-and-mechanics.md`
//! §3.2 与 §10。遍历顺序固定：座位升序、棋盘点顺序；不消耗随机流。

use std::collections::{BTreeMap, BTreeSet};

use crate::combat;
use crate::effects::{Compare, Effect, EffectCtx, Op, Pick, Selector, Side, Trigger};
use crate::engine::Engine;
use crate::state::{AuraEffect, GameState, InstanceId, MinionInstance, PlayerState, Seat};

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

    /// 只重算一个座位的棋盘光环。
    ///
    /// 光环只作用于来源同一玩家的棋盘，因此可以先移除该玩家棋盘实例的旧条目，
    /// 再并入新计算结果；其他玩家的条目原样保留。
    ///
    /// 同时清理已不在任何未出局玩家棋盘上的陈旧条目：本座位刚出售或移除的随从
    /// 实例已不在当前棋盘 id 集合内，若不清理会与全量重算的结果不一致。
    pub fn recompute_auras_for(&mut self, state: &mut GameState, seat: Seat) {
        let computed = {
            let Some(player) = state.players.iter().find(|player| player.seat == seat) else {
                return;
            };
            if player.eliminated {
                BTreeMap::new()
            } else {
                self.player_auras(state, player)
            }
        };
        let own_ids: BTreeSet<InstanceId> = state
            .players
            .iter()
            .find(|player| player.seat == seat)
            .map(|player| {
                player
                    .board
                    .iter()
                    .map(|minion| minion.instance_id)
                    .collect()
            })
            .unwrap_or_default();
        let live_ids: BTreeSet<InstanceId> = state
            .players
            .iter()
            .filter(|player| !player.eliminated)
            .flat_map(|player| player.board.iter().map(|minion| minion.instance_id))
            .collect();
        state
            .auras
            .retain(|id, _| !own_ids.contains(id) && live_ids.contains(id));
        state.auras.extend(computed);
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
                if !self.aura_active(state, player, effect, source) {
                    continue;
                }
                // 群聚的计数与目标无关，每段效果只算一次。
                let scale = scaling_count(self, state, player, effect, source);
                for target in aura_targets(self, state, effect, player, source) {
                    accumulate(effect, computed.entry(target).or_default(), scale);
                }
            }
        }
        computed
    }

    /// 收集某玩家所有光环来源：棋盘点顺序的随从，然后是英雄卡。
    ///
    /// 返回的效果直接借用 `self.catalog` 中的技能定义，避免每次重算都克隆整段效果。
    fn aura_sources<'a>(
        &'a self,
        player: &'a PlayerState,
    ) -> Vec<(Option<InstanceId>, Vec<&'a Effect>)> {
        // `source == None` 表示英雄卡：光环技能只从英雄池取，避免与同名随从串用。
        let board_sources = player.board.iter().map(|minion| {
            (
                Some(minion.instance_id),
                self.catalog
                    .cards
                    .get(&minion.card_id)
                    .map(|card| &card.skills),
            )
        });
        let hero_source = player
            .hero
            .as_ref()
            .map(|hero| (None, self.catalog.heroes.get(hero).map(|hero| &hero.skills)));

        let mut result = Vec::new();
        for (source, skills) in board_sources.chain(hero_source) {
            let Some(skills) = skills else {
                continue;
            };
            let mut effects = Vec::new();
            for skill_id in skills {
                let Some(skill) = self.catalog.skills.get(skill_id) else {
                    continue;
                };
                effects.extend(skill.effects.iter().filter(|effect| {
                    matches!(effect.trigger, Trigger::Aura | Trigger::ConditionalAura)
                }));
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
        Selector::Target { .. } => Vec::new(),
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
            let source_index = source.and_then(|id| {
                player
                    .board
                    .iter()
                    .position(|minion| minion.instance_id == id)
            });
            pick_targets(engine, state, &player.board, pool, *pick, source_index)
        }
    }
}

/// 群聚的计数：来源玩家棋盘上满足本段筛选、且不是来源自身的随从数量。
///
/// 没有 `perAttack` / `perHealth` 时为 0；英雄卡不是棋盘随从，天然不计数。
/// `maxCount` 缺省 6，负数按 0 处理。
fn scaling_count(
    engine: &Engine,
    state: &GameState,
    player: &PlayerState,
    effect: &Effect,
    source: Option<InstanceId>,
) -> i32 {
    if effect.params.per_attack.is_none() && effect.params.per_health.is_none() {
        return 0;
    }
    let Selector::Side { filters, .. } = &effect.selector else {
        return 0;
    };
    let ctx = aura_ctx(player.seat, source);
    let count = player
        .board
        .iter()
        .filter(|minion| {
            Some(minion.instance_id) != source
                && filters
                    .iter()
                    .all(|filter| engine.filter_matches(state, filter, minion.instance_id, &ctx))
        })
        .count();
    let max_count = effect.params.max_count.unwrap_or(6).max(0) as usize;
    count.min(max_count) as i32
}

/// 选取语义与 `exec::apply_pick` 对齐。
///
/// `TopAttack` 取攻击力最高者（并列时按棋盘顺序取前者）；没有显式目标的
/// `Single`、以及需要随机流的 `Random` 在光环求值中不产生目标（光环不消耗随机流）。
fn pick_targets(
    engine: &Engine,
    state: &GameState,
    board: &[MinionInstance],
    pool: Vec<InstanceId>,
    pick: Pick,
    source_index: Option<usize>,
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
        // 相邻只作用于来源左右两侧的存活随从；英雄卡没有来源实例，返回空。
        // 邻居仍需满足选择器筛选（`pool` 已过滤），携带者自身不在候选内。
        Pick::Adjacent => {
            let Some(index) = source_index else {
                return Vec::new();
            };
            let mut targets = Vec::with_capacity(2);
            for position in [index.checked_sub(1), index.checked_add(1)]
                .into_iter()
                .flatten()
            {
                let Some(neighbor) = board.get(position) else {
                    continue;
                };
                if combat::effective_health(&engine.catalog, neighbor) > 0
                    && pool.contains(&neighbor.instance_id)
                {
                    targets.push(neighbor.instance_id);
                }
            }
            targets
        }
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
        level_override: None,
    }
}

/// 把一段光环效果累加到目标派生值上；关键词幂等，数值相加。
///
/// `scale` 是群聚计数：基础 `attack`/`health` 之外，每个被计入的「其他同玩法
/// 友方随从」再加一次 `perAttack`/`perHealth`。金色不额外翻倍。
fn accumulate(effect: &Effect, target: &mut AuraEffect, scale: i32) {
    match effect.op {
        Op::ModifyStats => {
            target.attack = target
                .attack
                .saturating_add(effect.params.attack.unwrap_or(0))
                .saturating_add(effect.params.per_attack.unwrap_or(0).saturating_mul(scale));
            target.health = target
                .health
                .saturating_add(effect.params.health.unwrap_or(0))
                .saturating_add(effect.params.per_health.unwrap_or(0).saturating_mul(scale));
        }
        Op::GrantKeyword => {
            let Some(keyword) = &effect.params.keyword else {
                return;
            };
            // 与 `exec.rs` 的 GrantKeyword 分支一致：`armor N` 的层数写进
            // 关键词字符串，否则战斗折叠后会退化为无层数的 `armor`。
            let name = keyword.keyword.as_str();
            let layer = keyword.value.or(effect.params.value);
            match (name, layer) {
                (combat::ARMOR, Some(count)) => {
                    // 多个光环来源的减伤累加到同一条 `armor N`：关键词集合按
                    // 字符串去重，直接插入会让同层来源互相覆盖。
                    let existing = target.keywords.iter().find_map(|keyword| {
                        combat::armor_layer(keyword).map(|v| (keyword.clone(), v))
                    });
                    match existing {
                        Some((old, value)) => {
                            target.keywords.remove(&old);
                            target.keywords.insert(format!(
                                "{} {}",
                                combat::ARMOR,
                                value.saturating_add(count)
                            ));
                        }
                        None => {
                            target.keywords.insert(format!("{} {count}", combat::ARMOR));
                        }
                    }
                }
                _ => {
                    target.keywords.insert(name.to_string());
                }
            }
        }
        // 解析层已限制光环只能使用上述两个原语；其余按无效果处理。
        _ => return,
    }
    target.floor_one = true;
}
