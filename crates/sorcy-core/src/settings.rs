use std::env;
use std::path::PathBuf;

use anyhow::{anyhow, Result};

use crate::repo::{default_repo_cache_dir, RepoUpdateStrategy};

#[derive(Debug, Clone)]
pub struct HttpSettings {
    pub timeout_seconds: u64,
    pub retries: usize,
    pub retry_backoff_ms: u64,
}

#[derive(Debug, Clone)]
pub struct RegistrySettings {
    pub pypi_base_url: String,
    pub npm_base_url: String,
    pub crates_base_url: String,
}

#[derive(Debug, Clone)]
pub struct RepoSettings {
    pub cache_dir: PathBuf,
    pub update_strategy: RepoUpdateStrategy,
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub registry: RegistrySettings,
    pub http: HttpSettings,
    pub repo: RepoSettings,
}

#[derive(Debug, Clone, Default)]
pub struct SettingsOverrides {
    pub pypi_base_url: Option<String>,
    pub npm_base_url: Option<String>,
    pub crates_base_url: Option<String>,
    pub http_timeout_seconds: Option<u64>,
    pub http_retries: Option<usize>,
    pub http_retry_backoff_ms: Option<u64>,
    pub repo_cache_dir: Option<PathBuf>,
    pub repo_update_strategy: Option<RepoUpdateStrategy>,
}

impl Settings {
    pub fn resolve(overrides: SettingsOverrides) -> Result<Self> {
        Self::resolve_with_env(overrides, |key| env::var(key).ok())
    }

    fn resolve_with_env(
        overrides: SettingsOverrides,
        get_env: impl Fn(&str) -> Option<String>,
    ) -> Result<Self> {
        let pypi_base_url = pick_string(
            overrides.pypi_base_url,
            &get_env,
            "SORCY_PYPI_BASE_URL",
            "https://pypi.org/pypi",
        );
        let npm_base_url = pick_string(
            overrides.npm_base_url,
            &get_env,
            "SORCY_NPM_BASE_URL",
            "https://registry.npmjs.org",
        );
        let crates_base_url = pick_string(
            overrides.crates_base_url,
            &get_env,
            "SORCY_CRATES_BASE_URL",
            "https://crates.io/api/v1/crates",
        );

        let timeout_seconds = pick_u64(
            overrides.http_timeout_seconds,
            &get_env,
            "SORCY_HTTP_TIMEOUT_SECONDS",
            10,
        )?;
        let retries = pick_usize(overrides.http_retries, &get_env, "SORCY_HTTP_RETRIES", 3)?;
        let retry_backoff_ms = pick_u64(
            overrides.http_retry_backoff_ms,
            &get_env,
            "SORCY_HTTP_RETRY_BACKOFF_MS",
            120,
        )?;
        let repo_cache_dir = pick_pathbuf(
            overrides.repo_cache_dir,
            &get_env,
            "SORCY_REPO_CACHE_DIR",
            default_repo_cache_dir(),
        );
        let repo_update_strategy = pick_repo_update_strategy(
            overrides.repo_update_strategy,
            &get_env,
            "SORCY_REPO_UPDATE_STRATEGY",
            RepoUpdateStrategy::MissingOnly,
        )?;

        Ok(Self {
            registry: RegistrySettings {
                pypi_base_url,
                npm_base_url,
                crates_base_url,
            },
            http: HttpSettings {
                timeout_seconds,
                // Keep at least one attempt so callers can safely use 1..=retries.
                retries: retries.max(1),
                retry_backoff_ms,
            },
            repo: RepoSettings {
                cache_dir: repo_cache_dir,
                update_strategy: repo_update_strategy,
            },
        })
    }
}

fn pick_string(
    override_value: Option<String>,
    get_env: &impl Fn(&str) -> Option<String>,
    env_key: &str,
    default: &str,
) -> String {
    if let Some(value) = override_value {
        return value;
    }
    if let Some(value) = get_env(env_key) {
        return value;
    }
    default.to_string()
}

fn pick_u64(
    override_value: Option<u64>,
    get_env: &impl Fn(&str) -> Option<String>,
    env_key: &str,
    default: u64,
) -> Result<u64> {
    if let Some(value) = override_value {
        return Ok(value);
    }
    if let Some(value) = get_env(env_key) {
        return value
            .parse::<u64>()
            .map_err(|_| anyhow!("invalid value for {env_key}: {value}"));
    }
    Ok(default)
}

