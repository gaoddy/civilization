mod common;

use common::test_catalog;
use rule_engine::engine::Engine;
use rule_engine::state::{MinionInstance, Phase};

/// 卡面 3/4 的普通随从。
const FOUR: &str = "jianchihu";
/// 卡面 0/5 的学者随从。
const SCHOLAR: &str = "euclid";

#[test]
fn new_resolves_card_stats_and_golden_doubles() {
    let catalog = test_catalog();
    let plain = MinionInstance::new(&catalog, 1, FOUR, false);
    assert_eq!(
        (plain.stats.attack.current, plain.stats.health.current),
        (3, 4)
    );
    assert_eq!((plain.attack(), plain.health()), (3, 4));
    assert!(!plain.is_scholar);

    let golden = MinionInstance::new(&catalog, 2, FOUR, true);
    assert_eq!(
        (golden.stats.attack.initial, golden.stats.health.initial),
        (6, 8)
    );
    assert_eq!((golden.attack(), golden.health()), (6, 8));
}

#[test]
fn scholar_attack_projects_zero_while_layers_accumulate() {
    let catalog = test_catalog();
    let mut scholar = MinionInstance::new(&catalog, 1, SCHOLAR, false);
    assert!(scholar.is_scholar);
    scholar.stats.attack.add_permanent(1);
    assert_eq!(scholar.stats.attack.current, 1);
    assert_eq!(scholar.attack(), 0);
    assert_eq!(scholar.health(), 5);
}

#[test]
fn growth_layers_track_attack_and_health() {
    let catalog = test_catalog();
    let mut minion = MinionInstance::new(&catalog, 1, FOUR, false);

    // 普通成长：攻血各 +1。
    minion.stats.attack.add_growth(1);
    minion.stats.health.add_growth(1);
    assert_eq!((minion.attack(), minion.health()), (4, 5));

    // 只加生命的成长（原 `growth_health_only`）只进入生命层。
    minion.stats.health.add_growth(2);
    assert_eq!((minion.attack(), minion.health()), (4, 7));
    assert_eq!(minion.stats.health.growth, 3);
}

#[test]
fn remove_growth_keeps_permanent() {
    let catalog = test_catalog();
    let mut minion = MinionInstance::new(&catalog, 1, FOUR, false);
    minion.stats.attack.add_growth(2);
    minion.stats.attack.add_permanent(3);
    minion.stats.health.add_growth(1);
    minion.stats.health.add_permanent(1);

    minion.stats.attack.remove_growth();
    minion.stats.health.remove_growth();

    assert_eq!(minion.stats.attack.growth, 0);
    assert_eq!(minion.stats.health.growth, 0);
    assert_eq!(minion.stats.attack.permanent, 3);
    assert_eq!(minion.stats.health.permanent, 1);
    assert_eq!((minion.attack(), minion.health()), (6, 5));
}

#[test]
fn clear_turn_only_clears_turn_layer() {
    let catalog = test_catalog();
    let mut minion = MinionInstance::new(&catalog, 1, FOUR, false);
    minion.stats.attack.add_turn(2);
    minion.stats.health.add_turn(-1);
    minion.stats.health.add_permanent(1);

    minion.stats.attack.clear_turn();
    minion.stats.health.clear_turn();

    assert_eq!(minion.stats.attack.turn, 0);
    assert_eq!(minion.stats.health.turn, 0);
    assert_eq!((minion.attack(), minion.health()), (3, 5));
}

#[test]
fn apply_damage_only_moves_current() {
    let catalog = test_catalog();
    let mut minion = MinionInstance::new(&catalog, 1, FOUR, false);
    minion.stats.health.apply_damage(3);

    assert_eq!(minion.stats.health.current, 1);
    assert_eq!(minion.stats.health.layers(), 4);
    assert_eq!(minion.stats.health.damage(), 3);
    assert_eq!(minion.health(), 1);

    minion.stats.health.apply_damage(3);
    assert_eq!(minion.stats.health.current, -2);
    assert_eq!(minion.health(), 0);
}

#[test]
fn revive_to_may_exceed_layers() {
    let catalog = test_catalog();
    let mut minion = MinionInstance::new(&catalog, 1, FOUR, false);
    minion.stats.health.add_turn(-6);
    minion.stats.health.revive_to(1);

    assert_eq!(minion.stats.health.current, 1);
    assert_eq!(minion.stats.health.layers(), -2);
    assert_eq!(minion.health(), 1);
}

#[test]
fn contagion_writes_negative_health_permanent() {
    let catalog = test_catalog();
    let (mut engine, mut state) = Engine::new(catalog, [7; 32]);
    state.phase = Phase::Recruit;
    state.turn = 1;
    for player in &mut state.players {
        player.hp = 30;
    }
    let mut minion = MinionInstance::new(&engine.catalog, 1, FOUR, false);
    minion.contagion = 2;
    state.players[0].board.push(minion);

    engine.start_turn(&mut state).expect("开始回合应成功");

    let minion = &state.players[0].board[0];
    assert_eq!(minion.stats.health.permanent, -2);
    assert_eq!(minion.health(), 2);

    let mut clone = minion.clone();
    clone.stats.health.add_growth(5);
    clone.stats.health.remove_growth();
    assert_eq!(clone.stats.health.permanent, -2, "移除成长不影响传染层");
}

#[test]
fn stats_view_accessors_expose_layers() {
    let catalog = test_catalog();
    let mut minion = MinionInstance::new(&catalog, 1, FOUR, false);
    minion.stats_view_mut().attack.add_permanent(2);
    assert_eq!(minion.stats_view().attack.permanent, 2);
    assert_eq!(minion.stats_view().attack.current, 5);
}
