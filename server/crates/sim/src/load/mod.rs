//! 并发对局压测：steady（固定并发 + 补位）与 ramp（逐档找上限）。
//!
//! 压测器只通过 `sim` 的 bot 客户端打真实协议，不修改服务端行为。steady 维持
//! `rooms` 个活跃房间，结束补位；ramp 按档位递增并在首个不健康档停止。每档结束
//! 对全部房间发停止信号，机器人发送 `LeaveRoom` 让服务端回收房间。
//!
//! 本模块负责进程管理与资源采样，是 `sim`「确定性、不读时钟与环境」约束的唯一
//! 例外；规则引擎与 AI 路径不受影响。

pub mod proc;
pub mod room;
pub mod sample;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use serde::Serialize;
use tokio::sync::{mpsc, watch};
use tokio::time::{self, Duration, Instant};

use crate::bot::client::BotCounters;

/// 压测场景。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scenario {
    /// 固定并发，结束补位，持续指定时长。
    Steady,
    /// 逐档增加房间数，首个不健康档停止。
    Ramp,
}

impl Scenario {
    /// CLI 与报告使用的名字。
    pub fn as_str(self) -> &'static str {
        match self {
            Scenario::Steady => "steady",
            Scenario::Ramp => "ramp",
        }
    }
}

/// 远端服务端采样目标。
#[derive(Debug, Clone)]
pub struct RemoteSample {
    /// ssh 主机，例如 `root@dell.local`。
    pub host: String,
    /// 远端 room-service 进程号。
    pub pid: u32,
}

/// 压测配置。
#[derive(Debug, Clone)]
pub struct LoadConfig {
    /// 场景。
    pub scenario: Scenario,
    /// 已有服务端地址；与 `server_bin` 二选一。
    pub url: Option<String>,
    /// 本机 room-service 二进制；提供时由压测器起停并采样。
    pub server_bin: Option<PathBuf>,
    /// 本机服务端日志路径。
    pub server_log: PathBuf,
    /// steady 的并发房间数，或 ramp 的起始房间数。
    pub rooms: usize,
    /// ramp 的最大房间数。
    pub rooms_max: usize,
    /// ramp 每档增量。
    pub step: usize,
    /// steady 持续时长。
    pub duration_ms: u64,
    /// ramp 每档持续时长。
    pub step_ms: u64,
    /// 单个房间从建房到全员开局的允许时长。
    pub start_timeout_ms: u64,
    /// 资源采样间隔。
    pub sample_ms: u64,
    /// 机器人意图种子。
    pub intent_seed: u64,
    /// 建房机器人首选英雄卡 id。
    pub hero: String,
    /// 是否关闭机器人逐条错误输出。
    pub quiet: bool,
    /// 远端采样目标。
    pub remote_sample: Option<RemoteSample>,
    /// ramp 每档重启本机服务端，取得干净的内存基线。
    pub fresh_server_per_step: bool,
    /// 允许远端 ramp（账号与会话会在服务端累积）。
    pub allow_remote_ramp: bool,
    /// 启动失败率上限（千分比）。
    pub max_start_failure_permille: u32,
    /// 开局时延 p95 上限。
    pub max_start_p95_ms: u64,
}

/// 压测报告。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadReport {
    /// 场景名。
    pub scenario: &'static str,
    /// 压测器与服务端是否同机。
    pub co_hosted: bool,
    /// 本机服务端二进制路径。
    pub server_binary: Option<String>,
    /// 每档结果。
    pub steps: Vec<StepReport>,
    /// 上限判定。
    pub limit: LimitReport,
}

