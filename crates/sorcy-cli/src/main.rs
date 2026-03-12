use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args as ClapArgs, Parser, Subcommand, ValueEnum};
use sorcy_core::repo::RepoUpdateStrategy;
use sorcy_core::settings::{Settings, SettingsOverrides};

#[derive(Debug, Parser)]
#[command(
    name = "sorcy",
    about = "Scan dependency files and output dependency source URLs.",
    subcommand_precedence_over_arg = true
)]
struct CliArgs {
    #[command(subcommand)]
    command: Option<CliCommand>,

    #[command(flatten)]
    scan: ScanArgs,
}

#[derive(Debug, Subcommand)]
enum CliCommand {
    InstallSkill(InstallSkillArgs),
}

#[derive(Debug, ClapArgs)]
struct InstallSkillArgs {
    #[arg(long)]
    global: bool,
}

#[derive(Debug, ClapArgs)]
struct ScanArgs {
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
    let args = CliArgs::parse();
    if let Err(err) = run_cli(args) {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn run_cli(args: CliArgs) -> Result<()> {
    if let Some(CliCommand::InstallSkill(install_args)) = args.command {
        return run_install_skill(install_args);
    }
    run_scan(args.scan)
}

fn run_install_skill(args: InstallSkillArgs) -> Result<()> {
    let project_root =
        std::env::current_dir().context("failed to resolve current working directory")?;
    let scope = if args.global {
        sorcy_core::SkillInstallScope::Global
    } else {
        sorcy_core::SkillInstallScope::ProjectLocal
    };
    let installed = sorcy_core::install_sorcy_rank_skill(&project_root, scope)?;
    println!(
        "Installed sorcy-rank skill at {}",
        installed.target_dir.display()
    );
    Ok(())
}

fn run_scan(args: ScanArgs) -> Result<()> {
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
    let config = sorcy_core::SorcyConfig::from_settings(settings);
    let json = if args.materialize {
        let materialization = sorcy_core::materialize_project_with_config(&args.path, config)?;
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
        let records = sorcy_core::run_with_config(&args.path, config.registry)?;
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

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{CliArgs, CliCommand};

    #[test]
    fn parses_default_scan_mode_without_subcommand() {
        let args = CliArgs::parse_from(["sorcy", "."]);
        assert!(args.command.is_none());
        assert_eq!(args.scan.path.to_string_lossy(), ".");
    }

    #[test]
    fn parses_install_skill_subcommand() {
        let args = CliArgs::parse_from(["sorcy", "install-skill", "--global"]);
        match args.command {
            Some(CliCommand::InstallSkill(install_args)) => assert!(install_args.global),
            _ => panic!("expected install-skill command"),
        }
    }
}
