use asset_compress::{Backend, Quality};

fn parse(args: &[&str]) -> Result<asset_compress::Args, String> {
    let owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    asset_compress::cli::parse_args(&owned)
}

#[test]
fn parses_quality_and_backend() {
    let a = parse(&[
        "--input", "client/assets/cards/faces",
        "--quality", "standard",
        "--backend", "gpu",
    ])
    .unwrap();
    assert_eq!(a.quality, Quality::Standard);
    assert_eq!(a.backend, Backend::Gpu);
}

#[test]
fn rejects_unknown_quality() {
    let e = parse(&["--input", "x", "--quality", "ultra"]).unwrap_err();
    assert!(e.contains("quality"), "{e}");
}

#[test]
fn default_quality_is_standard_and_backend_auto() {
    let a = parse(&["--input", "x"]).unwrap();
    assert_eq!(a.quality, Quality::Standard);
    assert_eq!(a.backend, Backend::Auto);
    assert!(!a.check);
    assert!(a.jobs >= 1);
}
