//! 进化（生物专有关键词）：进入战场时从三项中选择一项，先选后结算。
//!
//! 规则口径见 `keywords-and-mechanics.md` §1「进化」与 §2.2：战吼时先由玩家选择，
//! 再结算；金色选两次，可重复选同一项；进化只作用于自身，且只有生物卡能携带
//! （§1.1）。每次结算对来源随从触发一次
//! `OnAllyEvolve`。
//!
//! 进化以 [`PendingChoice::Evolve`] 表达，只在招募阶段存在；选择由
//! `recruit::choose_discover` 与发现共用 FIFO 与按座位的消费规则。
//! `OnAllyEvolve` 只在进化实际完成（最后一次选择结算完）时触发。

use crate::data::Playstyle;
use crate::effects::{Duration, EffectCtx, Trigger};
use crate::engine::Engine;
use crate::state::{
    GameState, InstanceId, MinionInstance, PendingChoice, Seat, StatSourceLayer, StatSourceOrigin,
    StatSystemSource,
};

/// 进化的三项选项，下标即 `Intent::ChooseDiscover.choice` 的取值。
///
/// 与关键词机制 §1「进化：从 +2/+2、护盾、复生中选择一个」逐项对应。
/// 视图层下发 [`crate::view::PendingChoiceView::Evolve`] 时直接使用这份列表。
pub const EVOLVE_CHOICES: [&str; 3] = ["stats", "divine_shield", "reborn"];

/// `stats` 选项的数值。
const EVOLVE_STATS: i32 = 2;
/// 选项：护盾。
const CHOICE_DIVINE_SHIELD: &str = "divine_shield";
/// 选项：复生。
const CHOICE_REBORN: &str = "reborn";

impl Engine {
    /// 招募侧随从进入战场后登记进化待选。
    ///
    /// 调用时机是「落到战场之后、结算 `OnPlay` 战吼之前」（关键词机制 §2.2）；
    /// 多张同名进化副本同时入场时只登记一个待选：非金色选一次，金色连续选两次。
    /// 进化不推进随机流。
    pub(crate) fn register_evolve(&self, state: &mut GameState, seat: Seat, source: InstanceId) {
        let Some(player) = state.players.get(usize::from(seat)) else {
            return;
        };
        let Some(minion) = player
            .board
            .iter()
            .find(|minion| minion.instance_id == source)
        else {
            return;
        };
        let (card_id, golden) = (minion.card_id.clone(), minion.golden);
        if !self.card_has_keyword(&card_id, "evolve") {
            return;
        }
        // 进化是生物专有词；带 `play:machine` 的卡不能进化
        // （数据层已禁止，这里再兜底一次）。
        let evolvable = self
            .catalog
            .filter_facts(&card_id)
            .is_some_and(|facts| facts.play_mask & Playstyle::Machine.bit() == 0);
        if !evolvable {
            return;
        }
        state.pending.push_back(PendingChoice::Evolve {
            seat,
            source,
            remaining: if golden { 2 } else { 1 },
        });
    }

    /// 消费一次进化选择；调用方已确认该座位没有更早的发现待选。
    ///
    /// - `Ok(())`：选择已结算（或来源已离场，待选被丢弃）。
    /// - `Err(())`：下标越界，状态不变。
    pub(crate) fn resolve_evolve_choice(
        &self,
        state: &mut GameState,
        index: usize,
        choice: u8,
    ) -> Result<(), ()> {
        let choice = usize::from(choice);
        let Some(option) = EVOLVE_CHOICES.get(choice) else {
            return Err(());
        };
        let Some(PendingChoice::Evolve {
            seat,
            source,
            remaining,
        }) = state.pending.get(index).cloned()
        else {
            return Err(());
        };
        let Some(position) = state.players.get(usize::from(seat)).and_then(|player| {
            player
                .board
                .iter()
                .position(|minion| minion.instance_id == source)
        }) else {
            state.pending.remove(index);
            return Ok(());
        };

        apply_evolve_option(
            &mut state.players[usize::from(seat)].board[position],
            option,
        );
        if remaining > 1 {
            if let Some(PendingChoice::Evolve { remaining, .. }) = state.pending.get_mut(index) {
                *remaining = remaining.saturating_sub(1);
            }
            // 金色：第一项已生效，但整次进化尚未完成，继续等待下一次选择。
            // 两次选择连续发生，中间不插入观察者效果（§2.6）。
        } else {
            state.pending.remove(index);
            // 进化实际完成（最后一次选择结算完）后才触发 `OnAllyEvolve`。
            self.fire_ally_evolve_recruit(state, seat, source);
        }
        Ok(())
    }

