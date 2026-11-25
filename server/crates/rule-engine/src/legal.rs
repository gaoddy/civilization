//! 合法动作枚举：训练掩码与服务端兜底的唯一口径。
//!
//! 只读：不修改状态、不消耗随机流。返回顺序固定，一次枚举内的下标就是训练侧的
//! 动作编号；服务端与训练端都从同一函数重建，避免两套编号漂移。
//!
//! 阶段门控与 [`crate::view::view`] 一致：不在招募阶段、已出局或已结束招募时为空。
//! 有待选时只返回待选相关动作——目标待选给 `ChooseTarget` 与 `AbandonTarget`，
//! 发现/进化待选给 `ChooseDiscover`。

use crate::effects::{EVOLVE_CHOICES, EffectCtx, TargetRef, Trigger};
use crate::engine::Engine;
use crate::intent::Intent;
use crate::recruit::{
    check_buy, check_end_recruit, check_freeze, check_hero_power, check_minion_ability,
    check_play_card, check_reroll, check_sell, check_unfreeze, check_upgrade,
};
use crate::state::{GameState, HandCard, InstanceId, PendingChoice, Phase, Seat};
use crate::view::{hand_card_targets, hero_power_targets, minion_ability_targets};

/// 枚举指定座位当前全部合法动作，顺序固定。
pub fn legal_actions(engine: &Engine, state: &GameState, seat: Seat) -> Vec<Intent> {
    let Some(player) = state.players.get(usize::from(seat)) else {
        return Vec::new();
    };
    if state.phase != Phase::Recruit || player.eliminated || player.recruit_done {
        return Vec::new();
    }

    if let Some(pending) = pending_for(state, seat) {
        return match pending {
            PendingChoice::Target {
                source, hand_card, ..
            } => {
                let mut actions: Vec<Intent> =
                    pending_target_candidates(engine, state, seat, *source, hand_card)
                        .into_iter()
                        .map(|target| Intent::ChooseTarget { target })
                        .collect();
                actions.push(Intent::AbandonTarget);
                actions
            }
            PendingChoice::Discover { candidates, .. } => (0..candidates.len())
                .map(|choice| Intent::ChooseDiscover {
                    choice: u8::try_from(choice).unwrap_or(u8::MAX),
                })
                .collect(),
            PendingChoice::Evolve { .. } => (0..EVOLVE_CHOICES.len())
                .map(|choice| Intent::ChooseDiscover {
                    choice: u8::try_from(choice).unwrap_or(u8::MAX),
                })
                .collect(),
        };
    }

    let mut actions = Vec::new();

    // 买：商店槽位升序。
    for slot in 0..player.shop.len() {
        let slot = u8::try_from(slot).unwrap_or(u8::MAX);
        if check_buy(engine, state, seat, slot).is_ok() {
            actions.push(Intent::BuyMinion { slot });
        }
    }

    // 卖：棋盘升序。
    for minion in &player.board {
        if check_sell(state, seat, minion.instance_id).is_ok() {
            actions.push(Intent::SellMinion {
                minion: minion.instance_id,
            });
        }
    }

    if check_reroll(state, seat).is_ok() {
        actions.push(Intent::RerollShop);
    }
    if check_upgrade(engine, state, seat).is_ok() {
        actions.push(Intent::UpgradeEra);
    }

    // 打牌：手牌升序。随从牌给全部插入点（None 追加 + 0..len，避免重复）；
    // 效果牌按目标候选展开，需要目标却没有候选时不可打出。
    for (index, hand_card) in player.hand.iter().enumerate() {
        let hand_index = u8::try_from(index).unwrap_or(u8::MAX);
        if check_play_card(engine, state, seat, hand_index).is_err() {
            continue;
        }
        if hand_card.minion().is_some() {
            actions.push(Intent::PlayCard {
                hand_index,
                position: None,
                target: None,
            });
            for position in 0..player.board.len() {
                actions.push(Intent::PlayCard {
                    hand_index,
                    position: Some(u8::try_from(position).unwrap_or(u8::MAX)),
                    target: None,
                });
            }
        } else {
            let (requires_target, target_candidates) =
                hand_card_targets(engine, state, seat, hand_card);
            if requires_target {
                if target_candidates.is_empty() {
                    continue;
                }
                for target in target_candidates {
                    actions.push(Intent::PlayCard {
                        hand_index,
                        position: None,
                        target: Some(target),
                    });
                }
            } else {
                actions.push(Intent::PlayCard {
                    hand_index,
                    position: None,
                    target: None,
                });
            }
        }
    }

    // 站位：from 升序、to 升序；原地不动是空操作，不枚举。
    for from in 0..player.board.len() {
        for to in 0..player.board.len() {
            if from == to {
                continue;
            }
            actions.push(Intent::RearrangeBoard {
                from: u8::try_from(from).unwrap_or(u8::MAX),
                to: u8::try_from(to).unwrap_or(u8::MAX),
            });
        }
    }

    // 冻结与解冻：先全部槽位，再逐槽位。
    if check_freeze(state, seat, None).is_ok() {
        actions.push(Intent::FreezeShop { slot: None });
    }
    for slot in 0..player.shop.len() {
        let slot = u8::try_from(slot).unwrap_or(u8::MAX);
        if check_freeze(state, seat, Some(slot)).is_ok() {
            actions.push(Intent::FreezeShop { slot: Some(slot) });
        }
    }
    if check_unfreeze(state, seat, None).is_ok() {
        actions.push(Intent::UnfreezeShop { slot: None });
    }
    for slot in 0..player.shop.len() {
        let slot = u8::try_from(slot).unwrap_or(u8::MAX);
        if check_unfreeze(state, seat, Some(slot)).is_ok() {
            actions.push(Intent::UnfreezeShop { slot: Some(slot) });
        }
    }

    // 英雄技能：按英雄卡的技能顺序。
    if let Some(hero_id) = player.hero.as_deref()
        && let Some(hero) = engine.catalog.heroes.get(hero_id)
    {
        for skill_ref in &hero.skills {
            let skill = engine.catalog.skill(*skill_ref);
            // 被动技能没有费用，不能主动使用。
            if skill.cost.is_none() {
                continue;
            }
            let skill_id = skill.id.as_str();
            if check_hero_power(engine, state, seat, skill_id).is_err() {
                continue;
            }
            let (requires_target, target_candidates) =
                hero_power_targets(engine, state, seat, skill_id);
            if requires_target {
                if target_candidates.is_empty() {
                    continue;
                }
                for target in target_candidates {
                    actions.push(Intent::UseHeroPower {
                        skill_id: skill_id.to_string(),
                        target: Some(target),
                    });
                }
            } else {
                actions.push(Intent::UseHeroPower {
                    skill_id: skill_id.to_string(),
                    target: None,
                });
            }
        }
    }

    // 随从技能：随从升序、技能顺序。
    for minion in &player.board {
        let Some(card) = engine.catalog.cards.get(&minion.card_id) else {
            continue;
        };
        for skill_ref in &card.skills {
            let skill = engine.catalog.skill(*skill_ref);
            if skill.cost.is_none() {
                continue;
            }
            let skill_id = skill.id.as_str();
            if check_minion_ability(engine, state, seat, minion.instance_id, skill_id).is_err() {
                continue;
            }
            let (requires_target, target_candidates) =
                minion_ability_targets(engine, state, seat, minion, skill_id);
            if requires_target {
                if target_candidates.is_empty() {
                    continue;
                }
                for target in target_candidates {
                    actions.push(Intent::UseMinionAbility {
                        minion: minion.instance_id,
                        skill_id: skill_id.to_string(),
                        target: Some(target),
                    });
                }
            } else {
                actions.push(Intent::UseMinionAbility {
                    minion: minion.instance_id,
                    skill_id: skill_id.to_string(),
                    target: None,
                });
            }
        }
    }

    if check_end_recruit(state, seat).is_ok() {
        actions.push(Intent::EndRecruit);
    }

    actions
}

/// 该座位在队列中最靠前的待选。
fn pending_for(state: &GameState, seat: Seat) -> Option<&PendingChoice> {
    state.pending.iter().find(|pending| match pending {
        PendingChoice::Discover {
            seat: pending_seat, ..
        }
        | PendingChoice::Evolve {
            seat: pending_seat, ..
        }
        | PendingChoice::Target {
            seat: pending_seat, ..
        } => *pending_seat == seat,
    })
}

/// 目标待选的合法候选，与 `choose_target` 的结算预检同源。
fn pending_target_candidates(
    engine: &Engine,
    state: &GameState,
    seat: Seat,
    source: InstanceId,
    hand_card: &HandCard,
) -> Vec<TargetRef> {
    let ctx = EffectCtx {
        seat,
        source: Some(source),
        target: None,
        event_target: None,
        golden: hand_card.golden(),
        level_override: None,
    };
    engine.target_candidates(
        state,
        &engine.effects_of(hand_card.card_id(), Trigger::OnPlay),
        &ctx,
        false,
    )
}
