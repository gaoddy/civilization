//! C1 单座位构筑环境：阶段、血量与奖励来源。

use ai_core::{Action, CardIndex};
use ai_model::{ModelConfig, Weights};
use ai_train::data_c0::BoardSample;
use ai_train::env_recruit::{RecruitEnv, RewardMode};
use ai_train::random_policy::RandomPolicy;
use rule_engine::data::Catalog;
use rule_engine::state::{MinionInstance, MinionStats, Phase};

fn catalog() -> Catalog {
    Catalog::from_json(
        include_str!("../../../data/v0.1/cards.json"),
        include_str!("../../../data/v0.1/heroes.json"),
        include_str!("../../../data/v0.1/skills.json"),
        include_str!("../../../data/v0.1/config.json"),
        include_str!("../../../data/v0.1/taxonomy.json"),
    )
    .expect("卡表")
}

fn minion_card(catalog: &Catalog) -> String {
    catalog
        .cards
        .values()
        .find(|card| card.in_shop_pool && card.attack.is_some())
        .expect("有随从卡")
        .id
        .clone()
}

fn hero_id(catalog: &Catalog) -> String {
    catalog.heroes.keys().next().expect("有英雄").clone()
}

fn opponent_board(catalog: &Catalog) -> BoardSample {
    let card = minion_card(catalog);
    let board = (0..3)
        .map(|index| {
            let mut minion = MinionInstance::new(catalog, 1001 + index, &card, false);
            minion.stats = MinionStats::new(9, 9);
            minion
        })
        .collect();
    BoardSample {
        hero: hero_id(catalog),
        playstyles: 1,
        board,
    }
}

fn model_config() -> ModelConfig {
    ModelConfig {
        obs_dim: ai_core::OBS_DIM,
        card_emb: 16,
        hidden: 64,
        layers: 2,
    }
}

/// 随机打法跑若干回合：阶段永远不进 Settle，谁都不会被淘汰，血量不变。
#[test]
fn recruit_env_never_enters_settle_or_elimination() {
    let catalog = catalog();
    let opponent = opponent_board(&catalog);
    let mut env = RecruitEnv::new(&catalog, None, model_config());
    env.set_reward_mode(RewardMode::Combat);
    env.reset(5, &hero_id(&catalog), &opponent).expect("重置");

    let hp_before = env.hp();
    let mut rng = RandomPolicy::new(2026);
    let mut done = false;
    let mut steps = 0;
    while !done && steps < 4000 {
        let legal = env.legal();
        assert!(!legal.is_empty(), "回合 {} 没有合法动作", env.turn());
        let action = legal[rng.below(legal.len() as u64) as usize].clone();
        let stepped = env.step(action).expect("合法动作不应被拒绝");
        if stepped.done {
            assert_eq!(env.phase(), Phase::GameOver, "结束时阶段应为 GameOver");
        } else {
            assert_eq!(
                env.phase(),
                Phase::Recruit,
                "回合 {} 结束后阶段停在 {:?}",
                env.turn(),
                env.phase()
            );
        }
        if stepped.combat.is_some() {
            assert!(stepped.reward.is_finite());
        }
        done = stepped.done;
        steps += 1;
    }

    assert!(steps < 4000, "随机打法没有推进到对局结束");
    assert!(env.turn() >= 3, "只推进到回合 {}", env.turn());
    assert_eq!(env.hp(), hp_before, "学习座位血量变化");
    assert!(
        !env.state().players[0].eliminated,
        "学习座位在 C1 中不应被淘汰"
    );
    assert!(
        env.state().players[1..]
            .iter()
            .all(|player| player.eliminated),
        "其余座位应保持出局，牌池只由学习座位消耗"
    );
}

/// 战斗随机数只来自传入种子：多消耗游戏随机数不改变战斗结果与奖励。
#[test]
fn reward_uses_private_rng_not_game_rng() {
    let catalog = catalog();
    let opponent = opponent_board(&catalog);
    let hero = hero_id(&catalog);
    let mut mismatches = Vec::new();
    for seed in 0..8u64 {
        let mut fresh = RecruitEnv::new(&catalog, None, model_config());
        fresh.set_reward_mode(RewardMode::Combat);
        fresh.reset(seed, &hero, &opponent).expect("重置");
        let end = end_action(&fresh);
        let first = fresh.step(end).expect("结束招募");

        let mut rerolled = RecruitEnv::new(&catalog, None, model_config());
        rerolled.set_reward_mode(RewardMode::Combat);
        rerolled.reset(seed, &hero, &opponent).expect("重置");
        let reroll = rerolled
            .legal()
            .into_iter()
            .find(|action| matches!(action, Action::Reroll))
            .expect("第 1 回合可以重抽");
        rerolled.step(reroll).expect("重抽");
        let end = end_action(&rerolled);
        let second = rerolled.step(end).expect("结束招募");

        if first.combat != second.combat || first.reward != second.reward {
            mismatches.push((seed, first.combat, second.combat));
        }
    }
    assert!(
        mismatches.is_empty(),
        "战斗结果依赖了游戏随机数：{mismatches:?}"
    );
}

fn end_action(env: &RecruitEnv) -> Action {
    env.legal()
        .into_iter()
        .find(|action| matches!(action, Action::End))
        .expect("招募阶段总可以结束招募")
}

/// 观测与 C2 同布局：学习座位有英雄与棋盘槽位，其余座位不参与观测。
#[test]
fn observation_carries_learning_seat_hero_and_board() {
    let catalog = catalog();
    let index = CardIndex::build(&catalog);
    let opponent = opponent_board(&catalog);
    let hero = hero_id(&catalog);
    let mut env = RecruitEnv::new(&catalog, None, model_config());
    env.reset(3, &hero, &opponent).expect("重置");
    let obs = env.observe();
    assert_eq!(obs.seat, 0);
    assert_eq!(obs.hero, index.hero(&hero));
    assert_eq!(obs.phase, 1);
    assert_eq!(obs.board.iter().filter(|slot| slot.valid > 0.0).count(), 0);
    assert!(
        env.legal()
            .iter()
            .any(|action| matches!(action, Action::End))
    );
}

fn zero_weights(model: &ModelConfig) -> Weights {
    let mut weights = Weights::default();
    let mut width = model.obs_dim;
    for layer in 0..model.layers {
        weights.set(
            &format!("trunk.{layer}.weight"),
            vec![0.0; model.hidden * width],
        );
        weights.set(&format!("trunk.{layer}.bias"), vec![0.0; model.hidden]);
        width = model.hidden;
    }
    weights.set("value.0.weight", vec![0.0; model.hidden * model.hidden]);
    weights.set("value.0.bias", vec![0.0; model.hidden]);
    weights.set("value.1.weight", vec![0.0; model.hidden]);
    weights.set("value.1.bias", vec![0.0]);
    weights
}

/// 价值奖励走 C0 权重：全零权重下双方强度都是 0，奖励必须为 0 且有限。
#[test]
fn value_reward_runs_c0_board_strength() {
    let catalog = catalog();
    let model = model_config();
    let opponent = opponent_board(&catalog);
    let mut env = RecruitEnv::new(&catalog, Some(zero_weights(&model)), model);
    env.set_reward_mode(RewardMode::Value);
    env.reset(13, &hero_id(&catalog), &opponent).expect("重置");
    let end = end_action(&env);
    let stepped = env.step(end).expect("结束招募");
    assert!(stepped.reward.is_finite());
    assert_eq!(stepped.reward, 0.0);
}
