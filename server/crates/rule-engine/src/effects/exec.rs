//! 效果执行器：把解析完成的效果 AST 落成对局状态变化。
//!
//! 本模块实现招募阶段的六个即时动作（成长、属性修改、赋予关键词、移除成长、
//! 禁止攻击、造成伤害）以及 `OnPlay` / `OnPlayEffect` / `OnActivate` / `OnUse`
//! 四个触发时机。召唤、复制、发现、刷新商店、增加商店槽位、获得随从由
//! Task 12–14 实现；光环由 Task 11 实现；战斗内触发由 Task 13 实现。
//!
//! 规则口径见 `docs/versions/v0.1/product/rules/keywords-and-mechanics.md`
//! §2、§3、§8、§9、§10。所有随机性只来自 [`Stream::Effects`]，集合与遍历顺序
//! 全部确定。

use std::collections::BTreeMap;

use crate::combat;
use crate::data::{CardType, Catalog, Tribe};
use crate::effects::{
    Duration, Effect, Filter, KeywordSpec, Op, Pick, Selector, Side, Trigger, parse_effects,
};
use crate::engine::Engine;
use crate::events::GameEvent;
use crate::rng::Stream;
use crate::state::{GameState, InstanceId, MinionInstance, Modifier, Phase, PlayerState, Seat};

/// 禁止攻击时写入 `granted` 的占位关键词，用于记录该标记的时长来源。
const CANNOT_ATTACK_KEYWORD: &str = "禁攻";

/// 执行一段效果时需要的上下文。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectCtx {
    /// 效果归属的座位。
    pub seat: Seat,
    /// 效果来源的棋盘实例；效果牌与英雄技能为 `None`。
    pub source: Option<InstanceId>,
    /// 玩家指定的目标。
    pub target: Option<InstanceId>,
    /// 事件目标（如「它」），在 `target` 缺失时使用。
    pub event_target: Option<InstanceId>,
    /// 来源是否为金色（整段效果连续执行两次）。
    pub golden: bool,
}

impl Engine {
    /// 执行某个触发时机下、由上下文确定的卡牌效果。
    ///
    /// `OnPlay` / `OnActivate`（以及来源在棋盘上的 `OnPlayEffect`）从来源卡牌
    /// 收集该时机下的全部效果；`OnUse` 收集本座位所有响应打出效果牌的随从。
    /// 本任务未实现的触发时机返回空事件。
    pub fn fire(
        &mut self,
        state: &mut GameState,
        trigger: Trigger,
        ctx: EffectCtx,
    ) -> Vec<GameEvent> {
        match trigger {
            Trigger::OnPlay | Trigger::OnPlayEffect | Trigger::OnActivate => {
                match self.owner_card_id(state, trigger, &ctx) {
                    Some(card_id) => self.run_card_effects(state, &card_id, trigger, &ctx),
                    None => Vec::new(),
                }
            }
            Trigger::OnUse => self.fire_on_use(state, &ctx),
            _ => Vec::new(),
        }
    }

    /// 直接执行一张卡牌在某个触发时机下的效果。
    ///
    /// 效果牌打出时没有棋盘实例，调用方用本方法把牌面 `card_id` 交给执行器。
    pub fn fire_card(
        &mut self,
        state: &mut GameState,
        card_id: &str,
        trigger: Trigger,
        ctx: EffectCtx,
    ) -> Vec<GameEvent> {
        self.run_card_effects(state, card_id, trigger, &ctx)
    }

