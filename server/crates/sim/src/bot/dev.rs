//! `sim dev`：起 7 个机器人建房并打完一局，给桌面客户端留出第 8 个座位。
//!
//! 第 0 个机器人建房，通过 [`super::client::BotConfig::room_code_sink`] 把房间码
//! 交回调用方；其余机器人在拿到房间码后并发加入。开发者用桌面客户端作为第 8 人
//! 入座并准备，服务端满 8 人自动开局。

use std::collections::BTreeMap;
use std::time::Duration;

use futures_util::future::join_all;
use rule_engine::data::Card;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use crate::PLAYER_COUNT;
use crate::bot::client::{self, BotConfig, BotOutcome};

/// 机器人数量：给开发者留出恰好一个座位。
pub const DEV_BOTS: usize = PLAYER_COUNT as usize - 1;

/// 取房间码的超时。服务端未启动或建房被拒时不会无限等待。
const ROOM_CODE_TIMEOUT: Duration = Duration::from_secs(10);

/// `sim dev` 的运行参数。
pub struct DevConfig {
    /// WebSocket 地址，例如 `ws://127.0.0.1:8080/ws`。
    pub url: String,
    /// 第 0 个机器人的意图种子；第 `index` 个机器人使用 `intent_seed + index`。
    pub intent_seed: u64,
    /// 取到房间码后的回调；CLI 用它先打印房间码再等整局结束。
    pub room_code_sink: Option<oneshot::Sender<String>>,
    /// 取到房间码后等待开局与整局结束的总超时。
    pub run_timeout: Duration,
}

/// 一局结束后的汇总；以建房机器人的视角为准，8 个座位应当看到同一份排名。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevOutcome {
    /// 建房得到的 6 位房间码。
    pub room_code: String,
    /// 结束时的回合数。
    pub turns: u8,
    /// `(playerId, place)`，按服务端排名顺序。
    pub ranking: Vec<(String, u8)>,
}

/// 按卡牌 id 升序取 `needed` 张英雄卡。
///
/// `BTreeMap` 的迭代顺序即 id 升序，因此结果确定；英雄卡不足时返回错误，
/// 避免调用方越界取英雄。
fn take_heroes(cards: &BTreeMap<String, Card>, needed: usize) -> Result<Vec<String>, String> {
    let heroes: Vec<String> = cards
        .values()
        .filter(|card| card.is_hero)
        .take(needed)
        .map(|card| card.id.clone())
        .collect();
    if heroes.len() < needed {
        return Err(format!(
            "内嵌卡表只有 {} 张英雄卡，至少需要 {needed} 张",
            heroes.len()
        ));
    }
    Ok(heroes)
}

/// 起 7 个机器人并入房：第 0 个建房，其余在拿到房间码后加入。
///
/// # Errors
///
/// 卡表英雄卡不足、取房间码超时、机器人失败，或整局未能在 `run_timeout` 内结束。
pub async fn run(config: DevConfig) -> Result<DevOutcome, String> {
    let DevConfig {
        url,
        intent_seed,
        room_code_sink,
        run_timeout,
    } = config;

    let catalog = crate::embedded_catalog().map_err(|err| err.to_string())?;
    let heroes = take_heroes(&catalog.cards, DEV_BOTS)?;

    let (code_tx, code_rx) = oneshot::channel();
    let mut handles: Vec<JoinHandle<Result<BotOutcome, String>>> = Vec::with_capacity(DEV_BOTS);
    handles.push(tokio::spawn(client::run(BotConfig {
        url: url.clone(),
        room: String::new(),
        name: "bot0".to_string(),
        hero: heroes[0].clone(),
        create: true,
        intent_seed,
        room_code_sink: Some(code_tx),
    })));

    let room_code = match tokio::time::timeout(ROOM_CODE_TIMEOUT, code_rx).await {
        Ok(Ok(code)) => code,
        Ok(Err(_)) => {
            abort_all(&mut handles).await;
            return Err("建房机器人提前退出，未取得房间码".to_string());
        }
        Err(_) => {
            abort_all(&mut handles).await;
            return Err(format!(
                "{} 秒内未取得房间码：服务端可能未启动",
                ROOM_CODE_TIMEOUT.as_secs()
            ));
        }
    };
    if let Some(sink) = room_code_sink {
        let _ = sink.send(room_code.clone());
    }

    for (index, hero) in heroes.iter().enumerate().skip(1) {
        handles.push(tokio::spawn(client::run(BotConfig {
            url: url.clone(),
            room: room_code.clone(),
            name: format!("bot{index}"),
            hero: hero.clone(),
            create: false,
            intent_seed: intent_seed.saturating_add(index as u64),
            room_code_sink: None,
        })));
    }

    let joined = tokio::time::timeout(run_timeout, join_all(handles.iter_mut())).await;
    let results = match joined {
        Ok(results) => results,
        Err(_) => {
            abort_all(&mut handles).await;
            return Err(format!(
                "未完成开局：开发者未进房并准备，或对局未结束（{} 秒）",
                run_timeout.as_secs()
            ));
        }
    };

    let mut outcomes = Vec::with_capacity(results.len());
    let mut failure = None;
    for (index, result) in results.into_iter().enumerate() {
        match result {
            Ok(Ok(outcome)) => outcomes.push(outcome),
            Ok(Err(err)) => {
                failure.get_or_insert(format!("bot{index} 失败: {err}"));
            }
            Err(err) => {
                failure.get_or_insert(format!("bot{index} 任务异常: {err}"));
            }
        }
    }
    if let Some(message) = failure {
        abort_all(&mut handles).await;
        return Err(message);
    }

    let first = outcomes
        .into_iter()
        .next()
        .ok_or_else(|| "没有机器人返回结果".to_string())?;
    Ok(DevOutcome {
        room_code,
        turns: first.turns,
        ranking: first.ranking,
    })
}

/// 取消仍然存活的机器人任务。对已结束的任务 `abort` 是空操作。
async fn abort_all(handles: &mut Vec<JoinHandle<Result<BotOutcome, String>>>) {
    for handle in handles.iter() {
        handle.abort();
    }
    for handle in handles.drain(..) {
        let _ = handle.await;
    }
}

#[cfg(test)]
mod tests {
    use super::take_heroes;

    #[test]
    fn take_heroes_fails_when_catalog_has_too_few_heroes() {
        let error = take_heroes(&std::collections::BTreeMap::new(), 7).expect_err("空卡表应报错");
        assert!(error.contains("至少需要 7 张"), "实际：{error}");
    }
}
