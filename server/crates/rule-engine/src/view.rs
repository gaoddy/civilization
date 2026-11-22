//! 逐座位视图：把招募侧只读预检包装成「这个座位现在能看到什么、能做什么」。
//!
//! 视图是纯函数产物：只读 [`Engine`] 与 [`GameState`]，不修改状态、不消耗随机流、
//! 不读时钟或环境。它只含公告信息——其他座位仅以 [`PlayerSummaryView`] 的公开字段
//! 出现，绝不包含对手的金币、手牌、商店、刷新次数或种子。
//!
//! **阶段门控由本模块负责**：`recruit::check_*` 只做意图内校验，不检查阶段、
//! `recruit_done`、`eliminated` 与 `target`。因此 [`view`] 先算
//! `activatable = 处于 Recruit 阶段且未出局`；不可操作时所有布尔值为 `false`，
//! 每槽位的 `disabled_reason` 统一为 [`RejectCode::InvalidStage`]。座位已结束招募
//! 时，除 `can_end_recruit` 外的布尔值一律为 `false`（重复 `EndRecruit` 幂等成功）。

use crate::combat;
use crate::data::Era;
use crate::effects::{EVOLVE_CHOICES, EffectCtx, Op, TargetRef, Trigger};
use crate::engine::Engine;
use crate::intent::RejectCode;
use crate::recruit::{
    check_buy, check_end_recruit, check_hero_power, check_play_card, check_reroll, check_sell,
    check_upgrade,
};
use crate::settle;
use crate::state::{
    GameState, MinionInstance, PendingChoice, Phase, PlayerState, Seat, StatComponent,
};

/// 一个座位当前可见的全部状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeatView {
    pub phase: Phase,
    pub turn: u8,
    pub seat: Seat,
    /// 幽灵战为幽灵归属座位；查询者本人就是幽灵归属者或尚未配对时为 `None`。
    pub opponent_seat: Option<Seat>,
    pub self_view: SelfView,
    /// 全部座位的公开摘要，按座位升序。
    pub players: Vec<PlayerSummaryView>,
    /// 本局启用的玩法 id，按枚举顺序。
    pub enabled_playstyles: Vec<String>,
}

/// 英雄的单个主动技能及其本回合可用性。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeroPowerView {
    pub skill_id: String,
    pub uses_left: u8,
    pub requires_target: bool,
    pub target_candidates: Vec<TargetRef>,
    pub can_use: bool,
    pub disabled_reason: Option<RejectCode>,
    /// 七下西洋剩余航行回合；无航程或非七下西洋为 `None`。
    pub voyage_remaining: Option<u8>,
    /// 七下西洋已累计、未到账的金币；无航程为 0。
    pub voyage_pending: u16,
}

/// 本座位自己的私密数据与可操作项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfView {
    pub hp: i32,
    pub era: u8,
    pub gold: u8,
    pub hero_card_id: Option<String>,
    /// 英雄卡上的全部主动技能（`skill.cost.is_some()`），按英雄卡 `skills` 顺序排列。
    pub hero_powers: Vec<HeroPowerView>,
    pub shop: Vec<ShopSlotView>,
    pub hand: Vec<HandCardView>,
    pub board: Vec<BoardMinionView>,
    pub reroll_cost: u8,
    pub upgrade_cost: Option<u16>,
    /// 本回合是否已结束招募。
    pub recruited: bool,
    /// 本座位待选队列中的第一条。
    pub pending_choices: Vec<PendingChoiceView>,
    pub affordances: Affordances,
}

/// 商店槽位的公开视图。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShopSlotView {
    pub index: u8,
    /// 卡牌实例编号；空槽为 `None`。
    pub instance_id: Option<u64>,
    pub card_id: Option<String>,
    pub frozen: bool,
}

/// 手牌的公开视图。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandCardView {
    pub index: u8,
    /// 卡牌实例编号；买入、上手、三连合成都沿用同一编号。
    pub instance_id: u64,
    pub card_id: String,
    /// 随从牌为金色三连结果；效果牌恒为 `false`。
    pub golden: bool,
    /// 随从牌的真实攻击力（已含成长等全部层数）；效果牌为 `None`。
    pub attack: Option<i32>,
    /// 随从牌的真实生命值；效果牌为 `None`。
    pub health: Option<i32>,
    /// 「三连奖励」牌锁定的发现星级；其它牌为 `None`。
    pub reward_level: Option<u8>,
    /// 该牌是否需要玩家指定单一目标（效果牌口径，随从牌恒为 `false`）。
    pub requires_target: bool,
    /// 该效果牌的合法目标候选；随从牌恒为空。
    pub target_candidates: Vec<TargetRef>,
}

