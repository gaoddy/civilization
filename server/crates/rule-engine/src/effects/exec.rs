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

use crate::combat::{self, BattleField, CombatAction, CombatSetup, DamageSource};
use crate::data::{CardType, Catalog, Playstyle, Specialty, SpecialtyKind};
use crate::effects::{
    CopyState, Duration, Effect, Filter, Op, Pick, Selector, Side, TimesIf, Trigger, Zone,
};
use crate::engine::Engine;
use crate::events::GameEvent;
use crate::intent::{Reject, RejectCode};
use crate::recruit::new_minion;
use crate::rng::{RngStreams, Stream};
use crate::state::{
    GameState, HandCard, InstanceId, MinionInstance, Modifier, Phase, PlayerState, Seat, base_stats,
};

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
    pub discover_level: Option<u8>,
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
            Op::Summon => {
                self.summon_recruit_from_effect(state, effect, &targets, ctx);
                Vec::new()
            }
            Op::Copy => {
                self.apply_recruit_copy(state, effect, &targets, ctx);
                Vec::new()
            }
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
            Op::Detonate | Op::Convert | Op::Revive | Op::Recall | Op::Salvage | Op::Latent => {
                Vec::new()
            }
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
            if let Some(skill) = self.catalog.skills.get(skill_id) {
                effects.extend(
                    skill
                        .effects
                        .iter()
                        .filter(|effect| effect.trigger == trigger)
                        .filter(|effect| {
                            !(trigger == Trigger::OnPlay && is_enemy_selector(&effect.selector))
                        })
                        .cloned(),
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
    /// 该卡在该触发时机下是否存在「需要玩家指定单一目标」的效果。
    ///
    /// 只看选择器：`Selector::Target`，或 `Selector::Side { pick: Pick::Single, .. }`。
    /// **不考虑 `condition`**：条件依赖运行时状态，本判定要能在没有座位上下文时回答，
    /// 因此是保守超集——视图只用它做交互提示，真正的池判定在 `check_targets`。
    pub fn requires_target(&self, card_id: &str, trigger: Trigger) -> bool {
        self.effects_of(card_id, trigger).iter().any(|effect| {
            matches!(
                effect.selector,
                Selector::Target
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
                // 只记录裸 `armor` 会丢失 N。其余带层关键词暂无战斗侧读取方。
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

    /// 拟态：把目标当前攻防与关键词写到来源上。
    ///
    /// 目标缺失、不在敌方或仍在招募阶段时静默落空。攻防按 `ModifyStats`
    /// 的时长口径写入；关键词写入 `granted`，不复制成长、繁衍、传染与
    /// 护盾/复生等一次性状态。金色连续触发时逐次覆盖（同值反复写入不叠加）。
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
        let copied: Vec<(i32, i32, Vec<String>)> = targets
            .iter()
            .filter(|id| self.is_enemy_target(state, ctx.seat, **id))
            .filter_map(|id| {
                let (_, target) = locate(state, *id)?;
                let keywords = combat::keywords(&self.catalog, target)
                    .into_iter()
                    .collect::<Vec<_>>();
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
                Duration::Permanent | Duration::Instant => {
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

    /// `timesIf` 是否命中该卡牌/Token。`scholar` 只看学者标记，其余按筛选键匹配。
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
                    discover_level: None,
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
                discover_level: None,
            };
            let _ = self.run_card_effects(state, &card_id, Trigger::OnAllySummon, &ctx);
        }
    }

    /// 执行 `Copy`：复制随从进手牌或战场，不扣牌池。
    ///
    /// `state` 决定复制口径：`base` 只按卡面构造（不继承任何状态），
    /// `current` 为完全复制，继承成长、繁衍、传染、护盾、已授予关键词与金色标记。
    /// 金色随从复制的仍是金色，卡面基础值同样 ×2。
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
        let full = effect.params.state.unwrap_or(CopyState::Base) == CopyState::Current;

        let mut copies: Vec<MinionInstance> = Vec::new();
        if effect.params.from == Some(Zone::Hand) {
            if let Some(minion) = player.hand.iter().find_map(HandCard::minion) {
                copies.push(self.copy_of_catalog(minion, full));
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
                let effects = self.combat_effects(hero, Trigger::OnCombatStart);
                let ctx = EffectCtx {
                    seat: field.seat(side),
                    source: None,
                    target: None,
                    event_target: None,
                    golden: false,
                    discover_level: None,
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
            discover_level: None,
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
            discover_level: None,
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
                discover_level: None,
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
                Duration::Instant => {}
            }
        }
    }

    /// 战斗内拟态：读取目标当前攻防与关键词，写入来源并记录动作。
    ///
    /// 目标与来源同阵营、目标已离场时跳过；攻防按 `duration` 写入，关键词写入
    /// `granted`。目标当前值不含光环，也不复制成长、繁衍、传染与一次性状态。
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
        let Some((source_side, _)) = field.locate(source_id) else {
            return;
        };
        let duration = effect.params.duration.unwrap_or(Duration::Combat);
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
            let target_attack = combat::effective_attack(&self.catalog, target);
            let target_health = combat::effective_health(&self.catalog, target);
            let keywords: Vec<String> = combat::keywords(&self.catalog, target)
                .into_iter()
                .collect();
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

    /// 构造召唤实例；缺失 token/card 时返回 `None`，card 会记录扣池。
    fn combat_summon_instance(
        &self,
        field: &mut BattleField,
        effect: &Effect,
    ) -> Option<MinionInstance> {
        if let Some(token_id) = &effect.params.token {
            let token = self.catalog.tokens.get(token_id)?;
            return Some(new_minion(&self.catalog, 0, &token.id, false));
        }
        if let Some(card_id) = &effect.params.card {
            self.catalog.cards.get(card_id)?;
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

/// 筛选与 `timesIf` 需要的卡牌/Token 事实。
///
/// `catalog.cards` 查不到时回退 `catalog.tokens`；Token 缺玩法/分类时按
/// `Neutral` / `phenomenon` 处理。两者都找不到时返回 `None`。
struct CardFacts<'a> {
    playstyle: Playstyle,
    taxon: &'a str,
    specialty: &'a [Specialty],
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
            "playstyle" => self.playstyle.id() == value,
            "play" => self
                .specialty
                .iter()
                .any(|specialty| specialty.kind == SpecialtyKind::Play && specialty.value == value),
            "prey" | "host" => {
                let kind = if key == "prey" {
                    SpecialtyKind::Prey
                } else {
                    SpecialtyKind::Host
                };
                self.specialty.iter().any(|specialty| {
                    specialty.kind == kind && catalog.taxonomy.is_within(&specialty.value, value)
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
    if let Some(card) = catalog.cards.get(card_id) {
        return Some(CardFacts {
            playstyle: card.playstyle,
            taxon: &card.taxon,
            specialty: &card.specialty,
            subtype: card.subtype.as_deref(),
            id: &card.id,
            card_type: card.card_type,
            is_scholar: card.is_scholar,
        });
    }
    catalog.tokens.get(card_id).map(|token| CardFacts {
        playstyle: token.playstyle(),
        taxon: token.taxon(),
        specialty: &[],
        subtype: None,
        id: &token.id,
        card_type: CardType::Minion,
        is_scholar: false,
    })
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
