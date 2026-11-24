//! 单房间编排：1 个建房机器人 + 7 个加入机器人，开局后等待整局结束或停止信号。
//!
//! 开局顺序与 `sim dev` / 真实客户端一致：全员 `Ready` → 建房机器人（房主）
//! `StartGame` → 全员 `HeroPickStarted` 选英雄 → `GameStarted`。选英雄与房主
//! 开始都由 [`client::run_with`] 在同一连接循环里完成，本模块只等 8 条
//! `GameStarted` 事件。
//!
//! 停止信号触发后机器人发送 `LeaveRoom`：对局中按弃权结算，存活数降到 1 时
//! 服务端直接进入 `GameOver` 并销毁房间，不依赖 30 秒断线宽限。

use std::sync::Arc;

use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinSet;
use tokio::time::{self, Duration, Instant};

use crate::bot::client::{self, BotConfig, BotCounters, BotEvent, BotFinish, BotOptions};

/// 房间规模固定为 8 人。
const SEATS: usize = 8;
/// 建房后取房间码的超时。
const ROOM_CODE_TIMEOUT: Duration = Duration::from_secs(10);
/// 失败或停止后等待机器人离房的宽限。
const TEARDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// 一个房间的编排参数。
pub struct RoomSpec {
    /// WebSocket 地址。
    pub url: String,
    /// 全局房间序号；用于生成唯一手机号与机器人名。
    pub index: usize,
    /// 从建房到全员 `GameStarted` 的允许时长。
    pub start_timeout_ms: u64,
    /// 机器人意图种子。
    pub intent_seed: u64,
    /// 建房机器人首选英雄；加入机器人按服务端候选回退。
    pub hero: String,
    /// 是否关闭机器人逐条错误输出。
    pub quiet: bool,
    /// 全局高频计数器。
    pub counters: Arc<BotCounters>,
}

/// 一个房间的运行结果。
#[derive(Debug, Default)]
pub struct RoomResult {
    /// 房间序号；决定本房间 8 个机器人的手机号与展示名。
    pub index: usize,
    /// 8 个机器人是否全部收到 `GameStarted`。
    pub started: bool,
    /// 从房间任务开始到全员开局的毫秒数。
    pub start_latency_ms: Option<u64>,
    /// 正常收到 `GameOver` 的机器人数。
    pub finished_bots: usize,
    /// 启动或运行中的失败；`started` 为假时代表开局失败。
    pub failures: Vec<String>,
    /// 是否被压测停止信号中止。
    pub cancelled: bool,
}

/// 创建停止通道并启动房间任务；调用方持有发送端，可在压测到点时停止该房间。
pub fn spawn(spec: RoomSpec) -> (watch::Sender<bool>, tokio::task::JoinHandle<RoomResult>) {
    let (stop_tx, stop_rx) = watch::channel(false);
    let sender = stop_tx.clone();
    let task = tokio::spawn(run(spec, stop_rx, stop_tx));
    (sender, task)
}

