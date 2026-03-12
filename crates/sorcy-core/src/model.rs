use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::ranking::RelevanceTier;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Ecosystem {
    Python,
    Npm,
    Cargo,
    Cpp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyRef {
    pub name: String,
    pub ecosystem: Ecosystem,
    pub source_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedDependency {
    pub dependency: DependencyRef,
    pub manifest_path: PathBuf,
    pub manifest_kind: ManifestKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ManifestKind {
    PyProjectToml,
    RequirementsTxt,
    PackageJson,
    CargoToml,
    VcpkgJson,
    VcpkgConfigurationJson,
    ConanfileTxt,
    ConanfilePy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestFile {
    pub path: PathBuf,
    pub kind: ManifestKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectScan {
    pub root_path: PathBuf,
    pub manifests: Vec<ManifestRecord>,
    pub dependencies: Vec<DependencyRecord>,
    pub resolutions: Vec<ResolutionRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestRecord {
    pub manifest_path: PathBuf,
    pub manifest_kind: ManifestKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyRecord {
    pub dependency_name: String,
    pub ecosystem: Ecosystem,
    pub manifest_path: PathBuf,
    pub manifest_kind: ManifestKind,
    pub source_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionRecord {
    pub dependency_name: String,
    pub ecosystem: Ecosystem,
    pub manifest_path: PathBuf,
    pub manifest_kind: ManifestKind,
    pub source_hint: Option<String>,
    pub source_repo: Option<SourceRepo>,
    pub resolution_origin: ResolutionOrigin,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<RelevanceTier>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRepo {
    pub normalized_source_url: String,
    pub host: String,
    pub owner: String,
    pub repo: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolutionOrigin {
    SourceHint,
    RegistryMetadata,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct SourceRecord {
    pub dependency: String,
    pub source_url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManagedRepoStatus {
    Missing,
    Cloned,
    Updated,
    Unchanged,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedRepo {
    pub normalized_source_url: String,
    pub host: String,
    pub owner: String,
    pub repo: String,
    pub local_path: PathBuf,
    pub status: ManagedRepoStatus,
    pub last_materialized_unix_seconds: Option<u64>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoCache {
    pub root_path: PathBuf,
    pub metadata_path: PathBuf,
    pub total_managed_repos: usize,
    pub cloned_count: usize,
    pub updated_count: usize,
    pub unchanged_count: usize,
    pub failed_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterializedResolution {
    pub resolution: ResolutionRecord,
    pub managed_repo: Option<ManagedRepo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMaterialization {
    pub project_scan: ProjectScan,
    pub repo_cache: RepoCache,
    pub materialized_resolutions: Vec<MaterializedResolution>,
}
