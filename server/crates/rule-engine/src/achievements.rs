//! 局内成就标识与固定结算顺序。
//!
//! 成就的条件与奖励以 `docs/versions/v0.2/product/content/achievements/` 的实体
//! 文档为准；本模块只提供规则引擎内部的稳定标识，不解析展示文本。
//!
//! 六个成就的触发与结算分散在引擎各阶段，`ALL` 的顺序用于同一操作内多个成就
//! 同时满足时的固定结算顺序，保证确定性。

use crate::combat;
use crate::data::{CardType, Playstyle, card_has_playstyle, card_in_pool};
use crate::effects::Op;
use crate::engine::Engine;
use crate::keyword::Keyword;
use crate::rng::Stream;
use crate::state::{
    GameState, HandCard, InstanceId, PendingChoice, Seat, StatSourceLayer, StatSourceOrigin,
    StatSystemSource,
};

/// 机器轰鸣的触发卡牌：蒸汽机。
pub const STEAM_ENGINE_CARD_ID: &str = "steam-engine";

/// 触发成就所需的不同学科数量（有教无类）。
const EDUCATION_SUBTYPES: usize = 5;

/// 触发成就所需的成长结算次数（生物进化）。
const BIOLOGICAL_GROWTH_EVENTS: u8 = 3;

/// 一局内可触发的成就。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Achievement {
    BiologicalEvolution,
    EducationForAll,
    Genetics,
    MachinesRoar,
    OneSmallStep,
    TuringMoment,
}

impl Achievement {
    /// 全部成就，按实体文档 `id` 升序排列。
    pub const ALL: [Achievement; 6] = [
        Achievement::BiologicalEvolution,
        Achievement::EducationForAll,
        Achievement::Genetics,
        Achievement::MachinesRoar,
        Achievement::OneSmallStep,
        Achievement::TuringMoment,
    ];

    /// 与实体文档和生成数据一致的稳定 id。
    pub fn id(self) -> &'static str {
        match self {
            Achievement::BiologicalEvolution => "biological-evolution",
            Achievement::EducationForAll => "education-for-all",
            Achievement::Genetics => "genetics",
            Achievement::MachinesRoar => "machines-roar",
            Achievement::OneSmallStep => "one-small-step",
            Achievement::TuringMoment => "turing-moment",
        }
    }

    /// 由稳定 id 解析；未知 id 返回 `None`。
    pub fn from_id(id: &str) -> Option<Achievement> {
        Achievement::ALL
            .into_iter()
            .find(|achievement| achievement.id() == id)
    }
}

impl Engine {
    /// 触发一个本局尚未触发的成就并立即结算奖励。
    ///
    /// 同一成就本局只结算一次；不同成就之间不设每局总数上限。
    pub(crate) fn trigger_achievement(
        &self,
        state: &mut GameState,
        seat: Seat,
        achievement: Achievement,
    ) {
        self.trigger_achievement_on(state, seat, achievement, None);
    }

    /// 同 [`Engine::trigger_achievement`]，`target` 为奖励需要作用的随从。
    fn trigger_achievement_on(
        &self,
        state: &mut GameState,
        seat: Seat,
        achievement: Achievement,
        target: Option<InstanceId>,
    ) {
        let Some(player) = state.players.get_mut(usize::from(seat)) else {
            return;
        };
        if !player.achievements.unlocked.insert(achievement) {
            return;
        }
        self.apply_achievement_reward(state, seat, achievement, target);
    }

    /// 记录一次招募阶段的成长结算（每个被命中的随从一次）。
    ///
    /// `times` 是本次成长层数；层数不重复计数，一次结算只记 1 次。
    pub(crate) fn record_growth_achievement(
        &self,
        state: &mut GameState,
        seat: Seat,
        card_id: &str,
        times: i32,
    ) {
        if times <= 0 {
            return;
        }
        let subtype = self
            .catalog
            .cards
            .get(card_id)
            .and_then(|card| card.subtype.clone());
        {
            let Some(player) = state.players.get_mut(usize::from(seat)) else {
                return;
            };
            let progress = &mut player.achievements;
            progress.growth_events = progress.growth_events.saturating_add(1);
            if let Some(subtype) = subtype {
                progress.grown_subtypes.insert(subtype);
            }
        }
        let (growth_events, subtypes) = {
            let progress = &state.players[usize::from(seat)].achievements;
            (progress.growth_events, progress.grown_subtypes.len())
        };
        if growth_events >= BIOLOGICAL_GROWTH_EVENTS {
            self.trigger_achievement(state, seat, Achievement::BiologicalEvolution);
        }
        if subtypes >= EDUCATION_SUBTYPES {
            self.trigger_achievement(state, seat, Achievement::EducationForAll);
        }
    }