    /// 执行一段效果；`chosen` 可用于覆盖 `target` 选择器的显式目标。
    pub fn apply_op(
        &mut self,
        state: &mut GameState,
        effect: &Effect,
        ctx: &EffectCtx,
        chosen: Option<InstanceId>,
    ) -> Vec<GameEvent> {
        let mut effective = *ctx;
        if let Some(id) = chosen
            && (matches!(effect.selector, Selector::Target)
                || matches!(
                    effect.selector,
                    Selector::Side {
                        pick: Pick::Single,
                        ..
                    }
                ))
        {
            effective.target = Some(id);
        }

        if let Some(condition) = &effect.params.condition
            && !self.condition_holds(state, condition, &effective)
        {
            return Vec::new();
        }

        let mut targets = self.resolve_selector_mut(state, &effect.selector, &effective);

        if let Some(when) = &effect.params.when
            && (effect.op == Op::DealDamage || matches!(effect.selector, Selector::Target))
        {
            targets.retain(|id| self.filter_matches(state, when, *id, &effective));
        }

        match effect.op {
            Op::Growth => {
                self.apply_growth(state, effect, &targets);
                Vec::new()
            }
            Op::ModifyStats => {
                self.apply_modify_stats(state, effect, &targets);
                Vec::new()
            }
            Op::GrantKeyword => {
                self.apply_grant_keyword(state, effect, &targets);
                Vec::new()
            }
            Op::RemoveGrowth => {
                for id in targets {
                    if let Some((_, minion)) = locate_mut(state, id) {
                        minion.growth = 0;
                        minion.growth_health_only = 0;
                    }
                }
                Vec::new()
            }
            Op::CannotAttack => {
                // 卡面「本场无法攻击」是唯一写法；省略 duration 时按 `Combat` 处理，
                // 否则 `Instant` 标记将永远没有清除时点。
                let duration = match effect.params.duration {
                    Duration::Instant => Duration::Combat,
                    other => other,
                };
                for id in targets {
                    if let Some((_, minion)) = locate_mut(state, id) {
                        minion.cannot_attack = true;
                        record_grant(minion, CANNOT_ATTACK_KEYWORD, duration, false);
                    }
                }
                Vec::new()
            }
            Op::DealDamage => self.apply_deal_damage(state, effect, &targets),
            // Task 12–14 实现。
            Op::Summon => Vec::new(),
            Op::Copy => Vec::new(),
            Op::Discover => Vec::new(),
            Op::RefreshShop => Vec::new(),
            Op::AddShopSlot => Vec::new(),
            Op::GainMinion => Vec::new(),
        }
    }

    /// 解析选择器（只读）；`random` 在克隆的随机流上取样，不推进真实状态。
    ///
    /// 执行路径使用内部的 [`Engine::resolve_selector_mut`]，它会在每次随机选取时
    /// 恰好推进一次 [`Stream::Effects`]，因此同一状态下查询与执行选出的目标一致，
    /// 且执行序列的随机消费量与效果段数一一对应，确定性保持不变。
    pub fn resolve_selector(
        &self,
        state: &GameState,
        selector: &Selector,
        ctx: &EffectCtx,
    ) -> Vec<InstanceId> {
        match selector {
            Selector::SelfCard => ctx
                .source
                .filter(|id| board_contains(state, ctx.seat, *id))
                .map(|id| vec![id])
                .unwrap_or_default(),
            Selector::Target => ctx
                .target
                .or(ctx.event_target)
                .map(|id| vec![id])
                .unwrap_or_default(),
            Selector::Side {
                side,
                pick,
                filters,
            } => {
                let pool = self.filtered_side(state, *side, filters, ctx);
                let roll = if *pick == Pick::Random && !pool.is_empty() {
                    let mut rng = state.rng.clone();
                    Some(rng.below(Stream::Effects, pool.len() as u64) as usize)
                } else {
                    None
                };
                self.apply_pick(state, pool, *pick, ctx, roll)
            }
        }
    }

    /// Task 11 的光环重算入口；本任务只丢弃 0/0 的临时修正。
    pub fn recompute_stats(&mut self, _state: &mut GameState, minion: &mut MinionInstance) {
        minion
            .mods
            .retain(|modifier| modifier.attack != 0 || modifier.health != 0);
    }

    /// 在可变状态上解析选择器并推进随机流。
    fn resolve_selector_mut(
        &self,
        state: &mut GameState,
        selector: &Selector,
        ctx: &EffectCtx,
    ) -> Vec<InstanceId> {
        match selector {
            Selector::SelfCard | Selector::Target => self.resolve_selector(state, selector, ctx),
            Selector::Side {
                side,
                pick,
                filters,
            } => {
                let pool = self.filtered_side(state, *side, filters, ctx);
                let roll = if *pick == Pick::Random && !pool.is_empty() {
                    Some(state.rng.below(Stream::Effects, pool.len() as u64) as usize)
                } else {
                    None
                };
                self.apply_pick(state, pool, *pick, ctx, roll)
            }
        }
    }

    fn owner_card_id(
        &self,
        state: &GameState,
        trigger: Trigger,
        ctx: &EffectCtx,
    ) -> Option<String> {
        match trigger {
            Trigger::OnPlay | Trigger::OnPlayEffect => {
                ctx.source.and_then(|id| board_card_id(state, ctx.seat, id))
            }
            Trigger::OnActivate => match ctx.source {
                Some(id) => board_card_id(state, ctx.seat, id),
                None => state
                    .players
                    .get(usize::from(ctx.seat))
                    .and_then(|player| player.hero.clone()),
            },
            _ => None,
        }
    }

