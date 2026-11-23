use asset_compress::backend::resolve_backend;
use asset_compress::cli::{self, Quality};
use asset_compress::encode::{CpuEncoder, Encoder, GpuEncoder, StandardEncoder};
use asset_compress::gate;
use asset_compress::{Format, run};

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let code = match cli::parse_args(&argv) {
        Ok(args) => execute(args),
        Err(e) => {
            eprintln!("{e}");
            2
        }
    };
    std::process::exit(code);
}

fn execute(args: cli::Args) -> i32 {
    let gpu = GpuEncoder::new();
    let plan = match resolve_backend(args.backend, args.quality, gpu.is_ok()) {
        Ok(p) => p,
        Err(code) => {
            eprintln!(
                "没有可用的 GPU adapter：GPU 编码需要 Metal。无 GPU 环境请改用 --quality exact --backend cpu。"
            );
            return code as i32;
        }
    };
    if plan.used_cpu_fallback {
        eprintln!("警告：--backend cpu 下 --quality {} 降级为 exact（质量不降，速度更慢）。", quality_name(args.quality));
    }

    let enc: Box<dyn Encoder> = match plan.quality {
        Quality::Exact => Box::new(CpuEncoder::new(args.jobs)),
        _ => match gpu {
            Ok(g) => Box::new(StandardEncoder::from_gpu(g, args.jobs)),
            Err(e) => {
                eprintln!("{e}");
                return 3;
            }
        },
    };

    if args.check {
        return check(&*enc, &args);
    }

    match run::run_all(&*enc, &args.input, args.jobs) {
        Ok(r) => {
            println!(
                "encoded={} skipped={} wall={:.1}s",
                r.encoded,
                r.skipped,
                r.wall.as_secs_f64()
            );
            0
        }
        Err(e) => {
            eprintln!("{e}");
            2
        }
    }
}

fn quality_name(q: Quality) -> &'static str {
    match q {
        Quality::Fast => "fast",
        Quality::Standard => "standard",
        Quality::Exact => "exact",
    }
}

/// `--check`：取输入目录的前 60 张图跑质量门禁。BC7 判门限，ASTC 只报告。
fn check(enc: &dyn Encoder, args: &cli::Args) -> i32 {
    let files = run::find_pngs(&args.input);
    let mut sample = Vec::new();
    for f in files.iter().take(60) {
        match run::decode_png(f) {
            Ok(d) => sample.push(d),
            Err(e) => {
                eprintln!("{}: {e}", f.display());
                return 2;
            }
        }
    }
    if sample.is_empty() {
        eprintln!("{} 下没有 PNG", args.input.display());
        return 2;
    }
    let refs: Vec<(&[u8], u32, u32)> = sample
        .iter()
        .map(|d| (d.rgba.as_slice(), d.width, d.height))
        .collect();
    match gate::check_quality(enc, Format::Bc7, &refs) {
        Ok(r) => println!("BC7 mean={:.2} min={:.2} n={}", r.mean, r.min, r.count),
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    }
    match gate::measure(enc, Format::Astc4x4, &refs) {
        Ok(r) => println!("ASTC mean={:.2} min={:.2} n={}（仅报告）", r.mean, r.min, r.count),
        Err(e) => {
            eprintln!("ASTC 统计失败：{e}");
            return 1;
        }
    }
    0
}
