use std::collections::BTreeMap;
use std::time::Instant;

use ai_train::env::Env;
use ai_train::random_policy::RandomPolicy;
use rule_engine::data::Catalog;

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

fn heroes(catalog: &Catalog) -> BTreeMap<u8, String> {
    catalog
        .heroes
        .values()
        .take(8)
        .enumerate()
        .map(|(index, hero)| (index as u8, hero.id.clone()))
        .collect()
}

/// 用随机策略跑完一局，返回终局状态哈希与名次。
fn play_one(env: &mut Env, heroes: &BTreeMap<u8, String>, seed: u64) -> (u64, Vec<u8>) {
    env.reset(seed, heroes).expect("重置");
    let mut policy = RandomPolicy::new(seed);
    let mut steps = 0usize;
    while let Some(seat) = env.current_seat() {
        let actions = env.legal(seat);
        assert!(!actions.is_empty(), "座位 {seat} 没有合法动作");
        let index = policy.below(actions.len() as u64) as usize;
        env.step(actions[index].clone())
            .expect("合法动作必须被接受");
        steps += 1;
        assert!(steps < 20_000, "步数超限，疑似死循环");
    }
    let mut ranking: Vec<u8> = (0..8).filter_map(|seat| env.placement(seat)).collect();
    ranking.sort_unstable();
    (env.state_hash(), ranking)
}

#[test]
fn random_policy_plays_full_games() {
    let catalog = catalog();
    let heroes = heroes(&catalog);
    let mut env = Env::new(&catalog);
    for seed in 0..20u64 {
        let (_, ranking) = play_one(&mut env, &heroes, seed);
        assert_eq!(ranking, vec![1, 2, 3, 4, 5, 6, 7, 8], "种子 {seed}");
    }
}

#[test]
fn env_is_deterministic() {
    let catalog = catalog();
    let heroes = heroes(&catalog);
    let mut env = Env::new(&catalog);
    let first = play_one(&mut env, &heroes, 42);
    let second = play_one(&mut env, &heroes, 42);
    assert_eq!(first, second);
}

#[test]
fn throughput_is_measured() {
    let catalog = catalog();
    let heroes = heroes(&catalog);
    let mut env = Env::new(&catalog);
    let games = 20u64;
    let start = Instant::now();
    for seed in 100..100 + games {
        play_one(&mut env, &heroes, seed);
    }
    let elapsed = start.elapsed().as_secs_f64();
    let per_second = games as f64 / elapsed;
    println!("随机策略吞吐：{per_second:.1} 局/秒");
    // 墙钟吞吐在 `make check` 的并行门禁里会被其它目标挤占（8 路并行时 debug 实测
    // 1.5 局/秒，串行 release 实测 29.6 局/秒），debug 下只打印。release 下限按串行
    // 实测的一半取值，并行时仍有约 2 倍余量。
    if !cfg!(debug_assertions) {
        assert!(
            per_second >= 10.0,
            "吞吐过低：{per_second:.1} 局/秒（release 下限 10）"
        );
    }
}