    /// 收集一张卡牌在指定触发时机下的效果，按技能顺序与文本顺序执行。
    fn run_card_effects(
        &mut self,
        state: &mut GameState,
        card_id: &str,
        trigger: Trigger,
        ctx: &EffectCtx,
    ) -> Vec<GameEvent> {
        let Some(card) = self.catalog.cards.get(card_id) else {
            return Vec::new();
        };
        let mut effects = Vec::new();
        for skill_id in &card.skills {
            if let Some(skill) = self.catalog.skills.get(skill_id)
                && let Ok(parsed) = parse_effects(&skill.effects)
            {
                effects.extend(
                    parsed
                        .into_iter()
                        .filter(|effect| effect.trigger == trigger),
                );
            }
        }

        let passes = if ctx.golden { 2 } else { 1 };
        let mut events = Vec::new();
        for _ in 0..passes {
            for effect in &effects {
                events.extend(self.apply_op(state, effect, ctx, None));
            }
        }
        events
    }

    /// `OnUse`：本座位所有带响应效果的随从，按站位顺序依次结算。
    fn fire_on_use(&mut self, state: &mut GameState, ctx: &EffectCtx) -> Vec<GameEvent> {
        let Some(player) = state.players.get(usize::from(ctx.seat)) else {
            return Vec::new();
        };
        let owners: Vec<(InstanceId, String, bool)> = player
            .board
            .iter()
            .map(|minion| (minion.instance_id, minion.card_id.clone(), minion.golden))
            .collect();

        let mut events = Vec::new();
        for (instance_id, card_id, golden) in owners {
            if !self.card_has_trigger(&card_id, Trigger::OnUse) {
                continue;
            }
            let mut owner_ctx = *ctx;
            owner_ctx.source = Some(instance_id);
            // §2.6：金色随从的响应效果同样连续触发两次。
            owner_ctx.golden = golden;
            events.extend(self.run_card_effects(state, &card_id, Trigger::OnUse, &owner_ctx));
        }
        events
    }

    fn card_has_trigger(&self, card_id: &str, trigger: Trigger) -> bool {
        let Some(card) = self.catalog.cards.get(card_id) else {
            return false;
        };
        card.skills.iter().any(|skill_id| {
            self.catalog
                .skills
                .get(skill_id)
                .and_then(|skill| parse_effects(&skill.effects).ok())
                .is_some_and(|effects| effects.iter().any(|effect| effect.trigger == trigger))
        })
    }

    /// `condition` 是否成立：统计条件阵营、按条件筛选后的随从数。
    fn condition_holds(
        &self,
        state: &GameState,
        condition: &crate::effects::Condition,
        ctx: &EffectCtx,
    ) -> bool {
        let pool: Vec<InstanceId> = self
            .side_ids(state, condition.side, ctx)
            .into_iter()
            .filter(|id| self.filter_matches(state, &condition.filter, *id, ctx))
            .collect();
        let count = i32::try_from(pool.len()).unwrap_or(i32::MAX);
        match condition.compare {
            crate::effects::Compare::Ge => count >= condition.count,
            crate::effects::Compare::Le => count <= condition.count,
            crate::effects::Compare::Eq => count == condition.count,
            crate::effects::Compare::Gt => count > condition.count,
            crate::effects::Compare::Lt => count < condition.count,
        }
    }

    /// 阵营池：己方为当前座位棋盘；敌方为其余存活座位；招募阶段敌方为空。
    /// 阵营池，按座位升序合并；`Both` 时己方座位处于其自然顺序位置。
    fn side_ids(&self, state: &GameState, side: Side, ctx: &EffectCtx) -> Vec<InstanceId> {
        let include_enemy =
            matches!(side, Side::Enemy | Side::Both) && state.phase != Phase::Recruit;
        let mut ids = Vec::new();

        let mut players: Vec<&PlayerState> = state.players.iter().collect();
        players.sort_by_key(|player| player.seat);
        for player in players {
            let friendly = player.seat == ctx.seat;
            let enemy = !friendly && !player.eliminated && player.hp > 0;
            let selected = match side {
                Side::Friendly => friendly,
                Side::Enemy => include_enemy && enemy,
                Side::Both => friendly || (include_enemy && enemy),
            };
            if selected {
                ids.extend(player.board.iter().map(|minion| minion.instance_id));
            }
        }
        ids
    }

