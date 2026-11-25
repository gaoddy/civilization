//! 机器人策略开关：坏模型路径回退启发式、默认行为不变、神经决策延迟门禁。

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use room_service::clock::{Clock, FixedClock};
use room_service::envelope::SeatChannel;
use room_service::game::{GameStage, GameTask, RoomCommand, SeatUpdate, UpdateKind};
use room_service::room::PlayerSlot;
use rule_engine::data::{Catalog, Playstyle};
use rule_engine::state::Seat;

const CARDS_JSON: &str = include_str!("../../../data/v0.1/cards.json");
const HEROES_JSON: &str = include_str!("../../../data/v0.1/heroes.json");
const SKILLS_JSON: &str = include_str!("../../../data/v0.1/skills.json");
const CONFIG_JSON: &str = include_str!("../../../data/v0.1/config.json");
const TAXONOMY_JSON: &str = include_str!("../../../data/v0.1/taxonomy.json");

const HEROES: [&str; 8] = [
    "adam-smith",
    "darwin",
    "davinci",
    "edison",
    "einstein",
    "faraday",
    "galileo",
    "knuth",
];

const SEED: u64 = 7;

/// 环境变量是进程级共享状态，两个改环境的测试必须串行。
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn catalog() -> Catalog {
    Catalog::from_json(
        CARDS_JSON,
        HEROES_JSON,
        SKILLS_JSON,
        CONFIG_JSON,
        TAXONOMY_JSON,
    )
    .expect("卡表应能加载")
}

fn playstyles() -> [Playstyle; 5] {
    [
        Playstyle::Mammal,
        Playstyle::Bird,
        Playstyle::Plant,
        Playstyle::Virus,
        Playstyle::Machine,
    ]
}

fn candidates() -> BTreeMap<Seat, Vec<String>> {
    (0..8)
        .map(|seat| {
            (
                seat as Seat,
                (0..4)
                    .map(|offset| HEROES[(seat + offset) % HEROES.len()].to_string())
                    .collect(),
            )
        })
        .collect()
}

/// 8 个座位全是机器人，从选英雄阶段开始。
fn all_bot_task() -> (GameTask, FixedClock) {
    let clock = FixedClock::new(1_000_000);
    let players: Vec<PlayerSlot> = (0..8)
        .map(|seat| PlayerSlot {
            player_id: format!("p{seat}"),
            display_name: format!("玩家{seat}"),
            ready: true,
            is_bot: true,
            channel: SeatChannel::new(),
            connected: true,
            disconnected_at_ms: None,
        })
        .collect();
    let task = GameTask::new_picking(
        catalog(),
        SEED,
        playstyles(),
        candidates(),
        players,
        Arc::new(clock.clone()) as Arc<dyn Clock>,
        Arc::new(room_service::AchievementCatalog::embedded()),
    )
    .expect("选英雄阶段应能构造");
    (task.with_room_id("room_neural"), clock)
}

fn count_game_started(updates: &[SeatUpdate]) -> usize {
    updates
        .iter()
        .filter(|update| matches!(update.kind, UpdateKind::GameStarted))
        .count()
}

fn play_combat(task: &mut GameTask) -> Vec<SeatUpdate> {
    let mut updates = Vec::new();
    for seat in 0..8 {
        updates.extend(task.handle(RoomCommand::CombatPlayed {
            player_id: format!("p{seat}"),
        }));
    }
    updates
}

/// 打到出结果，返回最终名次列表；超过 30 回合视为异常。
fn drive_to_end(task: &mut GameTask, clock: &FixedClock) -> Vec<(String, u8)> {
    for _ in 0..30 {
        clock.advance(150_000);
        let mut updates = task.tick();
        updates.extend(play_combat(task));
        if updates
            .iter()
            .any(|update| matches!(update.kind, UpdateKind::GameOver(_)))
        {
            return task.placements();
        }
    }
    panic!("30 个回合内应结束对局");
}

fn set_env(policy: Option<&str>, model_dir: Option<&str>) {
    unsafe {
        match policy {
            Some(value) => std::env::set_var("BOT_POLICY", value),
            None => std::env::remove_var("BOT_POLICY"),
        }
        match model_dir {
            Some(value) => std::env::set_var("BOT_MODEL_DIR", value),
            None => std::env::remove_var("BOT_MODEL_DIR"),
        }
    }
}

