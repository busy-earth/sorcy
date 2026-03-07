use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Ecosystem {
    Python,
    Npm,
    Cargo,
    Cpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyRef {
    pub name: String,
    pub ecosystem: Ecosystem,
    pub source_hint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ParsedDependency {
    pub dependency: DependencyRef,
    pub manifest_path: PathBuf,
    pub manifest_kind: ManifestKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Debug, Clone)]
pub struct ManifestFile {
    pub path: PathBuf,
    pub kind: ManifestKind,
}

#[derive(Debug, Clone)]
pub struct ProjectScan {
    pub root_path: PathBuf,
    pub manifests: Vec<ManifestRecord>,
    pub dependencies: Vec<DependencyRecord>,
    pub resolutions: Vec<ResolutionRecord>,
}

#[derive(Debug, Clone)]
pub struct ManifestRecord {
    pub manifest_path: PathBuf,
    pub manifest_kind: ManifestKind,
}

#[derive(Debug, Clone)]
pub struct DependencyRecord {
    pub dependency_name: String,
    pub ecosystem: Ecosystem,
    pub manifest_path: PathBuf,
    pub manifest_kind: ManifestKind,
    pub source_hint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolutionRecord {
    pub dependency_name: String,
    pub ecosystem: Ecosystem,
    pub manifest_path: PathBuf,
    pub manifest_kind: ManifestKind,
    pub source_hint: Option<String>,
    pub source_repo: Option<SourceRepo>,
    pub resolution_origin: ResolutionOrigin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRepo {
    pub normalized_source_url: String,
    pub host: String,
    pub owner: String,
    pub repo: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