/// 单档结果。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepReport {
    /// 目标房间数。
    pub rooms: usize,
    /// `healthy` / `contended` / `unhealthy`。
    pub status: &'static str,
    /// 非健康时的原因。
    pub reason: Option<String>,
    /// 单档实际时长。
    pub elapsed_ms: u64,
    /// 同时存活的房间峰值。
    pub active_games_peak: usize,
    /// 发起的房间尝试总数（每个补位计一次）。
    pub attempts: usize,
    /// 达到全员开局的房间数。
    pub games_started: usize,
    /// 8 个机器人都收到 `GameOver` 的房间数。
    pub games_finished: usize,
    /// 启动失败房间数。
    pub start_failures: usize,
    /// 启动失败原因分布；键为归一化后的失败类别。
    pub failure_kinds: BTreeMap<String, usize>,
    /// 开局后运行期失败原因分布；与启动失败分开统计。
    pub runtime_failure_kinds: BTreeMap<String, usize>,
    /// 首次启动失败距本档开始的毫秒数。
    pub first_failure_ms: Option<u64>,
    /// 最近一次启动失败距本档开始的毫秒数。
    pub last_failure_ms: Option<u64>,
    /// 被停止信号中止的房间数。
    pub cancelled_rooms: usize,
    /// 开局时延。
    pub start_latency_ms: LatencyReport,
    /// 意图速率。
    pub intents_per_sec: f64,
    /// 服务端资源。
    pub server: sample::ProcessStats,
    /// 压测器资源。
    pub generator: sample::ProcessStats,
    /// 是否触及压测器与机器总容量。
    pub contended: bool,
    /// 服务端采样点数。
    pub server_samples: usize,
    /// 失败样本（最多 20 条）。
    pub failures: Vec<String>,
}

/// 开局时延分位。
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LatencyReport {
    /// p50。
    pub p50: Option<u64>,
    /// p95。
    pub p95: Option<u64>,
}

/// 上限判定。
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitReport {
    /// 是否找到不健康档。
    pub found: bool,
    /// 最后一个健康档的房间数。
    pub max_healthy_rooms: Option<usize>,
    /// 首个不健康档的房间数。
    pub first_unhealthy_rooms: Option<usize>,
    /// 停止爬坡的原因。
    pub reason: Option<String>,
}

/// 单档压测的原始计数。
#[derive(Debug, Default)]
struct ResidenceRaw {
    active_peak: usize,
    attempts: usize,
    started: usize,
    finished_rooms: usize,
    start_failures: usize,
    cancelled_rooms: usize,
    latencies: Vec<u64>,
    failures: Vec<String>,
    failure_kinds: BTreeMap<String, usize>,
    runtime_failure_kinds: BTreeMap<String, usize>,
    first_failure_ms: Option<u64>,
    last_failure_ms: Option<u64>,
    last_started_ms: Option<u64>,
    consecutive_failures: usize,
    last_failure_kind: Option<String>,
    aborted: Option<String>,
}

/// 连续启动失败达到该值、且距最近一次开局已超过 [`SUCCESS_STALL_MS`] 时判定后端不可达。
///
/// 后端断开时每次尝试只需毫秒级，不设上限会在几十秒内产生数百万次补位与数 GB 内存；
/// 该判定把「链路断了」与「服务端变慢」区分开，并立即结束本档。
const MAX_CONSECUTIVE_FAILURES: usize = 50;
/// 允许的连续失败时长；短于该时长的链路抖动由重试自行恢复。
const SUCCESS_STALL_MS: u64 = 10_000;

impl ResidenceRaw {
    fn record(&mut self, result: room::RoomResult, elapsed_ms: u64) {
        if result.started {
            self.started += 1;
            self.consecutive_failures = 0;
            self.last_started_ms = Some(elapsed_ms);
            if let Some(latency) = result.start_latency_ms {
                self.latencies.push(latency);
            }
            if result.finished_bots >= 8 {
                self.finished_rooms += 1;
            }
        } else if result.cancelled {
            self.cancelled_rooms += 1;
        } else {
            self.start_failures += 1;
            self.consecutive_failures += 1;
            self.first_failure_ms.get_or_insert(elapsed_ms);
            self.last_failure_ms = Some(elapsed_ms);
        }
        let kinds = if result.started {
            &mut self.runtime_failure_kinds
        } else {
            &mut self.failure_kinds
        };
        let index = result.index;
        for message in result.failures {
            let kind = failure_kind(&message);
            if !result.started {
                self.last_failure_kind = Some(kind.clone());
            }
            *kinds.entry(kind).or_default() += 1;
            if self.failures.len() < 20 {
                self.failures.push(format!("#{index} {message}"));
            }
        }
    }

