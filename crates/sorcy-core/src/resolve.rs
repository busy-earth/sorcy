use std::thread::sleep;
use std::time::Duration;

use anyhow::Result;
use reqwest::{blocking::Client, StatusCode};
use serde_json::Value;

use crate::model::{DependencyRef, Ecosystem};

const KNOWN_FORGE_HOSTS: &[&str] = &[
    "github.com",
    "gitlab.com",
    "bitbucket.org",
    "codeberg.org",
    "git.sr.ht",
];
const KNOWN_FORGE_HOST_TOKENS: &[&str] = &[
    "github",
    "gitlab",
    "bitbucket",
    "codeberg",
    "sourcehut",
    "gitea",
    "forgejo",
];
const FORGE_PATH_CUT_MARKERS: &[&str] = &[
    "/-/",
    "/tree/",
    "/blob/",
    "/src/",
    "/raw/",
    "/issues/",
    "/pull/",
    "/pulls/",
    "/merge_requests/",
    "/commit/",
    "/commits/",
    "/release/",
    "/releases/",
    "/wiki/",
];

#[derive(Debug, Clone)]
pub struct RegistryConfig {
    pub pypi_base_url: String,
    pub npm_base_url: String,
    pub crates_base_url: String,
    pub http_timeout_seconds: u64,
    pub http_retries: usize,
    pub http_retry_backoff_ms: u64,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            pypi_base_url: "https://pypi.org/pypi".to_string(),
            npm_base_url: "https://registry.npmjs.org".to_string(),
            crates_base_url: "https://crates.io/api/v1/crates".to_string(),
            http_timeout_seconds: 10,
            http_retries: 3,
            http_retry_backoff_ms: 120,
        }
    }
}

pub struct RegistryResolver {
    client: Client,
    config: RegistryConfig,
}

pub trait SourceResolver {
    fn resolve(&self, dep: &DependencyRef) -> Option<String>;
}

impl RegistryResolver {
    pub fn new(config: RegistryConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.http_timeout_seconds))
            .user_agent("sorcy/0.2")
            .build()?;
        Ok(Self { client, config })
    }

    fn resolve_inner(&self, dep: &DependencyRef) -> Option<String> {
        if let Some(hint) = &dep.source_hint {
            return normalize_source_url(hint, true);
        }
        match dep.ecosystem {
            Ecosystem::Python => self.resolve_python(&dep.name),
            Ecosystem::Npm => self.resolve_npm(&dep.name),
            Ecosystem::Cargo => self.resolve_cargo(&dep.name),
            Ecosystem::Cpp => None,
        }
    }

    fn resolve_python(&self, name: &str) -> Option<String> {
        let url = format!(
            "{}/{}/json",
            self.config.pypi_base_url.trim_end_matches('/'),
            name
        );
        let payload = self.fetch_json(&url)?;
        let info = payload.get("info")?.as_object()?;

        let mut preferred = Vec::new();
        let mut fallback = Vec::new();

        if let Some(project_urls) = info.get("project_urls").and_then(Value::as_object) {
            for (label, value) in project_urls {
                let Some(candidate) = value.as_str() else {
                    continue;
                };
                let cleaned = candidate.trim();
                if cleaned.is_empty() || is_pypi_project_url(cleaned) {
                    continue;
                }
                if looks_preferred_label(label) {
                    preferred.push(cleaned.to_string());
                } else {
                    fallback.push(cleaned.to_string());
                }
            }
        }

        for key in ["home_page", "project_url"] {
            let Some(candidate) = info.get(key).and_then(Value::as_str) else {
                continue;
            };
            let cleaned = candidate.trim();
            if cleaned.is_empty() || is_pypi_project_url(cleaned) {
                continue;
            }
            fallback.push(cleaned.to_string());
        }

        for candidate in preferred {
            if let Some(url) = normalize_source_url(&candidate, true) {
                return Some(url);
            }
        }
        for candidate in fallback {
            if let Some(url) = normalize_source_url(&candidate, false) {
                return Some(url);
            }
        }
        None
    }

    fn resolve_npm(&self, name: &str) -> Option<String> {
        let encoded = name.replace('/', "%2F");
        let url = format!(
            "{}/{}",
            self.config.npm_base_url.trim_end_matches('/'),
            encoded
        );
        let payload = self.fetch_json(&url)?;

        for candidate in npm_source_candidates(&payload) {
            if let Some(url) = normalize_source_url(&candidate, true) {
                return Some(url);
            }
        }
        None
    }

    fn resolve_cargo(&self, name: &str) -> Option<String> {
        let url = format!(
            "{}/{}",
            self.config.crates_base_url.trim_end_matches('/'),
            name
        );
        let payload = self.fetch_json(&url)?;
        let crate_obj = payload.get("crate")?.as_object()?;

        for key in ["repository", "homepage"] {
            let Some(candidate) = crate_obj.get(key).and_then(Value::as_str) else {
                continue;
            };
            if let Some(url) = normalize_source_url(candidate, true) {
                return Some(url);
            }
        }
        None
    }

    fn fetch_json(&self, url: &str) -> Option<Value> {
        let retries = self.config.http_retries.max(1);
        let mut backoff_ms = self.config.http_retry_backoff_ms;
        for attempt in 1..=retries {
            match self.client.get(url).send() {
                Ok(response) => {
                    if response.status().is_success() {
                        return response.json().ok();
                    }
                    if !should_retry_status(response.status()) || attempt == retries {
                        return None;
                    }
                }
                Err(error) => {
                    if !should_retry_error(&error) || attempt == retries {
                        return None;
                    }
                }
            }

            sleep(Duration::from_millis(backoff_ms));
            backoff_ms *= 2;
        }
        None
    }
}

