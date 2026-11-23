//! 后端语义：把 `--quality` / `--backend` 解析成实际执行计划。
//!
//! - `--backend cpu`：统一走 CPU exact（`fast`/`standard` 打印警告后降级）。
//! - `--backend gpu`：必须有 GPU adapter，否则报错退出。
//! - `--backend auto`：有 GPU 用它；没有 GPU 时只有 `exact` 可跑，
//!   `fast`/`standard` 直接报错（不静默降级）。

use crate::cli::{Backend, Quality};

/// 退出码：3 = 无可用 GPU 且未显式选择 CPU。
pub type ExitCode = u8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Plan {
    pub quality: Quality,
    /// 因 `--backend cpu` 把 fast/standard 降级为 exact。
    pub used_cpu_fallback: bool,
}

pub fn resolve_backend(backend: Backend, quality: Quality, gpu_ok: bool) -> Result<Plan, ExitCode> {
    match backend {
        Backend::Cpu => Ok(Plan {
            quality: Quality::Exact,
            used_cpu_fallback: quality != Quality::Exact,
        }),
        Backend::Gpu => {
            if gpu_ok {
                Ok(Plan {
                    quality,
                    used_cpu_fallback: false,
                })
            } else {
                Err(3)
            }
        }
        Backend::Auto => {
            if gpu_ok || quality == Quality::Exact {
                Ok(Plan {
                    quality,
                    used_cpu_fallback: false,
                })
            } else {
                Err(3)
            }
        }
    }
}
