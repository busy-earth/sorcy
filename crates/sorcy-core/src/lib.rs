pub mod model;
pub mod parse;
pub mod repo;
pub mod resolve;
pub mod scan;
pub mod settings;
pub mod source;

use std::path::{Path, PathBuf};

use anyhow::Result;
use parse::parse_dependencies;
use repo::{RepoManager, RepoManagerConfig, RepoUpdateStrategy};
use resolve::{RegistryConfig, RegistryResolver, SourceResolver};
use scan::discover_manifests;
use settings::{Settings, SettingsOverrides};

pub use model::{
    DependencyRecord, DependencyRef, Ecosystem, ManagedRepo, ManagedRepoStatus, ManifestKind,
    ManifestRecord, MaterializedResolution, ProjectMaterialization, ProjectScan, RepoCache,
    ResolutionOrigin, ResolutionRecord, SourceRecord, SourceRepo,
};
pub use repo::{default_repo_cache_dir, GitRunner};
pub use source::{
    find_files, get_local_repo_for_dependency, list_materialized_repos, read_repo_file,
    FindFilesQuery, MaterializedRepoLookup,
};

#[derive(Debug, Clone)]
pub struct SorcyConfig {
    pub registry: RegistryConfig,
    pub repo_cache_dir: PathBuf,
    pub repo_update_strategy: RepoUpdateStrategy,
}

impl SorcyConfig {
    pub fn from_settings(settings: Settings) -> Self {
        Self {
            registry: RegistryConfig {
                pypi_base_url: settings.registry.pypi_base_url,
                npm_base_url: settings.registry.npm_base_url,
                crates_base_url: settings.registry.crates_base_url,
                http_timeout_seconds: settings.http.timeout_seconds,
                http_retries: settings.http.retries,
                http_retry_backoff_ms: settings.http.retry_backoff_ms,
            },
            repo_cache_dir: settings.repo.cache_dir,
            repo_update_strategy: settings.repo.update_strategy,
        }
    }
}

pub fn scan_project(root: &Path) -> Result<ProjectScan> {
    scan_project_with_config(root, RegistryConfig::default())
}

pub fn scan_project_with_config(root: &Path, config: RegistryConfig) -> Result<ProjectScan> {
    let resolver = RegistryResolver::new(config)?;
    scan_project_with_resolver(root, &resolver)
}

pub fn scan_project_with_resolver(
    root: &Path,
    resolver: &impl SourceResolver,
) -> Result<ProjectScan> {
    let manifests = discover_manifests(root)?;
    let parsed_dependencies = parse_dependencies(&manifests)?;

    let mut manifest_records = manifests
        .iter()
        .map(|manifest| ManifestRecord {
            manifest_path: manifest.path.clone(),
            manifest_kind: manifest.kind,
        })
        .collect::<Vec<_>>();
    manifest_records.sort_by(|a, b| {
        a.manifest_path
            .cmp(&b.manifest_path)
            .then_with(|| a.manifest_kind.cmp(&b.manifest_kind))
    });
    manifest_records
        .dedup_by(|a, b| a.manifest_path == b.manifest_path && a.manifest_kind == b.manifest_kind);

    let mut dependencies = Vec::new();
    let mut resolutions = Vec::new();

    for parsed in parsed_dependencies {
        let dependency = parsed.dependency;
        let source_url = resolver.resolve(&dependency);
        let resolution_origin = match (dependency.source_hint.as_ref(), source_url.as_ref()) {
            (Some(_), Some(_)) => ResolutionOrigin::SourceHint,
            (None, Some(_)) => ResolutionOrigin::RegistryMetadata,
            _ => ResolutionOrigin::Unresolved,
        };

        let dependency_record = DependencyRecord {
            dependency_name: dependency.name.clone(),
            ecosystem: dependency.ecosystem.clone(),
            manifest_path: parsed.manifest_path.clone(),
            manifest_kind: parsed.manifest_kind,
            source_hint: dependency.source_hint.clone(),
        };

        let source_repo = source_url
            .as_deref()
            .and_then(source_repo_from_normalized_url);
        let resolution_record = ResolutionRecord {
            dependency_name: dependency.name,
            ecosystem: dependency.ecosystem,
            manifest_path: parsed.manifest_path,
            manifest_kind: parsed.manifest_kind,
            source_hint: dependency.source_hint,
            source_repo,
            resolution_origin,
        };

        dependencies.push(dependency_record);
        resolutions.push(resolution_record);
    }

    dependencies.sort_by(|a, b| {
        a.manifest_path
            .cmp(&b.manifest_path)
            .then_with(|| a.manifest_kind.cmp(&b.manifest_kind))
            .then_with(|| a.ecosystem.cmp(&b.ecosystem))
            .then_with(|| a.dependency_name.cmp(&b.dependency_name))
            .then_with(|| a.source_hint.cmp(&b.source_hint))
    });
    resolutions.sort_by(|a, b| {
        let a_url = a.source_repo.as_ref().map(|x| &x.normalized_source_url);
        let b_url = b.source_repo.as_ref().map(|x| &x.normalized_source_url);
        a.manifest_path
            .cmp(&b.manifest_path)
            .then_with(|| a.manifest_kind.cmp(&b.manifest_kind))
            .then_with(|| a.ecosystem.cmp(&b.ecosystem))
            .then_with(|| a.dependency_name.cmp(&b.dependency_name))
            .then_with(|| a.source_hint.cmp(&b.source_hint))
            .then_with(|| a_url.cmp(&b_url))
    });

    Ok(ProjectScan {
        root_path: root.to_path_buf(),
        manifests: manifest_records,
        dependencies,
        resolutions,
    })
}

