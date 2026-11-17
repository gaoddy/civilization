//! 基线报告与 JSON 序列化。
//!
//! 所有平均值都用整数运算：先累加分子，再做整数除法**向下取整**，并在字段说明中
//! 写明口径；报告不参与任何规则判定。

use std::collections::BTreeMap;

use rule_engine::data::Tribe;
use rule_engine::intent::Intent;
use rule_engine::state::Seat;
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::{BatchJson, GameLog, GameStats, PLAYER_COUNT, Step, StepRecord};

/// 基线报告。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    /// 局数。
    pub games: u64,
    /// 批量起始种子（第 i 局种子为 `seed + i`）。
    pub seed: u64,
    /// 各流派冠军数，按流派中文名升序；未夺冠的流派不出现。
    pub wins_by_tribe: BTreeMap<String, u64>,
    /// 各流派夺冠率（整数千分比）= 该流派 `wins_by_tribe` × 1000 / `games`，向下取整；
    /// 全部流派（含 0）均出现，键与 `wins_by_tribe` 的流派名一致。
    pub win_rate_permille_by_tribe: BTreeMap<String, u64>,
    /// 平均对局回合数 = 各局结束回合之和 / 局数，向下取整；单位为「回合」（第 1 回合起算，
    /// 结束回合即对局结束时的 `turn`）。
    pub avg_turns: u64,
    /// 每座位首次升时代的回合累加（下标为座位 0..8）= 各局同一座位
    /// `stats.era_upgrade_turns` 之和，对局结束时仍未升到下一时代的座位计 0。
    pub era_upgrade_turns_by_seat: [u64; 8],
    /// 每座位首次升时代的平均回合（下标为座位 0..8）= 上项 / `games`，向下取整；
    /// 未升级的座位按 0 计入，因此会拉低平均值。
    pub avg_era_upgrade_turn_by_seat: [u64; 8],
    /// 全座位首次升时代的平均回合 = 已记录首次升时代回合之和 / 记录到升时代的
    /// （座位, 对局）次数，向下取整；对局结束时仍未升到下一时代的座位不计入。
    /// 没有任何升时代记录时为 `null`。
    pub avg_era_upgrade_turn: Option<u64>,
    /// 有首次升时代记录的人数（座位 × 对局）。
    pub era_upgrade_count: u64,
    /// 每局三连次数之和 / 局数，向下取整。
    pub triples_per_game: u64,
    /// 到达第 20 回合的对局数（`stats.survivors_at_turn_20` 非空）。
    pub games_reaching_turn_20: u64,
    /// 上述对局在第 20 回合开始时平均存活人数（存活人数之和 / `games_reaching_turn_20`，
    /// 向下取整）；没有对局到达第 20 回合时为 `null`。
    pub survivors_at_turn_20: Option<u64>,
    /// 每局结束回合为 20 的对局数（对局在第 20 回合结算后结束）。
    pub games_ended_on_turn_20: u64,
    /// 1000 局（全部对局）平均结束存活人数 = 各局结束存活人数之和 / 局数，向下取整。
    pub avg_survivors_at_end: u64,
    /// 每次战斗失败的平均伤害（全部败者伤害之和 / 败者次数，向下取整）；
    /// 没有败者时为 `null`。
    pub avg_damage_per_loss: Option<u64>,
    /// 每局最终状态哈希（十六进制，按局序）。
    pub final_hashes: Vec<String>,
}

