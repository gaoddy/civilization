//! 服务端与压测器自身的资源采样：本机用 `ps`，远端经 ssh 读 `/proc`。
//!
//! CPU 统一取累计 CPU 时间，相邻两点差分除以墙钟得到「用了几个核」；
//! `ps` 的 `%cpu` 是生命周期平均值，不能当瞬时值使用。

use std::process::Command;

use serde::Serialize;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::{self, Duration};

/// 采样目标。
#[derive(Debug, Clone)]
pub enum ProcessTarget {
    /// 本机进程号。
    Local(u32),
    /// 远端主机上的进程号；经 ssh 读 `/proc`。
    Ssh { host: String, pid: u32 },
}

/// 采样配置。
#[derive(Debug, Clone)]
pub struct SampleConfig {
    /// 服务端进程；未配置时不采服务端指标。
    pub server: Option<ProcessTarget>,
    /// 压测器自身进程号；用于记录生成器开销。
    pub generator_pid: Option<u32>,
    /// 采样间隔，至少 200ms。
    pub interval_ms: u64,
}

/// 单次采样。
#[derive(Debug, Clone, Copy)]
pub struct ProcessSample {
    /// 常驻内存，单位 KB。
    pub rss_kb: u64,
    /// 累计 CPU 时间，单位秒。
    pub cpu_seconds: f64,
}

/// 一次压测档位内的全部采样。
#[derive(Debug, Default, Clone)]
pub struct Samples {
    /// 服务端采样点。
    pub server: Vec<ProcessSample>,
    /// 压测器采样点。
    pub generator: Vec<ProcessSample>,
}

/// 采样任务句柄；`finish` 停止并取回采样点。
pub struct Sampler {
    stop: oneshot::Sender<()>,
    task: JoinHandle<Samples>,
}

impl Sampler {
    /// 停止采样并等待任务返回。
    pub async fn finish(self) -> Samples {
        let _ = self.stop.send(());
        self.task.await.unwrap_or_default()
    }
}

/// 启动后台采样任务。
pub fn spawn(config: SampleConfig) -> Sampler {
    let (stop, mut stopped) = oneshot::channel();
    let interval = Duration::from_millis(config.interval_ms.max(200));
    let task = tokio::spawn(async move {
        let mut samples = Samples::default();
        loop {
            if let Some(target) = &config.server
                && let Some(sample) = sample_target(target).await
            {
                samples.server.push(sample);
            }
            if let Some(pid) = config.generator_pid
                && let Some(sample) = sample_local(pid)
            {
                samples.generator.push(sample);
            }
            tokio::select! {
                _ = &mut stopped => break,
                _ = time::sleep(interval) => {}
            }
        }
        samples
    });
    Sampler { stop, task }
}

/// 按目标类型采样；失败返回 `None`（例如进程已退出或远端缺 `/proc`）。
async fn sample_target(target: &ProcessTarget) -> Option<ProcessSample> {
    match target {
        ProcessTarget::Local(pid) => sample_local(*pid),
        ProcessTarget::Ssh { host, pid } => {
            let host = host.clone();
            let pid = *pid;
            tokio::task::spawn_blocking(move || sample_ssh(&host, pid))
                .await
                .ok()
                .flatten()
        }
    }
}

