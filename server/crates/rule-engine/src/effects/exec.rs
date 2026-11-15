//! 效果执行器：把解析完成的效果 AST 落成对局状态变化。
//!
//! 本模块实现招募阶段的六个即时动作（成长、属性修改、赋予关键词、移除成长、
//! 禁止攻击、造成伤害）、商店类原语（刷新商店、增加槽位、获得随从）与召唤、
//! 复制、发现，以及 `OnPlay` / `OnPlayEffect` / `OnActivate` / `OnUse` /
//! `OnTurnStart` / `OnFirstRefresh` 六个触发时机。光环由 Task 11 实现；
//! 发现候选取自 `triple.rs`，进化见 `evolve.rs`；战斗内触发由 Task 13 实现。
//!
//! 规则口径见 `docs/versions/v0.1/product/rules/keywords-and-mechanics.md`
//! §2、§3、§8、§9、§10。所有随机性只来自 [`Stream::Effects`]，集合与遍历顺序
//! 全部确定。

use std::collections::BTreeMap;

use crate::combat::{self, BattleField, CombatSetup};
use crate::data::{CardType, Catalog, Tribe};
use crate::effects::{
    Duration, Effect, Filter, KeywordSpec, Op, Pick, Selector, Side, Trigger, Zone, parse_effects,
};
use crate::engine::Engine;
use crate::events::GameEvent;
use crate::intent::{Reject, RejectCode};
use crate::recruit::new_minion;
use crate::rng::{RngStreams, Stream};
use crate::state::{
    GameState, HandCard, InstanceId, MinionInstance, Modifier, Phase, PlayerState, Seat,
};

