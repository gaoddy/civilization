//! 名次奖励公式：第 1 名 1.0、第 8 名 0.0，随名次线性递减。

use ai_train::env::{PLAYER_COUNT, placement_reward};

#[test]
fn placement_reward_is_one_for_first_and_zero_for_last() {
    assert!((placement_reward(1) - 1.0).abs() < 1e-6);
    assert!(placement_reward(PLAYER_COUNT as u8).abs() < 1e-6);
}

#[test]
fn placement_reward_decreases_by_one_step_per_place() {
    let step = 1.0 / (PLAYER_COUNT as f32 - 1.0);
    for place in 1..PLAYER_COUNT as u8 {
        let gap = placement_reward(place) - placement_reward(place + 1);
        assert!(
            (gap - step).abs() < 1e-6,
            "第 {place} 与第 {} 名的奖励差应为 {step}，实际 {gap}",
            place + 1
        );
    }
}
