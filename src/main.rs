use clap::Parser;
use sorcy::cli::{run_cli, Args};

fn main() {
    let args = Args::parse();
    if let Err(err) = run_cli(args) {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}
