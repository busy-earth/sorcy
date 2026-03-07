use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use crate::resolve::RegistryConfig;
use crate::run_with_config;
use crate::settings::{Settings, SettingsOverrides};

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

    #[arg(long)]
    pub pypi_base_url: Option<String>,

    #[arg(long)]
    pub npm_base_url: Option<String>,

    #[arg(long)]
    pub crates_base_url: Option<String>,

    #[arg(long)]
    pub http_timeout_seconds: Option<u64>,

    #[arg(long)]
    pub http_retries: Option<usize>,

    #[arg(long)]
    pub http_retry_backoff_ms: Option<u64>,
}

pub fn run_cli(args: Args) -> Result<()> {
    let settings = Settings::resolve(SettingsOverrides {
        pypi_base_url: args.pypi_base_url,
        npm_base_url: args.npm_base_url,
        crates_base_url: args.crates_base_url,
        http_timeout_seconds: args.http_timeout_seconds,
        http_retries: args.http_retries,
        http_retry_backoff_ms: args.http_retry_backoff_ms,
    })?;

    let config = RegistryConfig {
        pypi_base_url: settings.registry.pypi_base_url,
        npm_base_url: settings.registry.npm_base_url,
        crates_base_url: settings.registry.crates_base_url,
        http_timeout_seconds: settings.http.timeout_seconds,
        http_retries: settings.http.retries,
        http_retry_backoff_ms: settings.http.retry_backoff_ms,
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
