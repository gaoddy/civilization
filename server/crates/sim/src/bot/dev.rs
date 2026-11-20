//! `sim dev`：起 7 个机器人建房并打完一局，给桌面客户端留出第 8 个座位。
//!
//! 第 0 个机器人建房，通过 [`super::client::BotConfig::room_code_sink`] 把房间码
//! 交回调用方；其余机器人在拿到房间码后并发加入。开发者用桌面客户端作为第 8 人
//! 入座并准备，服务端满 8 人自动开局。

use std::collections::BTreeMap;
use std::time::Duration;

use futures_util::StreamExt;
use futures_util::stream::FuturesUnordered;
use rule_engine::data::HeroCard;
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
    /// 取到房间码后等待开局与整局结束的总超时；`None` 表示不设总时限，
    /// 但仍保留「任一机器人失败立即取消其余」的行为。
    pub run_timeout: Option<Duration>,
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
fn take_heroes(heroes: &BTreeMap<String, HeroCard>, needed: usize) -> Result<Vec<String>, String> {
    let heroes: Vec<String> = heroes
        .values()
        .take(needed)
        .map(|hero| hero.id.clone())
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
/// 卡表英雄卡不足、取房间码超时、机器人失败，或设置 `run_timeout` 时整局未在期限内结束。
pub async fn run(config: DevConfig) -> Result<DevOutcome, String> {
    let DevConfig {
        url,
        intent_seed,
        room_code_sink,
        run_timeout,
    } = config;

    let catalog = crate::embedded_catalog().map_err(|err| err.to_string())?;
    let heroes = take_heroes(&catalog.heroes, DEV_BOTS)?;

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
            // 建房机器人已退出。取回它的真实错误，否则调用方只看到
            // 「未取得房间码」，看不到连接被拒或认证失败的原因。
            let host = handles.remove(0);
            let detail = match host.await {
                Ok(Err(err)) => err,
                Ok(Ok(_)) => "建房机器人提前结束，未取得房间码".to_string(),
                Err(err) => format!("建房机器人任务异常: {err}"),
            };
            abort_all(&mut handles);
            return Err(format!("建房失败: {detail}"));
        }
        Err(_) => {
            abort_all(&mut handles);
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

    let results = match wait_all(&mut handles, run_timeout).await {
        Ok(outcomes) => outcomes,
        Err(message) => return Err(message),
    };
    // 结果按机器人序号收集，`wait_all` 已保证顺序。
    let first = results
        .into_iter()
        .next()
        .ok_or_else(|| "没有机器人返回结果".to_string())?;
    Ok(DevOutcome {
        room_code,
        turns: first.turns,
        ranking: first.ranking,
    })
}

/// 并发等待所有机器人，任一失败立即取消其余机器人并返回错误。
///
/// `run_timeout` 为 `Some` 时等待受该总时限约束；为 `None` 时不设截止时间，
/// 一直等到每个机器人都返回。无论是否设总时限，任一机器人失败都会立即取消其余任务。
///
/// 不能用 `join_all`：它要等所有 handle 结束才返回，一个机器人提前失败时
/// 其余的仍在等开发者入房，失败原因会被拖到总超时才暴露。
///
/// 返回值按机器人序号排列。失败时错误消息写明是哪个 bot。
async fn wait_all(
    handles: &mut Vec<JoinHandle<Result<BotOutcome, String>>>,
    run_timeout: Option<Duration>,
) -> Result<Vec<BotOutcome>, String> {
    let deadline = run_timeout.map(|timeout| (tokio::time::Instant::now() + timeout, timeout));
    let mut outcomes: Vec<Option<BotOutcome>> = vec![None; handles.len()];

    // `&mut JoinHandle` 本身是 `Future`，因此可以在不改所有权的情况下并发等待，
    // 失败时仍能对剩余 handle 调 `abort`。
    let mut pending = FuturesUnordered::new();
    for (index, handle) in handles.iter_mut().enumerate() {
        pending.push(async move { (index, handle.await) });
    }

    let failure = loop {
        let next = match deadline {
            Some((deadline, timeout)) => {
                match tokio::time::timeout_at(deadline, pending.next()).await {
                    Ok(next) => next,
                    Err(_) => {
                        break Some(format!(
                            "未完成开局：开发者未进房并准备，或对局未结束（{} 秒）",
                            timeout.as_secs()
                        ));
                    }
                }
            }
            None => pending.next().await,
        };
        let (index, joined) = match next {
            Some(item) => item,
            // 全部结束。缺口由末尾的逐项检查报告。
            None => break None,
        };
        match joined {
            Ok(Ok(outcome)) => outcomes[index] = Some(outcome),
            Ok(Err(err)) => break Some(format!("bot{index} 失败: {err}")),
            Err(err) => break Some(format!("bot{index} 任务异常: {err}")),
        }
    };
    drop(pending);

    if let Some(message) = failure {
        abort_all(handles);
        return Err(message);
    }
    outcomes
        .into_iter()
        .enumerate()
        .map(|(index, outcome)| outcome.ok_or_else(|| format!("bot{index} 未返回结果")))
        .collect()
}

/// 取消仍然存活的机器人任务。对已结束的任务 `abort` 是空操作。
fn abort_all(handles: &mut Vec<JoinHandle<Result<BotOutcome, String>>>) {
    for handle in handles.iter() {
        handle.abort();
    }
    handles.clear();
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{BotOutcome, take_heroes, wait_all};

    #[test]
    fn take_heroes_fails_when_catalog_has_too_few_heroes() {
        let error = take_heroes(&std::collections::BTreeMap::new(), 7).expect_err("空卡表应报错");
        assert!(error.contains("至少需要 7 张"), "实际：{error}");
    }

    #[tokio::test]
    async fn wait_all_reports_the_failing_bot_without_waiting_for_the_timeout() {
        let mut handles = vec![
            tokio::spawn(async { Err("连接被服务端关闭".to_string()) }),
            tokio::spawn(async {
                std::future::pending::<()>().await;
                Ok(BotOutcome {
                    turns: 0,
                    ranking: Vec::new(),
                })
            }),
        ];
        let started = Instant::now();
        let error = wait_all(&mut handles, Some(Duration::from_secs(30)))
            .await
            .expect_err("bot0 失败时应收敛为错误");
        assert!(
            error.contains("bot0"),
            "错误应指出失败的机器人，实际：{error}"
        );
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "不应等到 30 秒超时才返回，实际耗时 {:?}",
            started.elapsed()
        );
    }

    #[tokio::test]
    async fn wait_all_returns_every_outcome_in_order() {
        let mut handles = Vec::new();
        for turns in 1..=3_u8 {
            handles.push(tokio::spawn(async move {
                Ok(BotOutcome {
                    turns,
                    ranking: Vec::new(),
                })
            }));
        }
        let outcomes = wait_all(&mut handles, Some(Duration::from_secs(30)))
            .await
            .expect("全部成功时应返回结果");
        let turns: Vec<u8> = outcomes.iter().map(|outcome| outcome.turns).collect();
        assert_eq!(turns, vec![1, 2, 3], "结果应按机器人序号排列");
    }

    #[tokio::test]
    async fn wait_all_without_timeout_waits_for_every_bot() {
        let mut handles = Vec::new();
        for turns in 1..=3_u8 {
            handles.push(tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(20)).await;
                Ok(BotOutcome {
                    turns,
                    ranking: Vec::new(),
                })
            }));
        }
        let started = Instant::now();
        let outcomes = wait_all(&mut handles, None)
            .await
            .expect("未设总时限时也应等全部机器人返回");
        let turns: Vec<u8> = outcomes.iter().map(|outcome| outcome.turns).collect();
        assert_eq!(turns, vec![1, 2, 3], "结果应按机器人序号排列");
        assert!(
            started.elapsed() >= Duration::from_millis(20),
            "应等待机器人真正返回，实际耗时 {:?}",
            started.elapsed()
        );
    }
}
