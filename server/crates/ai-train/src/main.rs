//! `ai-train` 命令行入口。
//!
//! 子命令与训练阶段一一对应；`data-c0` 只依赖规则引擎与 `sim`，
//! 其余子命令在 `train` feature 下编译。

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use ai_train::data_c0::{BoardPair, BoardSample, generate_pairs};
use rule_engine::data::Catalog;
use serde::Serialize;

/// C0 数据文件的 schema 版本；字段变更时递增。
const DATA_SCHEMA_VERSION: u32 = 1;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("data-c0") => data_c0(&args[1..]),
        Some("train-c0") => train_c0(&args[1..]),
        Some("train-c1") => train_c1(&args[1..]),
        Some("train-c2") => train_c2(&args[1..]),
        Some("eval") => eval(&args[1..]),
        Some(other) => Err(format!("未知子命令：{other}")),
        None => Err("缺少子命令".to_string()),
    };
    if let Err(message) = result {
        eprintln!("{message}");
        eprintln!(
            "用法：ai-train data-c0 --out <路径> [--games 2000] [--seed 0]\n      ai-train train-c0 [--pairs 路径] [--out-dir 目录] [--steps 20000] [--seed 0]\n      ai-train train-c1 [--pairs 路径] [--c0 权重路径] [--out-dir 目录] [--episodes 200] [--distribution 200] [--seed 0]\n      ai-train train-c2 [--pairs 路径] [--c0 权重路径] [--c1 权重路径] [--out-dir 目录] [--episodes 1000] [--c1-mix 0.3] [--distribution 200] [--seed 0]\n      ai-train eval [--dir 目录] [--seeds 2000] [--learner-seats 8] [--anchor 权重路径]"
        );
        std::process::exit(2);
    }
}

/// 仓库根目录：`server/crates/ai-train` 往上三级。
///
/// 只在训练子命令里用到；`data-c0` 的产物路径由调用方显式给出。
///
/// `make` 目标在 `server/` 下执行，产物目录统一按仓库根解析，避免把
/// `output/` 写到 `server/` 里。
fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("仓库根应存在")
        .to_path_buf()
}

fn data_c0(args: &[String]) -> Result<(), String> {
    let mut out = None;
    let mut games = 2000usize;
    let mut seed = 0u64;
    let mut iter = args.iter();
    while let Some(flag) = iter.next() {
        let value = iter
            .next()
            .ok_or_else(|| format!("{flag} 缺少取值"))?
            .clone();
        match flag.as_str() {
            "--out" => out = Some(value),
            "--games" => {
                games = value
                    .parse()
                    .map_err(|_| format!("--games 不是整数：{value}"))?;
            }
            "--seed" => {
                seed = value
                    .parse()
                    .map_err(|_| format!("--seed 不是整数：{value}"))?;
            }
            other => return Err(format!("未知参数：{other}")),
        }
    }
    let out = out.ok_or("缺少 --out")?;
    let catalog = catalog();
    let pairs = generate_pairs(&catalog, games, seed);
    write_pairs(Path::new(&out), &catalog, games, seed, &pairs)
}

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

#[derive(Serialize)]
struct Header<'a> {
    kind: &'a str,
    schema: u32,
    catalog_hash: String,
    games: usize,
    seed: u64,
    pairs: usize,
}

#[derive(Serialize)]
struct SampleJson {
    hero: String,
    playstyles: u16,
    board: Vec<MinionJson>,
}

#[derive(Serialize)]
struct MinionJson {
    card: String,
    golden: bool,
    attack: i32,
    health: i32,
}

#[derive(Serialize)]
struct PairJson {
    a: SampleJson,
    b: SampleJson,
    a_wins: bool,
}

impl SampleJson {
    fn from_sample(sample: &BoardSample) -> Self {
        Self {
            hero: sample.hero.clone(),
            playstyles: sample.playstyles,
            board: sample
                .board
                .iter()
                .map(|minion| MinionJson {
                    card: minion.card_id.clone(),
                    golden: minion.golden,
                    attack: minion.attack(),
                    health: minion.health(),
                })
                .collect(),
        }
    }
}

