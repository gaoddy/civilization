use rule_engine::state::{MinionStats, StatComponent};

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
