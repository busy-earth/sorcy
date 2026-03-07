use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::model::{ManagedRepo, ManagedRepoStatus, RepoCache, SourceRepo};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepoUpdateStrategy {
    MissingOnly,
    FetchIfPresent,
}

impl Default for RepoUpdateStrategy {
    fn default() -> Self {
        Self::MissingOnly
    }
}

impl RepoUpdateStrategy {
    pub fn parse(value: &str) -> Option<Self> {
        let normalized = value.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "missing-only" | "missing_only" | "missingonly" => Some(Self::MissingOnly),
            "fetch-if-present" | "fetch_if_present" | "fetchifpresent" => {
                Some(Self::FetchIfPresent)
            }
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::MissingOnly => "missing-only",
            Self::FetchIfPresent => "fetch-if-present",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RepoManagerConfig {
    pub cache_dir: PathBuf,
    pub update_strategy: RepoUpdateStrategy,
}

impl Default for RepoManagerConfig {
    fn default() -> Self {
        Self {
            cache_dir: default_repo_cache_dir(),
            update_strategy: RepoUpdateStrategy::MissingOnly,
        }
    }
}

pub trait GitRunner: Send + Sync {
    fn run(&self, args: &[String]) -> Result<(), String>;
}

#[derive(Debug, Default)]
pub struct SystemGitRunner;

impl GitRunner for SystemGitRunner {
    fn run(&self, args: &[String]) -> Result<(), String> {
        let output = Command::new("git")
            .args(args)
            .output()
            .map_err(|err| format!("failed to execute git: {err}"))?;
        if output.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let message = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("git exited with status {}", output.status)
        };
        Err(message)
    }
}

pub struct RepoManager {
    config: RepoManagerConfig,
    git_runner: Box<dyn GitRunner>,
}

impl RepoManager {
    pub fn new(config: RepoManagerConfig) -> Self {
        Self::with_git_runner(config, Box::<SystemGitRunner>::default())
    }

    pub fn with_git_runner(config: RepoManagerConfig, git_runner: Box<dyn GitRunner>) -> Self {
        Self { config, git_runner }
    }

    pub fn cache_root(&self) -> &Path {
        &self.config.cache_dir
    }

    pub fn metadata_path(&self) -> PathBuf {
        self.config.cache_dir.join("index.json")
    }

    pub fn local_repo_path(&self, source_repo: &SourceRepo) -> PathBuf {
        self.repos_root()
            .join(sanitize_path_component(&source_repo.host))
            .join(sanitize_path_component(&source_repo.owner))
            .join(sanitize_path_component(&source_repo.repo))
    }

    pub fn load_managed_repos(&self) -> Result<Vec<ManagedRepo>, String> {
        let index = self.load_index()?;
        Ok(index
            .entries
            .into_iter()
            .map(ManagedRepo::from)
            .collect::<Vec<_>>())
    }

    pub fn materialize(&self, source_repo: &SourceRepo) -> ManagedRepo {
        let local_path = self.local_repo_path(source_repo);
        let mut repo = ManagedRepo {
            normalized_source_url: source_repo.normalized_source_url.clone(),
            host: source_repo.host.clone(),
            owner: source_repo.owner.clone(),
            repo: source_repo.repo.clone(),
            local_path: local_path.clone(),
            status: ManagedRepoStatus::Missing,
            last_materialized_unix_seconds: Some(unix_timestamp_now()),
            error_message: None,
        };

        if let Err(err) = self.ensure_cache_layout(&local_path) {
            repo.status = ManagedRepoStatus::Failed;
            repo.error_message = Some(err);
            let _ = self.persist_repo_state(&repo);
            return repo;
        }

        let exists = local_path.join(".git").exists() || local_path.exists();
        if !exists {
            let args = vec![
                "clone".to_string(),
                source_repo.normalized_source_url.clone(),
                local_path.display().to_string(),
            ];
            repo.status = match self.git_runner.run(&args) {
                Ok(()) => ManagedRepoStatus::Cloned,
                Err(err) => {
                    repo.error_message = Some(err);
                    ManagedRepoStatus::Failed
                }
            };
        } else {
            match self.config.update_strategy {
                RepoUpdateStrategy::MissingOnly => {
                    repo.status = ManagedRepoStatus::Unchanged;
                }
                RepoUpdateStrategy::FetchIfPresent => {
                    let args = vec![
                        "-C".to_string(),
                        local_path.display().to_string(),
                        "fetch".to_string(),
                        "--all".to_string(),
                        "--tags".to_string(),
                    ];
                    repo.status = match self.git_runner.run(&args) {
                        Ok(()) => ManagedRepoStatus::Updated,
                        Err(err) => {
                            repo.error_message = Some(err);
                            ManagedRepoStatus::Failed
                        }
                    };
                }
            }
        }

        if let Err(err) = self.persist_repo_state(&repo) {
            repo.status = ManagedRepoStatus::Failed;
            repo.error_message = Some(err);
        }

        repo
    }