    /// 判定后端是否已经不可达；一旦判定成立，本档不再补位。
    ///
    /// 连续失败达到阈值后，再看距最近一次开局（或本档开始）的时长：短于
    /// [`SUCCESS_STALL_MS`] 时允许链路抖动恢复，超过才判定不可达。
    fn unreachable_reason(&mut self, elapsed_ms: u64) -> Option<String> {
        if self.aborted.is_some() || self.consecutive_failures < MAX_CONSECUTIVE_FAILURES {
            return None;
        }
        let since_success = match self.last_started_ms {
            Some(started) => elapsed_ms.saturating_sub(started),
            None => elapsed_ms,
        };
        if since_success < SUCCESS_STALL_MS {
            return None;
        }
        let kind = self.last_failure_kind.as_deref().unwrap_or("未知原因");
        let reason = format!(
            "连续 {} 次启动失败、{}s 内无房间开局，判定后端不可达（最后失败：{kind}）",
            self.consecutive_failures,
            since_success / 1000
        );
        self.aborted = Some(reason.clone());
        Some(reason)
    }
}

/// 把失败消息归一化成可聚合的类别。
///
/// 只保留「阶段 + 错误码」或可稳定复现的底层原因，丢掉房间码、玩家名等逐次变化的部分；
/// 同一类失败在报告里合并计数，便于长窗口排查。
fn failure_kind(message: &str) -> String {
    // 建房失败由房间编排层加前缀；加入机器人的失败没有前缀，因此阶段是可选的。
    let (stage, rest) = match message.strip_prefix("建房失败: ") {
        Some(rest) => ("建房/", rest),
        None => ("", message),
    };
    for prefix in ["进入房间失败: ", "认证失败: "] {
        if let Some(detail) = rest.strip_prefix(prefix) {
            let code = detail.split_whitespace().next().unwrap_or(detail);
            let phase = prefix.trim_end_matches(": ");
            return format!("{stage}{phase}:{code}");
        }
    }
    if rest.starts_with("连接 ")
        && let Some((_, cause)) = rest.rsplit_once(" 失败: ")
    {
        // 建连失败要保留底层原因：拒绝连接说明端口/转发没了，超时说明链路黑洞。
        return format!("{stage}连接失败: {cause}");
    }
    if let Some(detail) = rest.strip_prefix("WebSocket 读取失败: ") {
        let cause = detail.rsplit(": ").next().unwrap_or(detail);
        return format!("{stage}WebSocket 读取: {cause}");
    }
    if let Some(detail) = rest.strip_prefix("WebSocket 发送失败: ") {
        let cause = detail.rsplit(": ").next().unwrap_or(detail);
        return format!("{stage}WebSocket 发送: {cause}");
    }
    if let Some(detail) = rest.strip_prefix("WebSocket 解析下行载荷失败: ") {
        return format!("{stage}下行载荷解析失败: {detail}");
    }
    if rest.ends_with("内未全员开局") {
        return "全员开局超时".into();
    }
    let trimmed: String = rest.chars().take(60).collect();
    format!("{stage}{trimmed}")
}

/// 生成 ramp 档位；非法参数返回错误。
///
/// # Errors
///
/// `rooms` 为 0、`step` 为 0，或 `rooms_max` 小于 `rooms`。
pub fn ramp_steps(rooms: usize, rooms_max: usize, step: usize) -> Result<Vec<usize>, String> {
    if rooms == 0 {
        return Err("--rooms 必须大于 0".into());
    }
    if step == 0 {
        return Err("ramp 的 --step 必须大于 0".into());
    }
    if rooms_max < rooms {
        return Err(format!(
            "--rooms-max（{rooms_max}）不能小于 --rooms（{rooms}）"
        ));
    }
    let mut steps = Vec::new();
    let mut current = rooms;
    while current <= rooms_max {
        steps.push(current);
        let next = current.saturating_add(step);
        if next == current {
            break;
        }
        current = next;
    }
    Ok(steps)
}