fn write_pairs(
    path: &Path,
    catalog: &Catalog,
    games: usize,
    seed: u64,
    pairs: &[BoardPair],
) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|error| format!("创建目录失败：{error}"))?;
    }
    let file = File::create(path).map_err(|error| format!("创建文件失败：{error}"))?;
    let mut writer = BufWriter::new(file);
    let header = Header {
        kind: "header",
        schema: DATA_SCHEMA_VERSION,
        catalog_hash: format!("{:016x}", catalog_hash(catalog)),
        games,
        seed,
        pairs: pairs.len(),
    };
    writeln!(
        writer,
        "{}",
        serde_json::to_string(&header).map_err(|error| format!("序列化失败：{error}"))?
    )
    .map_err(|error| format!("写入失败：{error}"))?;
    for pair in pairs {
        let line = PairJson {
            a: SampleJson::from_sample(&pair.a),
            b: SampleJson::from_sample(&pair.b),
            a_wins: pair.a_wins,
        };
        writeln!(
            writer,
            "{}",
            serde_json::to_string(&line).map_err(|error| format!("序列化失败：{error}"))?
        )
        .map_err(|error| format!("写入失败：{error}"))?;
    }
    writer
        .flush()
        .map_err(|error| format!("刷新失败：{error}"))?;
    println!("写入 {} 对棋盘到 {}", pairs.len(), path.display());
    Ok(())
}

/// 目录内容哈希：FNV-1a 覆盖卡牌、英雄与技能 id。
fn catalog_hash(catalog: &Catalog) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let mut feed = |bytes: &[u8]| {
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash ^= 0xff;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    };
    for id in catalog.cards.keys() {
        feed(id.as_bytes());
    }
    for id in catalog.heroes.keys() {
        feed(id.as_bytes());
    }
    for skill in catalog.skills() {
        feed(skill.id.as_bytes());
    }
    hash
}

#[cfg(feature = "train")]
fn train_c0(args: &[String]) -> Result<(), String> {
    use ai_train::model::C0Config;
    use ai_train::train_c0::{calibrate, read_pairs, split_pairs, train_c0 as train};

    let root = repo_root();
    let mut pairs_path = root.join("output/models/c0/pairs.jsonl");
    let mut out_dir = root.join("output/models/c0");
    let mut steps = 20000usize;
    let mut seed = 0u64;
    let mut iter = args.iter();
    while let Some(flag) = iter.next() {
        let value = iter
            .next()
            .ok_or_else(|| format!("{flag} 缺少取值"))?
            .clone();
        match flag.as_str() {
            "--pairs" => pairs_path = value.into(),
            "--out-dir" => out_dir = value.into(),
            "--steps" => {
                steps = value
                    .parse()
                    .map_err(|_| format!("--steps 不是整数：{value}"))?;
            }
            "--seed" => {
                seed = value
                    .parse()
                    .map_err(|_| format!("--seed 不是整数：{value}"))?;
            }
            other => return Err(format!("未知参数：{other}")),
        }
    }

    let catalog = catalog();
    let index = ai_core::CardIndex::build(&catalog);
    let pairs = read_pairs(&pairs_path, &catalog).map_err(|error| error.to_string())?;
    let cfg = C0Config {
        steps,
        seed,
        ..C0Config::default()
    };
    let weights = train(&pairs, &index, &cfg).map_err(|error| error.to_string())?;
    let (train_pairs, held) = split_pairs(&pairs, cfg.held_out);
    let calibration = calibrate(&weights, &cfg, &index, &held);
    weights
        .save(&out_dir.join("weights.json"))
        .map_err(|error| error.to_string())?;
    let manifest = serde_json::json!({
        "schema": DATA_SCHEMA_VERSION,
        "weights_version": ai_model::WEIGHTS_VERSION,
        "catalog_hash": format!("{:016x}", catalog_hash(&catalog)),
        "obs_dim": ai_core::OBS_DIM,
        "hidden": cfg.hidden,
        "layers": cfg.layers,
        "steps": cfg.steps,
        "lr": cfg.lr,
        "seed": cfg.seed,
        "pairs": pairs.len(),
        "train": train_pairs.len(),
        "held_out": held.len(),
        "brier": calibration.brier,
        "ece": calibration.ece,
    });
    fs::create_dir_all(&out_dir).map_err(|error| format!("创建目录失败：{error}"))?;
    fs::write(
        out_dir.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("写入失败：{error}"))?;
    println!(
        "C0 训练完成：{} 对，Brier {:.4}，ECE {:.4}",
        pairs.len(),
        calibration.brier,
        calibration.ece
    );
    Ok(())
}