    /// 入场路径调用：来源登记了进化则等选择结算，否则当场按「进化」处理。
    ///
    /// 数据中带「进化」关键词的卡总会在入场时登记待选；已登记时由选择结算触发。
    /// 未登记的情形（没有进化、来源已离场或未找到）在这里按「未发生进化」
    /// 处理，即不触发——`OnAllyEvolve` 不再是入场副作用，而是选择结果的副作用。
    pub(crate) fn fire_evolve_after_enter(
        &self,
        state: &mut GameState,
        seat: Seat,
        entered: InstanceId,
    ) {
        let waiting = state.pending.iter().any(|pending| {
            matches!(
                pending,
                PendingChoice::Evolve { seat: pending_seat, source, .. }
                    if *pending_seat == seat && *source == entered
            )
        });
        if !waiting {
            self.fire_ally_evolve_recruit(state, seat, entered);
        }
    }

    /// 触发该座位其他随从的 `OnAllyEvolve`，按站位从左到右。
    ///
    /// 只在进化实际完成时调用：进入战场路径在来源没有登记进化时直接调用，
    /// 登记了进化的来源等 `resolve_evolve_choice` 结算完最后一次选择再调用。
    /// 整次进化只触发一次：金色进化的两次选择属于同一次进化，第一次选完不触发，
    /// 最后一次选完才触发；两次选择之间不插入其他随从效果，观察者自身的
    /// 金色「触发两次」照常生效。
    /// `event_target` 为进化来源。
    pub(crate) fn fire_ally_evolve_recruit(
        &self,
        state: &mut GameState,
        seat: Seat,
        evolved: InstanceId,
    ) {
        let Some(player) = state.players.get(usize::from(seat)) else {
            return;
        };
        let owners: Vec<(InstanceId, String, bool)> = player
            .board
            .iter()
            .filter(|minion| minion.instance_id != evolved)
            .map(|minion| (minion.instance_id, minion.card_id.clone(), minion.golden))
            .collect();
        for (source, card_id, golden) in owners {
            if !self.card_has_trigger(&card_id, Trigger::OnAllyEvolve) {
                continue;
            }
            let ctx = EffectCtx {
                seat,
                source: Some(source),
                target: None,
                event_target: Some(evolved),
                golden,
                level_override: None,
            };
            // 观察者效果只修改观察者自身，不产生对外事件。
            let _ = self.run_card_effects(state, &card_id, Trigger::OnAllyEvolve, &ctx);
        }
    }
}

/// 结算一项进化选项：`stats` 为永久属性修改，护盾/复生为永久授予关键词。
///
/// 复生在招募侧只登记关键词，复活只在战斗内发生（§1「作用对象：自身」）。
fn apply_evolve_option(minion: &mut MinionInstance, option: &str) {
    match option {
        "stats" => {
            minion.stats.attack.add_permanent(EVOLVE_STATS);
            minion.stats.health.add_permanent(EVOLVE_STATS);
            minion.record_stat_source(
                StatSourceLayer::Permanent,
                StatSourceOrigin::System(StatSystemSource::Evolve),
                EVOLVE_STATS,
                EVOLVE_STATS,
            );
        }
        CHOICE_DIVINE_SHIELD => {
            minion
                .granted
                .push((CHOICE_DIVINE_SHIELD.to_string(), Duration::Permanent));
            minion.shield_used = false;
        }
        CHOICE_REBORN => minion
            .granted
            .push((CHOICE_REBORN.to_string(), Duration::Permanent)),
        _ => {}
    }
}
