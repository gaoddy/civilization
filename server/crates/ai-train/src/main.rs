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
        Some(other) => Err(format!("未知子命令：{other}")),
        None => Err("缺少子命令".to_string()),
    };
    if let Err(message) = result {
        eprintln!("{message}");
        eprintln!(
            "用法：ai-train data-c0 --out <路径> [--games 2000] [--seed 0]\n      ai-train train-c0 [--pairs 路径] [--out-dir 目录] [--steps 20000] [--seed 0]"
        );
        std::process::exit(2);
    }
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

    let mut pairs_path = Path::new("output/models/c0/pairs.jsonl").to_path_buf();
    let mut out_dir = Path::new("output/models/c0").to_path_buf();
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