    fn filtered_side(
        &self,
        state: &GameState,
        side: Side,
        filters: &[Filter],
        ctx: &EffectCtx,
    ) -> Vec<InstanceId> {
        self.side_ids(state, side, ctx)
            .into_iter()
            .filter(|id| {
                filters
                    .iter()
                    .all(|filter| self.filter_matches(state, filter, *id, ctx))
            })
            .collect()
    }

    fn filter_matches(
        &self,
        state: &GameState,
        filter: &Filter,
        id: InstanceId,
        ctx: &EffectCtx,
    ) -> bool {
        let Some((_, minion)) = locate(state, id) else {
            return false;
        };
        let Some(card) = self.catalog.cards.get(&minion.card_id) else {
            return false;
        };
        match filter.key.as_str() {
            "tribe" => tribe_name(card.tribe) == filter.value,
            "subtype" => card.subtype.as_deref() == Some(filter.value.as_str()),
            "name" => card.name == filter.value,
            "type" => match filter.value.as_str() {
                "minion" | "随从" => card.card_type == CardType::Minion,
                "effect" | "效果" => card.card_type == CardType::Effect,
                _ => false,
            },
            "scholar" => match filter.value.as_str() {
                "true" => card.is_scholar,
                "false" => !card.is_scholar,
                _ => false,
            },
            "keyword" => combat::keywords(&self.catalog, minion).contains(&filter.value),
            "other" => {
                if filter.value == "true" {
                    ctx.source != Some(id)
                } else {
                    true
                }
            }
            // 加载期已拒绝未知筛选键；条件里的未知键按「匹配不到任何结果」处理。
            _ => false,
        }
    }

    fn apply_pick(
        &self,
        state: &GameState,
        pool: Vec<InstanceId>,
        pick: Pick,
        ctx: &EffectCtx,
        roll: Option<usize>,
    ) -> Vec<InstanceId> {
        match pick {
            Pick::All => pool,
            Pick::Single => ctx
                .target
                .or(ctx.event_target)
                .filter(|id| pool.contains(id))
                .map(|id| vec![id])
                .unwrap_or_default(),
            Pick::Random => roll
                .and_then(|index| pool.get(index).copied())
                .map(|id| vec![id])
                .unwrap_or_default(),
            Pick::Leftmost => pool.first().copied().map(|id| vec![id]).unwrap_or_default(),
            Pick::LeftmostAdjacent => pool.iter().take(2).copied().collect(),
            Pick::TopAttack => {
                let mut best: Option<(InstanceId, i32)> = None;
                for id in pool {
                    let Some((_, minion)) = locate(state, id) else {
                        continue;
                    };
                    let attack = combat::effective_attack(&self.catalog, minion);
                    if best.is_none_or(|(_, current)| attack > current) {
                        best = Some((id, attack));
                    }
                }
                best.map(|(id, _)| vec![id]).unwrap_or_default()
            }
        }
    }

    fn apply_growth(&self, state: &mut GameState, effect: &Effect, targets: &[InstanceId]) {
        let base_times = effect.params.times.unwrap_or(1);
        let health_only = effect.params.health_only == Some(true);

        for id in targets {
            let Some((_, minion)) = locate_mut(state, *id) else {
                continue;
            };
            let times = match (&effect.params.times_if, self.card_tribe(&minion.card_id)) {
                (Some((tribe, value)), Some(target_tribe))
                    if tribe_name(target_tribe) == *tribe =>
                {
                    *value
                }
                _ => base_times,
            };
            let scholar = self
                .card_tribe(&minion.card_id)
                .is_some_and(|tribe| tribe == Tribe::Scholar);
            if health_only || scholar {
                minion.growth_health_only = minion.growth_health_only.saturating_add(times);
            } else {
                minion.growth = minion.growth.saturating_add(times);
            }
        }
    }

    fn apply_modify_stats(&self, state: &mut GameState, effect: &Effect, targets: &[InstanceId]) {
        let modifier = Modifier {
            attack: effect.params.attack.unwrap_or(0),
            health: effect.params.health.unwrap_or(0),
            duration: effect.params.duration,
        };
        for id in targets {
            if let Some((_, minion)) = locate_mut(state, *id) {
                minion.mods.push(modifier.clone());
            }
        }
    }