fn pick_pathbuf(
    override_value: Option<PathBuf>,
    get_env: &impl Fn(&str) -> Option<String>,
    env_key: &str,
    default: PathBuf,
) -> PathBuf {
    if let Some(value) = override_value {
        return value;
    }
    if let Some(value) = get_env(env_key) {
        return PathBuf::from(value);
    }
    default
}

fn pick_usize(
    override_value: Option<usize>,
    get_env: &impl Fn(&str) -> Option<String>,
    env_key: &str,
    default: usize,
) -> Result<usize> {
    if let Some(value) = override_value {
        return Ok(value);
    }
    if let Some(value) = get_env(env_key) {
        return value
            .parse::<usize>()
            .map_err(|_| anyhow!("invalid value for {env_key}: {value}"));
    }
    Ok(default)
}

fn pick_repo_update_strategy(
    override_value: Option<RepoUpdateStrategy>,
    get_env: &impl Fn(&str) -> Option<String>,
    env_key: &str,
    default: RepoUpdateStrategy,
) -> Result<RepoUpdateStrategy> {
    if let Some(value) = override_value {
        return Ok(value);
    }
    if let Some(value) = get_env(env_key) {
        return RepoUpdateStrategy::parse(&value)
            .ok_or_else(|| anyhow!("invalid value for {env_key}: {value}"));
    }
    Ok(default)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use super::{Settings, SettingsOverrides};
    use crate::repo::RepoUpdateStrategy;

    #[test]
    fn uses_defaults_when_no_overrides_or_env() {
        let settings = Settings::resolve_with_env(SettingsOverrides::default(), |_key| None)
            .expect("settings resolve");
        assert_eq!(settings.registry.pypi_base_url, "https://pypi.org/pypi");
        assert_eq!(settings.registry.npm_base_url, "https://registry.npmjs.org");
        assert_eq!(
            settings.registry.crates_base_url,
            "https://crates.io/api/v1/crates"
        );
        assert_eq!(settings.http.timeout_seconds, 10);
        assert_eq!(settings.http.retries, 3);
        assert_eq!(settings.http.retry_backoff_ms, 120);
        assert_eq!(
            settings.repo.update_strategy,
            RepoUpdateStrategy::MissingOnly
        );
    }

    #[test]
    fn cli_overrides_take_precedence_over_env() {
        let mut env_map = HashMap::new();
        env_map.insert("SORCY_HTTP_RETRIES".to_string(), "9".to_string());

        let settings = Settings::resolve_with_env(
            SettingsOverrides {
                http_retries: Some(2),
                ..SettingsOverrides::default()
            },
            |key| env_map.get(key).cloned(),
        )
        .expect("settings resolve");

        assert_eq!(settings.http.retries, 2);
    }

    #[test]
    fn reads_env_when_cli_override_not_present() {
        let mut env_map = HashMap::new();
        env_map.insert(
            "SORCY_PYPI_BASE_URL".to_string(),
            "http://localhost:7777/pypi".to_string(),
        );
        env_map.insert("SORCY_HTTP_TIMEOUT_SECONDS".to_string(), "25".to_string());

        let settings = Settings::resolve_with_env(SettingsOverrides::default(), |key| {
            env_map.get(key).cloned()
        })
        .expect("settings resolve");

        assert_eq!(
            settings.registry.pypi_base_url,
            "http://localhost:7777/pypi"
        );
        assert_eq!(settings.http.timeout_seconds, 25);
    }

    #[test]
    fn invalid_numeric_env_returns_error() {
        let mut env_map = HashMap::new();
        env_map.insert("SORCY_HTTP_RETRIES".to_string(), "nope".to_string());

        let err = Settings::resolve_with_env(SettingsOverrides::default(), |key| {
            env_map.get(key).cloned()
        })
        .expect_err("expected invalid env parse error");

        assert!(err.to_string().contains("SORCY_HTTP_RETRIES"));
    }

    #[test]
    fn repo_overrides_take_precedence_and_env_strategy_is_parsed() {
        let mut env_map = HashMap::new();
        env_map.insert(
            "SORCY_REPO_UPDATE_STRATEGY".to_string(),
            "fetch-if-present".to_string(),
        );

        let settings = Settings::resolve_with_env(
            SettingsOverrides {
                repo_cache_dir: Some(PathBuf::from("/tmp/custom-sorcy-cache")),
                ..SettingsOverrides::default()
            },
            |key| env_map.get(key).cloned(),
        )
        .expect("settings resolve");

        assert_eq!(
            settings.repo.cache_dir,
            PathBuf::from("/tmp/custom-sorcy-cache")
        );
        assert_eq!(
            settings.repo.update_strategy,
            RepoUpdateStrategy::FetchIfPresent
        );
    }
}