/// 本机 `ps` 采样。
fn sample_local(pid: u32) -> Option<ProcessSample> {
    let output = Command::new("ps")
        .args(["-o", "rss=", "-o", "time=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_ps(&String::from_utf8_lossy(&output.stdout))
}

/// 解析 `ps -o rss= -o time=` 的两列输出。
fn parse_ps(text: &str) -> Option<ProcessSample> {
    let mut fields = text.split_whitespace();
    let rss_kb = fields.next()?.parse().ok()?;
    let cpu_seconds = parse_cpu_time(fields.next()?)?;
    Some(ProcessSample {
        rss_kb,
        cpu_seconds,
    })
}

/// 解析 `ps` 的累计 CPU 时间：`0:00.00`、`12:34.56`、`1:02:03`、`2-03:04:05.67`。
fn parse_cpu_time(text: &str) -> Option<f64> {
    let (days, rest) = match text.split_once('-') {
        Some((days, rest)) => (days.parse::<u64>().ok()?, rest),
        None => (0_u64, text),
    };
    let parts: Vec<&str> = rest.split(':').collect();
    let (hours, minutes, seconds) = match parts.as_slice() {
        [hours, minutes, seconds] => (
            hours.parse::<u64>().ok()?,
            minutes.parse::<u64>().ok()?,
            seconds.parse::<f64>().ok()?,
        ),
        [minutes, seconds] => (
            0,
            minutes.parse::<u64>().ok()?,
            seconds.parse::<f64>().ok()?,
        ),
        [seconds] => (0, 0, seconds.parse::<f64>().ok()?),
        _ => return None,
    };
    Some(days as f64 * 86_400.0 + hours as f64 * 3_600.0 + minutes as f64 * 60.0 + seconds)
}

/// 远端经 ssh 读 `/proc`；一次连接取时钟频率、状态与 stat。
fn sample_ssh(host: &str, pid: u32) -> Option<ProcessSample> {
    let script = format!("getconf CLK_TCK; cat /proc/{pid}/status; cat /proc/{pid}/stat");
    let output = Command::new("ssh")
        .args([
            "-o",
            "BatchMode=yes",
            "-o",
            "ConnectTimeout=5",
            host,
            &script,
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_proc(&String::from_utf8_lossy(&output.stdout))
}

/// 解析 `getconf CLK_TCK` + `/proc/pid/status` + `/proc/pid/stat` 的合并输出。
fn parse_proc(text: &str) -> Option<ProcessSample> {
    let mut lines = text.lines();
    let hz: f64 = lines.next()?.trim().parse().ok()?;
    let mut rss_kb = None;
    let mut stat_line = None;
    for line in lines {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            rss_kb = rest.split_whitespace().next()?.parse().ok();
        } else if line.ends_with(')') || line.contains(") ") {
            stat_line = Some(line);
        }
    }
    let stat_line = stat_line?;
    let fields: Vec<&str> = stat_line.rsplit_once(')')?.1.split_whitespace().collect();
    // `/proc/<pid>/stat`：comm 之后 state 为字段 3；utime/stime 为字段 14/15。
    let utime = fields.get(11)?.parse::<u64>().ok()?;
    let stime = fields.get(12)?.parse::<u64>().ok()?;
    Some(ProcessSample {
        rss_kb: rss_kb?,
        cpu_seconds: (utime.saturating_add(stime)) as f64 / hz,
    })
}

/// 采样汇总：RSS 峰值与 CPU 核数（相邻采样差分）。
#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessStats {
    /// RSS 峰值，单位 KB。
    pub rss_kb_peak: u64,
    /// 平均占用核数。
    pub cpu_cores_avg: f64,
    /// 峰值占用核数。
    pub cpu_cores_peak: f64,
}

/// 由采样点计算统计值；点数不足时 CPU 记 0。
///
/// `interval_ms` 是采样间隔：相邻累计 CPU 时间差除以间隔即占用核数。
pub fn stats(samples: &[ProcessSample], interval_ms: u64) -> ProcessStats {
    let rss_kb_peak = samples
        .iter()
        .map(|sample| sample.rss_kb)
        .max()
        .unwrap_or(0);
    let interval = (interval_ms.max(200) as f64) / 1000.0;
    let mut cores: Vec<f64> = Vec::new();
    for pair in samples.windows(2) {
        let delta_cpu = pair[1].cpu_seconds - pair[0].cpu_seconds;
        if delta_cpu < 0.0 {
            continue;
        }
        cores.push(delta_cpu / interval);
    }
    let cpu_cores_peak = cores.iter().copied().fold(0.0_f64, f64::max);
    let cpu_cores_avg = if cores.is_empty() {
        0.0
    } else {
        cores.iter().sum::<f64>() / cores.len() as f64
    };
    ProcessStats {
        rss_kb_peak,
        cpu_cores_avg,
        cpu_cores_peak,
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_cpu_time, parse_proc, parse_ps};

    #[test]
    fn parses_ps_cpu_time_formats() {
        assert!((parse_cpu_time("0:00.00").expect("应可解析") - 0.0).abs() < 1e-9);
        assert!((parse_cpu_time("1:02.50").expect("应可解析") - 62.5).abs() < 1e-9);
        assert!((parse_cpu_time("1:02:03").expect("应可解析") - 3723.0).abs() < 1e-9);
        assert!((parse_cpu_time("2-03:04:05.5").expect("应可解析") - 183_845.5).abs() < 1e-9);
        assert!(parse_cpu_time("bad").is_none());
    }

    #[test]
    fn parses_ps_columns() {
        let sample = parse_ps("  12345  1:02.50").expect("应可解析");
        assert_eq!(sample.rss_kb, 12_345);
        assert!((sample.cpu_seconds - 62.5).abs() < 1e-9);
    }

    #[test]
    fn parses_proc_remote_output() {
        let text = "100\nName:\troom-service\nVmRSS:\t  204800 kB\n1234 (room service) S 1 1234 1234 0 -1 4194560 100 0 0 0 250 150 0 0 20 0 4 0 1000 0 0\n";
        let sample = parse_proc(text).expect("应可解析");
        assert_eq!(sample.rss_kb, 204_800);
        assert!((sample.cpu_seconds - 4.0).abs() < 1e-9);
    }
}