#[test]
fn bad_model_path_falls_back_to_heuristic() {
    let _guard = ENV_LOCK.lock().expect("环境锁");
    set_env(Some("neural"), Some("/nonexistent/bot-model"));

    let (mut task, clock) = all_bot_task();
    task.apply_env_policy();
    clock.advance(30_000);
    let updates = task.tick();

    set_env(None, None);

    assert_eq!(task.stage(), GameStage::Playing, "回退后对局照常推进");
    assert_eq!(count_game_started(&updates), 8, "8 个机器人都应开局");
}

#[test]
fn heuristic_default_is_unchanged() {
    let _guard = ENV_LOCK.lock().expect("环境锁");
    set_env(None, None);
    let (mut plain, plain_clock) = all_bot_task();

    set_env(Some("heuristic"), None);
    let (mut explicit, explicit_clock) = all_bot_task();
    explicit.apply_env_policy();
    set_env(None, None);

    assert_eq!(
        drive_to_end(&mut plain, &plain_clock),
        drive_to_end(&mut explicit, &explicit_clock),
        "显式 heuristic 与默认策略必须打出完全相同的一局"
    );
}

#[test]
fn neural_decision_latency_p99_under_20ms() {
    let catalog = catalog();
    let (engine, state) = started_game(&catalog);
    let policy = bot_ai::neural::NeuralPolicy::from_weights(zero_weights(&catalog), &catalog)
        .expect("零权重策略");
    use bot_ai::Policy;

    let mut samples = Vec::with_capacity(1000);
    for _ in 0..1000 {
        let start = Instant::now();
        let _ = policy.plan(&state, 0, &engine, &[]);
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    samples.sort_by(f64::total_cmp);
    let p50 = samples[samples.len() / 2];
    let p99 = samples[samples.len() * 99 / 100];
    println!("NeuralPolicy 决策延迟：p50 {p50:.2}ms，p99 {p99:.2}ms");
    // 生产为 release 构建；debug 下前向慢 20 倍以上，只打印不设门禁。
    if !cfg!(debug_assertions) {
        assert!(p99 < 20.0, "p99 {p99:.2}ms 超过 20ms");
    }
}

fn started_game(catalog: &Catalog) -> (rule_engine::engine::Engine, rule_engine::state::GameState) {
    let engine = rule_engine::engine::Engine::from_catalog(catalog.clone());
    let mut state = engine.initial_state(rule_engine::rng::seed_from_u64(SEED), playstyles());
    let heroes: BTreeMap<Seat, String> = HEROES
        .iter()
        .enumerate()
        .map(|(seat, hero)| (seat as Seat, (*hero).to_string()))
        .collect();
    engine.start_game(&mut state, &heroes).expect("开局");
    (engine, state)
}

/// 全零权重：logits 全 0，argmax 取第一个合法动作；延迟测量与数值无关。
fn zero_weights(catalog: &Catalog) -> ai_model::Weights {
    use ai_model::ModelConfig;
    let cfg = ModelConfig {
        obs_dim: ai_core::OBS_DIM,
        card_emb: 16,
        hidden: 64,
        layers: 2,
    };
    let index = ai_core::CardIndex::build(catalog);
    let hidden = cfg.hidden;
    let mut tensors = BTreeMap::new();
    let mut put = |name: &str, len: usize| {
        tensors.insert(name.to_string(), vec![0.0f32; len]);
    };
    put("trunk.0.weight", hidden * cfg.obs_dim);
    put("trunk.0.bias", hidden);
    put("trunk.1.weight", hidden * hidden);
    put("trunk.1.bias", hidden);
    put("value.0.weight", hidden * hidden);
    put("value.0.bias", hidden);
    put("value.1.weight", hidden);
    put("value.1.bias", 1);
    put("cand.0.weight", hidden * cfg.candidate_input());
    put("cand.0.bias", hidden);
    put("cand.1.weight", hidden);
    put("cand.1.bias", 1);
    put("card.emb", index.card_count() * cfg.card_emb);
    put("hero.emb", index.hero_count() * cfg.card_emb);
    put("skill.emb", index.skill_count() * cfg.card_emb);
    ai_model::Weights {
        tensors,
        card_count: index.card_count(),
        hero_count: index.hero_count(),
        skill_count: index.skill_count(),
    }
}