    /// 扫描本座位棋盘，触发图灵时刻。
    ///
    /// 关键词取卡面派生 + 已授予 + 光环派生；同一机械随从同时拥有
    /// `ignore_taunt` 与 `windfury` 时触发。
    pub(crate) fn check_keyword_achievements(&self, state: &mut GameState, seat: Seat) {
        let Some(player) = state.players.get(usize::from(seat)) else {
            return;
        };
        if player
            .achievements
            .unlocked
            .contains(&Achievement::TuringMoment)
        {
            return;
        }
        let mut found = None;
        for minion in &player.board {
            let Some(card) = self.catalog.cards.get(&minion.card_id) else {
                continue;
            };
            if !card_has_playstyle(card, Playstyle::Machine) {
                continue;
            }
            let keywords = combat::keyword_set(&self.catalog, minion);
            let aura = state.auras.get(&minion.instance_id);
            let has_ignore_taunt = keywords.contains(Keyword::IgnoreTaunt)
                || aura.is_some_and(|aura| aura.keywords.contains(Keyword::IgnoreTaunt));
            let has_windfury = keywords.contains(Keyword::Windfury)
                || aura.is_some_and(|aura| aura.keywords.contains(Keyword::Windfury));
            if has_ignore_taunt && has_windfury {
                found = Some(minion.instance_id);
                break;
            }
        }
        if let Some(instance_id) = found {
            self.trigger_achievement_on(state, seat, Achievement::TuringMoment, Some(instance_id));
        }
    }

    fn apply_achievement_reward(
        &self,
        state: &mut GameState,
        seat: Seat,
        achievement: Achievement,
        target: Option<InstanceId>,
    ) {
        match achievement {
            Achievement::BiologicalEvolution => {
                if let Some(player) = state.players.get_mut(usize::from(seat)) {
                    player.gold = player.gold.saturating_add(1);
                }
            }
            Achievement::EducationForAll => self.grant_random_effect_card(state, seat),
            Achievement::Genetics => self.grant_plant_discover(state, seat),
            Achievement::MachinesRoar => {
                if let Some(player) = state.players.get_mut(usize::from(seat)) {
                    player.achievements.machine_discount_turn = Some(state.turn);
                }
            }
            Achievement::OneSmallStep => {
                if let Some(player) = state.players.get_mut(usize::from(seat)) {
                    player.achievements.next_turn_gold_bonus =
                        player.achievements.next_turn_gold_bonus.saturating_add(1);
                }
            }
            Achievement::TuringMoment => {
                let instance_id = target
                    .or_else(|| self.first_turing_minion(state, seat))
                    .unwrap_or(0);
                if let Some(player) = state.players.get_mut(usize::from(seat))
                    && let Some(minion) = player
                        .board
                        .iter_mut()
                        .find(|minion| minion.instance_id == instance_id)
                {
                    minion.stats.attack.add_permanent(2);
                    minion.stats.health.add_permanent(2);
                    minion.record_stat_source(
                        StatSourceLayer::Permanent,
                        StatSourceOrigin::System(StatSystemSource::Unknown),
                        2,
                        2,
                    );
                }
            }
        }
    }

    fn first_turing_minion(&self, state: &GameState, seat: Seat) -> Option<InstanceId> {
        let player = state.players.get(usize::from(seat))?;
        player
            .board
            .iter()
            .find(|minion| {
                self.catalog
                    .cards
                    .get(&minion.card_id)
                    .is_some_and(|card| card_has_playstyle(card, Playstyle::Machine))
            })
            .map(|minion| minion.instance_id)
    }

    /// 有教无类：随机获得一张本局启用的效果牌。
    ///
    /// 排除含 `Growth` 的效果牌，避免奖励直接加速成长类成就；手牌已满时作废。
    fn grant_random_effect_card(&self, state: &mut GameState, seat: Seat) {
        let hand_slots = self.catalog.config.hand_slots;
        if state.players[usize::from(seat)].hand.len() >= hand_slots {
            return;
        }
        let candidates: Vec<String> = self
            .catalog
            .cards
            .values()
            .filter(|card| {
                card.card_type == CardType::Effect
                    && card.in_shop_pool
                    && card_in_pool(card, &state.enabled_playstyles)
            })
            .filter(|card| !self.card_has_growth_effect(&card.id))
            .map(|card| card.id.clone())
            .collect();
        if candidates.is_empty() {
            return;
        }
        let index = state.rng.below(Stream::Effects, candidates.len() as u64) as usize;
        let card_id = candidates[index].clone();
        let instance_id = state.next_instance_id;
        state.next_instance_id = instance_id.saturating_add(1);
        if let Some(player) = state.players.get_mut(usize::from(seat)) {
            player.hand.push(HandCard::effect(instance_id, &card_id));
        }
    }

    fn card_has_growth_effect(&self, card_id: &str) -> bool {
        let Some(card) = self.catalog.cards.get(card_id) else {
            return false;
        };
        card.skills.iter().any(|skill_id| {
            self.catalog
                .skill(*skill_id)
                .effects
                .iter()
                .any(|effect| effect.op == Op::Growth)
        })
    }

    /// 遗传学：以当前时代为星级（6 星封顶）发现一张植物随从。
    fn grant_plant_discover(&self, state: &mut GameState, seat: Seat) {
        let level = state
            .players
            .get(usize::from(seat))
            .map(|player| (player.era as u8).min(6))
            .unwrap_or(1);
        let candidates = self.draw_discover_candidates_for(state, level, Some(Playstyle::Plant));
        if candidates.is_empty() {
            return;
        }
        state.pending.push_back(PendingChoice::Discover {
            seat,
            candidates,
            source: 0,
        });
    }
}
