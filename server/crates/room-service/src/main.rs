//! room-service 命令行入口：启动 WebSocket 房间服务。

use std::process::ExitCode;
use std::sync::Arc;

use room_service::{Db, SystemClock, bind, bind_with_db, serve};

const DEFAULT_LISTEN: &str = "127.0.0.1:8080";

const USAGE: &str = "用法：room-service [--listen ADDR]
  --listen ADDR  监听地址，默认 127.0.0.1:8080（写 127.0.0.1:0 由系统分配端口）

环境变量：
  ROOM_SERVICE_LISTEN  未提供 --listen 时的监听地址，默认 127.0.0.1:8080
  DATABASE_URL   设置后启用持久化（账号、好友、邀请等）；未设置时使用内存存储。";

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
    let mut listen = std::env::var("ROOM_SERVICE_LISTEN")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_LISTEN.to_string());
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
    let database_url = std::env::var("DATABASE_URL").unwrap_or_default();
    if database_url.trim().is_empty() {
        println!("room-service: 未配置 DATABASE_URL，使用内存存储（重启后账号与好友丢失）");
        let (addr, router) = bind(&listen, clock).await?;
        println!("room-service: 监听 ws://{addr}/ws");
        return serve(addr, router).await;
    }

    let db = Arc::new(Db::connect(&database_url).await?);
    let (addr, router) = bind_with_db(&listen, clock, db).await?;
    println!("room-service: 监听 ws://{addr}/ws（已连接数据库）");
    serve(addr, router).await
}
