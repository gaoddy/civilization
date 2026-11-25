//! 启发式行为的特征化回归：整局驱动重构后，同一批种子的对局必须逐位不变。

#[test]
fn heuristic_hashes_are_unchanged() {
    let seed7 = sim::run_games(1, 7, 8).expect("seed 7");
    assert_eq!(seed7.games[0].final_hash, 0xc6c7_b16b_0487_2047);
    assert_eq!(seed7.games[0].event_hash, 0x3198_d209_9e25_7c09);

    let seed11 = sim::run_games(1, 11, 8).expect("seed 11");
    assert_eq!(seed11.games[0].final_hash, 0xb021_a40b_d552_06e1);
    assert_eq!(seed11.games[0].event_hash, 0x0699_1da2_4b61_da2c);
}
