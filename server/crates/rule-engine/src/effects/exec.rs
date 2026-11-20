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

use std::collections::{BTreeMap, BTreeSet};

use crate::combat::{self, BattleField, CombatAction, CombatSetup, DamageSource};
use crate::data::{
    CardType, Catalog, Category, CategoryKind, Playstyle, card_in_pool, has_play, play_categories,
};
use crate::effects::{
    CopyState, Duration, Effect, Filter, Op, Pick, Selector, Side, TargetRef, TimesIf, Trigger,
    Zone,
};
use crate::engine::Engine;
use crate::events::GameEvent;
use crate::intent::{Reject, RejectCode};
use crate::recruit::new_minion;
use crate::rng::{RngStreams, Stream};
use crate::state::{
    GameState, HandCard, InstanceId, MinionInstance, Modifier, Phase, PlayerState, Seat, base_stats,
};
use crate::triple::REWARD_CARD_ID;

/// 禁止攻击时写入 `granted` 的占位关键词，用于记录该标记的时长来源。
const CANNOT_ATTACK_KEYWORD: &str = "cannot_attack";
/// 播种关键词；死亡结算时在亡语之后单独召唤幼苗。
const SOW_KEYWORD: &str = "sow";

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
    /// 本次发现锁定的星级（「三连奖励」）；`None` 时发现按玩家时代取星级。
    pub level_override: Option<u8>,
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
            | Trigger::OnFirstRefresh => {
                let effects = self.owner_effects(state, trigger, &ctx);
                self.run_effects(state, &effects, &ctx)
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

    /// 直接执行单个技能在某个触发时机下的效果。
    ///
    /// 英雄可以有多个主动技能；`fire` 按归属收集英雄全部技能，主动技能必须只结算
    /// 被选中的那一个，否则多技能英雄会一次放出全部技能。
    pub(crate) fn fire_skill(
        &mut self,
        state: &mut GameState,
        skill_id: &str,
        trigger: Trigger,
        ctx: EffectCtx,
    ) -> Vec<GameEvent> {
        let effects = self.skill_effects_of(skill_id, trigger);
        self.run_effects(state, &effects, &ctx)
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
            && (matches!(effect.selector, Selector::Target { .. })
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
            && (effect.op == Op::DealDamage || matches!(effect.selector, Selector::Target { .. }))
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
                        minion.stats.attack.remove_growth();
                        minion.stats.health.remove_growth();
                    }
                }
                Vec::new()
            }
            Op::CannotAttack => {
                // 卡面「本场无法攻击」是唯一写法；省略 duration 时按 `Combat` 处理，
                // 否则 `Instant` 标记将永远没有清除时点。
                let duration = match effect.params.duration {
                    Some(Duration::Instant) | None => Duration::Combat,
                    Some(other) => other,
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
            Op::GrantMinion => self.apply_grant_minion(state, effect, ctx),
            Op::Summon => {
                self.summon_recruit_from_effect(state, effect, &targets, ctx);
                Vec::new()
            }
            Op::Copy => self.apply_recruit_copy(state, effect, &targets, ctx),
            Op::Discover => self.apply_discover(state, effect, ctx),
            // 分解只发生在战斗内；招募阶段没有事件目标，落空。
            Op::Decompose => Vec::new(),
            Op::Mimic => {
                self.apply_mimic(state, effect, &targets, ctx);
                Vec::new()
            }
            Op::Devour => {
                self.apply_devour(state, effect, &targets, ctx);
                Vec::new()
            }
            // 新玩法的原语在招募阶段尚无数据使用；结算由各玩法机制任务接入。
            Op::Detonate | Op::Convert | Op::Revive | Op::Recall | Op::Salvage => Vec::new(),
            Op::Latent => self.apply_latent(state, effect, ctx),
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
            Selector::Target { .. } => ctx
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

    /// 在可变状态上解析选择器并推进随机流。
    fn resolve_selector_mut(
        &self,
        state: &mut GameState,
        selector: &Selector,
        ctx: &EffectCtx,
    ) -> Vec<InstanceId> {
        match selector {
            Selector::SelfCard | Selector::Target { .. } => {
                self.resolve_selector(state, selector, ctx)
            }
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

    /// 本次触发所归属的卡牌效果：棋盘实例走随从池，无实例的主动/回合触发走英雄池。
    fn owner_effects(&self, state: &GameState, trigger: Trigger, ctx: &EffectCtx) -> Vec<Effect> {
        match trigger {
            Trigger::OnPlay | Trigger::OnPlayEffect => ctx
                .source
                .and_then(|id| board_card_id(state, ctx.seat, id))
                .map(|card_id| self.effects_of(&card_id, trigger))
                .unwrap_or_default(),
            Trigger::OnActivate | Trigger::OnTurnStart | Trigger::OnFirstRefresh => {
                match ctx.source {
                    Some(id) => board_card_id(state, ctx.seat, id)
                        .map(|card_id| self.effects_of(&card_id, trigger))
                        .unwrap_or_default(),
                    None => state
                        .players
                        .get(usize::from(ctx.seat))
                        .and_then(|player| player.hero.as_deref())
                        .map(|hero_id| self.hero_effects_of(hero_id, trigger))
                        .unwrap_or_default(),
                }
            }
            _ => Vec::new(),
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
        self.skill_effects(&card.skills, trigger)
    }

    /// 收集一张英雄卡在指定触发时机下的效果；英雄查询只走英雄池。
    pub(crate) fn hero_effects_of(&self, hero_id: &str, trigger: Trigger) -> Vec<Effect> {
        let Some(hero) = self.catalog.heroes.get(hero_id) else {
            return Vec::new();
        };
        self.skill_effects(&hero.skills, trigger)
    }

    /// 收集单个技能在指定触发时机下的效果。
    ///
    /// 与 `effects_of` 的过滤口径一致：只取该时机、且招募阶段的 `OnPlay` 不含敌方选择器。
    pub(crate) fn skill_effects_of(&self, skill_id: &str, trigger: Trigger) -> Vec<Effect> {
        let Some(skill) = self.catalog.skills.get(skill_id) else {
            return Vec::new();
        };
        skill
            .effects
            .iter()
            .filter(|effect| effect.trigger == trigger)
            .filter(|effect| !(trigger == Trigger::OnPlay && is_enemy_selector(&effect.selector)))
            .cloned()
            .collect()
    }

    fn skill_effects(&self, skills: &[String], trigger: Trigger) -> Vec<Effect> {
        skills
            .iter()
            .flat_map(|skill_id| self.skill_effects_of(skill_id, trigger))
            .collect()
    }

    pub(crate) fn run_card_effects(
        &mut self,
        state: &mut GameState,
        card_id: &str,
        trigger: Trigger,
        ctx: &EffectCtx,
    ) -> Vec<GameEvent> {
        let effects = self.effects_of(card_id, trigger);
        self.run_effects(state, &effects, ctx)
    }

    /// 依次执行一段已收集的效果；金色随从连续执行两遍。
    fn run_effects(
        &mut self,
        state: &mut GameState,
        effects: &[Effect],
        ctx: &EffectCtx,
    ) -> Vec<GameEvent> {
        let passes = if ctx.golden { 2 } else { 1 };
        let mut events = Vec::new();
        for _ in 0..passes {
            for effect in effects {
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
    /// 该卡在该触发时机下是否存在「需要玩家指定单一目标」的效果。
    ///
    /// 只看选择器：`Selector::Target`，或 `Selector::Side { pick: Pick::Single, .. }`。
    /// **不考虑 `condition`**：条件依赖运行时状态，本判定要能在没有座位上下文时回答，
    /// 因此是保守超集——视图只用它做交互提示，真正的池判定在 `check_targets`。
    pub fn requires_target(&self, card_id: &str, trigger: Trigger) -> bool {
        Self::effects_require_target(&self.effects_of(card_id, trigger))
    }

    /// 英雄卡是否在指定触发时机下存在需要玩家指定单一目标的效果。
    ///
    /// 与 [`Self::requires_target`] 的判定口径一致，但技能只走英雄池；同名随从
    /// 卡的效果不得影响英雄技能的瞄准与可用性。
    pub fn hero_requires_target(&self, hero_id: &str, trigger: Trigger) -> bool {
        Self::effects_require_target(&self.hero_effects_of(hero_id, trigger))
    }

    /// 单个技能是否在指定触发时机下存在需要玩家指定单一目标的效果。
    ///
    /// 判定口径与 [`Self::requires_target`] 一致，只作用于给定的技能。
    pub(crate) fn skill_requires_target(&self, skill_id: &str, trigger: Trigger) -> bool {
        Self::effects_require_target(&self.skill_effects_of(skill_id, trigger))
    }

    fn effects_require_target(effects: &[Effect]) -> bool {
        effects.iter().any(|effect| {
            matches!(
                effect.selector,
                Selector::Target { .. }
                    | Selector::Side {
                        pick: Pick::Single,
                        ..
                    }
            )
        })
    }

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
                Selector::Target { zone, filters } => {
                    let pool = self.target_pool(state, ctx, *zone, filters);
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

    /// 当前状态下该组效果的合法目标候选，按「我方棋盘 → 手牌 → 商店」排列。
    ///
    /// 先用「未指定目标」探一次 [`Engine::check_targets`]：返回 `Ok` 说明当前状态
    /// 不需要目标，候选为空。否则逐个尝试本人在三个区域中的实例编号，同时满足
    /// 效果预检（以及 `include_on_use` 时的响应预检）的才保留。
    ///
    /// 与拒绝校验共用同一套 `check_targets`，避免视图与结算漂移；只读，不推进随机流。
    pub fn target_candidates(
        &self,
        state: &GameState,
        effects: &[Effect],
        ctx: &EffectCtx,
        include_on_use: bool,
    ) -> Vec<TargetRef> {
        let probe = EffectCtx {
            target: None,
            ..*ctx
        };
        if self.check_targets(state, effects, &probe, false).is_ok() {
            return Vec::new();
        }

        let mut candidates = Vec::new();
        for target in own_target_refs(state, ctx.seat) {
            let chosen = EffectCtx {
                target: Some(target.id),
                ..*ctx
            };
            if self.check_targets(state, effects, &chosen, false).is_err() {
                continue;
            }
            if include_on_use && self.check_on_use_targets(state, &chosen).is_err() {
                continue;
            }
            candidates.push(target);
        }
        candidates
    }

    /// 区域对应当前座位可见的实例池。
    fn target_pool(
        &self,
        state: &GameState,
        ctx: &EffectCtx,
        zone: Zone,
        filters: &[Filter],
    ) -> Vec<InstanceId> {
        let Some(player) = state.players.get(usize::from(ctx.seat)) else {
            return Vec::new();
        };
        match zone {
            // 战斗内的事件目标可能指向任意座位；招募期的提交边界另行限制为本人。
            Zone::Board => state
                .players
                .iter()
                .flat_map(|player| player.board.iter())
                .filter(|minion| {
                    filters
                        .iter()
                        .all(|filter| self.filter_matches(state, filter, minion.instance_id, ctx))
                })
                .map(|minion| minion.instance_id)
                .collect(),
            Zone::Hand => player
                .hand
                .iter()
                .filter(|card| {
                    filters.iter().all(|filter| {
                        self.filter_matches_card(
                            filter,
                            card.card_id(),
                            card.instance_id(),
                            card.minion(),
                            ctx,
                        )
                    })
                })
                .map(|card| card.instance_id())
                .collect(),
            Zone::Shop => player
                .shop
                .iter()
                .flatten()
                .filter(|slot| {
                    filters.iter().all(|filter| {
                        self.filter_matches_card(filter, &slot.card_id, slot.instance_id, None, ctx)
                    })
                })
                .map(|slot| slot.instance_id)
                .collect(),
        }
    }

    pub(crate) fn card_has_trigger(&self, card_id: &str, trigger: Trigger) -> bool {
        let Some(card) = self.catalog.cards.get(card_id) else {
            return false;
        };
        card.skills.iter().any(|skill_id| {
            self.catalog
                .skills
                .get(skill_id)
                .is_some_and(|skill| skill.effects.iter().any(|effect| effect.trigger == trigger))
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
        self.filter_matches_card(filter, &minion.card_id, id, Some(minion), ctx)
    }

    /// 按卡牌事实判定筛选；`minion` 为手牌随从实例，商店槽恒为 `None`。
    ///
    /// 关键词筛选需要随从实例：棋盘与手牌随从按当前关键词判定，商店槽与效果牌
    /// 没有实例关键词，恒不匹配。
    fn filter_matches_card(
        &self,
        filter: &Filter,
        card_id: &str,
        id: InstanceId,
        minion: Option<&MinionInstance>,
        ctx: &EffectCtx,
    ) -> bool {
        let Some(facts) = card_facts(&self.catalog, card_id) else {
            return false;
        };
        match filter.key.as_str() {
            "keyword" => minion.is_some_and(|minion| {
                combat::keywords(&self.catalog, minion).contains(&filter.value)
            }),
            "other" => {
                if filter.value == "true" {
                    ctx.source != Some(id)
                } else {
                    true
                }
            }
            // 加载期已拒绝未知筛选键；条件里的未知键按「匹配不到任何结果」处理。
            key => facts.matches(&self.catalog, key, &filter.value),
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
            Pick::LowestAttack => {
                let mut best: Option<(InstanceId, i32)> = None;
                for id in pool {
                    let Some((_, minion)) = locate(state, id) else {
                        continue;
                    };
                    let attack = combat::effective_attack(&self.catalog, minion);
                    if best.is_none_or(|(_, current)| attack < current) {
                        best = Some((id, attack));
                    }
                }
                best.map(|(id, _)| vec![id]).unwrap_or_default()
            }
            // 相邻只服务光环，招募路径不会用到；机制任务接入光环实现。
            Pick::Adjacent => Vec::new(),
        }
    }

    fn apply_growth(&self, state: &mut GameState, effect: &Effect, targets: &[InstanceId]) {
        let base_times = effect.params.times.unwrap_or(1);
        let health_only = effect.params.health_only == Some(true);

        for id in targets {
            let Some((_, minion)) = locate_mut(state, *id) else {
                continue;
            };
            let times = match &effect.params.times_if {
                Some(times_if) if self.times_if_matches(&minion.card_id, times_if) => {
                    times_if.times
                }
                _ => base_times,
            };
            if health_only || minion.is_scholar {
                minion.stats.health.add_growth(times);
            } else {
                minion.stats.attack.add_growth(times);
                minion.stats.health.add_growth(times);
            }
        }
    }

    fn apply_modify_stats(&self, state: &mut GameState, effect: &Effect, targets: &[InstanceId]) {
        let attack = effect.params.attack.unwrap_or(0);
        let health = effect.params.health.unwrap_or(0);
        let turn_scoped = effect.params.duration == Some(Duration::Combat);
        for id in targets {
            if let Some((_, minion)) = locate_mut(state, *id) {
                if turn_scoped {
                    minion.stats.attack.add_turn(attack);
                    minion.stats.health.add_turn(health);
                } else {
                    // `Instant` 在数据中未使用；保持现有「永不清除」语义。
                    minion.stats.attack.add_permanent(attack);
                    minion.stats.health.add_permanent(health);
                }
            }
        }
    }

    fn apply_grant_keyword(&self, state: &mut GameState, effect: &Effect, targets: &[InstanceId]) {
        let Some(keyword) = &effect.params.keyword else {
            return;
        };
        let name = keyword.keyword.as_str();
        let layer = keyword.value.or(effect.params.value);
        let duration = effect.params.duration.unwrap_or(Duration::Instant);

        for id in targets {
            let Some((_, minion)) = locate_mut(state, *id) else {
                continue;
            };
            match (name, &layer) {
                ("breed", Some(count)) => {
                    minion.breed = minion.breed.saturating_add(*count);
                }
                ("contagion", Some(count)) => {
                    minion.contagion = minion.contagion.saturating_add(*count);
                }
                // 减伤层数保存在关键词字符串 `armor N` 中，供 `combat::armor_value` 解析；
                // 只记录裸 `armor` 会丢失 N。同层与不同层的授予都累加到同一条
                // `armor N` 上，`armor_value` 再与卡面自带层相加。
                (combat::ARMOR, Some(count)) => {
                    if duration == Duration::Instant {
                        // 与 `record_grant` 的旧口径一致：即时授予不进入 `granted`。
                        continue;
                    }
                    match minion
                        .granted
                        .iter_mut()
                        .find(|(name, _)| combat::armor_layer(name).is_some())
                    {
                        Some(existing) => {
                            let merged = combat::armor_layer(&existing.0)
                                .unwrap_or(0)
                                .saturating_add(*count);
                            existing.0 = format!("{} {merged}", combat::ARMOR);
                            if duration_rank(duration) > duration_rank(existing.1) {
                                existing.1 = duration;
                            }
                        }
                        None => minion
                            .granted
                            .push((format!("{} {count}", combat::ARMOR), duration)),
                    }
                }
                ("divine_shield", _) => {
                    record_grant(minion, name, duration, false);
                    minion.shield_used = false;
                }
                _ => record_grant(minion, name, duration, false),
            }
        }
    }

    /// 拟态：把目标当前攻防（以及非 `statsOnly` 时的关键词）写到来源上。
    ///
    /// 目标缺失或已离场时静默落空；目标可为敌方或友方，由选择器决定。
    /// 攻防按 `ModifyStats` 的时长口径写入；关键词写入 `granted`，不复制
    /// 成长、繁衍、传染与护盾/复生等一次性状态。金色连续触发时逐次覆盖
    /// （同值反复写入不叠加）。
    fn apply_mimic(
        &self,
        state: &mut GameState,
        effect: &Effect,
        targets: &[InstanceId],
        ctx: &EffectCtx,
    ) {
        let Some(source_id) = ctx.source else {
            return;
        };
        if locate(state, source_id).is_none() {
            return;
        }
        let duration = effect.params.duration.unwrap_or(Duration::Combat);
        let stats_only = effect.params.stats_only.unwrap_or(false);
        let copied: Vec<(i32, i32, Vec<String>)> = targets
            .iter()
            .filter_map(|id| {
                let (_, target) = locate(state, *id)?;
                if target.health() <= 0 {
                    return None;
                }
                let keywords = if stats_only {
                    Vec::new()
                } else {
                    combat::keywords_without_aura(&self.catalog, target)
                        .into_iter()
                        .collect::<Vec<_>>()
                };
                Some((target.attack(), target.health(), keywords))
            })
            .collect();

        let Some((_, source)) = locate_mut(state, source_id) else {
            return;
        };
        for (target_attack, target_health, keywords) in copied {
            // 复制当前值：写入「目标当前值 - 来源当前值」的增量，金色第二次
            // 触发时增量为 0，因此以最后一次结果为准，不会叠加。
            let attack = target_attack.saturating_sub(source.attack());
            let health = target_health.saturating_sub(source.health());
            match duration {
                Duration::Combat => {
                    source.stats.attack.add_turn(attack);
                    source.stats.health.add_turn(health);
                }
                Duration::Permanent | Duration::Instant | Duration::Aura => {
                    source.stats.attack.add_permanent(attack);
                    source.stats.health.add_permanent(health);
                }
            }
            for keyword in keywords {
                record_grant(source, &keyword, duration, false);
            }
        }
    }

    /// 吞噬：对攻击力不超过 `value` 的敌方目标施加致死伤害，并把其卡面
    /// 基础攻防永久加到来源上。
    ///
    /// 目标缺失、不在敌方、仍在招募阶段或攻击力超限时静默落空。伤害直接
    /// 写入当前生命，不经过护盾判定，因此护盾不能阻止；获得值取卡面基础值，
    /// 不含成长与光环。
    fn apply_devour(
        &self,
        state: &mut GameState,
        effect: &Effect,
        targets: &[InstanceId],
        ctx: &EffectCtx,
    ) {
        let Some(source_id) = ctx.source else {
            return;
        };
        if locate(state, source_id).is_none() {
            return;
        }
        let limit = effect.params.value.unwrap_or(0);
        for id in targets {
            if !self.is_enemy_target(state, ctx.seat, *id) {
                continue;
            }
            let Some((base_attack, base_health)) = locate(state, *id).and_then(|(_, target)| {
                if target.attack() > limit {
                    return None;
                }
                base_stats(&self.catalog, &target.card_id, target.golden)
            }) else {
                continue;
            };
            let Some((_, target)) = locate_mut(state, *id) else {
                continue;
            };
            // 致死伤害：直接把当前生命打到 0；不经过护盾判定，护盾不能阻止。
            target
                .stats
                .health
                .apply_damage(target.stats.health.current.max(0));
            let Some((_, source)) = locate_mut(state, source_id) else {
                continue;
            };
            source.stats.attack.add_permanent(base_attack);
            source.stats.health.add_permanent(base_health);
        }
    }

    /// 目标是否属于本效果来源的敌方：座位不同、玩家仍存活，且已离开招募阶段。
    fn is_enemy_target(&self, state: &GameState, seat: Seat, id: InstanceId) -> bool {
        if state.phase == Phase::Recruit {
            return false;
        }
        locate(state, id).is_some_and(|(target_seat, _)| {
            target_seat != seat
                && state
                    .players
                    .get(usize::from(target_seat))
                    .is_some_and(|player| !player.eliminated && player.hp > 0)
        })
    }

    /// 潜伏：按传染层数永久增加攻击力，达到阈值时爆发一次。
    ///
    /// `OnTurnStart` 在 `Recruit` 阶段执行，此时没有配对；「敌方」按除来源外
    /// 所有存活玩家的棋盘结算（`side_ids` 在本阶段敌方池为空，不要用它推导
    /// 范围）。爆发伤害按招募阶段的通用伤害规则处理护盾与减伤，击杀的随从
    /// 立即离场并结算亡语。爆发每局最多一次（`latent_used`）；金色按通用规则
    /// 触发两遍，但第一遍爆发会清零层数，第二遍因此只加 0 点攻击且不会再次爆发。
    fn apply_latent(
        &mut self,
        state: &mut GameState,
        effect: &Effect,
        ctx: &EffectCtx,
    ) -> Vec<GameEvent> {
        let Some(source_id) = ctx.source else {
            return Vec::new();
        };
        let Some(player) = state.players.get_mut(usize::from(ctx.seat)) else {
            return Vec::new();
        };
        let Some(source) = player
            .board
            .iter_mut()
            .find(|minion| minion.instance_id == source_id)
        else {
            return Vec::new();
        };

        let layers = source.contagion.max(0);
        let amount = effect.params.amount.unwrap_or(0).max(0);
        source
            .stats
            .attack
            .add_permanent(layers.saturating_mul(amount));

        // `count = 0` 时没有爆发可结算：不清层、不置标记，避免白白消耗
        // 「每局一次」的爆发机会。
        let count = effect.params.count.unwrap_or(0).max(0);
        let burst = count > 0
            && !source.latent_used
            && effect
                .params
                .threshold
                .is_some_and(|threshold| layers >= threshold);
        if !burst {
            return Vec::new();
        }
        source.contagion = 0;
        source.latent_used = true;

        let mut events = Vec::new();
        let mut hit_seats: Vec<Seat> = Vec::new();
        for target in &mut state.players {
            if target.seat == ctx.seat || target.eliminated || target.hp <= 0 {
                continue;
            }
            let mut hit = false;
            for minion in &mut target.board {
                if combat::effective_health(&self.catalog, minion) <= 0 {
                    continue;
                }
                let dealt = self.deal_recruit_damage(minion, count);
                if dealt > 0 {
                    events.push(GameEvent::DamageTaken {
                        seat: target.seat,
                        amount: dealt,
                    });
                    hit = true;
                }
            }
            if hit {
                hit_seats.push(target.seat);
            }
        }
        for seat in hit_seats {
            self.resolve_recruit_deaths(state, seat);
        }
        events
    }

    /// 招募阶段对单个随从造成伤害：先消耗圣盾，再扣减伤，剩余伤害写入永久层。
    ///
    /// 返回实际造成的伤害；圣盾抵消或减伤归零时为 0。生命值 ≤ 0 的随从由
    /// [`Engine::resolve_recruit_deaths`] 统一移除，这里不重复判定。
    fn deal_recruit_damage(&self, minion: &mut MinionInstance, amount: i32) -> i32 {
        let amount = amount.max(0);
        if amount == 0 {
            return 0;
        }
        if combat::keywords(&self.catalog, minion).contains("divine_shield") && !minion.shield_used
        {
            minion.shield_used = true;
            return 0;
        }
        let dealt = amount
            .saturating_sub(combat::armor_value(&self.catalog, minion))
            .max(0);
        if dealt == 0 {
            return 0;
        }
        minion.stats.health.add_permanent(-dealt);
        dealt
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
            minion.stats.health.add_turn(health);
            // 事件记录实际损失的生命值；负向修改可能被 `canKill=false` 夹紧。
            events.push(GameEvent::DamageTaken {
                seat,
                amount: -health,
            });
        }
        events
    }

    /// `timesIf` 是否命中该卡牌。`scholar` 只看学者标记，其余按筛选键匹配。
    fn times_if_matches(&self, card_id: &str, times_if: &TimesIf) -> bool {
        let Some(facts) = card_facts(&self.catalog, card_id) else {
            return false;
        };
        if times_if.key == "scholar" {
            return facts.is_scholar;
        }
        let Some(value) = times_if.value.as_deref() else {
            return false;
        };
        facts.matches(&self.catalog, &times_if.key, value)
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

    /// 解析召唤来源：`token` 只命中非池卡（衍生物），`card` 只命中池内卡；
    /// 返回 `(card_id, 是否扣池)`。
    fn recruit_summon_card(&self, effect: &Effect) -> Option<(String, bool)> {
        if let Some(token_id) = &effect.params.token
            && self
                .catalog
                .cards
                .get(token_id)
                .is_some_and(|card| !card.in_shop_pool)
        {
            return Some((token_id.clone(), false));
        }
        if let Some(card_id) = &effect.params.card
            && self
                .catalog
                .cards
                .get(card_id)
                .is_some_and(|card| card.in_shop_pool)
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
                    level_override: None,
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
        board.insert(position, new_minion(&self.catalog, id, card_id, false));
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
                level_override: None,
            };
            let _ = self.run_card_effects(state, &card_id, Trigger::OnAllySummon, &ctx);
        }
    }

    /// 执行 `Copy`：复制随从进手牌或战场，不扣牌池。
    ///
    /// `state` 决定复制口径：`base` 只按卡面构造（不继承任何状态），
    /// `current` 为完全复制，继承成长、繁衍、传染、护盾、已授予关键词与金色标记。
    /// 金色随从复制的仍是金色，卡面基础值同样 ×2。
    /// 复制品落地后与购买、发现、获得一致立即做三连判定（核心规则 §6.1、§6.10）。
    fn apply_recruit_copy(
        &mut self,
        state: &mut GameState,
        effect: &Effect,
        targets: &[InstanceId],
        ctx: &EffectCtx,
    ) -> Vec<GameEvent> {
        let Some(player) = state.players.get(usize::from(ctx.seat)) else {
            return Vec::new();
        };
        let hand_slots = self.catalog.config.hand_slots;
        let board_slots = self.catalog.config.board_slots;
        let to = effect.params.to.unwrap_or(Zone::Hand);
        let full = effect.params.state.unwrap_or(CopyState::Base) == CopyState::Current;

        let mut copies: Vec<MinionInstance> = Vec::new();
        if matches!(
            effect.selector,
            Selector::Target {
                zone: Zone::Hand,
                ..
            }
        ) {
            // 手牌目标：只复制 `targets` 指向的本人手牌随从；目标缺失或已离手则落空。
            for id in targets {
                if let Some(minion) = player.hand.iter().find_map(|card| match card {
                    HandCard::Minion(minion) if minion.instance_id == *id => Some(minion),
                    _ => None,
                }) {
                    copies.push(self.copy_of_catalog(minion, full));
                }
            }
        } else {
            for id in targets {
                if let Some(minion) = player.board.iter().find(|m| m.instance_id == *id) {
                    copies.push(self.copy_of_catalog(minion, full));
                }
            }
        }

        for mut copy in copies {
            let player = &mut state.players[usize::from(ctx.seat)];
            match to {
                Zone::Hand => {
                    if player.hand.len() < hand_slots {
                        let id = state.next_instance_id;
                        state.next_instance_id = id.saturating_add(1);
                        copy.instance_id = id;
                        copy.sources.clear();
                        player.hand.push(HandCard::Minion(copy));
                    }
                }
                Zone::Board => {
                    if player.board.len() < board_slots {
                        let id = state.next_instance_id;
                        state.next_instance_id = id.saturating_add(1);
                        copy.instance_id = id;
                        copy.sources.clear();
                        player.board.push(copy);
                    }
                }
                Zone::Shop => {}
            }
        }
        self.check_triple(state, ctx.seat)
    }

    /// 按复制口径生成新实例：`full` 时整体克隆来源，否则按卡面重建。
    ///
    /// 两种口径都给副本分配新的实例编号并清空来源记录；战场临时属性层
    /// （`turn`、`aura`）不随副本离开战场。
    fn copy_of_catalog(&self, source: &MinionInstance, full: bool) -> MinionInstance {
        let mut copy = if full {
            source.clone()
        } else {
            new_minion(
                &self.catalog,
                source.instance_id,
                &source.card_id,
                source.golden,
            )
        };
        copy.stats.attack.clear_turn();
        copy.stats.health.clear_turn();
        copy.stats.attack.set_aura(0);
        copy.stats.health.set_aura(0);
        copy.attacks_left = 0;
        copy.cannot_attack = false;
        copy.sources.clear();
        // 副本是新的入场，延迟战吼不继承来源的结算标记。
        copy.delayed_battlecry_used = false;
        copy
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
            effects.extend(
                skill
                    .effects
                    .iter()
                    .filter(|effect| effect.trigger == trigger)
                    .cloned(),
            );
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
                let effects = self.combat_hero_effects(hero, Trigger::OnCombatStart);
                let ctx = EffectCtx {
                    seat: field.seat(side),
                    source: None,
                    target: None,
                    event_target: None,
                    golden: false,
                    level_override: None,
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
            level_override: None,
        };
        self.fire_combat_effects(field, rng, &effects, ctx);
    }

    /// 延迟战吼：招募阶段因目标在敌方而未结算的 `OnPlay` 效果。
    ///
    /// 每个实例只结算一次：首次尝试后即置 `delayed_battlecry_used`，后续战斗
    /// 开始时不再重放。置位发生在执行前，避免效果内的再入触发重复结算。
    pub(crate) fn fire_delayed_battlecry(
        &self,
        field: &mut BattleField,
        rng: &mut RngStreams,
        id: InstanceId,
    ) {
        let Some(minion) = field.minion(id).cloned() else {
            return;
        };
        if minion.delayed_battlecry_used {
            return;
        }
        let Some((side, _)) = field.locate(id) else {
            return;
        };
        let effects: Vec<Effect> = self
            .combat_effects(&minion.card_id, Trigger::OnPlay)
            .into_iter()
            .filter(|effect| is_enemy_selector(&effect.selector))
            .collect();
        if effects.is_empty() {
            return;
        }
        if let Some(source) = field.minion_mut(id) {
            source.delayed_battlecry_used = true;
        }
        let ctx = EffectCtx {
            seat: field.seat(side),
            source: Some(id),
            target: None,
            event_target: None,
            golden: minion.golden,
            level_override: None,
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

    /// 读取 `OnDeath | Revive` 的声明加成；没有该效果时返回 `None`。
    ///
    /// 复活本身由死亡结算执行；这里只负责提供卡面参数，避免把 `Revive`
    /// 当成普通效果重复执行（金色也只复活一次）。
    pub(crate) fn death_revive(&self, card_id: &str) -> Option<(i32, i32)> {
        self.combat_effects(card_id, Trigger::OnDeath)
            .into_iter()
            .find_map(|effect| match effect.op {
                Op::Revive => Some((
                    effect.params.attack.unwrap_or(0),
                    effect.params.health.unwrap_or(0),
                )),
                _ => None,
            })
    }

    /// 是否携带 `OnDeath | Recall | to=hand`。
    ///
    /// 只有显式回到手牌的 Recall 才由死亡结算处理；`to` 为其他区域时落空。
    pub(crate) fn death_recall(&self, card_id: &str) -> bool {
        self.combat_effects(card_id, Trigger::OnDeath)
            .into_iter()
            .any(|effect| effect.op == Op::Recall && effect.params.to == Some(Zone::Hand))
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
                level_override: None,
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
            && (effect.op == Op::DealDamage || matches!(effect.selector, Selector::Target { .. }))
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
                    let times = match &effect.params.times_if {
                        Some(times_if) if self.times_if_matches(&minion.card_id, times_if) => {
                            times_if.times
                        }
                        _ => base_times,
                    };
                    field.add_growth(id, times, health_only || minion.is_scholar);
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
                    duration: effect.params.duration.unwrap_or(Duration::Instant),
                };
                for id in targets {
                    field.modify_stats(id, modifier.clone());
                }
            }
            Op::GrantKeyword => {
                let Some(keyword) = &effect.params.keyword else {
                    return;
                };
                let name = keyword.keyword.as_str();
                let layer = keyword.value.or(effect.params.value);
                let duration = effect.params.duration.unwrap_or(Duration::Instant);
                for id in targets {
                    let Some(minion) = field.minion_mut(id) else {
                        continue;
                    };
                    match (name, &layer) {
                        ("breed", Some(count)) => {
                            minion.breed = minion.breed.saturating_add(*count);
                        }
                        ("contagion", Some(count)) => {
                            minion.contagion = minion.contagion.saturating_add(*count);
                        }
                        // 与招募期一致：`armor N` 的层数写进关键词字符串。
                        (combat::ARMOR, Some(count)) => {
                            record_grant(
                                minion,
                                &format!("{} {count}", combat::ARMOR),
                                duration,
                                false,
                            );
                        }
                        ("divine_shield", _) => {
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
                // 效果伤害归因给施放该效果的随从；无来源效果不产生击杀。
                for id in targets {
                    match ctx.source {
                        Some(source) => field.apply_damage(
                            &self.catalog,
                            id,
                            amount,
                            can_kill,
                            DamageSource::Minion(source),
                        ),
                        None => field.deal_damage(&self.catalog, id, amount, can_kill),
                    };
                }
            }
            // 分解：取事件目标（被分解随从）的生命值，加到选择器选出的随从上。
            Op::Decompose => self.apply_combat_decompose(field, effect, &targets, ctx),
            Op::Mimic => self.apply_combat_mimic(field, effect, &targets, ctx),
            Op::Devour => self.apply_combat_devour(field, effect, &targets, ctx),
            Op::Detonate => self.apply_combat_detonate(field, effect, &targets, ctx),
            Op::Convert => self.apply_combat_convert(field, rng, effect, &targets, ctx),
            Op::Salvage => self.apply_combat_salvage(field, rng, effect, ctx),
            Op::Summon => self.apply_combat_summon(field, rng, effect, ctx),
            Op::Copy => self.apply_combat_copy(field, effect, ctx, &targets),
            _ => {}
        }
    }

    /// 分解：按 `basis` 读取 `event_target` 的生命值，加到 `targets` 上。
    ///
    /// 目标已离场、快照缺失或获得量 ≤0 时静默跳过；`duration` 由数据校验保证
    /// 只取 `permanent` / `combat`。
    ///
    /// `when` 在战斗路径里只对 `DealDamage` 与单体目标筛选生效（见
    /// [`Engine::apply_combat_effect`]），因此这里显式按被分解者（`event_target`）
    /// 应用一次，与文档「`when` 可按事件载荷筛选」一致；没有 `when` 时行为不变。
    fn apply_combat_decompose(
        &self,
        field: &mut BattleField,
        effect: &Effect,
        targets: &[InstanceId],
        ctx: &EffectCtx,
    ) {
        let (Some(basis), Some(duration)) = (effect.params.basis, effect.params.duration) else {
            return;
        };
        let Some(victim) = ctx.event_target else {
            return;
        };
        if let Some(when) = &effect.params.when
            && !self.combat_filter_matches(field, when, victim, ctx)
        {
            return;
        }
        let Some(amount) = field.decompose_health(&self.catalog, victim, basis) else {
            return;
        };
        if amount <= 0 {
            return;
        }
        for id in targets {
            let Some(minion) = field.minion_mut(*id) else {
                continue;
            };
            match duration {
                Duration::Permanent => minion.stats.health.add_permanent(amount),
                Duration::Combat => minion.stats.health.add_turn(amount),
                // 数据校验拒绝 `instant`；执行期保守跳过。
                Duration::Instant | Duration::Aura => {}
            }
        }
    }

    /// 战斗内拟态：读取目标当前攻防与关键词，写入来源并记录动作。
    ///
    /// 目标已离场时跳过；敌方与友方目标都允许。攻防按 `duration` 写入，
    /// 非 `statsOnly` 时关键词写入 `granted`。战斗克隆的当前值已含光环，这里
    /// 先扣掉目标自身的光环层，并剔除光环授予的关键词；成长、繁衍、传染与
    /// 一次性状态同样不复制。
    fn apply_combat_mimic(
        &self,
        field: &mut BattleField,
        effect: &Effect,
        targets: &[InstanceId],
        ctx: &EffectCtx,
    ) {
        let Some(source_id) = ctx.source else {
            return;
        };
        if field.locate(source_id).is_none() {
            return;
        }
        let duration = effect.params.duration.unwrap_or(Duration::Combat);
        let stats_only = effect.params.stats_only.unwrap_or(false);
        for target_id in targets {
            if !field.is_alive(*target_id) {
                continue;
            }
            let Some(target) = field.minion(*target_id) else {
                continue;
            };
            // 战斗克隆已折叠光环；拟态按技术方案 §3.8 只复制不含光环的值。
            let target_attack = combat::effective_attack(&self.catalog, target)
                .saturating_sub(target.stats.attack.aura);
            let target_health = combat::effective_health(&self.catalog, target)
                .saturating_sub(target.stats.health.aura);
            let keywords: Vec<String> = if stats_only {
                Vec::new()
            } else {
                combat::keywords_without_aura(&self.catalog, target)
                    .into_iter()
                    .collect()
            };
            let Some(source) = field.minion(source_id) else {
                continue;
            };
            // 复制当前值：写入差值，金色第二次触发时差值为 0，以最后一次为准。
            let modifier = Modifier {
                attack: target_attack
                    .saturating_sub(combat::effective_attack(&self.catalog, source)),
                health: target_health
                    .saturating_sub(combat::effective_health(&self.catalog, source)),
                duration,
            };

            field.modify_stats(source_id, modifier);
            if let Some(source) = field.minion_mut(source_id) {
                for keyword in keywords {
                    record_grant(source, &keyword, duration, false);
                }
            }
            let states = field.snapshots(&self.catalog, &[source_id, *target_id]);
            field.push_action(
                CombatAction::Mimicked {
                    source: source_id,
                    target: *target_id,
                },
                states,
            );
        }
    }

    /// 战斗内吞噬：攻击力不超过 `value` 的目标被致死，来源永久获得其卡面基础值。
    ///
    /// 伤害直接走 `BattleField::apply_damage`，不经过护盾判定；致死量为当前生命
    /// 加上目标减伤，保证结算后生命恰为 0。来源记击杀，便于触发击杀类钩子。
    fn apply_combat_devour(
        &self,
        field: &mut BattleField,
        effect: &Effect,
        targets: &[InstanceId],
        ctx: &EffectCtx,
    ) {
        let Some(source_id) = ctx.source else {
            return;
        };
        let Some((source_side, _)) = field.locate(source_id) else {
            return;
        };
        let limit = effect.params.value.unwrap_or(0);
        for target_id in targets {
            let Some((target_side, _)) = field.locate(*target_id) else {
                continue;
            };
            if target_side == source_side || !field.is_alive(*target_id) {
                continue;
            }
            let Some(target) = field.minion(*target_id) else {
                continue;
            };
            if combat::effective_attack(&self.catalog, target) > limit {
                continue;
            }
            let Some((base_attack, base_health)) =
                base_stats(&self.catalog, &target.card_id, target.golden)
            else {
                continue;
            };
            // 护盾不参与判定；减伤仍会吞掉部分伤害，因此致死量含减伤。
            let amount = target
                .stats
                .health
                .current
                .max(0)
                .saturating_add(combat::armor_value(&self.catalog, target));
            field.apply_damage(
                &self.catalog,
                *target_id,
                amount,
                true,
                DamageSource::Minion(source_id),
            );
            if let Some(source) = field.minion_mut(source_id) {
                source.stats.attack.add_permanent(base_attack);
                source.stats.health.add_permanent(base_health);
            }
            let states = field.snapshots(&self.catalog, &[source_id, *target_id]);
            field.push_action(
                CombatAction::Devoured {
                    source: source_id,
                    target: *target_id,
                },
                states,
            );
        }
    }

    /// 战斗内引爆：移除目标全部传染层数，按层造成伤害，达到阈值时直接致死。
    ///
    /// 每层伤害走 [`BattleField::apply_damage`]（受减伤影响），护盾可抵消这一
    /// 段伤害；阈值致死绕过护盾与减伤。已移除的层数不会再参与回合掉血。
    fn apply_combat_detonate(
        &self,
        field: &mut BattleField,
        effect: &Effect,
        targets: &[InstanceId],
        ctx: &EffectCtx,
    ) {
        let Some(source_id) = ctx.source else {
            return;
        };
        let Some((source_side, _)) = field.locate(source_id) else {
            return;
        };
        let amount = effect.params.amount.unwrap_or(0).max(0);
        for target_id in targets {
            let Some((target_side, _)) = field.locate(*target_id) else {
                continue;
            };
            if target_side == source_side || !field.is_alive(*target_id) {
                continue;
            }
            let Some(target) = field.minion(*target_id) else {
                continue;
            };
            let layers = target.contagion;
            if layers <= 0 {
                continue;
            }
            let shielded = combat::keywords(&self.catalog, target).contains("divine_shield")
                && !target.shield_used;
            if let Some(target) = field.minion_mut(*target_id) {
                target.contagion = 0;
            }

            let raw = layers.saturating_mul(amount);
            let damage = if raw <= 0 {
                0
            } else if shielded {
                // 护盾抵消整段按层伤害并置为已用；不记录为实际伤害。
                if let Some(target) = field.minion_mut(*target_id) {
                    target.shield_used = true;
                }
                0
            } else {
                field.apply_damage(
                    &self.catalog,
                    *target_id,
                    raw,
                    true,
                    DamageSource::Minion(source_id),
                )
            };

            let reached = effect
                .params
                .threshold
                .is_some_and(|threshold| layers >= threshold);
            // 按层伤害已经致死时不再补一次致死伤害，避免重复击杀记录。
            if reached
                && field
                    .minion(*target_id)
                    .is_some_and(|target| target.stats.health.current > 0)
            {
                self.apply_lethal_damage(field, *target_id, source_id);
            }

            let states = field.snapshots(&self.catalog, &[*target_id]);
            field.push_action(
                CombatAction::Detonated {
                    target: *target_id,
                    layers,
                    damage,
                },
                states,
            );
        }
    }

    /// 战斗内转化：目标层数达到阈值时致死，并在来源方棋盘召唤 Token。
    ///
    /// 致死不经过护盾判定，层数作为门槛保留、不消耗。召唤走既有路径并使用
    /// 来源随从的真实 side；目标死亡留到外层 effect 结算，不提前触发死亡钩子。
    fn apply_combat_convert(
        &self,
        field: &mut BattleField,
        rng: &mut RngStreams,
        effect: &Effect,
        targets: &[InstanceId],
        ctx: &EffectCtx,
    ) {
        let Some(source_id) = ctx.source else {
            return;
        };
        let Some((source_side, _)) = field.locate(source_id) else {
            return;
        };
        let Some(threshold) = effect.params.threshold else {
            return;
        };
        for target_id in targets {
            let Some((target_side, _)) = field.locate(*target_id) else {
                continue;
            };
            if target_side == source_side || !field.is_alive(*target_id) {
                continue;
            }
            let Some(target) = field.minion(*target_id) else {
                continue;
            };
            if target.stats.health.current <= 0 || target.contagion < threshold {
                continue;
            }

            self.apply_lethal_damage(field, *target_id, source_id);
            let token = effect.params.token.clone().unwrap_or_default();
            let states = field.snapshots(&self.catalog, &[*target_id]);
            field.push_action(
                CombatAction::Converted {
                    target: *target_id,
                    token,
                },
                states,
            );
            // 目标死亡交由外层 effect 结算；召唤使用来源随从的真实 side，
            // 幽灵战两侧同座时不会退回 side 0。
            self.apply_combat_summon_on_side(field, rng, effect, ctx, source_side);
        }
    }

    /// 零件回收：死亡时从来源随从所属玩法中随机获得 `count` 张牌。
    ///
    /// 随机池按载体（`ctx.source`）的玩法筛选，而不是固定机械：技能被复制或
    /// 授予到其他流派随从时，池随载体变化。随机池按卡牌 id 升序遍历，随机
    /// 只推进 [`Stream::Combat`]；同一段效果内不重复取同一张（`count` 大于池
    /// 大小时按实际数量）。牌写入 [`BattleField::record_hand_copy`]，手牌已满、
    /// 池为空或来源离场时静默跳过，且不记录动作。获得的是卡面基础牌，不继承
    /// 来源状态。
    fn apply_combat_salvage(
        &self,
        field: &mut BattleField,
        rng: &mut RngStreams,
        effect: &Effect,
        ctx: &EffectCtx,
    ) {
        let Some(source_id) = ctx.source else {
            return;
        };
        let Some((source_side, _)) = field.locate(source_id) else {
            return;
        };
        let Some(carrier_card_id) = field.minion(source_id).map(|minion| minion.card_id.clone())
        else {
            return;
        };
        let count = effect.params.count.unwrap_or(1).max(0);
        if count == 0 {
            return;
        }

        // `pool=playstyle`：载体所属玩法的随从牌，排除非池卡、英雄专属与奖励牌。
        // 多玩法载体按任意一条 `play:` 取并集。
        let carrier_playstyles: BTreeSet<Playstyle> = self
            .catalog
            .cards
            .get(&carrier_card_id)
            .map(|card| {
                play_categories(&card.categories)
                    .filter_map(Playstyle::from_id)
                    .collect()
            })
            .unwrap_or_default();
        let mut candidates: Vec<String> = self
            .catalog
            .cards
            .values()
            .filter(|card| card.card_type == CardType::Minion)
            .filter(|card| card.id != REWARD_CARD_ID)
            .filter(|card| card.in_shop_pool)
            .filter(|card| {
                play_categories(&card.categories)
                    .filter_map(Playstyle::from_id)
                    .any(|playstyle| carrier_playstyles.contains(&playstyle))
            })
            .filter(|card| {
                field
                    .playstyles
                    .as_ref()
                    .is_none_or(|enabled| card_in_pool(card, enabled))
            })
            .map(|card| card.id.clone())
            .collect();

        for _ in 0..count {
            if candidates.is_empty() || field.hand_len[source_side] >= field.hand_slots {
                break;
            }
            let roll = rng.below(Stream::Combat, candidates.len() as u64) as usize;
            let card_id = candidates.remove(roll);
            if !field.record_hand_copy(source_side, &card_id, false) {
                break;
            }
            let states = field.snapshots(&self.catalog, &[source_id]);
            field.push_action(
                CombatAction::Salvaged {
                    seat: field.seat(source_side),
                    card_id,
                },
                states,
            );
        }
    }

    /// 对目标施加必死伤害：伤害量为当前生命加目标减伤，护盾与减伤均不参与判定。
    fn apply_lethal_damage(
        &self,
        field: &mut BattleField,
        target_id: InstanceId,
        source_id: InstanceId,
    ) {
        let Some(target) = field.minion(target_id) else {
            return;
        };
        let amount = target
            .stats
            .health
            .current
            .max(0)
            .saturating_add(combat::armor_value(&self.catalog, target));
        if amount > 0 {
            field.apply_damage(
                &self.catalog,
                target_id,
                amount,
                true,
                DamageSource::Minion(source_id),
            );
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
        self.apply_combat_summon_on_side(field, rng, effect, ctx, side);
    }

    /// 在指定阵营召唤；`side` 必须是来源随从的真实 side，供幽灵战同座场景使用。
    fn apply_combat_summon_on_side(
        &self,
        field: &mut BattleField,
        rng: &mut RngStreams,
        effect: &Effect,
        ctx: &EffectCtx,
        side: usize,
    ) {
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
            if !field.has_room(&self.catalog, side) {
                break;
            }
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
            let Some(summoned) = summoned else {
                continue;
            };
            field.summon_count = field.summon_count.saturating_add(1);
            self.fire_ally_summon(field, rng, side, summoned);
        }
    }

    /// 构造召唤实例；`token` 只命中非池卡、`card` 只命中池内卡，card 会记录扣池。
    fn combat_summon_instance(
        &self,
        field: &mut BattleField,
        effect: &Effect,
    ) -> Option<MinionInstance> {
        if let Some(token_id) = &effect.params.token {
            self.catalog
                .cards
                .get(token_id)
                .filter(|card| !card.in_shop_pool)?;
            return Some(new_minion(&self.catalog, 0, token_id, false));
        }
        if let Some(card_id) = &effect.params.card {
            self.catalog
                .cards
                .get(card_id)
                .filter(|card| card.in_shop_pool)?;
            field.record_pool_spend(card_id);
            return Some(new_minion(&self.catalog, 0, card_id, false));
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
                    // 阵营取来源随从的真实 side，不能用座位反推：幽灵战两侧同座。
                    let Some((side, _)) = field.locate(*id) else {
                        continue;
                    };
                    let Some(minion) = field.minion(*id) else {
                        continue;
                    };
                    let card_id = minion.card_id.clone();
                    let golden = minion.golden;
                    field.record_hand_copy(side, &card_id, golden);
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
                    if !field.has_room(&self.catalog, side) {
                        break;
                    }
                    copy.instance_id = field.new_instance_id();
                    copy.sources.clear();
                    let _ = field.append(&self.catalog, side, copy);
                }
            }
            Zone::Shop => {}
        }
    }

    fn combat_effects(&self, card_id: &str, trigger: Trigger) -> Vec<Effect> {
        self.combat_effects_filtered(card_id, trigger, None)
    }

    /// 英雄的战斗时机效果：技能只走英雄池，同名随从卡的战斗技能不参与。
    fn combat_hero_effects(&self, hero_id: &str, trigger: Trigger) -> Vec<Effect> {
        let Some(hero) = self.catalog.heroes.get(hero_id) else {
            return Vec::new();
        };
        self.skill_effects(&hero.skills, trigger)
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
            effects.extend(
                skill
                    .effects
                    .iter()
                    .filter(|effect| effect.trigger == trigger)
                    .cloned(),
            );
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
            Selector::Target { .. } => ctx
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
                // 潜行随从不能被敌方指向性效果选中；全体伤害（`Pick::All`）仍可命中。
                if *pick != Pick::All {
                    pool.retain(|id| {
                        let is_enemy = field.locate(*id).is_some_and(|(side, _)| side != friendly);
                        !is_enemy
                            || field
                                .minion(*id)
                                .is_none_or(|minion| !combat::has_stealth(&self.catalog, minion))
                    });
                }
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
            Pick::Single => {
                let chosen = ctx.target.or(ctx.event_target);
                match chosen {
                    Some(id) if pool.contains(&id) => vec![id],
                    // 显式目标不在池中时不改选，保持既有「目标非法即落空」语义。
                    Some(_) => Vec::new(),
                    // 战斗内没有玩家待选：单一目标按最左侧结算，保证效果可推进。
                    None => pool.first().copied().map(|id| vec![id]).unwrap_or_default(),
                }
            }
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
            Pick::LowestAttack => {
                let mut best: Option<(InstanceId, i32)> = None;
                for id in pool {
                    let Some(minion) = field.minion(id) else {
                        continue;
                    };
                    let attack = combat::effective_attack(&self.catalog, minion);
                    if best.is_none_or(|(_, current)| attack < current) {
                        best = Some((id, attack));
                    }
                }
                best.map(|(id, _)| vec![id]).unwrap_or_default()
            }
            // 相邻只服务光环；战斗路径的选择器不使用该选取方式。
            Pick::Adjacent => Vec::new(),
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
        let Some(facts) = card_facts(&self.catalog, &minion.card_id) else {
            return false;
        };
        match filter.key.as_str() {
            "keyword" => combat::keywords(&self.catalog, minion).contains(&filter.value),
            "other" => {
                if filter.value == "true" {
                    ctx.source != Some(id)
                } else {
                    true
                }
            }
            key => facts.matches(&self.catalog, key, &filter.value),
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
        for effect in &skill.effects {
            *coverage.entry((effect.trigger, effect.op)).or_insert(0) += 1;
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

/// 目标是否落在该座位本人的指定区域内。
///
/// 意图提交的第一道边界：棋盘只认本人棋盘、手牌只认本人手牌、商店只认本人商店槽
/// （`Target` 的棋盘池在引擎内仍取全部座位，以保留战斗内的 `event_target` 语义）。
pub(crate) fn target_in_own_zone(state: &GameState, seat: Seat, target: TargetRef) -> bool {
    state
        .players
        .get(usize::from(seat))
        .is_some_and(|player| player_has_target(player, target))
}

/// 目标是否落在除 `seat` 之外任一玩家的对应区域内。
///
/// 用于把「目标存在但属于敌方」与「目标根本不存在」区分成不同错误码：
/// 前者返回 `TARGET_NOT_ALLOWED`，后者返回 `NO_VALID_TARGET`。
pub(crate) fn target_in_other_zone(state: &GameState, seat: Seat, target: TargetRef) -> bool {
    state
        .players
        .iter()
        .any(|player| player.seat != seat && player_has_target(player, target))
}

fn player_has_target(player: &PlayerState, target: TargetRef) -> bool {
    match target.zone {
        Zone::Board => player
            .board
            .iter()
            .any(|minion| minion.instance_id == target.id),
        Zone::Hand => player
            .hand
            .iter()
            .any(|card| card.instance_id() == target.id),
        Zone::Shop => player
            .shop
            .iter()
            .flatten()
            .any(|slot| slot.instance_id == target.id),
    }
}

/// 本座位三个区域的实例编号，按「棋盘 → 手牌 → 商店」排列。
fn own_target_refs(state: &GameState, seat: Seat) -> Vec<TargetRef> {
    let Some(player) = state.players.get(usize::from(seat)) else {
        return Vec::new();
    };
    let mut refs = Vec::new();
    refs.extend(player.board.iter().map(|minion| TargetRef {
        zone: Zone::Board,
        id: minion.instance_id,
    }));
    refs.extend(player.hand.iter().map(|card| TargetRef {
        zone: Zone::Hand,
        id: card.instance_id(),
    }));
    refs.extend(player.shop.iter().flatten().map(|slot| TargetRef {
        zone: Zone::Shop,
        id: slot.instance_id,
    }));
    refs
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

/// 筛选与 `timesIf` 需要的卡牌事实。
///
/// 衍生物与非池卡同样在 `catalog.cards` 中；按 `categories` 的多条 `play:` 判定玩法。
/// 查不到时返回 `None`。
struct CardFacts<'a> {
    taxon: &'a str,
    categories: &'a [Category],
    subtype: Option<&'a str>,
    id: &'a str,
    card_type: CardType,
    is_scholar: bool,
}

impl CardFacts<'_> {
    /// 按筛选键匹配；`keyword` / `other` 由调用方单独处理。
    fn matches(&self, catalog: &Catalog, key: &str, value: &str) -> bool {
        match key {
            "taxon" => catalog.taxonomy.is_within(self.taxon, value),
            // `playstyle` 与 `play` 同口径：任一条 `play:` 命中即可。
            "playstyle" | "play" => has_play(self.categories, value),
            "prey" | "host" => {
                let kind = if key == "prey" {
                    CategoryKind::Prey
                } else {
                    CategoryKind::Host
                };
                self.categories.iter().any(|category| {
                    category.kind == kind && catalog.taxonomy.is_within(&category.value, value)
                })
            }
            "subtype" => self.subtype == Some(value),
            "id" => self.id == value,
            "type" => match value {
                "minion" => self.card_type == CardType::Minion,
                "effect" => self.card_type == CardType::Effect,
                _ => false,
            },
            "scholar" => match value {
                "true" => self.is_scholar,
                "false" => !self.is_scholar,
                _ => false,
            },
            _ => false,
        }
    }
}

fn card_facts<'a>(catalog: &'a Catalog, card_id: &str) -> Option<CardFacts<'a>> {
    catalog.cards.get(card_id).map(|card| CardFacts {
        taxon: &card.taxon,
        categories: &card.categories,
        subtype: card.subtype.as_deref(),
        id: &card.id,
        card_type: card.card_type,
        is_scholar: card.is_scholar,
    })
}

fn duration_rank(duration: Duration) -> u8 {
    match duration {
        // `Aura` 不由效果写入；与 `Instant` 同为最低优先级。
        Duration::Instant | Duration::Aura => 0,
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