#[cfg(not(feature = "train"))]
fn train_c0(_args: &[String]) -> Result<(), String> {
    Err("train-c0 需要 --features train".to_string())
}

#[cfg(feature = "train")]
fn train_c1(args: &[String]) -> Result<(), String> {
    use ai_train::eval::eval_c1;
    use ai_train::train_c1::{C1Config, train_c1 as train};

    let root = repo_root();
    let mut pairs_path = root.join("output/models/c0/pairs.jsonl");
    let mut c0_path = root.join("output/models/c0/weights.json");
    let mut out_dir = root.join("output/models/c1");
    let mut episodes = 200usize;
    let mut distribution = 200usize;
    let mut seed = 0u64;
    let mut iter = args.iter();
    while let Some(flag) = iter.next() {
        let value = iter
            .next()
            .ok_or_else(|| format!("{flag} 缺少取值"))?
            .clone();
        match flag.as_str() {
            "--pairs" => pairs_path = value.into(),
            "--c0" => c0_path = value.into(),
            "--out-dir" => out_dir = value.into(),
            "--episodes" => {
                episodes = value
                    .parse()
                    .map_err(|_| format!("--episodes 不是整数：{value}"))?;
            }
            "--distribution" => {
                distribution = value
                    .parse()
                    .map_err(|_| format!("--distribution 不是整数：{value}"))?;
            }
            "--seed" => {
                seed = value
                    .parse()
                    .map_err(|_| format!("--seed 不是整数：{value}"))?;
            }
            other => return Err(format!("未知参数：{other}")),
        }
    }

    let catalog = catalog();
    let pairs =
        ai_train::train_c0::read_pairs(&pairs_path, &catalog).map_err(|error| error.to_string())?;
    let c0 = ai_model::Weights::load(&c0_path).map_err(|error| error.to_string())?;
    let pool = distribution_boards(&pairs, distribution);
    let cfg = C1Config {
        episodes,
        seed,
        ..C1Config::default()
    };
    let output = train(&catalog, &pool, Some(&c0), &cfg).map_err(|error| error.to_string())?;
    output
        .weights
        .save(&out_dir.join("weights.json"))
        .map_err(|error| error.to_string())?;

    let seeds: Vec<u64> = (0..32).collect();
    let report = eval_c1(&catalog, &output.weights, &pool, &seeds);
    let manifest = serde_json::json!({
        "schema": DATA_SCHEMA_VERSION,
        "weights_version": ai_model::WEIGHTS_VERSION,
        "catalog_hash": format!("{:016x}", catalog_hash(&catalog)),
        "obs_dim": cfg.model.obs_dim,
        "hidden": cfg.model.hidden,
        "layers": cfg.model.layers,
        "card_emb": cfg.model.card_emb,
        "episodes": output.episodes,
        "seed": cfg.seed,
        "distribution": pool.len(),
        "mean_return": output.mean_return,
        "last_metrics": {
            "policy_loss": output.last_metrics.policy_loss,
            "value_loss": output.last_metrics.value_loss,
            "entropy": output.last_metrics.entropy,
            "clip_fraction": output.last_metrics.clip_fraction,
        },
    });
    let eval = serde_json::json!({
        "schema": DATA_SCHEMA_VERSION,
        "win_rate": report.win_rate,
        "mean_margin": report.mean_margin,
        "games": report.games,
        "seeds": seeds.len(),
    });
    fs::create_dir_all(&out_dir).map_err(|error| format!("创建目录失败：{error}"))?;
    fs::write(
        out_dir.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("写入失败：{error}"))?;
    fs::write(
        out_dir.join("eval.json"),
        serde_json::to_vec_pretty(&eval).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("写入失败：{error}"))?;
    println!(
        "C1 训练完成：{} 局，平均回报 {:.3}，胜率 {:.3}，均差 {:.3}",
        output.episodes, output.mean_return, report.win_rate, report.mean_margin
    );
    Ok(())
}

#[cfg(not(feature = "train"))]
fn train_c1(_args: &[String]) -> Result<(), String> {
    Err("train-c1 需要 --features train".to_string())
}

