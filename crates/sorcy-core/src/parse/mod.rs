pub mod cargo;
pub mod cpp;
pub mod npm;
pub mod python;

use std::collections::BTreeSet;
use std::fs;

use anyhow::{anyhow, Context, Result};

use crate::model::{DependencyRef, ManifestFile, ManifestKind, ParsedDependency};

pub trait ManifestParser {
    fn supports(&self, kind: ManifestKind) -> bool;
    fn parse(&self, kind: ManifestKind, content: &str) -> Result<Vec<DependencyRef>>;
}

fn parser_registry() -> Vec<Box<dyn ManifestParser>> {
    vec![
        Box::new(python::PythonParser),
        Box::new(npm::NpmParser),
        Box::new(cargo::CargoParser),
        Box::new(cpp::CppParser),
    ]
}

pub fn parse_dependencies(manifests: &[ManifestFile]) -> Result<Vec<ParsedDependency>> {
    let mut output = Vec::new();
    let mut seen = BTreeSet::new();
    let parsers = parser_registry();

    for manifest in manifests {
        let content = fs::read_to_string(&manifest.path)
            .with_context(|| format!("failed reading {}", manifest.path.display()))?;

        let parsed = parsers
            .iter()
            .find(|parser| parser.supports(manifest.kind))
            .ok_or_else(|| anyhow!("no parser registered for {:?}", manifest.kind))?
            .parse(manifest.kind, &content)?;

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
                    manifest_kind: manifest.kind,
                });
            }
        }
    }

    Ok(output)
}
