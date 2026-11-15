//! room-service 命令行入口。
//!
//! 本任务只解析 `--listen` 并打印监听意图；WebSocket 路由由后续任务接入。

use std::process::ExitCode;

const DEFAULT_LISTEN: &str = "127.0.0.1:8080";

const USAGE: &str = "用法：room-service [--listen ADDR]
  --listen ADDR  监听地址，默认 127.0.0.1:8080";

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("room-service: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
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
    println!("room-service: 准备监听 {listen}（WebSocket 路由尚未接入）");
    Ok(())
}
