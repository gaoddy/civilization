//! C0：棋盘对训练、校准与数据读入。

use std::fs;
use std::path::Path;

use ai_core::CardIndex;
use ai_model::{Weights, board_value};
use rule_engine::data::Catalog;
use rule_engine::state::{MinionInstance, MinionStats};
use serde::Deserialize;

use crate::data_c0::{BoardPair, BoardSample};
use crate::model::{C0Config, board_observation, fit};

/// 训练与数据读入的错误。
#[derive(Debug)]
pub enum TrainError {
    Candle(candle_core::Error),
    Io(std::io::Error),
    Json(serde_json::Error),
    NoData(String),
}

impl std::fmt::Display for TrainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Candle(error) => write!(f, "训练失败：{error}"),
            Self::Io(error) => write!(f, "读写失败：{error}"),
            Self::Json(error) => write!(f, "数据解析失败：{error}"),
            Self::NoData(message) => write!(f, "没有可用样本：{message}"),
        }
    }
}

impl std::error::Error for TrainError {}

impl From<candle_core::Error> for TrainError {
    fn from(error: candle_core::Error) -> Self {
        Self::Candle(error)
    }
}

impl From<std::io::Error> for TrainError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for TrainError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

/// 校准指标。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Calibration {
    pub brier: f32,
    pub ece: f32,
}

/// 按英雄与玩法分层切训练/留出：先按分组键排序，再等距抽取。
pub fn split_pairs(pairs: &[BoardPair], held_out: f64) -> (Vec<BoardPair>, Vec<BoardPair>) {
    let mut order: Vec<usize> = (0..pairs.len()).collect();
    order.sort_by_key(|index| {
        let pair = &pairs[*index];
        (
            pair.a.hero.clone(),
            pair.a.playstyles,
            pair.b.hero.clone(),
            pair.b.playstyles,
        )
    });
    let step = if held_out <= 0.0 {
        0
    } else {
        (1.0 / held_out).round().max(2.0) as usize
    };
    let mut train = Vec::new();
    let mut held = Vec::new();
    for (position, index) in order.into_iter().enumerate() {
        if step > 0 && position % step == 0 {
            held.push(pairs[index].clone());
        } else {
            train.push(pairs[index].clone());
        }
    }
    // 分层键过窄时可能把全部样本分到一侧，退回按顺序等距抽取。
    if (train.is_empty() || held.is_empty()) && pairs.len() >= 2 {
        train.clear();
        held.clear();
        for (index, pair) in pairs.iter().enumerate() {
            if step > 0 && index % step == 0 {
                held.push(pair.clone());
            } else {
                train.push(pair.clone());
            }
        }
    }
    (train, held)
}

/// 训练 C0 估值；留出集按 [`split_pairs`] 切分，实际只用训练集。
pub fn train_c0(
    pairs: &[BoardPair],
    index: &CardIndex,
    cfg: &C0Config,
) -> Result<Weights, TrainError> {
    if pairs.is_empty() {
        return Err(TrainError::NoData("棋盘对为空".to_string()));
    }
    let (train, _) = split_pairs(pairs, cfg.held_out);
    let data = if train.is_empty() { pairs } else { &train };
    let obs_a: Vec<Vec<f32>> = data
        .iter()
        .map(|pair| board_observation(&pair.a, index).to_vec())
        .collect();
    let obs_b: Vec<Vec<f32>> = data
        .iter()
        .map(|pair| board_observation(&pair.b, index).to_vec())
        .collect();
    let a_wins: Vec<bool> = data.iter().map(|pair| pair.a_wins).collect();
    Ok(fit(&obs_a, &obs_b, &a_wins, cfg)?)
}

/// 留出集上的 Brier 与 ECE（10 个等宽分箱）。
pub fn calibrate(
    weights: &Weights,
    cfg: &C0Config,
    index: &CardIndex,
    pairs: &[BoardPair],
) -> Calibration {
    let model = cfg.model_config();
    let mut brier = 0.0f64;
    let mut bins = [(0.0f64, 0.0f64, 0usize); 10];
    for pair in pairs {
        let a = board_observation(&pair.a, index);
        let b = board_observation(&pair.b, index);
        let diff = f64::from(board_value(weights, &model, &a).unwrap_or(0.0))
            - f64::from(board_value(weights, &model, &b).unwrap_or(0.0));
        let probability = 1.0 / (1.0 + (-diff).exp());
        let target = if pair.a_wins { 1.0 } else { 0.0 };
        brier += (probability - target).powi(2);
        let bin = ((probability * 10.0).floor() as usize).min(9);
        bins[bin].0 += probability;
        bins[bin].1 += target;
        bins[bin].2 += 1;
    }
    let count = pairs.len().max(1) as f64;
    let brier = (brier / count) as f32;
    let mut ece = 0.0f64;
    for (sum_p, sum_y, items) in bins {
        if items == 0 {
            continue;
        }
        let items = items as f64;
        ece += (items / count) * ((sum_p / items) - (sum_y / items)).abs();
    }
    Calibration {
        brier,
        ece: ece as f32,
    }
}

#[derive(Deserialize)]
struct PairLine {
    a: SampleLine,
    b: SampleLine,
    a_wins: bool,
}

#[derive(Deserialize)]
struct SampleLine {
    hero: String,
    playstyles: u16,
    board: Vec<MinionLine>,
}

#[derive(Deserialize)]
struct MinionLine {
    card: String,
    golden: bool,
    attack: i32,
    health: i32,
}

/// 读 JSONL 棋盘对；首行表头跳过。
pub fn read_pairs(path: &Path, catalog: &Catalog) -> Result<Vec<BoardPair>, TrainError> {
    let text = fs::read_to_string(path)?;
    let mut pairs = Vec::new();
    let mut next_id = 1u64;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line)?;
        if value.get("kind").and_then(|kind| kind.as_str()) == Some("header") {
            continue;
        }
        let line: PairLine = serde_json::from_value(value)?;
        let a = sample_from_line(&line.a, catalog, &mut next_id);
        let b = sample_from_line(&line.b, catalog, &mut next_id);
        pairs.push(BoardPair {
            a,
            b,
            a_wins: line.a_wins,
        });
    }
    if pairs.is_empty() {
        return Err(TrainError::NoData(path.display().to_string()));
    }
    Ok(pairs)
}

fn sample_from_line(line: &SampleLine, catalog: &Catalog, next_id: &mut u64) -> BoardSample {
    let board = line
        .board
        .iter()
        .map(|minion| {
            let mut instance = MinionInstance::new(catalog, *next_id, &minion.card, minion.golden);
            *next_id += 1;
            instance.stats = MinionStats::new(minion.attack, minion.health);
            instance
        })
        .collect();
    BoardSample {
        hero: line.hero.clone(),
        playstyles: line.playstyles,
        board,
    }
}
