//! 进化（生物专有关键词）：进入战场时从三项中选择一项，先选后结算。
//!
//! 规则口径见 `keywords-and-mechanics.md` §1「进化」与 §2.2：战吼时先由玩家选择，
//! 再结算；金色选两次，可重复选同一项；进化只作用于自身；学者没有进化
//! （§1.1，数据层禁止学者携带该关键词）。每次结算对来源随从触发一次
//! `OnAllyEvolve`。
//!
//! 进化以 [`PendingChoice::Evolve`] 表达，只在招募阶段存在；选择由
//! `recruit::choose_discover` 与发现共用 FIFO 与按座位的消费规则。

use crate::data::Tribe;
use crate::effects::{Duration, EffectCtx, Trigger};
use crate::engine::Engine;
use crate::state::{GameState, InstanceId, MinionInstance, Modifier, PendingChoice, Seat};

/// 进化的三项选项，下标即 `Intent::ChooseDiscover.choice` 的取值。
///
/// 与关键词机制 §1「进化：从 +2/+2、护盾、复生中选择一个」逐项对应。
pub(crate) const EVOLVE_CHOICES: [&str; 3] = ["+2/+2", "护盾", "复生"];

/// `+2/+2` 选项的数值。
const EVOLVE_STATS: i32 = 2;
/// 选项：护盾。
const CHOICE_DIVINE_SHIELD: &str = "护盾";
/// 选项：复生。
const CHOICE_REBORN: &str = "复生";

impl Engine {
    /// 招募侧随从进入战场后登记进化待选。
    ///
    /// 调用时机是「落到战场之后、结算 `OnPlay` 战吼之前」（关键词机制 §2.2）；
    /// 多张同名进化副本同时入场时只登记一个待选：非金色选一次，金色连续选两次。
    /// 进化不推进随机流。
    pub(crate) fn register_evolve(
        &mut self,
        state: &mut GameState,
        seat: Seat,
        source: InstanceId,
    ) {
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
        if !self.card_has_keyword(&card_id, "进化") {
            return;
        }
        // 进化是生物专有词；学者不能进化（数据层已禁止，这里再兜底一次）。
        let evolvable = self
            .catalog
            .cards
            .get(&card_id)
            .is_some_and(|card| card.tribe == Tribe::Biology && !card.is_scholar);
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
        &mut self,
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
        } else {
            state.pending.remove(index);
        }
        Ok(())
    }

    /// 来源随从结算完战吼后，触发该座位其他随从的 `OnAllyEvolve`，按站位从左到右。
    ///
    /// `event_target` 为进化来源；金色观察者由执行器连续触发两次。
    pub(crate) fn fire_ally_evolve_recruit(
        &mut self,
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
            };
            // 观察者效果只修改观察者自身，不产生对外事件。
            let _ = self.run_card_effects(state, &card_id, Trigger::OnAllyEvolve, &ctx);
        }
    }
}

/// 结算一项进化选项：`+2/+2` 为永久属性修改，护盾/复生为永久授予关键词。
///
/// 复生在招募侧只登记关键词，复活只在战斗内发生（§1「作用对象：自身」）。
fn apply_evolve_option(minion: &mut MinionInstance, option: &str) {
    match option {
        "+2/+2" => minion.mods.push(Modifier {
            attack: EVOLVE_STATS,
            health: EVOLVE_STATS,
            duration: Duration::Permanent,
        }),
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
