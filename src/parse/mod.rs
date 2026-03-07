pub mod cargo;
pub mod cpp;
pub mod npm;
pub mod python;

use std::collections::BTreeSet;
use std::fs;

use anyhow::{Context, Result};

use crate::model::{ManifestFile, ManifestKind, ParsedDependency};

pub fn parse_dependencies(manifests: &[ManifestFile]) -> Result<Vec<ParsedDependency>> {
    let mut output = Vec::new();
    let mut seen = BTreeSet::new();

    for manifest in manifests {
        let content = fs::read_to_string(&manifest.path)
            .with_context(|| format!("failed reading {}", manifest.path.display()))?;

        let parsed = match manifest.kind {
            ManifestKind::PyProjectToml => python::parse_pyproject_toml(&content),
            ManifestKind::RequirementsTxt => python::parse_requirements_txt(&content),
            ManifestKind::PackageJson => npm::parse_package_json(&content),
            ManifestKind::CargoToml => cargo::parse_cargo_toml(&content),
            ManifestKind::VcpkgJson => cpp::parse_vcpkg_json(&content),
            ManifestKind::VcpkgConfigurationJson => cpp::parse_vcpkg_configuration_json(&content),
            ManifestKind::ConanfileTxt => cpp::parse_conanfile_txt(&content),
            ManifestKind::ConanfilePy => cpp::parse_conanfile_py(&content),
        }?;

        for dep in parsed {
            let key = (
                manifest.path.display().to_string(),
                dep.ecosystem.clone(),
                dep.name.clone(),
                dep.source_hint.clone(),
            );
            if seen.insert(key) {
                output.push(ParsedDependency {
                    dependency: dep,
                    manifest_path: manifest.path.clone(),
                });
            }
        }
    }

    Ok(output)
}
