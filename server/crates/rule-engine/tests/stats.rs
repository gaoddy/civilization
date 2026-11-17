mod common;

use common::{test_catalog, test_playstyles};
use rule_engine::engine::Engine;
use rule_engine::state::{MinionInstance, MinionStats, Phase, StatComponent};

/// 卡面 3/4 的普通随从。
const FOUR: &str = "jianchihu";
/// 卡面 0/5 的学者随从。
const SCHOLAR: &str = "euclid";

#[test]
fn new_sets_current_to_initial() {
    let stat = StatComponent::new(3);
    assert_eq!(stat.current, 3);
    assert_eq!(stat.layers(), 3);
    assert_eq!(stat.damage(), 0);
}

#[test]
fn layer_additions_track_current() {
    let mut stat = StatComponent::new(2);
    stat.add_growth(1);
    stat.add_permanent(2);
    stat.add_turn(3);
    stat.set_aura(4);
    assert_eq!(
        (stat.growth, stat.permanent, stat.turn, stat.aura),
        (1, 2, 3, 4)
    );
    assert_eq!(stat.current, 12);
    assert_eq!(stat.layers(), 12);
    assert_eq!(stat.damage(), 0);
}

#[test]
fn remove_growth_keeps_permanent() {
    let mut stat = StatComponent::new(2);
    stat.add_growth(3);
    stat.add_permanent(2);
    stat.remove_growth();
    assert_eq!(stat.growth, 0);
    assert_eq!(stat.permanent, 2);
    assert_eq!(stat.current, 4);
}

#[test]
fn clear_turn_keeps_permanent() {
    let mut stat = StatComponent::new(2);
    stat.add_turn(-3);
    stat.add_permanent(1);
    stat.clear_turn();
    assert_eq!(stat.turn, 0);
    assert_eq!(stat.current, 3);
}

#[test]
fn set_initial_applies_delta() {
    let mut stat = StatComponent::new(2);
    stat.add_growth(1);
    stat.set_initial(4);
    assert_eq!(stat.initial, 4);
    assert_eq!(stat.current, 5);
}

#[test]
fn apply_damage_only_reduces_current() {
    let mut stat = StatComponent::new(5);
    stat.apply_damage(3);
    assert_eq!(stat.current, 2);
    assert_eq!(stat.layers(), 5);
    assert_eq!(stat.damage(), 3);
}

#[test]
fn apply_damage_can_go_negative() {
    let mut stat = StatComponent::new(2);
    stat.apply_damage(5);
    assert_eq!(stat.current, -3);
    assert_eq!(stat.current_non_negative(), 0);
    assert_eq!(stat.damage(), 5);
}

#[test]
fn apply_damage_ignores_negative_amount() {
    // 负伤害不是治疗：只允许经 `revive_to` 调整当前值。
    let mut stat = StatComponent::new(5);
    stat.apply_damage(-3);
    assert_eq!(stat.current, 5);
    assert_eq!(stat.layers(), 5);
    assert_eq!(stat.damage(), 0);
}

#[test]
fn set_aura_decrease_lowers_current() {
    // 光环按差值更新：光环变小（或消失）时当前值同步回落。
    let mut stat = StatComponent::new(2);
    stat.set_aura(4);
    stat.set_aura(1);
    assert_eq!(stat.aura, 1);
    assert_eq!(stat.current, 3);
    assert_eq!(stat.damage(), 0);
    stat.set_aura(0);
    assert_eq!(stat.current, 2);
    assert_eq!(stat.damage(), 0);
}

#[test]
fn revive_to_may_exceed_layers() {
    let mut stat = StatComponent::new(4);
    stat.add_turn(-6);
    stat.revive_to(1);
    assert_eq!(stat.current, 1);
    assert_eq!(stat.layers(), -2);
    assert_eq!(stat.damage(), -3);
}

#[test]
fn minion_stats_pairs_attack_and_health() {
    let stats = MinionStats::new(2, 5);
    assert_eq!(stats.attack.current, 2);
    assert_eq!(stats.health.current, 5);
}

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
fn contagion_writes_negative_health_permanent() {
    let catalog = test_catalog();
    let (mut engine, mut state) = Engine::new(catalog, [7; 32], test_playstyles());
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