    pub fn cache_summary(&self, materialized: &[ManagedRepo]) -> RepoCache {
        let mut cloned = 0usize;
        let mut updated = 0usize;
        let mut unchanged = 0usize;
        let mut failed = 0usize;
        for item in materialized {
            match item.status {
                ManagedRepoStatus::Missing => {}
                ManagedRepoStatus::Cloned => cloned += 1,
                ManagedRepoStatus::Updated => updated += 1,
                ManagedRepoStatus::Unchanged => unchanged += 1,
                ManagedRepoStatus::Failed => failed += 1,
            }
        }
        let total_managed_repos = self
            .load_index()
            .map(|index| index.entries.len())
            .unwrap_or(0);
        RepoCache {
            root_path: self.config.cache_dir.clone(),
            metadata_path: self.metadata_path(),
            total_managed_repos,
            cloned_count: cloned,
            updated_count: updated,
            unchanged_count: unchanged,
            failed_count: failed,
        }
    }

    fn ensure_cache_layout(&self, local_repo_path: &Path) -> Result<(), String> {
        fs::create_dir_all(self.repos_root())
            .map_err(|err| format!("failed creating cache root: {err}"))?;
        let Some(parent) = local_repo_path.parent() else {
            return Err("failed computing repo parent path".to_string());
        };
        fs::create_dir_all(parent).map_err(|err| format!("failed creating repo directory: {err}"))
    }

    fn repos_root(&self) -> PathBuf {
        self.config.cache_dir.join("repos")
    }

    fn persist_repo_state(&self, repo: &ManagedRepo) -> Result<(), String> {
        fs::create_dir_all(&self.config.cache_dir)
            .map_err(|err| format!("failed creating cache dir: {err}"))?;
        let mut index = self.load_index().unwrap_or_default();
        let entry = RepoCacheEntry::from(repo.clone());
        if let Some(existing) = index
            .entries
            .iter_mut()
            .find(|item| item.normalized_source_url == entry.normalized_source_url)
        {
            *existing = entry;
        } else {
            index.entries.push(entry);
        }
        index
            .entries
            .sort_by(|a, b| a.normalized_source_url.cmp(&b.normalized_source_url));
        let payload = serde_json::to_string_pretty(&index)
            .map_err(|err| format!("failed serializing repo metadata: {err}"))?;
        fs::write(self.metadata_path(), format!("{payload}\n"))
            .map_err(|err| format!("failed writing repo metadata: {err}"))
    }

    fn load_index(&self) -> Result<RepoCacheIndex, String> {
        let path = self.metadata_path();
        if !path.exists() {
            return Ok(RepoCacheIndex::default());
        }
        let payload = fs::read_to_string(&path)
            .map_err(|err| format!("failed reading repo metadata {}: {err}", path.display()))?;
        serde_json::from_str::<RepoCacheIndex>(&payload)
            .map_err(|err| format!("failed parsing repo metadata {}: {err}", path.display()))
    }
}

pub fn default_repo_cache_dir() -> PathBuf {
    if let Some(xdg_cache_home) = env::var_os("XDG_CACHE_HOME") {
        return PathBuf::from(xdg_cache_home).join("sorcy");
    }
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home).join(".cache").join("sorcy");
    }
    PathBuf::from(".sorcy")
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct RepoCacheIndex {
    entries: Vec<RepoCacheEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RepoCacheEntry {
    normalized_source_url: String,
    host: String,
    owner: String,
    repo: String,
    local_path: PathBuf,
    status: ManagedRepoStatus,
    last_materialized_unix_seconds: Option<u64>,
    error_message: Option<String>,
}

impl From<ManagedRepo> for RepoCacheEntry {
    fn from(value: ManagedRepo) -> Self {
        Self {
            normalized_source_url: value.normalized_source_url,
            host: value.host,
            owner: value.owner,
            repo: value.repo,
            local_path: value.local_path,
            status: value.status,
            last_materialized_unix_seconds: value.last_materialized_unix_seconds,
            error_message: value.error_message,
        }
    }
}

impl From<RepoCacheEntry> for ManagedRepo {
    fn from(value: RepoCacheEntry) -> Self {
        Self {
            normalized_source_url: value.normalized_source_url,
            host: value.host,
            owner: value.owner,
            repo: value.repo,
            local_path: value.local_path,
            status: value.status,
            last_materialized_unix_seconds: value.last_materialized_unix_seconds,
            error_message: value.error_message,
        }
    }
}

fn sanitize_path_component(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-' {
            output.push(ch);
        } else {
            output.push('_');
        }
    }
    if output.is_empty() {
        "_".to_string()
    } else {
        output
    }
}

fn unix_timestamp_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