/// 从棋盘对里取冻结的对手分布：按顺序交替取两侧棋盘，去重英雄无关的重复。
#[cfg(feature = "train")]
fn distribution_boards(pairs: &[BoardPair], wanted: usize) -> Vec<BoardSample> {
    let mut boards = Vec::with_capacity(wanted);
    for (index, pair) in pairs.iter().enumerate() {
        if boards.len() >= wanted {
            break;
        }
        let sample = if index % 2 == 0 { &pair.a } else { &pair.b };
        if sample.board.is_empty() {
            continue;
        }
        boards.push(sample.clone());
    }
    boards
}

#[cfg(feature = "train")]
fn train_c2(args: &[String]) -> Result<(), String> {
    use ai_train::train_c2::{C2Config, train_c2 as train};

    let root = repo_root();
    let mut pairs_path = root.join("output/models/c0/pairs.jsonl");
    let mut c0_path = root.join("output/models/c0/weights.json");
    let mut c1_path = root.join("output/models/c1/weights.json");
    let mut out_dir = root.join("output/models/c2");
    let mut episodes = 1000usize;
    let mut c1_mix = 0.3f32;
    let mut distribution = 200usize;
    let mut seed = 0u64;
    let mut iter = args.iter();
    while let Some(flag) = iter.next() {
        let value = iter
            .next()
            .ok_or_else(|| format!("{flag} 缺少取值"))?
            .clone();
        match flag.as_str() {
            "--pairs" => pairs_path = value.into(),
            "--c0" => c0_path = value.into(),
            "--c1" => c1_path = value.into(),
            "--out-dir" => out_dir = value.into(),
            "--episodes" => {
                episodes = value
                    .parse()
                    .map_err(|_| format!("--episodes 不是整数：{value}"))?;
            }
            "--c1-mix" => {
                c1_mix = value
                    .parse()
                    .map_err(|_| format!("--c1-mix 不是小数：{value}"))?;
            }
            "--distribution" => {
                distribution = value
                    .parse()
                    .map_err(|_| format!("--distribution 不是整数：{value}"))?;
            }
            "--seed" => {
                seed = value
                    .parse()
                    .map_err(|_| format!("--seed 不是整数：{value}"))?;
            }
            other => return Err(format!("未知参数：{other}")),
        }
    }

    let catalog = catalog();
    let pairs =
        ai_train::train_c0::read_pairs(&pairs_path, &catalog).map_err(|error| error.to_string())?;
    let c0 = ai_model::Weights::load(&c0_path).map_err(|error| error.to_string())?;
    // C1 权重缺失时允许从随机初始化开始，便于只跑 C2 的冒烟。
    let c1 = ai_model::Weights::load(&c1_path).ok();
    if c1.is_none() {
        eprintln!(
            "没有读到 C1 权重（{}），演员从随机初始化开始",
            c1_path.display()
        );
    }
    let pool = distribution_boards(&pairs, distribution);
    let cfg = C2Config {
        episodes,
        c1_mix,
        seed,
        ..C2Config::default()
    };
    let output =
        train(&catalog, &pool, Some(&c0), c1.as_ref(), &cfg).map_err(|error| error.to_string())?;
    output
        .weights
        .save(&out_dir.join("weights.json"))
        .map_err(|error| error.to_string())?;

    let manifest = serde_json::json!({
        "schema": DATA_SCHEMA_VERSION,
        "weights_version": ai_model::WEIGHTS_VERSION,
        "catalog_hash": format!("{:016x}", catalog_hash(&catalog)),
        "obs_dim": cfg.model.obs_dim,
        "hidden": cfg.model.hidden,
        "layers": cfg.model.layers,
        "card_emb": cfg.model.card_emb,
        "episodes": output.episodes,
        "c1_mix": cfg.c1_mix,
        "league_size": cfg.league_size,
        "snapshot_every": cfg.snapshot_every,
        "snapshots": output.snapshots,
        "seed": cfg.seed,
        "distribution": pool.len(),
        "mean_placement": output.mean_placement,
        "mean_return": output.mean_return,
        "last_metrics": {
            "policy_loss": output.last_metrics.policy_loss,
            "value_loss": output.last_metrics.value_loss,
            "entropy": output.last_metrics.entropy,
            "clip_fraction": output.last_metrics.clip_fraction,
        },
    });
    fs::create_dir_all(&out_dir).map_err(|error| format!("创建目录失败：{error}"))?;
    fs::write(
        out_dir.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("写入失败：{error}"))?;
    println!(
        "C2 训练完成：{} 局，平均名次 {:.2}，平均回报 {:.3}，快照 {}",
        output.episodes, output.mean_placement, output.mean_return, output.snapshots
    );
    Ok(())
}