/// 由批量结果构造报告。
pub fn build(batch: &BatchJson) -> Report {
    let games = batch.games.len() as u64;
    let mut wins_by_tribe: BTreeMap<String, u64> = BTreeMap::new();
    let mut turns_sum = 0u64;
    let mut era_by_seat = [0u64; 8];
    let mut era_sum = 0u64;
    let mut era_count = 0u64;
    let mut triples_sum = 0u64;
    let mut survivors_sum = 0u64;
    let mut survivors_games = 0u64;
    let mut ended_on_turn_20 = 0u64;
    let mut survivors_at_end_sum = 0u64;
    let mut damage_sum = 0u64;
    let mut losses = 0u64;
    let mut final_hashes = Vec::with_capacity(batch.games.len());

    for game in &batch.games {
        *wins_by_tribe
            .entry(tribe_name(game.stats.winner_tribe).to_string())
            .or_insert(0) += 1;
        turns_sum += u64::from(game.turns);
        if game.turns == 20 {
            ended_on_turn_20 += 1;
        }
        survivors_at_end_sum += u64::from(game.stats.survivors_at_end);
        for (seat, turn) in game.stats.era_upgrade_turns.iter().enumerate() {
            if let Some(turn) = turn {
                era_by_seat[seat] += u64::from(*turn);
                era_sum += u64::from(*turn);
                era_count += 1;
            }
        }
        triples_sum += u64::from(game.stats.triples);
        if let Some(survivors) = game.stats.survivors_at_turn_20 {
            survivors_sum += u64::from(survivors);
            survivors_games += 1;
        }
        for damage in &game.stats.damage_per_loss {
            damage_sum += u64::try_from((*damage).max(0)).unwrap_or(0);
            losses += 1;
        }
        final_hashes.push(format_hash(game.final_hash));
    }

    let all_tribes = [
        Tribe::Human,
        Tribe::Machine,
        Tribe::Biology,
        Tribe::Plant,
        Tribe::Virus,
        Tribe::Scholar,
        Tribe::General,
    ];
    let win_rate_permille_by_tribe = all_tribes
        .iter()
        .map(|tribe| {
            let wins = wins_by_tribe.get(tribe_name(*tribe)).copied().unwrap_or(0);
            (
                tribe_name(*tribe).to_string(),
                divide(wins.saturating_mul(1000), games),
            )
        })
        .collect();

    Report {
        games,
        seed: batch.seed,
        wins_by_tribe,
        win_rate_permille_by_tribe,
        avg_turns: divide(turns_sum, games),
        era_upgrade_turns_by_seat: era_by_seat,
        avg_era_upgrade_turn_by_seat: era_by_seat.map(|sum| divide(sum, games)),
        avg_era_upgrade_turn: (era_count > 0).then(|| divide(era_sum, era_count)),
        era_upgrade_count: era_count,
        triples_per_game: divide(triples_sum, games),
        games_reaching_turn_20: survivors_games,
        survivors_at_turn_20: (survivors_games > 0).then(|| divide(survivors_sum, survivors_games)),
        games_ended_on_turn_20: ended_on_turn_20,
        avg_survivors_at_end: divide(survivors_at_end_sum, games),
        avg_damage_per_loss: (losses > 0).then(|| divide(damage_sum, losses)),
        final_hashes,
    }
}

/// M0 批量日志校验结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BatchCheck {
    /// 批量日志中的局数。
    pub games: u64,
    /// 期望局数；未指定 `--expect-games` 时等于 `games`（只校验逐局契约）。
    pub expected_games: u64,
    /// 全部对局共记录的排名条目数（每局应为 8）。
    pub ranking_entries: u64,
    /// 逐局全部通过的局数。
    pub valid_games: u64,
}