/// 禁止攻击时写入 `granted` 的占位关键词，用于记录该标记的时长来源。
const CANNOT_ATTACK_KEYWORD: &str = "禁攻";
/// 播种关键词；死亡结算时在亡语之后单独召唤幼苗。
const SOW_KEYWORD: &str = "播种";

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
            Trigger::OnPlay
            | Trigger::OnPlayEffect
            | Trigger::OnActivate
            | Trigger::OnTurnStart
            | Trigger::OnFirstRefresh => match self.owner_card_id(state, trigger, &ctx) {
                Some(card_id) => self.run_card_effects(state, &card_id, trigger, &ctx),
                None => Vec::new(),
            },
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
                self.fire_ally_growth_recruit(state, ctx.seat, &targets);
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
            Op::RefreshShop => self.apply_refresh_shop(state, effect, ctx),
            Op::AddShopSlot => self.apply_add_shop_slot(state, effect, ctx),
            Op::GainMinion => self.apply_gain_minion(state, effect, ctx),
            Op::Summon => {
                self.summon_recruit_from_effect(state, effect, &targets, ctx);
                Vec::new()
            }
            Op::Copy => {
                self.apply_recruit_copy(state, effect, &targets, ctx);
                Vec::new()
            }
            Op::Discover => self.apply_discover(state, effect, ctx),
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
            Trigger::OnActivate | Trigger::OnTurnStart | Trigger::OnFirstRefresh => {
                match ctx.source {
                    Some(id) => board_card_id(state, ctx.seat, id),
                    None => state
                        .players
                        .get(usize::from(ctx.seat))
                        .and_then(|player| player.hero.clone()),
                }
            }
            _ => None,
        }
    }

    /// 收集一张卡牌在指定触发时机下的效果，按技能顺序与文本顺序执行。
    /// 收集一张卡在指定触发时机下的全部效果。
    ///
    /// 与执行路径的过滤口径一致：只取该时机、且招募阶段的 `OnPlay` 不含敌方选择器
    /// （这类战吼延迟到战斗开始，§2.7.4）。
    pub(crate) fn effects_of(&self, card_id: &str, trigger: Trigger) -> Vec<Effect> {
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
                        .filter(|effect| effect.trigger == trigger)
                        .filter(|effect| {
                            !(trigger == Trigger::OnPlay && is_enemy_selector(&effect.selector))
                        }),
                );
            }
        }
        effects
    }

    pub(crate) fn run_card_effects(
        &mut self,
        state: &mut GameState,
        card_id: &str,
        trigger: Trigger,
        ctx: &EffectCtx,
    ) -> Vec<GameEvent> {
        let effects = self.effects_of(card_id, trigger);

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

    /// 意图执行前的只读目标预检。
    ///
    /// 口径（core-rules §4.4/§4.5）：效果需要单一显式目标时，若存在合法目标而
    /// `target` 缺失或不在池中，返回 `NoValidTarget`。池为空时的处理由
    /// `reject_on_empty_pool` 决定：主动技能按卡牌文档视为不可用（拒绝），
    /// 战吼与效果牌允许提交、效果落空。
    ///
    /// 池只含意图提交前已存在的随从：即将入场的随从客户端还看不到实例编号，
    /// 把它算进池会让「棋盘为空时买入带目标战吼的随从」变成无法补选的死锁。
    ///
    /// 只读：不修改状态、不消耗随机流，调用方可在任何状态修改之前失败返回。
    pub(crate) fn check_targets(
        &self,
        state: &GameState,
        effects: &[Effect],
        ctx: &EffectCtx,
        reject_on_empty_pool: bool,
    ) -> Result<(), Reject> {
        for effect in effects {
            if let Some(condition) = &effect.params.condition
                && !self.condition_holds(state, condition, ctx)
            {
                continue;
            }
            match &effect.selector {
                Selector::Target => {
                    let chosen = ctx.target.or(ctx.event_target);
                    if chosen.is_none_or(|id| locate(state, id).is_none()) {
                        return Err(reject_no_valid_target());
                    }
                }
                Selector::Side {
                    side,
                    pick: Pick::Single,
                    filters,
                } => {
                    let pool = self.filtered_side(state, *side, filters, ctx);
                    if pool.is_empty() {
                        if reject_on_empty_pool {
                            return Err(reject_no_valid_target());
                        }
                        continue;
                    }
                    let chosen = ctx.target.or(ctx.event_target);
                    if !chosen.is_some_and(|id| pool.contains(&id)) {
                        return Err(reject_no_valid_target());
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// 打出效果牌时，本座位所有 `OnUse` 响应效果的目标预检。
    pub(crate) fn check_on_use_targets(
        &self,
        state: &GameState,
        ctx: &EffectCtx,
    ) -> Result<(), Reject> {
        let Some(player) = state.players.get(usize::from(ctx.seat)) else {
            return Ok(());
        };
        let owners: Vec<(InstanceId, String, bool)> = player
            .board
            .iter()
            .map(|minion| (minion.instance_id, minion.card_id.clone(), minion.golden))
            .collect();
        for (instance_id, card_id, golden) in owners {
            if !self.card_has_trigger(&card_id, Trigger::OnUse) {
                continue;
            }
            let mut owner_ctx = *ctx;
            owner_ctx.source = Some(instance_id);
            owner_ctx.golden = golden;
            self.check_targets(
                state,
                &self.effects_of(&card_id, Trigger::OnUse),
                &owner_ctx,
                false,
            )?;
        }
        Ok(())
    }

    pub(crate) fn card_has_trigger(&self, card_id: &str, trigger: Trigger) -> bool {
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

    pub(crate) fn filter_matches(
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

    // -----------------------------------------------------------------------
    // 招募侧召唤与复制
    // -----------------------------------------------------------------------

    /// 执行 `Summon`：token 直接创建，card 从池中扣 1（池空创建副本）。
    fn summon_recruit_from_effect(
        &mut self,
        state: &mut GameState,
        effect: &Effect,
        _targets: &[InstanceId],
        ctx: &EffectCtx,
    ) {
        let count = effect.params.count.unwrap_or(1).max(0);
        let adjacent = matches!(
            effect.selector,
            Selector::Side {
                pick: Pick::LeftmostAdjacent,
                ..
            }
        );
        let source_position = ctx.source.and_then(|id| {
            state
                .players
                .get(usize::from(ctx.seat))
                .and_then(|player| player.board.iter().position(|m| m.instance_id == id))
        });

        for offset in 0..count {
            let Some((card_id, charge_pool)) = self.recruit_summon_card(effect) else {
                break;
            };
            let position = adjacent.then(|| source_position.unwrap_or(0) + 1 + offset as usize);
            if self
                .summon_recruit(state, ctx.seat, &card_id, position, charge_pool)
                .is_none()
            {
                break;
            }
        }
    }

    /// 解析召唤来源：token 优先，其次卡牌；返回 `(card_id, 是否扣池)`。
    fn recruit_summon_card(&self, effect: &Effect) -> Option<(String, bool)> {
        if let Some(token_id) = &effect.params.token
            && self.catalog.tokens.contains_key(token_id)
        {
            return Some((token_id.clone(), false));
        }
        if let Some(card_id) = &effect.params.card
            && self.catalog.cards.contains_key(card_id)
        {
            return Some((card_id.clone(), true));
        }
        None
    }

    /// 友方随从获得成长后，触发其他随从的 `OnAllyGrowth`。
    fn fire_ally_growth_recruit(
        &mut self,
        state: &mut GameState,
        seat: Seat,
        grown: &[InstanceId],
    ) {
        if grown.is_empty() {
            return;
        }
        let Some(player) = state.players.get(usize::from(seat)) else {
            return;
        };
        let owners: Vec<(InstanceId, String, bool)> = player
            .board
            .iter()
            .map(|minion| (minion.instance_id, minion.card_id.clone(), minion.golden))
            .collect();
        for target in grown {
            for (source, card_id, golden) in &owners {
                if !self.card_has_trigger(card_id, Trigger::OnAllyGrowth) {
                    continue;
                }
                let ctx = EffectCtx {
                    seat,
                    source: Some(*source),
                    target: None,
                    event_target: Some(*target),
                    golden: *golden,
                };
                let _ = self.run_card_effects(state, card_id, Trigger::OnAllyGrowth, &ctx);
            }
        }
    }

    /// 在招募侧棋盘召唤一个随从；棋盘满时返回 `None`。
    ///
    /// `position` 为 `None` 时追加到最右侧；`charge_pool` 为真且池中有份数时扣 1。
    pub(crate) fn summon_recruit(
        &mut self,
        state: &mut GameState,
        seat: Seat,
        card_id: &str,
        position: Option<usize>,
        charge_pool: bool,
    ) -> Option<InstanceId> {
        let index = usize::from(seat);
        let slots = self.catalog.config.board_slots;
        if state.players.get(index)?.board.len() >= slots {
            return None;
        }
        if charge_pool && let Some(count) = state.pool.get_mut(card_id) {
            *count = count.saturating_sub(1);
        }
        let id = state.next_instance_id;
        state.next_instance_id = id.saturating_add(1);
        let board = &mut state.players[index].board;
        let position = position.unwrap_or(board.len()).min(board.len());
        board.insert(position, new_minion(id, card_id, false));
        self.fire_ally_summon_recruit(state, seat, id);
        Some(id)
    }

    /// 召唤后触发同一棋盘上其他随从的 `OnAllySummon`。
    pub(crate) fn fire_ally_summon_recruit(
        &mut self,
        state: &mut GameState,
        seat: Seat,
        summoned: InstanceId,
    ) {
        let Some(player) = state.players.get(usize::from(seat)) else {
            return;
        };
        let owners: Vec<(InstanceId, String, bool)> = player
            .board
            .iter()
            .filter(|minion| minion.instance_id != summoned)
            .map(|minion| (minion.instance_id, minion.card_id.clone(), minion.golden))
            .collect();
        for (source, card_id, golden) in owners {
            if !self.card_has_trigger(&card_id, Trigger::OnAllySummon) {
                continue;
            }
            let ctx = EffectCtx {
                seat,
                source: Some(source),
                target: None,
                event_target: Some(summoned),
                golden,
            };
            let _ = self.run_card_effects(state, &card_id, Trigger::OnAllySummon, &ctx);
        }
    }

    /// 执行 `Copy`：复制随从进手牌或战场，不扣牌池。
    fn apply_recruit_copy(
        &mut self,
        state: &mut GameState,
        effect: &Effect,
        targets: &[InstanceId],
        ctx: &EffectCtx,
    ) {
        let Some(player) = state.players.get(usize::from(ctx.seat)) else {
            return;
        };
        let hand_slots = self.catalog.config.hand_slots;
        let board_slots = self.catalog.config.board_slots;
        let to = effect.params.to.unwrap_or(Zone::Hand);

        let mut copies: Vec<(String, bool)> = Vec::new();
        if effect.params.from == Some(Zone::Hand) {
            if let Some(card) = player.hand.iter().find(|card| {
                self.catalog
                    .cards
                    .get(&card.card_id)
                    .is_some_and(|card| card.card_type == CardType::Minion)
            }) {
                copies.push((card.card_id.clone(), card.golden));
            }
        } else {
            for id in targets {
                if let Some(minion) = player.board.iter().find(|m| m.instance_id == *id) {
                    copies.push((minion.card_id.clone(), minion.golden));
                }
            }
        }

        for (card_id, golden) in copies {
            let player = &mut state.players[usize::from(ctx.seat)];
            match to {
                Zone::Hand => {
                    if player.hand.len() < hand_slots {
                        player.hand.push(HandCard { card_id, golden });
                    }
                }
                Zone::Board => {
                    if player.board.len() < board_slots {
                        let id = state.next_instance_id;
                        state.next_instance_id = id.saturating_add(1);
                        player.board.push(new_minion(id, &card_id, golden));
                    }
                }
                Zone::Shop => {}
            }
        }
    }

    /// 回合开始传染致死：结算亡语（不含播种技能本身）。
    pub(crate) fn fire_death_recruit(
        &mut self,
        state: &mut GameState,
        card_id: &str,
        ctx: &EffectCtx,
    ) {
        let effects = self.recruit_effects_filtered(card_id, Trigger::OnDeath, Some(SOW_KEYWORD));
        let passes = if ctx.golden { 2 } else { 1 };
        for _ in 0..passes {
            for effect in &effects {
                let _ = self.apply_op(state, effect, ctx, None);
            }
        }
    }

    fn recruit_effects_filtered(
        &self,
        card_id: &str,
        trigger: Trigger,
        skip_keyword: Option<&str>,
    ) -> Vec<Effect> {
        let Some(card) = self.catalog.cards.get(card_id) else {
            return Vec::new();
        };
        let mut effects = Vec::new();
        for skill_id in &card.skills {
            let Some(skill) = self.catalog.skills.get(skill_id) else {
                continue;
            };
            if let Some(skip) = skip_keyword
                && skill.keyword.as_deref() == Some(skip)
            {
                continue;
            }
            if let Ok(parsed) = parse_effects(&skill.effects) {
                effects.extend(
                    parsed
                        .into_iter()
                        .filter(|effect| effect.trigger == trigger),
                );
            }
        }
        effects
    }

    // -----------------------------------------------------------------------
    // 战斗侧效果
    // -----------------------------------------------------------------------

    /// 战斗开始：英雄被动 → 先手方随从 → 后手方随从；每条效果后判定死亡。
    pub(crate) fn fire_combat_start(
        &self,
        field: &mut BattleField,
        rng: &mut RngStreams,
        setup: &CombatSetup<'_>,
    ) {
        let first = field.first_side;
        // §2.7.1：英雄被动按先手方 → 后手方结算。
        for side in [first, 1 - first] {
            let hero = if side == 0 {
                setup.attacker_hero
            } else {
                setup.defender_hero
            };
            if let Some(hero) = hero {
                let effects = self.combat_effects(hero, Trigger::OnCombatStart);
                let ctx = EffectCtx {
                    seat: field.seat(side),
                    source: None,
                    target: None,
                    event_target: None,
                    golden: false,
                };
                self.fire_combat_effects(field, rng, &effects, ctx);
            }
        }

        for side in [first, 1 - first] {
            let ids = field.alive_ids(side);
            for id in ids {
                if !field.is_alive(id) {
                    continue;
                }
                self.fire_combat_single(field, rng, id, Trigger::OnCombatStart, None);
                if field.is_alive(id) {
                    self.fire_delayed_battlecry(field, rng, id);
                }
            }
        }
    }

    /// 对单个棋盘随从触发指定时机。
    pub(crate) fn fire_combat_single(
        &self,
        field: &mut BattleField,
        rng: &mut RngStreams,
        id: InstanceId,
        trigger: Trigger,
        event_target: Option<InstanceId>,
    ) {
        let Some(minion) = field.minion(id).cloned() else {
            return;
        };
        let Some((side, _)) = field.locate(id) else {
            return;
        };
        let effects = self.combat_effects(&minion.card_id, trigger);
        let ctx = EffectCtx {
            seat: field.seat(side),
            source: Some(id),
            target: None,
            event_target,
            golden: minion.golden,
        };
        self.fire_combat_effects(field, rng, &effects, ctx);
    }

    /// 延迟战吼：招募阶段因目标在敌方而未结算的 `OnPlay` 效果。
    pub(crate) fn fire_delayed_battlecry(
        &self,
        field: &mut BattleField,
        rng: &mut RngStreams,
        id: InstanceId,
    ) {
        let Some(minion) = field.minion(id).cloned() else {
            return;
        };
        let Some((side, _)) = field.locate(id) else {
            return;
        };
        let effects: Vec<Effect> = self
            .combat_effects(&minion.card_id, Trigger::OnPlay)
            .into_iter()
            .filter(|effect| is_enemy_selector(&effect.selector))
            .collect();
        let ctx = EffectCtx {
            seat: field.seat(side),
            source: Some(id),
            target: None,
            event_target: None,
            golden: minion.golden,
        };
        self.fire_combat_effects(field, rng, &effects, ctx);
    }

    /// 死亡结算中的亡语；播种技能本身由死亡结算单独处理。
    pub(crate) fn fire_combat_death(
        &self,
        field: &mut BattleField,
        rng: &mut RngStreams,
        card_id: &str,
        ctx: EffectCtx,
    ) {
        let effects = self.combat_effects_filtered(card_id, Trigger::OnDeath, Some(SOW_KEYWORD));
        self.fire_combat_effects(field, rng, &effects, ctx);
    }

    /// 召唤后触发召唤者棋盘的 `OnAllySummon`。
    pub(crate) fn fire_ally_summon(
        &self,
        field: &mut BattleField,
        rng: &mut RngStreams,
        side: usize,
        summoned: InstanceId,
    ) {
        self.fire_combat_board_trigger(
            field,
            rng,
            Trigger::OnAllySummon,
            side,
            Some(summoned),
            Some(summoned),
        );
    }

    /// 对一整侧存活随从（可排除一个）触发指定时机，按站位从左到右。
    pub(crate) fn fire_combat_board_trigger(
        &self,
        field: &mut BattleField,
        rng: &mut RngStreams,
        trigger: Trigger,
        side: usize,
        exclude: Option<InstanceId>,
        event_target: Option<InstanceId>,
    ) {
        for id in field.alive_ids(side) {
            if Some(id) == exclude {
                continue;
            }
            let Some(minion) = field.minion(id).cloned() else {
                continue;
            };
            if !self.card_has_trigger(&minion.card_id, trigger) {
                continue;
            }
            let effects = self.combat_effects(&minion.card_id, trigger);
            let ctx = EffectCtx {
                seat: field.seat(side),
                source: Some(id),
                target: None,
                event_target,
                golden: minion.golden,
            };
            self.fire_combat_effects(field, rng, &effects, ctx);
        }
    }

    /// 执行一组战斗内效果；金色来源连续执行两遍，每条效果后判定死亡。
    fn fire_combat_effects(
        &self,
        field: &mut BattleField,
        rng: &mut RngStreams,
        effects: &[Effect],
        ctx: EffectCtx,
    ) {
        if effects.is_empty() {
            return;
        }
        let passes = if ctx.golden { 2 } else { 1 };
        for _ in 0..passes {
            if !field.take_trigger() {
                return;
            }
            for effect in effects {
                self.apply_combat_effect(field, rng, effect, &ctx);
                field.resolve_deaths(Some(self), &self.catalog, rng);
            }
        }
    }

    fn apply_combat_effect(
        &self,
        field: &mut BattleField,
        rng: &mut RngStreams,
        effect: &Effect,
        ctx: &EffectCtx,
    ) {
        if let Some(condition) = &effect.params.condition
            && !self.combat_condition_holds(field, condition, ctx)
        {
            return;
        }

        let mut targets = self.combat_targets(field, &effect.selector, ctx, rng);
        if let Some(when) = &effect.params.when
            && (effect.op == Op::DealDamage || matches!(effect.selector, Selector::Target))
        {
            targets.retain(|id| self.combat_filter_matches(field, when, *id, ctx));
        }

        match effect.op {
            Op::Growth => {
                let health_only = effect.params.health_only == Some(true);
                let base_times = effect.params.times.unwrap_or(1);
                for id in targets {
                    let Some(minion) = field.minion(id) else {
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
                    field.add_growth(id, times, health_only || scholar);
                    if let Some((side, _)) = field.locate(id) {
                        self.fire_combat_board_trigger(
                            field,
                            rng,
                            Trigger::OnAllyGrowth,
                            side,
                            Some(id),
                            Some(id),
                        );
                    }
                }
            }
            Op::ModifyStats => {
                let modifier = Modifier {
                    attack: effect.params.attack.unwrap_or(0),
                    health: effect.params.health.unwrap_or(0),
                    duration: effect.params.duration,
                };
                for id in targets {
                    field.modify_stats(id, modifier.clone());
                }
            }
            Op::GrantKeyword => {
                let Some(KeywordSpec::Keyword(name, layer)) = &effect.params.keyword else {
                    return;
                };
                let duration = effect.params.duration;
                for id in targets {
                    let Some(minion) = field.minion_mut(id) else {
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
            Op::RemoveGrowth => {
                for id in targets {
                    field.remove_growth(id);
                }
            }
            Op::CannotAttack => {
                for id in targets {
                    field.cannot_attack(id);
                }
            }
            Op::DealDamage => {
                let amount = effect.params.amount.unwrap_or(0);
                let can_kill = effect.params.can_kill != Some(false);
                for id in targets {
                    field.deal_damage(id, amount, can_kill);
                }
            }
            Op::Summon => self.apply_combat_summon(field, rng, effect, ctx),
            Op::Copy => self.apply_combat_copy(field, effect, ctx, &targets),
            _ => {}
        }
    }

    fn apply_combat_summon(
        &self,
        field: &mut BattleField,
        rng: &mut RngStreams,
        effect: &Effect,
        ctx: &EffectCtx,
    ) {
        let Some(side) = field.side_of(ctx.seat) else {
            return;
        };
        let count = effect.params.count.unwrap_or(1).max(0);
        let adjacent = matches!(
            effect.selector,
            Selector::Side {
                pick: Pick::LeftmostAdjacent,
                ..
            }
        );
        let source_position = ctx
            .source
            .and_then(|id| field.locate(id))
            .and_then(|(source_side, position)| (source_side == side).then_some(position));

        for offset in 0..count {
            let Some(mut instance) = self.combat_summon_instance(field, effect) else {
                break;
            };
            instance.instance_id = field.new_instance_id();
            let summoned = match (adjacent, source_position) {
                (true, Some(position)) => field.insert_at(
                    &self.catalog,
                    side,
                    position + 1 + offset as usize,
                    instance,
                ),
                _ => field.append(&self.catalog, side, instance),
            };
            field.summon_count = field.summon_count.saturating_add(1);
            self.fire_ally_summon(field, rng, side, summoned);
        }
    }

    /// 构造召唤实例；缺失 token/card 时返回 `None`，card 会记录扣池。
    fn combat_summon_instance(
        &self,
        field: &mut BattleField,
        effect: &Effect,
    ) -> Option<MinionInstance> {
        if let Some(token_id) = &effect.params.token {
            let token = self.catalog.tokens.get(token_id)?;
            return Some(new_minion(0, &token.id, false));
        }
        if let Some(card_id) = &effect.params.card {
            self.catalog.cards.get(card_id)?;
            field.record_pool_spend(card_id);
            return Some(new_minion(0, card_id, false));
        }
        None
    }

    fn apply_combat_copy(
        &self,
        field: &mut BattleField,
        effect: &Effect,
        ctx: &EffectCtx,
        targets: &[InstanceId],
    ) {
        let to = effect.params.to.unwrap_or(Zone::Hand);
        // 战斗内无法读取持久手牌；`from=hand` 在此阶段按无目标处理。
        if effect.params.from == Some(Zone::Hand) {
            return;
        }
        match to {
            Zone::Hand => {
                for id in targets {
                    let Some(minion) = field.minion(*id) else {
                        continue;
                    };
                    let card_id = minion.card_id.clone();
                    let golden = minion.golden;
                    field.record_hand_copy(ctx.seat, &card_id, golden);
                }
            }
            Zone::Board => {
                let Some(side) = field.side_of(ctx.seat) else {
                    return;
                };
                for id in targets {
                    let Some(mut copy) = field.minion(*id).cloned() else {
                        continue;
                    };
                    copy.instance_id = field.new_instance_id();
                    field.append(&self.catalog, side, copy);
                }
            }
            Zone::Shop => {}
        }
    }

    fn combat_effects(&self, card_id: &str, trigger: Trigger) -> Vec<Effect> {
        self.combat_effects_filtered(card_id, trigger, None)
    }

    fn combat_effects_filtered(
        &self,
        card_id: &str,
        trigger: Trigger,
        skip_keyword: Option<&str>,
    ) -> Vec<Effect> {
        let Some(card) = self.catalog.cards.get(card_id) else {
            return Vec::new();
        };
        let mut effects = Vec::new();
        for skill_id in &card.skills {
            let Some(skill) = self.catalog.skills.get(skill_id) else {
                continue;
            };
            if let Some(skip) = skip_keyword
                && skill.keyword.as_deref() == Some(skip)
            {
                continue;
            }
            if let Ok(parsed) = parse_effects(&skill.effects) {
                effects.extend(
                    parsed
                        .into_iter()
                        .filter(|effect| effect.trigger == trigger),
                );
            }
        }
        effects
    }

    /// 战斗内选择器：友方 = 效果归属座位，敌方 = 对位座位，`both` 先友后敌。
    fn combat_targets(
        &self,
        field: &BattleField,
        selector: &Selector,
        ctx: &EffectCtx,
        rng: &mut RngStreams,
    ) -> Vec<InstanceId> {
        match selector {
            Selector::SelfCard => ctx
                .source
                // 亡语期间来源已被标记为死亡，但「自身」仍应能定位到它。
                .filter(|id| field.minion(*id).is_some())
                .map(|id| vec![id])
                .unwrap_or_default(),
            Selector::Target => ctx
                .target
                .or(ctx.event_target)
                .filter(|id| field.is_alive(*id))
                .map(|id| vec![id])
                .unwrap_or_default(),
            Selector::Side {
                side,
                pick,
                filters,
            } => {
                let Some(friendly) = field.side_of(ctx.seat) else {
                    return Vec::new();
                };
                let mut pool = match side {
                    Side::Friendly => field.alive_ids(friendly),
                    Side::Enemy => field.alive_ids(1 - friendly),
                    Side::Both => {
                        let mut ids = field.alive_ids(friendly);
                        ids.extend(field.alive_ids(1 - friendly));
                        ids
                    }
                };
                pool.retain(|id| {
                    filters
                        .iter()
                        .all(|filter| self.combat_filter_matches(field, filter, *id, ctx))
                });
                self.combat_apply_pick(field, pool, *pick, ctx, rng)
            }
        }
    }

    fn combat_apply_pick(
        &self,
        field: &BattleField,
        pool: Vec<InstanceId>,
        pick: Pick,
        ctx: &EffectCtx,
        rng: &mut RngStreams,
    ) -> Vec<InstanceId> {
        match pick {
            Pick::All => pool,
            Pick::Single => ctx
                .target
                .or(ctx.event_target)
                .filter(|id| pool.contains(id))
                .map(|id| vec![id])
                .unwrap_or_default(),
            Pick::Random => {
                if pool.is_empty() {
                    return Vec::new();
                }
                let offset = rng.below(Stream::Combat, pool.len() as u64) as usize;
                pool.get(offset)
                    .copied()
                    .map(|id| vec![id])
                    .unwrap_or_default()
            }
            Pick::Leftmost => pool.first().copied().map(|id| vec![id]).unwrap_or_default(),
            Pick::LeftmostAdjacent => pool.iter().take(2).copied().collect(),
            Pick::TopAttack => {
                let mut best: Option<(InstanceId, i32)> = None;
                for id in pool {
                    let Some(minion) = field.minion(id) else {
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

    fn combat_filter_matches(
        &self,
        field: &BattleField,
        filter: &Filter,
        id: InstanceId,
        ctx: &EffectCtx,
    ) -> bool {
        let Some(minion) = field.minion(id) else {
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
            _ => false,
        }
    }

    fn combat_condition_holds(
        &self,
        field: &BattleField,
        condition: &crate::effects::Condition,
        ctx: &EffectCtx,
    ) -> bool {
        let Some(friendly) = field.side_of(ctx.seat) else {
            return false;
        };
        let mut pool = match condition.side {
            Side::Friendly => field.alive_ids(friendly),
            Side::Enemy => field.alive_ids(1 - friendly),
            Side::Both => {
                let mut ids = field.alive_ids(friendly);
                ids.extend(field.alive_ids(1 - friendly));
                ids
            }
        };
        pool.retain(|id| self.combat_filter_matches(field, &condition.filter, *id, ctx));
        let count = i32::try_from(pool.len()).unwrap_or(i32::MAX);
        match condition.compare {
            crate::effects::Compare::Ge => count >= condition.count,
            crate::effects::Compare::Le => count <= condition.count,
            crate::effects::Compare::Eq => count == condition.count,
            crate::effects::Compare::Gt => count > condition.count,
            crate::effects::Compare::Lt => count < condition.count,
        }
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

fn reject_no_valid_target() -> Reject {
    Reject::new(RejectCode::NoValidTarget, "该效果需要一个合法目标")
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

/// 选择器是否指向敌方；用于识别延迟到战斗开始的战吼。
fn is_enemy_selector(selector: &Selector) -> bool {
    matches!(
        selector,
        Selector::Side {
            side: Side::Enemy,
            ..
        }
    )
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
