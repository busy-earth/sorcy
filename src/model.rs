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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct SourceRecord {
    pub dependency: String,
    pub source_url: String,
}