/// 校验批量日志满足 M0 的「稳定性」与「可判定」：局数与期望一致，且每局排名恰为
/// 8 个不同座位、名次从第 1 名连续排到第 8 名。
///
/// 返回首个错误的描述与全部失败计数；通过时返回计数摘要。
pub fn check_m0_batch(
    batch: &BatchJson,
    expected_games: Option<u64>,
) -> Result<BatchCheck, String> {
    let games = batch.games.len() as u64;
    let expected_games = expected_games.unwrap_or(games);
    if games != expected_games {
        return Err(format!(
            "局数不符：批量日志 {games} 局，期望 {expected_games} 局"
        ));
    }
    let mut bad_ranking = 0u64;
    let mut first_error = None;
    let mut ranking_entries = 0u64;
    for (index, game) in batch.games.iter().enumerate() {
        ranking_entries += game.ranking.len() as u64;
        let mut seats = game.ranking.clone();
        seats.sort_unstable();
        let error = if game.ranking.len() != usize::from(PLAYER_COUNT) {
            Some(format!(
                "排名 {} 条，应为 {PLAYER_COUNT}",
                game.ranking.len()
            ))
        } else if seats != (0..PLAYER_COUNT).collect::<Vec<Seat>>() {
            Some("排名座位重复或越界".to_string())
        } else {
            None
        };
        if let Some(error) = error {
            bad_ranking += 1;
            if first_error.is_none() {
                first_error = Some(format!("第 {index} 局（种子 {}）：{error}", game.seed));
            }
        }
    }
    if let Some(error) = first_error {
        return Err(format!(
            "排名未全部判定：{bad_ranking}/{games} 局异常，首例 {error}"
        ));
    }
    Ok(BatchCheck {
        games,
        expected_games,
        ranking_entries,
        valid_games: games,
    })
}

fn divide(numerator: u64, denominator: u64) -> u64 {
    if denominator == 0 {
        0
    } else {
        numerator / denominator
    }
}

/// 状态哈希的十六进制形式（16 位、小写、补零）。
pub fn format_hash(hash: u64) -> String {
    format!("{hash:016x}")
}

/// 流派 id，与 `data/v0.1` 中的写法一致。
pub fn tribe_name(tribe: Tribe) -> &'static str {
    match tribe {
        Tribe::Human => "human",
        Tribe::Machine => "machine",
        Tribe::Biology => "biology",
        Tribe::Plant => "plant",
        Tribe::Virus => "virus",
        Tribe::Scholar => "scholar",
        Tribe::General => "general",
    }
}

/// 单局日志的 JSON 形式：`seed`、`intents`、`final_hash`、`ranking`，并附带
/// （`final_hash` 为 16 位小写十六进制字符串；`replay` 输出为数值哈希，供测试直接比较。）
/// `turns`、`stats` 与 `players` 便于报告与排查。
///
/// `intents` 是驱动序列：招募意图为 `{"seat", "intent"}`，阶段边界为
/// `{"phase": "finish_recruit" | "resolve_combat" | "resolve_settle" | "start_turn"}`。
pub fn game_to_json(game: &GameLog) -> Value {
    let intents: Vec<Value> = game.steps.iter().map(step_to_json).collect();
    json!({
        "seed": game.seed,
        "players": PLAYER_COUNT,
        "turns": game.turns,
        "intents": intents,
        "final_hash": format_hash(game.final_hash),
        "ranking": ranking_to_json(&game.ranking),
        "stats": stats_to_json(&game.stats, game.turns),
    })
}

fn step_to_json(record: &StepRecord) -> Value {
    match record.step {
        Step::Intent | Step::EndSeat => json!({
            "seat": record.seat,
            "intent": intent_to_json(&record.intent),
        }),
        Step::FinishRecruit => json!({ "phase": "finish_recruit" }),
        Step::ResolveCombat => json!({ "phase": "resolve_combat" }),
        Step::ResolveSettle => json!({ "phase": "resolve_settle" }),
        Step::StartTurn => json!({ "phase": "start_turn" }),
    }
}

fn ranking_to_json(ranking: &[Seat]) -> Value {
    Value::Array(
        ranking
            .iter()
            .enumerate()
            .map(|(index, seat)| json!({ "place": index + 1, "seat": seat }))
            .collect(),
    )
}

