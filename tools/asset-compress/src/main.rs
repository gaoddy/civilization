use asset_compress::cli;

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let code = match cli::parse_args(&argv) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("{e}");
            2
        }
    };
    std::process::exit(code);
}