/// 单项属性（攻击或生命）的分层构成视图。
///
/// `permanent` 已把可移除的成长与不可移除的永久增量合并；
/// `damage` 为层和与已夹非负的 `current` 之差，保证
/// `initial + permanent + turn + aura - damage == current`。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StatBreakdownView {
    pub initial: i32,
    pub permanent: i32,
    pub turn: i32,
    pub aura: i32,
    pub damage: i32,
    pub current: i32,
}

/// 随从攻击与生命的分层构成视图。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MinionStatsView {
    pub attack: StatBreakdownView,
    pub health: StatBreakdownView,
}

/// 棋盘随从的公开视图。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardMinionView {
    pub index: u8,
    pub minion_id: u64,
    pub card_id: String,
    pub golden: bool,
    pub attack: i32,
    pub health: i32,
    /// 攻击与生命的当前量与分层构成。
    pub stats: MinionStatsView,
    /// 当前生效的关键词，按字典序。
    pub keywords: Vec<String>,
    /// 当前是否仍处于潜行：携带 `stealth` 关键词且尚未被打破。
    pub stealth: bool,
}

/// 其他座位的公开摘要；不含任何私密数据。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerSummaryView {
    pub seat: Seat,
    pub hp: i32,
    pub era: u8,
    pub alive: bool,
    pub board_count: u8,
    pub hero_card_id: Option<String>,
    pub recruited: bool,
    /// 最终名次；未进入排名时为 `None`。
    pub placement: Option<u8>,
    /// 局内实时名次；对局结束后与 `placement` 一致。
    pub rank: Option<u8>,
}

/// 等待该座位做出的选择。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingChoiceView {
    Discover {
        candidates: Vec<String>,
    },
    Evolve {
        options: Vec<String>,
    },
    /// 战吼目标待选：合法的目标候选按「我方棋盘 → 手牌 → 商店」排列。
    Target {
        source_minion_id: u64,
        candidates: Vec<TargetRef>,
    },
}

/// 本座位各操作的可用性；`*_disabled_reason` 在可操作时为 `None`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Affordances {
    pub shop_slots: Vec<ShopAffordanceView>,
    pub hand_cards: Vec<HandAffordanceView>,
    pub board_minions: Vec<BoardAffordanceView>,
    pub can_reroll: bool,
    /// 冻结按钮是否处于「解冻」状态：本回合冻结过商店且仍有冻结槽位。
    pub can_unfreeze: bool,
    pub can_upgrade_era: bool,
    pub upgrade_disabled_reason: Option<RejectCode>,
    pub can_end_recruit: bool,
}

/// 单个商店槽位的可购买性。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShopAffordanceView {
    pub index: u8,
    pub buyable: bool,
    pub disabled_reason: Option<RejectCode>,
}

/// 单张手牌的可打出性。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandAffordanceView {
    pub index: u8,
    pub playable: bool,
    pub disabled_reason: Option<RejectCode>,
}

/// 单个棋盘随从的可出售性。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardAffordanceView {
    pub index: u8,
    pub sellable: bool,
    pub disabled_reason: Option<RejectCode>,
}

