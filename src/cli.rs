use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use crate::resolve::RegistryConfig;
use crate::run_with_config;

#[derive(Debug, Parser)]
#[command(
    name = "sorcy",
    about = "Scan dependency files and output dependency source URLs."
)]
pub struct Args {
    #[arg(default_value = ".")]
    pub path: PathBuf,

    #[arg(short, long)]
    pub output: Option<PathBuf>,

    #[arg(long)]
    pub pretty: bool,

    #[arg(long, default_value = "https://pypi.org/pypi")]
    pub pypi_base_url: String,

    #[arg(long, default_value = "https://registry.npmjs.org")]
    pub npm_base_url: String,

    #[arg(long, default_value = "https://crates.io/api/v1/crates")]
    pub crates_base_url: String,
}

pub fn run_cli(args: Args) -> Result<()> {
    let config = RegistryConfig {
        pypi_base_url: args.pypi_base_url,
        npm_base_url: args.npm_base_url,
        crates_base_url: args.crates_base_url,
    };
    let records = run_with_config(&args.path, config)?;

    let json = if args.pretty {
        serde_json::to_string_pretty(&records)?
    } else {
        serde_json::to_string(&records)?
    };

    if let Some(output) = args.output {
        fs::write(&output, format!("{json}\n"))
            .with_context(|| format!("failed writing {}", output.display()))?;
    } else {
        println!("{json}");
    }

    Ok(())
}
