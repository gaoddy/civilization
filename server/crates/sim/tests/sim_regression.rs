//! 启发式行为的特征化回归：整局驱动重构后，同一批种子的对局必须逐位不变。
//!
//! 2026-11-26 重定：手牌已满时发现不再登记待选（原先会让座位卡在无法消费的
//! 待选上），发现触发点与随机流消耗改变，seed 7 与 seed 11 的最终哈希与事件
//! 哈希随之改变。

#[test]
fn heuristic_hashes_are_unchanged() {
    let seed7 = sim::run_games(1, 7, 8).expect("seed 7");
    assert_eq!(seed7.games[0].final_hash, 0xca17_8c83_1ae4_e577);
    assert_eq!(seed7.games[0].event_hash, 0x9fd4_e247_6a82_9da0);

    let seed11 = sim::run_games(1, 11, 8).expect("seed 11");
    assert_eq!(seed11.games[0].final_hash, 0x0365_d351_0854_0a4c);
    assert_eq!(seed11.games[0].event_hash, 0x9497_6da5_867f_250c);
}