/// 构造指定座位的视图；座位不存在时返回 `None`。
///
/// 纯函数：不修改 `state` / `engine`，不推进随机流。
pub fn view(engine: &Engine, state: &GameState, seat: Seat) -> Option<SeatView> {
    let player = state.players.get(usize::from(seat))?;

    // 阶段门控集中在这里；`check_*` 只做意图内校验。
    let activatable = state.phase == Phase::Recruit && !player.eliminated;
    // 队首是目标待选时，该座位只能选定目标：其余操作一律按阶段不可用处理。
    let head_is_target = state
        .pending
        .iter()
        .find(|pending| pending_seat(pending) == seat)
        .is_some_and(|pending| matches!(pending, PendingChoice::Target { .. }));
    let has_pending = state
        .pending
        .iter()
        .any(|pending| pending_seat(pending) == seat);
    let gate = if activatable && !player.recruit_done && !head_is_target {
        None
    } else {
        Some(RejectCode::InvalidStage)
    };

    let shop_slots = (0..player.shop.len())
        .map(|index| {
            let index = u8::try_from(index).unwrap_or(u8::MAX);
            match gate {
                Some(code) => ShopAffordanceView {
                    index,
                    buyable: false,
                    disabled_reason: Some(code),
                },
                None => match check_buy(engine, state, seat, index) {
                    Ok(()) => ShopAffordanceView {
                        index,
                        buyable: true,
                        disabled_reason: None,
                    },
                    Err(reject) => ShopAffordanceView {
                        index,
                        buyable: false,
                        disabled_reason: Some(reject.code),
                    },
                },
            }
        })
        .collect();

    let board_minions = player
        .board
        .iter()
        .enumerate()
        .map(|(index, minion)| {
            let index = u8::try_from(index).unwrap_or(u8::MAX);
            let sellable = match gate {
                Some(code) => (false, Some(code)),
                None => match check_sell(state, seat, minion.instance_id) {
                    Ok(()) => (true, None),
                    Err(reject) => (false, Some(reject.code)),
                },
            };
            BoardAffordanceView {
                index,
                sellable: sellable.0,
                disabled_reason: sellable.1,
            }
        })
        .collect();

    let can_reroll = gate.is_none() && check_reroll(state, seat).is_ok();

    // 解冻权只在冻结的当回合有效；冻结标记跨回合保留，新回合按钮仍按「冻结」显示。
    let can_unfreeze = gate.is_none()
        && player.froze_this_turn
        && player.shop.iter().flatten().any(|slot| slot.frozen);

    let (can_upgrade_era, upgrade_disabled_reason) = match gate {
        Some(code) => (false, Some(code)),
        None => match check_upgrade(engine, state, seat) {
            Ok(()) => (true, None),
            Err(reject) => (false, Some(reject.code)),
        },
    };

    let hero_powers = player
        .hero
        .as_deref()
        .and_then(|hero_id| engine.catalog.heroes.get(hero_id))
        .map(|hero| {
            hero.skills
                .iter()
                .filter_map(|skill_id| {
                    let skill = engine.catalog.skill(*skill_id);
                    skill.cost.map(|_| skill)
                })
                .map(|skill| {
                    let skill_id = skill.id.as_str();
                    let limit = skill.uses_per_turn.unwrap_or(1);
                    let used = player.hero_uses.get(skill_id).copied().unwrap_or(0);
                    let uses_left = limit.saturating_sub(used);
                    let requires_target =
                        engine.skill_requires_target(skill_id, Trigger::OnActivate);
                    let target_candidates = if requires_target {
                        let ctx = EffectCtx {
                            seat,
                            source: None,
                            target: None,
                            event_target: None,
                            golden: false,
                            level_override: None,
                        };
                        let effects = engine.skill_effects_of(skill_id, Trigger::OnActivate);
                        engine.target_candidates(state, &effects, &ctx, false)
                    } else {
                        Vec::new()
                    };
                    let (can_use, disabled_reason) = match gate {
                        Some(code) => (false, Some(code)),
                        // 需要目标而候选为空：该技能没有合法落点，按不可使用处理。
                        None if requires_target && target_candidates.is_empty() => {
                            (false, Some(RejectCode::NoValidTarget))
                        }
                        None => match check_hero_power(engine, state, seat, skill_id) {
                            Ok(()) => (true, None),
                            Err(reject) => (false, Some(reject.code)),
                        },
                    };
                    let voyage = if engine
                        .skill_effects_of(skill_id, Trigger::OnActivate)
                        .iter()
                        .any(|effect| effect.op == Op::SevenVoyages)
                    {
                        player.seven_voyages
                    } else {
                        None
                    };
                    HeroPowerView {
                        skill_id: skill_id.to_string(),
                        uses_left,
                        requires_target,
                        target_candidates,
                        can_use,
                        disabled_reason,
                        voyage_remaining: voyage.map(|state| state.remaining),
                        voyage_pending: voyage.map_or(0, |state| state.pending),
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    // 已结束招募时重复 `EndRecruit` 幂等成功；出局、非招募阶段或仍有待选时不可操作。
    let can_end_recruit = activatable && !has_pending && check_end_recruit(state, seat).is_ok();

    let shop = player
        .shop
        .iter()
        .enumerate()
        .map(|(index, shop_slot)| ShopSlotView {
            index: u8::try_from(index).unwrap_or(u8::MAX),
            instance_id: shop_slot.as_ref().map(|slot| slot.instance_id),
            card_id: shop_slot.as_ref().map(|slot| slot.card_id.clone()),
            frozen: shop_slot.as_ref().is_some_and(|slot| slot.frozen),
        })
        .collect();

    let hand = player
        .hand
        .iter()
        .enumerate()
        .map(|(index, hand_card)| {
            let minion = hand_card.minion();
            let requires_target = minion.is_none()
                && engine
                    .requires_target(hand_card.card_id(), crate::effects::Trigger::OnPlayEffect);
            // 效果牌的合法目标与出牌预检同源：候选为空时该牌不可打出。
            let target_candidates = if requires_target {
                let ctx = EffectCtx {
                    seat,
                    source: None,
                    target: None,
                    event_target: None,
                    golden: false,
                    level_override: hand_card.reward_level(),
                };
                engine.target_candidates(
                    state,
                    &engine.effects_of(hand_card.card_id(), Trigger::OnPlayEffect),
                    &ctx,
                    true,
                )
            } else {
                Vec::new()
            };
            HandCardView {
                index: u8::try_from(index).unwrap_or(u8::MAX),
                instance_id: hand_card.instance_id(),
                card_id: hand_card.card_id().to_string(),
                golden: hand_card.golden(),
                attack: minion.map(|minion| minion.attack()),
                health: minion.map(|minion| minion.health()),
                reward_level: hand_card.reward_level(),
                requires_target,
                target_candidates,
            }
        })
        .collect::<Vec<_>>();

    let hand_cards = hand
        .iter()
        .map(|hand_card| {
            let index = hand_card.index;
            match gate {
                Some(code) => HandAffordanceView {
                    index,
                    playable: false,
                    disabled_reason: Some(code),
                },
                None => match check_play_card(engine, state, seat, index) {
                    // 需要目标而候选为空：效果无法落点，按不可打出处理。
                    Ok(())
                        if hand_card.requires_target && hand_card.target_candidates.is_empty() =>
                    {
                        HandAffordanceView {
                            index,
                            playable: false,
                            disabled_reason: Some(RejectCode::NoValidTarget),
                        }
                    }
                    Ok(()) => HandAffordanceView {
                        index,
                        playable: true,
                        disabled_reason: None,
                    },
                    Err(reject) => HandAffordanceView {
                        index,
                        playable: false,
                        disabled_reason: Some(reject.code),
                    },
                },
            }
        })
        .collect();

    // 棋盘数值与关键词必须先折叠光环，否则客户端看到的是未加成的数值。
    let folded_board = settle::fold_auras(&engine.catalog, &state.auras, &player.board);
    let board = folded_board
        .iter()
        .enumerate()
        .map(|(index, minion)| BoardMinionView {
            index: u8::try_from(index).unwrap_or(u8::MAX),
            minion_id: minion.instance_id,
            card_id: minion.card_id.clone(),
            golden: minion.golden,
            attack: combat::effective_attack(&engine.catalog, minion),
            health: combat::effective_health(&engine.catalog, minion),
            stats: stats_breakdown(minion),
            keywords: combat::keyword_set(&engine.catalog, minion).names(),
            stealth: combat::has_stealth(&engine.catalog, minion),
        })
        .collect();

    let pending_choices = state
        .pending
        .iter()
        .find(|pending| pending_seat(pending) == seat)
        .map(|pending| match pending {
            PendingChoice::Discover { candidates, .. } => PendingChoiceView::Discover {
                candidates: candidates.clone(),
            },
            PendingChoice::Evolve { .. } => PendingChoiceView::Evolve {
                options: EVOLVE_CHOICES
                    .iter()
                    .map(|option| (*option).to_string())
                    .collect(),
            },
            PendingChoice::Target {
                seat: pending_seat,
                source,
                hand_card,
                ..
            } => {
                let ctx = EffectCtx {
                    seat: *pending_seat,
                    source: Some(*source),
                    target: None,
                    event_target: None,
                    golden: hand_card.golden(),
                    level_override: None,
                };
                PendingChoiceView::Target {
                    source_minion_id: *source,
                    candidates: engine.target_candidates(
                        state,
                        &engine.effects_of(hand_card.card_id(), Trigger::OnPlay),
                        &ctx,
                        false,
                    ),
                }
            }
        })
        .into_iter()
        .collect();

    let players = state
        .players
        .iter()
        .map(|other| player_summary(state, other))
        .collect();

    let opponent_seat = state.pairings.iter().find_map(|pairing| {
        if pairing.a != seat && pairing.b != seat {
            return None;
        }
        // 幽灵归属者本人观战：不指向自己（仅防御异常构造；正常配对不会把
        // 幽灵归属者放进 `a`/`b`）。
        if pairing.ghost == Some(seat) {
            return None;
        }
        // 落单且没有可对战的幽灵：设计表格规定 `opponentPlayerId` 为 `null`。
        if pairing.ghost.is_none() && pairing.a == pairing.b {
            return None;
        }
        if let Some(ghost) = pairing.ghost {
            return Some(ghost);
        }
        Some(if pairing.a == seat {
            pairing.b
        } else {
            pairing.a
        })
    });

    Some(SeatView {
        phase: state.phase,
        turn: state.turn,
        seat,
        opponent_seat,
        self_view: SelfView {
            hp: player.hp,
            era: player.era as u8,
            gold: player.gold,
            hero_card_id: player.hero.clone(),
            hero_powers,
            shop,
            hand,
            board,
            reroll_cost: u8::from(player.first_refresh_used),
            upgrade_cost: engine
                .catalog
                .config
                .era_costs
                .get(era_index(player.era))
                .copied(),
            recruited: player.recruit_done,
            pending_choices,
            affordances: Affordances {
                shop_slots,
                hand_cards,
                board_minions,
                can_reroll,
                can_unfreeze,
                can_upgrade_era,
                upgrade_disabled_reason,
                can_end_recruit,
            },
        },
        players,
        enabled_playstyles: state
            .enabled_playstyles
            .iter()
            .map(|playstyle| playstyle.id().to_string())
            .collect(),
    })
}

/// 构造随从的分层构成视图。
///
/// `current` 先取 [`MinionInstance::attack`] / [`MinionInstance::health`]
/// （已夹非负），再反推 `damage`，因此招募期负血量会被 `damage` 吸收。
/// 服务层的协议映射复用它，保证规则引擎与服务端视图口径一致。
pub fn stats_breakdown(minion: &MinionInstance) -> MinionStatsView {
    MinionStatsView {
        attack: breakdown(&minion.stats.attack, minion.attack()),
        health: breakdown(&minion.stats.health, minion.health()),
    }
}

/// 单项属性的分层构成；`permanent` 合并成长与其他永久增量。
fn breakdown(stat: &StatComponent, current: i32) -> StatBreakdownView {
    let permanent = stat.growth.saturating_add(stat.permanent);
    StatBreakdownView {
        initial: stat.initial,
        permanent,
        turn: stat.turn,
        aura: stat.aura,
        damage: stat.layers().saturating_sub(current),
        current,
    }
}

/// `ranking` 下标 + 1；仅在 `GameOver` 阶段公开名次，其余阶段返回 `None`。
///
/// 对局进行中 `ranking` 只是「已出局者在前、存活者补齐在尾部」的临时块，
/// 下标 + 1 并不等于最终名次；协议要求结束前 `placement` 为 `null`。
pub fn placement_of(state: &GameState, seat: Seat) -> Option<u8> {
    if state.phase != Phase::GameOver {
        return None;
    }
    state
        .ranking
        .iter()
        .position(|ranked| *ranked == seat)
        .map(|index| u8::try_from(index + 1).unwrap_or(u8::MAX))
}

fn pending_seat(pending: &PendingChoice) -> Seat {
    match pending {
        PendingChoice::Discover { seat, .. }
        | PendingChoice::Evolve { seat, .. }
        | PendingChoice::Target { seat, .. } => *seat,
    }
}

fn player_summary(state: &GameState, player: &PlayerState) -> PlayerSummaryView {
    PlayerSummaryView {
        seat: player.seat,
        hp: player.hp,
        era: player.era as u8,
        alive: !player.eliminated,
        board_count: u8::try_from(player.board.len()).unwrap_or(u8::MAX),
        hero_card_id: player.hero.clone(),
        recruited: player.recruit_done,
        placement: placement_of(state, player.seat),
        rank: settle::rank_of(state, player.seat),
    }
}

/// 时代在 `config.era_costs` 中的下标；未来时代为 6，超出费用表。
fn era_index(era: Era) -> usize {
    (era as usize).saturating_sub(1)
}
