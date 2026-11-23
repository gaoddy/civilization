//! 本机 room-service 进程管理：起停、就绪探测与日志落盘。
//!
//! 以 `--listen 127.0.0.1:0` 启动，从 stdout 的就绪行取真实地址；两个输出管道都
//! 持续 drain 到日志文件，避免服务端 forfeit 日志写满管道后阻塞。

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// 就绪行等待上限。
const READY_TIMEOUT: Duration = Duration::from_secs(15);

/// 由压测器管理的 room-service 子进程。
pub struct ServerProcess {
    bin: PathBuf,
    log_path: PathBuf,
    child: Child,
    url: String,
}

impl ServerProcess {
    /// 启动服务并等到 stdout 输出监听地址。
    ///
    /// # Errors
    ///
    /// 二进制无法启动、进程在就绪前退出，或 15 秒内没有就绪行。
    pub fn spawn(bin: &Path, log_path: &Path) -> Result<Self, String> {
        if let Some(parent) = log_path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)
                .map_err(|err| format!("创建日志目录 {} 失败: {err}", parent.display()))?;
        }
        let mut child = Command::new(bin)
            .args(["--listen", "127.0.0.1:0"])
            .env_remove("DATABASE_URL")
            .env_remove("ROOM_SERVICE_LISTEN")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| format!("启动 {} 失败: {err}", bin.display()))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "无法读取服务端 stdout".to_string())?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "无法读取服务端 stderr".to_string())?;
        let (ready_tx, ready_rx) = mpsc::channel();

        spawn_drain(stdout, log_path.to_path_buf(), Some(ready_tx));
        spawn_drain(stderr, log_path.to_path_buf(), None);

        let url = match ready_rx.recv_timeout(READY_TIMEOUT) {
            Ok(Ok(url)) => url,
            Ok(Err(message)) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("{message}\n{}", readable_log(log_path)));
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "{} 秒内未读到服务端就绪行\n{}",
                    READY_TIMEOUT.as_secs(),
                    readable_log(log_path)
                ));
            }
        };

        Ok(Self {
            bin: bin.to_path_buf(),
            log_path: log_path.to_path_buf(),
            child,
            url,
        })
    }

    /// 服务端进程号。
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// WebSocket 地址，例如 `ws://127.0.0.1:54321/ws`。
    pub fn url(&self) -> &str {
        &self.url
    }

    /// 终止当前进程并重新启动；用于 `--fresh-server-per-step`。
    ///
    /// # Errors
    ///
    /// 同 [`ServerProcess::spawn`]。
    pub fn restart(&mut self) -> Result<(), String> {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let mut fresh = Self::spawn(&self.bin, &self.log_path)?;
        std::mem::swap(self, &mut fresh);
        Ok(())
    }
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// 探测 WebSocket 就绪：成功握手一次即可。
///
/// # Errors
///
/// 2 秒内握手不成功。
pub async fn probe(url: &str) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match tokio_tungstenite::connect_async(url).await {
            Ok((mut ws, _response)) => {
                let _ = ws.close(None).await;
                return Ok(());
            }
            Err(error) => {
                if Instant::now() >= deadline {
                    return Err(format!("服务端就绪探测失败（{url}）: {error}"));
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    }
}

/// 从一行输出里取 `ws://…/ws`。
fn parse_ready_line(line: &str) -> Option<String> {
    let start = line.find("ws://")?;
    let rest = &line[start..];
    let end = rest
        .find(|ch: char| ch.is_whitespace() || ch == '（')
        .unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

/// 持续读取子进程输出并写入日志；`ready` 为 `Some` 时把就绪地址发回。
fn spawn_drain<R: std::io::Read + Send + 'static>(
    reader: R,
    log_path: PathBuf,
    ready: Option<mpsc::Sender<Result<String, String>>>,
) {
    std::thread::spawn(move || {
        let mut log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .ok();
        let mut ready = ready;
        for line in BufReader::new(reader).lines() {
            let Ok(line) = line else { break };
            if let Some(file) = log.as_mut() {
                let _ = writeln!(file, "{line}");
            }
            if let Some(sender) = ready.take() {
                match parse_ready_line(&line) {
                    Some(url) => {
                        let _ = sender.send(Ok(url));
                    }
                    None if line.is_empty() => ready = Some(sender),
                    None => {
                        // 非就绪行继续等，直到进程退出；把发送端留在手里。
                        ready = Some(sender);
                    }
                }
            }
        }
        if let Some(sender) = ready {
            let _ = sender.send(Err("服务端在输出监听地址前退出".to_string()));
        }
    });
}

/// 读取日志末尾，用于启动失败时附在错误里。
fn readable_log(path: &Path) -> String {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let tail: Vec<&str> = text.lines().rev().take(20).collect();
    tail.into_iter().rev().collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use super::parse_ready_line;

    #[test]
    fn parses_ready_line_with_and_without_database() {
        assert_eq!(
            parse_ready_line("room-service: 监听 ws://127.0.0.1:8080/ws"),
            Some("ws://127.0.0.1:8080/ws".to_string())
        );
        assert_eq!(
            parse_ready_line("room-service: 监听 ws://127.0.0.1:9000/ws（已连接数据库）"),
            Some("ws://127.0.0.1:9000/ws".to_string())
        );
        assert_eq!(parse_ready_line("别的输出"), None);
    }
}
