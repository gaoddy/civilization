//! room-service 命令行入口：启动 WebSocket 房间服务。

use std::process::ExitCode;
use std::sync::Arc;

use room_service::{SystemClock, bind, serve};

const DEFAULT_LISTEN: &str = "127.0.0.1:8080";

const USAGE: &str = "用法：room-service [--listen ADDR]
  --listen ADDR  监听地址，默认 127.0.0.1:8080（写 127.0.0.1:0 由系统分配端口）";

#[tokio::main]
async fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("room-service: {message}");
            ExitCode::FAILURE
        }
    }
}

async fn run(args: Vec<String>) -> Result<(), String> {
    let mut listen = DEFAULT_LISTEN.to_string();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--listen" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "--listen 缺少地址参数".to_string())?;
                value
                    .parse::<std::net::SocketAddr>()
                    .map_err(|_| format!("无法解析监听地址：{value}"))?;
                listen = value;
            }
            "--help" | "-h" => {
                println!("{USAGE}");
                return Ok(());
            }
            other => return Err(format!("未知参数：{other}\n{USAGE}")),
        }
    }

    let clock: Arc<dyn room_service::Clock> = Arc::new(SystemClock);
    let (addr, router) = bind(&listen, clock).await?;
    println!("room-service: 监听 ws://{addr}/ws");
    serve(addr, router).await
}