/// 意图的 JSON 形式，字段名使用 snake_case。
pub fn intent_to_json(intent: &Intent) -> Value {
    match intent {
        Intent::BuyMinion { slot } => json!({ "kind": "buy_minion", "slot": slot }),
        Intent::SellMinion { minion } => json!({ "kind": "sell_minion", "minion": minion }),
        Intent::RerollShop => json!({ "kind": "reroll_shop" }),
        Intent::UpgradeEra => json!({ "kind": "upgrade_era" }),
        Intent::PlayCard {
            hand_index,
            position,
            target,
        } => json!({
            "kind": "play_card", "hand_index": hand_index,
            "position": position, "target": target,
        }),
        Intent::RearrangeBoard { from, to } => json!({
            "kind": "rearrange_board", "from": from, "to": to,
        }),
        Intent::FreezeShop { slot } => json!({ "kind": "freeze_shop", "slot": slot }),
        Intent::UseHeroPower { skill_id, target } => json!({
            "kind": "use_hero_power", "skill_id": skill_id, "target": target,
        }),
        Intent::EndRecruit => json!({ "kind": "end_recruit" }),
        Intent::ChooseDiscover { choice } => json!({
            "kind": "choose_discover", "choice": choice,
        }),
        Intent::ChooseTarget { minion } => json!({
            "kind": "choose_target", "minion": minion,
        }),
    }
}

fn stats_to_json(stats: &GameStats, turns: u8) -> Value {
    let era: Vec<Value> = stats
        .era_upgrade_turns
        .iter()
        .map(|turn| json!(turn))
        .collect();
    json!({
        "era_upgrade_turns": era,
        "triples": stats.triples,
        "survivors_at_turn_20": stats.survivors_at_turn_20,
        "survivors_at_end": stats.survivors_at_end,
        "damage_per_loss": stats.damage_per_loss,
        "winner_seat": stats.winner,
        "winner_tribe": tribe_name(stats.winner_tribe),
        "turns": turns,
    })
}

/// 批量日志 JSON：`seed` + `games`（每局 `seed` / `intents` / `final_hash` / `ranking`
/// / `turns` / `stats`）。
pub fn batch_to_json(batch: &BatchJson) -> Value {
    let games: Vec<Value> = batch.games.iter().map(game_to_json).collect();
    json!({ "seed": batch.seed, "games": games })
}

/// 解析单局 JSON（日志文件中的一局）。`seed`、`intents`、`final_hash` 与 `stats` 中的
/// `winner_tribe` / `winner_seat` 缺失或非法时返回错误，不做静默回落。
pub fn game_from_json(value: &Value) -> Result<GameLog, String> {
    let seed = value
        .get("seed")
        .and_then(Value::as_u64)
        .ok_or("日志缺少 seed")?;
    let intents = value
        .get("intents")
        .and_then(Value::as_array)
        .ok_or("日志缺少 intents")?;
    let mut steps = Vec::with_capacity(intents.len());
    for entry in intents {
        if let Some(phase) = entry.get("phase").and_then(Value::as_str) {
            let step = match phase {
                "finish_recruit" => Step::FinishRecruit,
                "resolve_combat" => Step::ResolveCombat,
                "resolve_settle" => Step::ResolveSettle,
                "start_turn" => Step::StartTurn,
                other => return Err(format!("未知阶段记录 {other}")),
            };
            steps.push(StepRecord {
                seat: 0,
                intent: Intent::EndRecruit,
                step,
            });
            continue;
        }
        let seat = entry
            .get("seat")
            .and_then(Value::as_u64)
            .and_then(|seat| u8::try_from(seat).ok())
            .ok_or("意图记录缺少 seat")?;
        let intent = intent_from_json(entry.get("intent").ok_or("意图记录缺少 intent")?)?;
        let step = if matches!(intent, Intent::EndRecruit) {
            Step::EndSeat
        } else {
            Step::Intent
        };
        steps.push(StepRecord { seat, intent, step });
    }
    let final_hash = value
        .get("final_hash")
        .and_then(Value::as_str)
        .ok_or("日志缺少 final_hash")?;
    let final_hash = parse_hash(final_hash)?;
    let ranking = match value.get("ranking").and_then(Value::as_array) {
        Some(ranking) => ranking
            .iter()
            .map(|entry| {
                entry
                    .get("seat")
                    .and_then(Value::as_u64)
                    .and_then(|seat| u8::try_from(seat).ok())
                    .ok_or_else(|| "排名记录缺少 seat".to_string())
            })
            .collect::<Result<Vec<Seat>, String>>()?,
        None => Vec::new(),
    };
    let turns = value
        .get("turns")
        .and_then(Value::as_u64)
        .and_then(|turns| u8::try_from(turns).ok())
        .unwrap_or(0);
    let stats = match value.get("stats") {
        Some(stats) => stats_from_json(stats)?,
        None => GameStats::default(),
    };
    Ok(GameLog {
        seed,
        steps,
        final_hash,
        ranking,
        turns,
        stats,
    })
}