impl SourceResolver for RegistryResolver {
    fn resolve(&self, dep: &DependencyRef) -> Option<String> {
        self.resolve_inner(dep)
    }
}

fn should_retry_status(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::REQUEST_TIMEOUT
            | StatusCode::TOO_MANY_REQUESTS
            | StatusCode::BAD_GATEWAY
            | StatusCode::SERVICE_UNAVAILABLE
            | StatusCode::GATEWAY_TIMEOUT
    ) || status.as_u16() == 425
}

fn should_retry_error(error: &reqwest::Error) -> bool {
    error.is_timeout() || error.is_connect()
}

fn npm_source_candidates(payload: &Value) -> Vec<String> {
    let mut candidates = Vec::new();

    push_npm_repo_candidates(payload, &mut candidates);
    push_if_str(payload.get("homepage"), &mut candidates);

    let latest_version = payload
        .get("dist-tags")
        .and_then(|tags| tags.get("latest"))
        .and_then(Value::as_str);
    if let Some(latest) = latest_version {
        if let Some(version_payload) = payload
            .get("versions")
            .and_then(Value::as_object)
            .and_then(|versions| versions.get(latest))
        {
            push_npm_repo_candidates(version_payload, &mut candidates);
            push_if_str(version_payload.get("homepage"), &mut candidates);
        }
    }

    candidates
}

fn push_npm_repo_candidates(payload: &Value, candidates: &mut Vec<String>) {
    let Some(repository) = payload.get("repository") else {
        return;
    };
    push_if_str(Some(repository), candidates);
    push_if_str(repository.get("url"), candidates);
}

fn push_if_str(value: Option<&Value>, output: &mut Vec<String>) {
    let Some(candidate) = value.and_then(Value::as_str) else {
        return;
    };
    let trimmed = candidate.trim();
    if trimmed.is_empty() {
        return;
    }
    if output.iter().any(|item| item == trimmed) {
        return;
    }
    output.push(trimmed.to_string());
}

