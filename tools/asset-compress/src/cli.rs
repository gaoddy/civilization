//! 命令行参数解析。
//!
//! 退出码约定：0 成功、1 质量门禁失败、2 参数错误、3 无可用 GPU 且未显式允许 CPU。

use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quality {
    Fast,
    Standard,
    Exact,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    Auto,
    Gpu,
    Cpu,
}

#[derive(Debug)]
pub struct Args {
    pub input: PathBuf,
    pub quality: Quality,
    pub backend: Backend,
    pub jobs: usize,
    pub check: bool,
}

pub fn parse_args(argv: &[String]) -> Result<Args, String> {
    let mut input = None;
    let mut quality = Quality::Standard;
    let mut backend = Backend::Auto;
    let mut jobs = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let mut check = false;

    let mut it = argv.iter();
    while let Some(k) = it.next() {
        match k.as_str() {
            "--input" => input = Some(PathBuf::from(it.next().ok_or("--input 缺少值")?)),
            "--quality" => {
                let v = it.next().ok_or("--quality 缺少值")?;
                quality = match v.as_str() {
                    "fast" => Quality::Fast,
                    "standard" => Quality::Standard,
                    "exact" => Quality::Exact,
                    _ => return Err(format!("未知 quality：{v}（可选 fast/standard/exact）")),
                };
            }
            "--backend" => {
                let v = it.next().ok_or("--backend 缺少值")?;
                backend = match v.as_str() {
                    "auto" => Backend::Auto,
                    "gpu" => Backend::Gpu,
                    "cpu" => Backend::Cpu,
                    _ => return Err(format!("未知 backend：{v}（可选 auto/gpu/cpu）")),
                };
            }
            "--jobs" => {
                let v = it.next().ok_or("--jobs 缺少值")?;
                jobs = v.parse().map_err(|_| format!("jobs 必须是正整数：{v}"))?;
                if jobs == 0 {
                    return Err("jobs 必须是正整数".into());
                }
            }
            "--check" => check = true,
            x => return Err(format!("未知参数：{x}")),
        }
    }

    let input = input.ok_or("缺少 --input")?;
    Ok(Args {
        input,
        quality,
        backend,
        jobs,
        check,
    })
}