fn parse_hash(text: &str) -> Result<u64, String> {
    u64::from_str_radix(text, 16).map_err(|err| format!("final_hash 不是十六进制: {err}"))
}

fn intent_from_json(value: &Value) -> Result<Intent, String> {
    let kind = value
        .get("kind")
        .and_then(Value::as_str)
        .ok_or("意图缺少 kind")?;
    let target = value
        .get("target")
        .and_then(Value::as_u64)
        .filter(|target| *target > 0);
    let field = |name: &str| {
        value
            .get(name)
            .and_then(Value::as_u64)
            .and_then(|number| u8::try_from(number).ok())
            .ok_or_else(|| format!("意图 {kind} 缺少字段 {name}"))
    };
    Ok(match kind {
        "buy_minion" => Intent::BuyMinion {
            slot: field("slot")?,
        },
        "sell_minion" => Intent::SellMinion {
            minion: value
                .get("minion")
                .and_then(Value::as_u64)
                .ok_or("缺少 minion")?,
        },
        "reroll_shop" => Intent::RerollShop,
        "upgrade_era" => Intent::UpgradeEra,
        "play_card" => Intent::PlayCard {
            hand_index: field("hand_index")?,
            position: value
                .get("position")
                .and_then(Value::as_u64)
                .and_then(|position| u8::try_from(position).ok()),
            target,
        },
        "rearrange_board" => Intent::RearrangeBoard {
            from: field("from")?,
            to: field("to")?,
        },
        "freeze_shop" => Intent::FreezeShop {
            slot: value
                .get("slot")
                .and_then(Value::as_u64)
                .and_then(|slot| u8::try_from(slot).ok()),
        },
        "use_hero_power" => Intent::UseHeroPower {
            skill_id: value
                .get("skill_id")
                .and_then(Value::as_str)
                .ok_or("缺少 skill_id")?
                .to_string(),
            target,
        },
        "end_recruit" => Intent::EndRecruit,
        "choose_discover" => Intent::ChooseDiscover {
            choice: field("choice")?,
        },
        "choose_target" => Intent::ChooseTarget {
            minion: value
                .get("minion")
                .and_then(Value::as_u64)
                .ok_or("缺少 minion")?,
        },
        other => return Err(format!("未知意图 kind {other}")),
    })
}

fn stats_from_json(value: &Value) -> Result<GameStats, String> {
    let mut era_upgrade_turns = [None; 8];
    if let Some(era) = value.get("era_upgrade_turns").and_then(Value::as_array) {
        for (index, entry) in era.iter().take(8).enumerate() {
            era_upgrade_turns[index] = entry.as_u64().and_then(|turn| u8::try_from(turn).ok());
        }
    }
    let triples = value
        .get("triples")
        .and_then(Value::as_u64)
        .and_then(|triples| u32::try_from(triples).ok())
        .unwrap_or(0);
    let survivors_at_turn_20 = value
        .get("survivors_at_turn_20")
        .and_then(Value::as_u64)
        .and_then(|survivors| u8::try_from(survivors).ok());
    let survivors_at_end = value
        .get("survivors_at_end")
        .and_then(Value::as_u64)
        .and_then(|survivors| u8::try_from(survivors).ok())
        .unwrap_or(0);
    let mut damage_per_loss = Vec::new();
    if let Some(list) = value.get("damage_per_loss").and_then(Value::as_array) {
        for entry in list {
            let amount = entry
                .as_i64()
                .ok_or_else(|| "damage_per_loss 含非整数".to_string())?;
            damage_per_loss
                .push(i32::try_from(amount).map_err(|_| "damage_per_loss 越界".to_string())?);
        }
    }
    let winner = value
        .get("winner_seat")
        .and_then(Value::as_u64)
        .and_then(|seat| u8::try_from(seat).ok())
        .ok_or("stats 缺少 winner_seat")?;
    let winner_tribe_name = value
        .get("winner_tribe")
        .and_then(Value::as_str)
        .ok_or("stats 缺少 winner_tribe")?;
    let winner_tribe = tribe_from_name(winner_tribe_name)
        .ok_or_else(|| format!("stats 的 winner_tribe「{winner_tribe_name}」不是已知流派"))?;
    Ok(GameStats {
        era_upgrade_turns,
        triples,
        survivors_at_turn_20,
        survivors_at_end,
        damage_per_loss,
        winner,
        winner_tribe,
    })
}