/// 按最近秩取分位；空切片返回 `None`。
///
/// # Errors
///
/// `permille` 不在 1–1000。
pub fn percentile(values: &[u64], permille: u32) -> Result<Option<u64>, String> {
    if !(1..=1000).contains(&permille) {
        return Err(format!("分位必须在 1–1000 之间，实际为 {permille}"));
    }
    if values.is_empty() {
        return Ok(None);
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let rank = (sorted.len() * permille as usize).div_ceil(1000);
    let index = rank.saturating_sub(1).min(sorted.len() - 1);
    Ok(Some(sorted[index]))
}

/// 执行压测。
///
/// # Errors
///
/// 本机服务端无法启动、缺少地址、ramp 参数非法，或远端 ramp 未显式允许。
pub async fn run(config: LoadConfig) -> Result<LoadReport, String> {
    let mut server = match &config.server_bin {
        Some(bin) => Some(proc::ServerProcess::spawn(bin, &config.server_log)?),
        None => None,
    };
    if let Some(server) = &server {
        proc::probe(server.url()).await?;
    }
    if config.scenario == Scenario::Ramp && server.is_none() && !config.allow_remote_ramp {
        return Err(
            "远端 ramp 需要 --allow-remote-ramp：房间会被清理，但账号与会话会在服务端累积".into(),
        );
    }

    let ladder = match config.scenario {
        Scenario::Steady => vec![config.rooms],
        Scenario::Ramp => ramp_steps(config.rooms, config.rooms_max, config.step)?,
    };
    let cores = std::thread::available_parallelism()
        .map(|value| value.get() as f64)
        .unwrap_or(1.0);
    let mut steps = Vec::with_capacity(ladder.len());
    let mut limit = LimitReport::default();

    for (index, &rooms) in ladder.iter().enumerate() {
        if config.fresh_server_per_step
            && let Some(server) = server.as_mut()
        {
            server.restart()?;
            proc::probe(server.url()).await?;
        }
        let url = match (&server, &config.url) {
            (Some(server), _) => server.url().to_string(),
            (None, Some(url)) => url.clone(),
            (None, None) => return Err("缺少服务端地址".into()),
        };
        let target = match (&server, &config.remote_sample) {
            (Some(server), _) => Some(sample::ProcessTarget::Local(server.pid())),
            (None, Some(remote)) => Some(sample::ProcessTarget::Ssh {
                host: remote.host.clone(),
                pid: remote.pid,
            }),
            (None, None) => None,
        };
        let sampler = sample::spawn(sample::SampleConfig {
            server: target,
            generator_pid: Some(std::process::id()),
            interval_ms: config.sample_ms,
        });
        let counters = Arc::new(BotCounters::default());
        let duration_ms = match config.scenario {
            Scenario::Steady => config.duration_ms,
            Scenario::Ramp => config.step_ms,
        };
        let began = Instant::now();
        let raw = run_residence(url, &config, rooms, duration_ms, Arc::clone(&counters)).await;
        let elapsed = began.elapsed().as_secs_f64().max(0.001);
        let samples = sampler.finish().await;

        let server_stats = sample::stats(&samples.server, config.sample_ms);
        let generator_stats = sample::stats(&samples.generator, config.sample_ms);
        let contended = server_stats.cpu_cores_avg + generator_stats.cpu_cores_avg > cores * 0.9;
        let p50 = percentile(&raw.latencies, 500).unwrap_or(None);
        let p95 = percentile(&raw.latencies, 950).unwrap_or(None);
        let (status, reason) = evaluate(&raw, p95, contended, &config);
        let intents = counters.intents_sent.load(Ordering::Relaxed);
        let step = StepReport {
            rooms,
            status,
            reason: reason.clone(),
            elapsed_ms: began.elapsed().as_millis().try_into().unwrap_or(u64::MAX),
            active_games_peak: raw.active_peak,
            attempts: raw.attempts,
            games_started: raw.started,
            games_finished: raw.finished_rooms,
            start_failures: raw.start_failures,
            failure_kinds: raw.failure_kinds.clone(),
            runtime_failure_kinds: raw.runtime_failure_kinds.clone(),
            first_failure_ms: raw.first_failure_ms,
            last_failure_ms: raw.last_failure_ms,
            cancelled_rooms: raw.cancelled_rooms,
            start_latency_ms: LatencyReport { p50, p95 },
            intents_per_sec: intents as f64 / elapsed,
            server: server_stats,
            generator: generator_stats,
            contended,
            server_samples: samples.server.len(),
            failures: raw.failures.clone(),
        };
        steps.push(step);

        if status != "healthy" {
            limit = LimitReport {
                found: true,
                max_healthy_rooms: if index == 0 {
                    None
                } else {
                    Some(ladder[index - 1])
                },
                first_unhealthy_rooms: Some(rooms),
                reason,
            };
            break;
        }
    }

    if !limit.found {
        limit.max_healthy_rooms = ladder.last().copied();
    }

    Ok(LoadReport {
        scenario: config.scenario.as_str(),
        co_hosted: server.is_some(),
        server_binary: config
            .server_bin
            .as_ref()
            .map(|path| path.display().to_string()),
        steps,
        limit,
    })
}

/// 健康判据：启动失败率、开局 p95 与生成器/机器容量。
fn evaluate(
    raw: &ResidenceRaw,
    p95: Option<u64>,
    contended: bool,
    config: &LoadConfig,
) -> (&'static str, Option<String>) {
    if let Some(reason) = &raw.aborted {
        return ("unhealthy", Some(reason.clone()));
    }
    if raw.attempts == 0 {
        return ("unhealthy", Some("未发起任何房间".into()));
    }
    let failure_permille =
        u32::try_from(raw.start_failures.saturating_mul(1000) / raw.attempts).unwrap_or(1000);
    if failure_permille > config.max_start_failure_permille {
        return (
            "unhealthy",
            Some(format!(
                "启动失败 {}/{}（{failure_permille}‰ > {}‰）",
                raw.start_failures, raw.attempts, config.max_start_failure_permille
            )),
        );
    }
    if raw.started == 0 {
        return ("unhealthy", Some("没有任何房间开局".into()));
    }
    if let Some(p95) = p95
        && p95 > config.max_start_p95_ms
    {
        return (
            "unhealthy",
            Some(format!(
                "开局时延 p95 {p95}ms > {}ms",
                config.max_start_p95_ms
            )),
        );
    }
    if contended {
        return (
            "contended",
            Some("server 与压测器 CPU 已接近机器核数，结果是生成器受限".into()),
        );
    }
    ("healthy", None)
}

/// 维持固定并发房间直到时长结束；结束后停止并等待全部房间离房。
async fn run_residence(
    url: String,
    config: &LoadConfig,
    rooms: usize,
    duration_ms: u64,
    counters: Arc<BotCounters>,
) -> ResidenceRaw {
    let (done_tx, mut done_rx) = mpsc::unbounded_channel();
    let mut stops: Vec<watch::Sender<bool>> = Vec::new();
    let mut outstanding = 0_usize;
    let mut raw = ResidenceRaw::default();
    let began = Instant::now();
    let deadline = began + Duration::from_millis(duration_ms);
    let mut stopped = false;

    loop {
        while !stopped && Instant::now() < deadline && outstanding < rooms {
            let spec = room::RoomSpec {
                url: url.clone(),
                index: raw.attempts,
                start_timeout_ms: config.start_timeout_ms,
                intent_seed: config.intent_seed.saturating_add(raw.attempts as u64),
                hero: config.hero.clone(),
                quiet: config.quiet,
                counters: Arc::clone(&counters),
            };
            let (stop, task) = room::spawn(spec);
            stops.push(stop);
            outstanding += 1;
            raw.attempts += 1;
            let done = done_tx.clone();
            tokio::spawn(async move {
                let result = task.await.unwrap_or_else(|error| room::RoomResult {
                    failures: vec![format!("房间任务异常: {error}")],
                    ..room::RoomResult::default()
                });
                let _ = done.send(result);
            });
        }
        raw.active_peak = raw.active_peak.max(outstanding);
        if outstanding == 0 {
            break;
        }

        if stopped || Instant::now() >= deadline {
            if !stopped {
                for stop in &stops {
                    let _ = stop.send(true);
                }
                stopped = true;
            }
            match done_rx.recv().await {
                Some(result) => {
                    outstanding -= 1;
                    let elapsed_ms = began.elapsed().as_millis().try_into().unwrap_or(u64::MAX);
                    raw.record(result, elapsed_ms);
                    if raw.unreachable_reason(elapsed_ms).is_some() && !stopped {
                        for stop in &stops {
                            let _ = stop.send(true);
                        }
                        stopped = true;
                    }
                }
                None => break,
            }
            continue;
        }

        tokio::select! {
            result = done_rx.recv() => match result {
                Some(result) => {
                    outstanding -= 1;
                    let elapsed_ms =
                        began.elapsed().as_millis().try_into().unwrap_or(u64::MAX);
                    raw.record(result, elapsed_ms);
                    if raw.unreachable_reason(elapsed_ms).is_some() && !stopped {
                        for stop in &stops {
                            let _ = stop.send(true);
                        }
                        stopped = true;
                    }
                }
                None => break,
            },
            _ = time::sleep_until(deadline) => {
                for stop in &stops {
                    let _ = stop.send(true);
                }
                stopped = true;
            }
        }
    }
    raw
}

#[cfg(test)]
mod tests {
    use super::{ResidenceRaw, failure_kind, percentile, ramp_steps, room};

    #[test]
    fn ramp_ladder_includes_bounds() {
        assert_eq!(
            ramp_steps(10, 40, 10).expect("应可生成"),
            vec![10, 20, 30, 40]
        );
        assert_eq!(ramp_steps(10, 35, 10).expect("应可生成"), vec![10, 20, 30]);
        assert_eq!(ramp_steps(5, 5, 10).expect("应可生成"), vec![5]);
    }

    #[test]
    fn ramp_rejects_invalid_arguments() {
        assert!(ramp_steps(0, 10, 1).is_err());
        assert!(ramp_steps(10, 10, 0).is_err());
        assert!(ramp_steps(20, 10, 1).is_err());
    }

    #[test]
    fn failure_kind_keeps_stage_and_code() {
        assert_eq!(
            failure_kind("建房失败: 进入房间失败: ALREADY_IN_ROOM 已在房间中"),
            "建房/进入房间失败:ALREADY_IN_ROOM"
        );
        assert_eq!(
            failure_kind("建房失败: 认证失败: INTERNAL_ERROR 存储不可用"),
            "建房/认证失败:INTERNAL_ERROR"
        );
        assert_eq!(
            failure_kind("进入房间失败: ROOM_FULL 房间已满"),
            "进入房间失败:ROOM_FULL"
        );
        assert_eq!(
            failure_kind(
                "建房失败: 连接 ws://127.0.0.1:18080/ws 失败: \
                 IO error: Connection refused (os error 61)"
            ),
            "建房/连接失败: IO error: Connection refused (os error 61)"
        );
        assert_eq!(
            failure_kind(
                "建房失败: WebSocket 读取失败: WebSocket protocol error: \
                 Connection reset without closing handshake"
            ),
            "建房/WebSocket 读取: Connection reset without closing handshake"
        );
        assert_eq!(
            failure_kind(
                "WebSocket 读取失败: WebSocket protocol error: \
                 Connection reset without closing handshake"
            ),
            "WebSocket 读取: Connection reset without closing handshake"
        );
        assert_eq!(failure_kind("10000ms 内未全员开局"), "全员开局超时");
    }

    #[test]
    fn consecutive_failures_only_abort_after_stall_window() {
        let failure = || room::RoomResult {
            failures: vec![
                "建房失败: 连接 ws://127.0.0.1:18080/ws 失败: \
                 IO error: Connection refused (os error 61)"
                    .into(),
            ],
            ..room::RoomResult::default()
        };
        let mut raw = ResidenceRaw::default();
        for _ in 0..super::MAX_CONSECUTIVE_FAILURES {
            raw.record(failure(), 500);
        }
        assert!(
            raw.unreachable_reason(9_999).is_none(),
            "抖动窗口内不应判定"
        );

        let mut raw = ResidenceRaw::default();
        for _ in 0..super::MAX_CONSECUTIVE_FAILURES {
            raw.record(failure(), 500);
        }
        let reason = raw
            .unreachable_reason(super::SUCCESS_STALL_MS)
            .expect("超过停摆窗口应判定后端不可达");
        assert!(reason.contains("后端不可达"), "{reason}");
        assert!(reason.contains("Connection refused"), "{reason}");
    }

    #[test]
    fn one_started_room_resets_consecutive_failures() {
        let mut raw = ResidenceRaw::default();
        for _ in 0..super::MAX_CONSECUTIVE_FAILURES {
            raw.record(
                room::RoomResult {
                    failures: vec!["建房失败: 连接 ws://x 失败: refused".into()],
                    ..room::RoomResult::default()
                },
                500,
            );
        }
        raw.record(
            room::RoomResult {
                started: true,
                start_latency_ms: Some(300),
                ..room::RoomResult::default()
            },
            12_000,
        );
        assert!(
            raw.unreachable_reason(60_000).is_none(),
            "有房间开局后不应判定不可达"
        );
    }

    #[test]
    fn percentile_uses_nearest_rank() {
        assert_eq!(percentile(&[], 950).expect("应可计算"), None);
        assert_eq!(
            percentile(&[5, 1, 3, 2, 4], 500).expect("应可计算"),
            Some(3)
        );
        assert_eq!(
            percentile(&[5, 1, 3, 2, 4], 950).expect("应可计算"),
            Some(5)
        );
    }
}