#[cfg(not(feature = "train"))]
fn train_c2(_args: &[String]) -> Result<(), String> {
    Err("train-c2 需要 --features train".to_string())
}

fn eval(args: &[String]) -> Result<(), String> {
    use ai_train::eval_full::{EvalConfig, eval_full};
    use ai_train::league::{League, PolicyId};

    let root = repo_root();
    let mut dir = root.join("output/models/c2");
    let mut seeds = 2000usize;
    let mut learner_seats = 8usize;
    let mut anchor_path: Option<std::path::PathBuf> = None;
    let mut iter = args.iter();
    while let Some(flag) = iter.next() {
        let value = iter
            .next()
            .ok_or_else(|| format!("{flag} 缺少取值"))?
            .clone();
        match flag.as_str() {
            "--dir" | "--run" => dir = value.into(),
            "--seeds" => {
                seeds = value
                    .parse()
                    .map_err(|_| format!("--seeds 不是整数：{value}"))?;
            }
            "--learner-seats" => {
                learner_seats = value
                    .parse()
                    .map_err(|_| format!("--learner-seats 不是整数：{value}"))?;
            }
            "--anchor" => anchor_path = Some(value.into()),
            other => return Err(format!("未知参数：{other}")),
        }
    }

    let catalog = catalog();
    let mut league = League::new(0, 0);
    let anchors = match anchor_path {
        Some(path) => {
            let weights = ai_model::Weights::load(&path).map_err(|error| error.to_string())?;
            league.set_anchor(weights);
            vec![PolicyId::Anchor]
        }
        None => Vec::new(),
    };

    let mut checkpoints = Vec::new();
    let root_weights = dir.join("weights.json");
    if root_weights.is_file() {
        checkpoints.push(root_weights);
    }
    for entry in fs::read_dir(&dir).map_err(|error| format!("读取目录失败：{error}"))? {
        let path = entry.map_err(|error| error.to_string())?.path();
        let candidate = path.join("weights.json");
        if candidate.is_file() {
            checkpoints.push(candidate);
        }
    }
    checkpoints.sort();
    if checkpoints.is_empty() {
        return Err(format!("{} 下没有检查点", dir.display()));
    }

    let mut rows = Vec::new();
    let mut best: Option<(f32, std::path::PathBuf)> = None;
    for path in checkpoints {
        let weights = ai_model::Weights::load(&path).map_err(|error| error.to_string())?;
        let cfg = EvalConfig {
            seeds: (0..seeds as u64).collect(),
            learner_seats,
            anchors: anchors.clone(),
            ..EvalConfig::default()
        };
        let report =
            eval_full(&catalog, &weights, &league, &cfg).map_err(|error| error.to_string())?;
        println!(
            "{}：{} 局，平均名次 {:.2}，吃鸡 {:.2}，前四 {:.2}，±{:.2}",
            path.display(),
            report.games,
            report.mean_placement,
            report.win_rate,
            report.top4_rate,
            report.ci95
        );
        let better = best
            .as_ref()
            .map(|(mean, _)| report.mean_placement < *mean)
            .unwrap_or(true);
        if better {
            best = Some((report.mean_placement, path.clone()));
        }
        rows.push(serde_json::json!({
            "path": path.display().to_string(),
            "report": report,
        }));
    }

    let best_path = best
        .as_ref()
        .map(|(_, path)| path.clone())
        .expect("至少一个检查点");
    let summary = serde_json::json!({
        "schema": DATA_SCHEMA_VERSION,
        "seeds": seeds,
        "learner_seats": learner_seats,
        "anchors": anchors.len(),
        "checkpoints": rows,
        "best": best_path.display().to_string(),
    });
    fs::write(
        dir.join("eval.json"),
        serde_json::to_vec_pretty(&summary).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("写入失败：{error}"))?;
    println!("最佳检查点：{}", best_path.display());
    Ok(())
}