fn tribe_from_name(name: &str) -> Option<Tribe> {
    Some(match name {
        "human" => Tribe::Human,
        "machine" => Tribe::Machine,
        "biology" => Tribe::Biology,
        "plant" => Tribe::Plant,
        "virus" => Tribe::Virus,
        "scholar" => Tribe::Scholar,
        "general" => Tribe::General,
        _ => return None,
    })
}

/// 解析日志：批量文件（顶层含 `games`）返回其中全部对局；单局文件返回一个元素。
pub fn logs_from_json(value: &Value) -> Result<Vec<GameLog>, String> {
    if value.get("games").is_some() {
        Ok(batch_from_json(value)?.games)
    } else {
        Ok(vec![game_from_json(value)?])
    }
}

/// 解析批量日志（`seed` + `games`）。
pub fn batch_from_json(value: &Value) -> Result<BatchJson, String> {
    let seed = value
        .get("seed")
        .and_then(Value::as_u64)
        .ok_or("批量日志缺少 seed")?;
    let games = value
        .get("games")
        .and_then(Value::as_array)
        .ok_or("批量日志缺少 games")?;
    let games = games
        .iter()
        .enumerate()
        .map(|(index, game)| game_from_json(game).map_err(|err| format!("第 {index} 局: {err}")))
        .collect::<Result<Vec<GameLog>, String>>()?;
    Ok(BatchJson { seed, games })
}

/// 报告 JSON（字段按声明顺序输出）。
pub fn report_to_json(report: &Report) -> Value {
    let mut object = Map::new();
    object.insert("games".into(), json!(report.games));
    object.insert("seed".into(), json!(report.seed));
    object.insert("wins_by_tribe".into(), json!(report.wins_by_tribe));
    object.insert(
        "win_rate_permille_by_tribe".into(),
        json!(report.win_rate_permille_by_tribe),
    );
    object.insert("avg_turns".into(), json!(report.avg_turns));
    object.insert(
        "era_upgrade_turns_by_seat".into(),
        json!(report.era_upgrade_turns_by_seat),
    );
    object.insert(
        "avg_era_upgrade_turn_by_seat".into(),
        json!(report.avg_era_upgrade_turn_by_seat),
    );
    object.insert(
        "avg_era_upgrade_turn".into(),
        json!(report.avg_era_upgrade_turn),
    );
    object.insert("era_upgrade_count".into(), json!(report.era_upgrade_count));
    object.insert("triples_per_game".into(), json!(report.triples_per_game));
    object.insert(
        "games_reaching_turn_20".into(),
        json!(report.games_reaching_turn_20),
    );
    object.insert(
        "survivors_at_turn_20".into(),
        json!(report.survivors_at_turn_20),
    );
    object.insert(
        "games_ended_on_turn_20".into(),
        json!(report.games_ended_on_turn_20),
    );
    object.insert(
        "avg_survivors_at_end".into(),
        json!(report.avg_survivors_at_end),
    );
    object.insert(
        "avg_damage_per_loss".into(),
        json!(report.avg_damage_per_loss),
    );
    object.insert("final_hashes".into(), json!(report.final_hashes));
    Value::Object(object)
}