    fn apply_grant_keyword(&self, state: &mut GameState, effect: &Effect, targets: &[InstanceId]) {
        let Some(KeywordSpec::Keyword(name, layer)) = &effect.params.keyword else {
            return;
        };
        let duration = effect.params.duration;

        for id in targets {
            let Some((_, minion)) = locate_mut(state, *id) else {
                continue;
            };
            match (name.as_str(), layer) {
                ("繁衍", Some(count)) => {
                    minion.breed = minion.breed.saturating_add(*count);
                }
                ("传染", Some(count)) => {
                    minion.contagion = minion.contagion.saturating_add(*count);
                }
                ("护盾", _) => {
                    record_grant(minion, name, duration, false);
                    minion.shield_used = false;
                }
                _ => record_grant(minion, name, duration, false),
            }
        }
    }

    fn apply_deal_damage(
        &self,
        state: &mut GameState,
        effect: &Effect,
        targets: &[InstanceId],
    ) -> Vec<GameEvent> {
        let amount = effect.params.amount.unwrap_or(0);
        let cannot_kill = effect.params.can_kill == Some(false);
        let mut events = Vec::new();

        for id in targets {
            let Some((seat, minion)) = locate_mut(state, *id) else {
                continue;
            };
            let health = if cannot_kill {
                let current = combat::effective_health(&self.catalog, minion);
                let ceiling = (current - 1).max(0);
                -amount.min(ceiling)
            } else {
                -amount
            };
            minion.mods.push(Modifier {
                attack: 0,
                health,
                duration: Duration::Combat,
            });
            // 事件记录实际损失的生命值；负向修改可能被 `canKill=false` 夹紧。
            events.push(GameEvent::DamageTaken {
                seat,
                amount: -health,
            });
        }
        events
    }

    fn card_tribe(&self, card_id: &str) -> Option<Tribe> {
        self.catalog.cards.get(card_id).map(|card| card.tribe)
    }
}

/// 统计每个「触发 × 动作」组合被多少段效果使用；总和应等于数据中的效果段数。
pub fn effect_coverage(catalog: &Catalog) -> BTreeMap<(Trigger, Op), usize> {
    let mut coverage = BTreeMap::new();
    for skill in catalog.skills.values() {
        if let Ok(effects) = parse_effects(&skill.effects) {
            for effect in effects {
                *coverage.entry((effect.trigger, effect.op)).or_insert(0) += 1;
            }
        }
    }
    coverage
}

fn locate(state: &GameState, id: InstanceId) -> Option<(Seat, &MinionInstance)> {
    for player in &state.players {
        if let Some(minion) = player.board.iter().find(|minion| minion.instance_id == id) {
            return Some((player.seat, minion));
        }
    }
    None
}

fn locate_mut(state: &mut GameState, id: InstanceId) -> Option<(Seat, &mut MinionInstance)> {
    for index in 0..state.players.len() {
        if let Some(position) = state.players[index]
            .board
            .iter()
            .position(|minion| minion.instance_id == id)
        {
            let seat = state.players[index].seat;
            return Some((seat, &mut state.players[index].board[position]));
        }
    }
    None
}

fn board_contains(state: &GameState, seat: Seat, id: InstanceId) -> bool {
    state
        .players
        .get(usize::from(seat))
        .is_some_and(|player| player.board.iter().any(|minion| minion.instance_id == id))
}

fn board_card_id(state: &GameState, seat: Seat, id: InstanceId) -> Option<String> {
    state
        .players
        .get(usize::from(seat))
        .and_then(|player| player.board.iter().find(|minion| minion.instance_id == id))
        .map(|minion| minion.card_id.clone())
}

fn tribe_name(tribe: Tribe) -> &'static str {
    match tribe {
        Tribe::Human => "人类",
        Tribe::Machine => "机械",
        Tribe::Biology => "生物",
        Tribe::Plant => "植物",
        Tribe::Virus => "病毒",
        Tribe::Scholar => "学者",
        Tribe::General => "通用",
    }
}

fn duration_rank(duration: Duration) -> u8 {
    match duration {
        Duration::Instant => 0,
        Duration::Combat => 1,
        Duration::Permanent => 2,
    }
}

/// 记录授予关键词，同名只保留时长更长的版本；`Instant` 默认不落盘。
fn record_grant(minion: &mut MinionInstance, name: &str, duration: Duration, keep_instant: bool) {
    if duration == Duration::Instant && !keep_instant {
        return;
    }
    if let Some(existing) = minion
        .granted
        .iter_mut()
        .find(|(existing, _)| existing == name)
    {
        if duration_rank(duration) > duration_rank(existing.1) {
            existing.1 = duration;
        }
        return;
    }
    minion.granted.push((name.to_string(), duration));
}
