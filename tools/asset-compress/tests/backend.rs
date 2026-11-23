use asset_compress::backend::{self, resolve_backend};
use asset_compress::cli::{Backend, Quality};

#[test]
fn auto_without_gpu_exits_3() {
    let e = resolve_backend(Backend::Auto, Quality::Standard, false).unwrap_err();
    assert_eq!(e, 3);
}

#[test]
fn cpu_standard_maps_to_exact_with_warning() {
    let p = resolve_backend(Backend::Cpu, Quality::Standard, false).unwrap();
    assert!(p.used_cpu_fallback && p.quality == Quality::Exact);
}

#[test]
fn gpu_standard_keeps_hybrid() {
    let p = resolve_backend(Backend::Gpu, Quality::Standard, true).unwrap();
    assert!(!p.used_cpu_fallback && p.quality == Quality::Standard);
}

#[test]
fn exact_runs_on_cpu_without_gpu() {
    let p = resolve_backend(Backend::Auto, Quality::Exact, false).unwrap();
    assert!(!p.used_cpu_fallback && p.quality == Quality::Exact);
    let p = resolve_backend(Backend::Cpu, Quality::Exact, false).unwrap();
    assert!(!p.used_cpu_fallback && p.quality == Quality::Exact);
}

#[test]
fn cpu_fast_also_maps_to_exact() {
    let p = resolve_backend(Backend::Cpu, Quality::Fast, false).unwrap();
    assert!(p.used_cpu_fallback && p.quality == Quality::Exact);
}

#[test]
fn explicit_gpu_without_adapter_fails() {
    assert_eq!(resolve_backend(Backend::Gpu, Quality::Fast, false).unwrap_err(), 3);
    assert_eq!(resolve_backend(Backend::Gpu, Quality::Exact, false).unwrap_err(), 3);
}

#[test]
fn backend_is_exported_as_module() {
    // 计划里的调用形式 `backend::resolve_backend` 必须可用。
    let _ = backend::resolve_backend(Backend::Auto, Quality::Exact, false).unwrap();
}
