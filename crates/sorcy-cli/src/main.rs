use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use sorcy_core::repo::RepoUpdateStrategy;
use sorcy_core::resolve::RegistryConfig;
use sorcy_core::settings::{Settings, SettingsOverrides};

#[derive(Debug, Parser)]
#[command(
    name = "sorcy",
    about = "Scan dependency files and output dependency source URLs."
)]
struct Args {
    #[arg(default_value = ".")]
    path: PathBuf,

    #[arg(short, long)]
    output: Option<PathBuf>,

    #[arg(long)]
    pretty: bool,

    #[arg(long)]
    pypi_base_url: Option<String>,

    #[arg(long)]
    npm_base_url: Option<String>,

    #[arg(long)]
    crates_base_url: Option<String>,

    #[arg(long)]
    http_timeout_seconds: Option<u64>,

    #[arg(long)]
    http_retries: Option<usize>,

    #[arg(long)]
    http_retry_backoff_ms: Option<u64>,

    #[arg(long)]
    materialize: bool,

    #[arg(long, requires = "materialize")]
    materialize_rich: bool,

    #[arg(long)]
    repo_cache_dir: Option<PathBuf>,

    #[arg(long, value_enum)]
    repo_update_strategy: Option<CliRepoUpdateStrategy>,
}

#[derive(Clone, Debug, ValueEnum)]
enum CliRepoUpdateStrategy {
    MissingOnly,
    FetchIfPresent,
}

impl CliRepoUpdateStrategy {
    fn into_core(self) -> RepoUpdateStrategy {
        match self {
            Self::MissingOnly => RepoUpdateStrategy::MissingOnly,
            Self::FetchIfPresent => RepoUpdateStrategy::FetchIfPresent,
        }
    }
}

fn main() {
    let args = Args::parse();
    if let Err(err) = run_cli(args) {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn run_cli(args: Args) -> Result<()> {
    let settings = Settings::resolve(SettingsOverrides {
        pypi_base_url: args.pypi_base_url,
        npm_base_url: args.npm_base_url,
        crates_base_url: args.crates_base_url,
        http_timeout_seconds: args.http_timeout_seconds,
        http_retries: args.http_retries,
        http_retry_backoff_ms: args.http_retry_backoff_ms,
        repo_cache_dir: args.repo_cache_dir,
        repo_update_strategy: args
            .repo_update_strategy
            .map(CliRepoUpdateStrategy::into_core),
    })?;
    let json = if args.materialize {
        let materialization = sorcy_core::materialize_project_with_config(
            &args.path,
            sorcy_core::SorcyConfig::from_settings(settings),
        )?;
        if args.materialize_rich {
            if args.pretty {
                serde_json::to_string_pretty(&materialization)?
            } else {
                serde_json::to_string(&materialization)?
            }
        } else {
            let records = sorcy_core::compatibility_records(&materialization.project_scan);
            if args.pretty {
                serde_json::to_string_pretty(&records)?
            } else {
                serde_json::to_string(&records)?
            }
        }
    } else {
        let config = RegistryConfig {
            pypi_base_url: settings.registry.pypi_base_url,
            npm_base_url: settings.registry.npm_base_url,
            crates_base_url: settings.registry.crates_base_url,
            http_timeout_seconds: settings.http.timeout_seconds,
            http_retries: settings.http.retries,
            http_retry_backoff_ms: settings.http.retry_backoff_ms,
        };
        let records = sorcy_core::run_with_config(&args.path, config)?;
        if args.pretty {
            serde_json::to_string_pretty(&records)?
        } else {
            serde_json::to_string(&records)?
        }
    };

    if let Some(output) = args.output {
        fs::write(&output, format!("{json}\n"))
            .with_context(|| format!("failed writing {}", output.display()))?;
    } else {
        println!("{json}");
    }

    Ok(())
}