pub fn run(root: &Path) -> Result<Vec<SourceRecord>> {
    run_with_config(root, RegistryConfig::default())
}

pub fn run_with_config(root: &Path, config: RegistryConfig) -> Result<Vec<SourceRecord>> {
    let resolver = RegistryResolver::new(config)?;
    run_with_resolver(root, &resolver)
}

pub fn run_with_resolver(root: &Path, resolver: &impl SourceResolver) -> Result<Vec<SourceRecord>> {
    let scan = scan_project_with_resolver(root, resolver)?;
    Ok(compatibility_records_from_scan(&scan))
}

pub fn materialize_project(root: &Path) -> Result<ProjectMaterialization> {
    let settings = Settings::resolve(SettingsOverrides::default())?;
    materialize_project_with_config(root, SorcyConfig::from_settings(settings))
}

pub fn materialize_project_with_config(
    root: &Path,
    config: SorcyConfig,
) -> Result<ProjectMaterialization> {
    let resolver = RegistryResolver::new(config.registry)?;
    let repo_manager = RepoManager::new(RepoManagerConfig {
        cache_dir: config.repo_cache_dir,
        update_strategy: config.repo_update_strategy,
    });
    materialize_project_with_resolver(root, &resolver, &repo_manager)
}

pub fn materialize_project_with_resolver(
    root: &Path,
    resolver: &impl SourceResolver,
    repo_manager: &RepoManager,
) -> Result<ProjectMaterialization> {
    let project_scan = scan_project_with_resolver(root, resolver)?;
    let mut materialized_repos = Vec::new();
    let mut materialized_resolutions = Vec::with_capacity(project_scan.resolutions.len());
    for resolution in &project_scan.resolutions {
        let managed_repo = resolution.source_repo.as_ref().map(|source_repo| {
            let managed = repo_manager.materialize(source_repo);
            materialized_repos.push(managed.clone());
            managed
        });
        materialized_resolutions.push(MaterializedResolution {
            resolution: resolution.clone(),
            managed_repo,
        });
    }

    Ok(ProjectMaterialization {
        repo_cache: repo_manager.cache_summary(&materialized_repos),
        project_scan,
        materialized_resolutions,
    })
}

pub fn compatibility_records(scan: &ProjectScan) -> Vec<SourceRecord> {
    compatibility_records_from_scan(scan)
}

fn compatibility_records_from_scan(scan: &ProjectScan) -> Vec<SourceRecord> {
    let mut records = scan
        .resolutions
        .iter()
        .filter_map(|resolution| {
            resolution
                .source_repo
                .as_ref()
                .map(|source_repo| SourceRecord {
                    dependency: resolution.dependency_name.clone(),
                    source_url: source_repo.normalized_source_url.clone(),
                })
        })
        .collect::<Vec<_>>();
    records.sort();
    records.dedup();
    records
}

fn source_repo_from_normalized_url(url: &str) -> Option<SourceRepo> {
    let trimmed = url.trim();
    let without_scheme = trimmed.strip_prefix("https://")?;
    let (host, tail) = without_scheme.split_once('/')?;
    let mut parts = tail.split('/');
    let owner = parts.next()?.trim();
    let repo = parts.next()?.trim();
    if host.trim().is_empty() || owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some(SourceRepo {
        normalized_source_url: trimmed.to_string(),
        host: host.to_string(),
        owner: owner.to_string(),
        repo: repo.to_string(),
    })
}