/// 运行一个房间，直到全员打完、被停止或开局失败。
async fn run(
    spec: RoomSpec,
    stop: watch::Receiver<bool>,
    stop_tx: watch::Sender<bool>,
) -> RoomResult {
    let started_at = Instant::now();
    let base = spec.index.saturating_mul(SEATS);
    let phone = |seat: usize| format!("139{:08}", base + seat);
    let name = |seat: usize| format!("ld{}", base + seat);

    let (events_tx, mut events_rx) = mpsc::unbounded_channel();
    let (code_tx, code_rx) = oneshot::channel();
    let mut bots: JoinSet<Result<BotFinish, String>> = JoinSet::new();
    let mut result = RoomResult {
        index: spec.index,
        ..RoomResult::default()
    };

    bots.spawn(client::run_with(
        BotConfig {
            url: spec.url.clone(),
            room: String::new(),
            name: name(0),
            hero: spec.hero.clone(),
            create: true,
            intent_seed: spec.intent_seed,
            room_code_sink: Some(code_tx),
        },
        BotOptions {
            events: Some(events_tx.clone()),
            stop: Some(stop.clone()),
            phone: Some(phone(0)),
            counters: Some(spec.counters.clone()),
            quiet: spec.quiet,
        },
    ));

    let code = tokio::select! {
        _ = wait_stop(stop.clone()) => {
            result.cancelled = true;
            teardown(&mut bots, &stop_tx).await;
            return result;
        }
        waited = time::timeout(ROOM_CODE_TIMEOUT, code_rx) => match waited {
            Ok(Ok(code)) => code,
            Ok(Err(_)) => {
                let message = collect_failures(&mut bots).await;
                result.failures.push(format!("建房失败: {message}"));
                teardown(&mut bots, &stop_tx).await;
                return result;
            }
            Err(_) => {
                result.failures.push("建房超时".into());
                teardown(&mut bots, &stop_tx).await;
                return result;
            }
        },
    };

    for seat in 1..SEATS {
        bots.spawn(client::run_with(
            BotConfig {
                url: spec.url.clone(),
                room: code.clone(),
                name: name(seat),
                hero: spec.hero.clone(),
                create: false,
                intent_seed: spec.intent_seed.saturating_add(seat as u64),
                room_code_sink: None,
            },
            BotOptions {
                events: Some(events_tx.clone()),
                stop: Some(stop.clone()),
                phone: Some(phone(seat)),
                counters: Some(spec.counters.clone()),
                quiet: spec.quiet,
            },
        ));
    }
    drop(events_tx);

    // 阶段一：等 8 个 `GameStarted`；任何机器人先退出都视为开局失败。
    let deadline = started_at + Duration::from_millis(spec.start_timeout_ms);
    let mut started_count = 0_usize;
    while started_count < SEATS {
        if Instant::now() >= deadline {
            result
                .failures
                .push(format!("{}ms 内未全员开局", spec.start_timeout_ms));
            teardown(&mut bots, &stop_tx).await;
            return result;
        }
        tokio::select! {
            _ = wait_stop(stop.clone()) => {
                result.cancelled = true;
                teardown(&mut bots, &stop_tx).await;
                return result;
            }
            event = events_rx.recv() => match event {
                Some(BotEvent::GameStarted) => started_count += 1,
                Some(BotEvent::Failed { message }) => {
                    result.failures.push(message);
                    teardown(&mut bots, &stop_tx).await;
                    return result;
                }
                Some(BotEvent::Finished { .. }) => {
                    result.failures.push("开局前收到 GameOver".into());
                    teardown(&mut bots, &stop_tx).await;
                    return result;
                }
                None => {
                    result.failures.push("事件通道提前关闭".into());
                    teardown(&mut bots, &stop_tx).await;
                    return result;
                }
                _ => {}
            },
            joined = bots.join_next() => match joined {
                Some(Ok(Ok(BotFinish::Left))) => {
                    result.cancelled = true;
                    teardown(&mut bots, &stop_tx).await;
                    return result;
                }
                Some(Ok(Err(message))) => {
                    result.failures.push(message);
                    teardown(&mut bots, &stop_tx).await;
                    return result;
                }
                Some(Ok(Ok(BotFinish::GameOver(_)))) => {
                    result.failures.push("开局前有机器人打完一局".into());
                    teardown(&mut bots, &stop_tx).await;
                    return result;
                }
                Some(Err(error)) => {
                    result.failures.push(format!("机器人任务异常: {error}"));
                    teardown(&mut bots, &stop_tx).await;
                    return result;
                }
                None => {
                    result.failures.push("全部机器人提前退出".into());
                    teardown(&mut bots, &stop_tx).await;
                    return result;
                }
            },
        }
    }

    result.started = true;
    result.start_latency_ms = u64::try_from(started_at.elapsed().as_millis()).ok();

    // 阶段二：等整局结束；停止信号触发时统一下发 `LeaveRoom`。
    loop {
        if bots.is_empty() {
            break;
        }
        tokio::select! {
            _ = wait_stop(stop.clone()) => {
                result.cancelled = true;
                teardown(&mut bots, &stop_tx).await;
                return result;
            }
            joined = bots.join_next() => match joined {
                Some(Ok(Ok(BotFinish::GameOver(_)))) => result.finished_bots += 1,
                Some(Ok(Ok(BotFinish::Left))) => result.cancelled = true,
                Some(Ok(Err(message))) => result.failures.push(message),
                Some(Err(error)) => result.failures.push(format!("机器人任务异常: {error}")),
                None => break,
            },
        }
    }
    result
}

/// 等待停止信号；已置位时立即返回。
async fn wait_stop(mut receiver: watch::Receiver<bool>) {
    if *receiver.borrow() {
        return;
    }
    let _ = receiver.changed().await;
}

/// 通知所有机器人离房，等待它们退出；超时后强制中止。
async fn teardown(bots: &mut JoinSet<Result<BotFinish, String>>, stop_tx: &watch::Sender<bool>) {
    let _ = stop_tx.send(true);
    let deadline = Instant::now() + TEARDOWN_TIMEOUT;
    while !bots.is_empty() {
        match time::timeout_at(deadline, bots.join_next()).await {
            Ok(Some(_)) => {}
            Ok(None) => break,
            Err(_) => break,
        }
    }
    bots.abort_all();
    while bots.join_next().await.is_some() {}
}

/// 收集已退出的机器人错误；用于建房机器人在取码前失败时取回原因。
async fn collect_failures(bots: &mut JoinSet<Result<BotFinish, String>>) -> String {
    let Some(joined) = bots.join_next().await else {
        return "建房机器人提前退出".into();
    };
    match joined {
        Ok(Ok(BotFinish::GameOver(_))) => "建房机器人提前打完一局".into(),
        Ok(Ok(BotFinish::Left)) => "建房机器人收到停止信号".into(),
        Ok(Err(message)) => message,
        Err(error) => format!("建房机器人任务异常: {error}"),
    }
}