fn normalize_source_url(candidate_url: &str, allow_unlisted_host: bool) -> Option<String> {
    let mut cleaned = candidate_url.trim().to_string();
    if cleaned.starts_with("git+") {
        cleaned = cleaned.trim_start_matches("git+").to_string();
    }
    let (host, path) = parse_host_path(&cleaned)?;
    let host = host.to_ascii_lowercase();
    if host == "pypi.org" {
        return None;
    }
    if !allow_unlisted_host && !looks_like_forge_host(&host) {
        return None;
    }
    let repo_path = extract_repo_path(&path)?;
    Some(format!("https://{host}/{repo_path}"))
}

fn parse_host_path(candidate: &str) -> Option<(String, String)> {
    if candidate.contains("://") {
        let after_scheme = candidate.split_once("://")?.1;
        let (authority, tail) = after_scheme
            .split_once('/')
            .map_or((after_scheme, ""), |(a, b)| (a, b));
        let authority = authority
            .rsplit_once('@')
            .map_or(authority, |(_, rest)| rest);
        let host = authority.split_once(':').map_or(authority, |(h, _)| h);
        if host.is_empty() {
            return None;
        }
        let path = if tail.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", tail.split(['?', '#']).next().unwrap_or(""))
        };
        return Some((host.to_string(), path));
    }

    let at_split = candidate
        .rsplit_once('@')
        .map_or(candidate, |(_, rest)| rest);
    let (host, path) = at_split.split_once(':')?;
    Some((host.to_string(), path.to_string()))
}

fn extract_repo_path(path: &str) -> Option<String> {
    let mut trimmed = path.trim().trim_matches('/').to_string();
    for marker in FORGE_PATH_CUT_MARKERS {
        if let Some((prefix, _)) = trimmed.split_once(marker) {
            trimmed = prefix.to_string();
        }
    }
    if trimmed.ends_with(".git") {
        trimmed = trimmed.trim_end_matches(".git").to_string();
    }
    let parts: Vec<&str> = trimmed.split('/').filter(|x| !x.is_empty()).collect();
    if parts.len() < 2 {
        return None;
    }
    Some(format!("{}/{}", parts[0], parts[1]))
}

fn looks_preferred_label(label: &str) -> bool {
    let lowered = label.to_ascii_lowercase();
    ["source", "repository", "repo", "code", "github"]
        .iter()
        .any(|token| lowered.contains(token))
}

fn is_pypi_project_url(url: &str) -> bool {
    let lowered = url.to_ascii_lowercase();
    lowered.starts_with("https://pypi.org/project/")
        || lowered.starts_with("http://pypi.org/project/")
}

fn looks_like_forge_host(host: &str) -> bool {
    KNOWN_FORGE_HOSTS.contains(&host)
        || KNOWN_FORGE_HOST_TOKENS
            .iter()
            .any(|token| host.contains(token))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{normalize_source_url, npm_source_candidates};

    #[test]
    fn source_url_normalization() {
        assert_eq!(
            normalize_source_url("git+https://github.com/pallets/flask.git", true),
            Some("https://github.com/pallets/flask".into())
        );
        assert_eq!(
            normalize_source_url("git@github.com:psf/requests.git", true),
            Some("https://github.com/psf/requests".into())
        );
        assert_eq!(
            normalize_source_url("https://gitlab.com/pallets/flask/-/tree/main", true),
            Some("https://gitlab.com/pallets/flask".into())
        );
    }

    #[test]
    fn npm_candidates_fallback_to_latest_version_metadata() {
        let payload = json!({
            "name": "demo",
            "dist-tags": {
                "latest": "2.0.0"
            },
            "versions": {
                "2.0.0": {
                    "repository": {
                        "type": "git",
                        "url": "git+https://github.com/example/demo.git"
                    },
                    "homepage": "https://github.com/example/demo#readme"
                }
            }
        });

        let candidates = npm_source_candidates(&payload);
        assert_eq!(
            candidates,
            vec![
                "git+https://github.com/example/demo.git".to_string(),
                "https://github.com/example/demo#readme".to_string()
            ]
        );
        assert_eq!(
            normalize_source_url(&candidates[0], true),
            Some("https://github.com/example/demo".to_string())
        );
    }
}
