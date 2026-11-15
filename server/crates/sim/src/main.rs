//! 模拟器命令行：`play`、`batch`、`replay`、`report`、`check` 五个子命令。
//!
//! 结果写入 stdout（JSON）；任何参数错误、模拟失败或日志不一致都以非零码退出，
//! 并只向 stderr 输出原因。不读时钟与环境变量。

use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;

use serde_json::Value;
use sim::report;

const USAGE: &str = "用法：
  sim play   --seed N [--players 8]
  sim batch  --games N --seed N --out PATH [--players 8]
  sim replay --log PATH
  sim report --log PATH
  sim check  --log PATH [--expect-games N]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("sim: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let Some((command, rest)) = args.split_first() else {
        return Err(format!("缺少子命令\n{USAGE}"));
    };
    match command.as_str() {
        "play" => {
            let options = parse_options(rest, &["--seed", "--players"])?;
            let seed = options.required_u64("--seed")?;
            let players = options.players()?;
            let log = sim::run_games(1, seed, players)
                .map_err(|err| err.to_string())?
                .games
                .remove(0);
            print_json(&report::game_to_json(&log))
        }
        "batch" => {
            let options = parse_options(rest, &["--games", "--seed", "--out", "--players"])?;
            let games = options.required_usize("--games")?;
            let seed = options.required_u64("--seed")?;
            let out = options.required("--out")?;
            let players = options.players()?;
            let batch = sim::run_games(games, seed, players).map_err(|err| err.to_string())?;
            let json = report::batch_to_json(&batch);
            write_json(Path::new(out), &json)?;
            println!("已写入 {out}（{} 局，起始种子 {seed}）", batch.games.len());
            Ok(())
        }
        "replay" => {
            let options = parse_options(rest, &["--log"])?;
            let path = options.required("--log")?;
            let value = read_json(Path::new(path))?;
            let logs = report::logs_from_json(&value)?;
            let mut hashes = Vec::with_capacity(logs.len());
            for (index, log) in logs.iter().enumerate() {
                let hash = sim::replay(log).map_err(|err| format!("第 {index} 局: {err}"))?;
                hashes.push(hash);
            }
            // 单局重放输出该局：`final_hash` 为数值，与日志中的十六进制 `final_hash` 对应；
            // 批量日志则输出 `hashes` 数组（按局序）。
            if logs.len() == 1 {
                let hash = hashes[0];
                let mut object = serde_json::Map::new();
                object.insert("seed".into(), Value::from(logs[0].seed));
                object.insert("final_hash".into(), Value::from(hash));
                print_json(&Value::Object(object))
            } else {
                let mut object = serde_json::Map::new();
                object.insert("games".into(), Value::from(logs.len()));
                object.insert("hashes".into(), Value::from(hashes));
                print_json(&Value::Object(object))
            }
        }
        "report" => {
            let options = parse_options(rest, &["--log"])?;
            let path = options.required("--log")?;
            let value = read_json(Path::new(path))?;
            let batch = report::batch_from_json(&value)?;
            let check = report::check_m0_batch(&batch, None)?;
            let summary = report::build(&batch);
            print_json(&report::report_to_json(&summary))?;
            // 校验摘要写到 stderr，stdout 保持纯 JSON。
            eprintln!(
                "已校验 {} 局：排名条目 {}，全部可判定",
                check.valid_games, check.ranking_entries
            );
            Ok(())
        }
        "check" => {
            let options = parse_options(rest, &["--log", "--expect-games"])?;
            let path = options.required("--log")?;
            let expected = match options.values.get("--expect-games") {
                Some(_) => Some(options.required_u64("--expect-games")?),
                None => None,
            };
            let value = read_json(Path::new(path))?;
            let batch = report::batch_from_json(&value)?;
            let check = report::check_m0_batch(&batch, expected)?;
            println!(
                "已校验 {} 局（期望 {} 局）：排名条目 {}，全部可判定",
                check.games, check.expected_games, check.ranking_entries
            );
            Ok(())
        }
        "--help" | "-h" | "help" => {
            println!("{USAGE}");
            Ok(())
        }
        other => Err(format!("未知子命令 {other}\n{USAGE}")),
    }
}

fn print_json(value: &Value) -> Result<(), String> {
    let text =
        serde_json::to_string_pretty(value).map_err(|err| format!("序列化 JSON 失败: {err}"))?;
    println!("{text}");
    Ok(())
}

fn read_json(path: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("读取 {} 失败: {err}", path.display()))?;
    serde_json::from_str(&text).map_err(|err| format!("解析 {} 失败: {err}", path.display()))
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    let text = serde_json::to_string(value).map_err(|err| format!("序列化 JSON 失败: {err}"))?;
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("创建目录 {} 失败: {err}", parent.display()))?;
    }
    std::fs::write(path, text).map_err(|err| format!("写入 {} 失败: {err}", path.display()))
}

/// 已解析的 `--key value` / `--key=value` 选项。
struct Options {
    values: BTreeMap<String, String>,
}

impl Options {
    fn required(&self, key: &str) -> Result<&str, String> {
        self.values
            .get(key)
            .map(String::as_str)
            .ok_or_else(|| format!("缺少参数 {key}\n{USAGE}"))
    }

    fn required_u64(&self, key: &str) -> Result<u64, String> {
        let text = self.required(key)?;
        text.parse::<u64>()
            .map_err(|err| format!("参数 {key} 不是无符号整数「{text}」: {err}"))
    }

    fn required_usize(&self, key: &str) -> Result<usize, String> {
        let text = self.required(key)?;
        text.parse::<usize>()
            .map_err(|err| format!("参数 {key} 不是非负整数「{text}」: {err}"))
    }

    fn players(&self) -> Result<usize, String> {
        let Some(text) = self.values.get("--players") else {
            return Ok(usize::from(sim::PLAYER_COUNT));
        };
        let players = text
            .parse::<u8>()
            .map_err(|err| format!("参数 --players 不是整数「{text}」: {err}"))?;
        if players != sim::PLAYER_COUNT {
            return Err(format!(
                "只支持 {} 人局，--players 为 {players}",
                sim::PLAYER_COUNT
            ));
        }
        Ok(usize::from(players))
    }
}

fn parse_options(args: &[String], allowed: &[&str]) -> Result<Options, String> {
    let mut values = BTreeMap::new();
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        let (key, inline) = match arg.split_once('=') {
            Some((key, value)) => (key.to_string(), Some(value.to_string())),
            None => (arg.clone(), None),
        };
        if !key.starts_with("--") {
            return Err(format!("意外的参数 {arg}\n{USAGE}"));
        }
        if !allowed.contains(&key.as_str()) {
            return Err(format!("未知参数 {key}\n{USAGE}"));
        }
        let value = match inline {
            Some(value) => value,
            None => {
                index += 1;
                args.get(index)
                    .cloned()
                    .ok_or_else(|| format!("参数 {key} 缺少取值\n{USAGE}"))?
            }
        };
        if values.insert(key.clone(), value).is_some() {
            return Err(format!("参数 {key} 重复"));
        }
        index += 1;
    }
    Ok(Options { values })
}
