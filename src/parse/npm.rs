use anyhow::Result;
use serde_json::Value;

use crate::model::{DependencyRef, Ecosystem, ManifestKind};

use super::ManifestParser;

pub struct NpmParser;

impl ManifestParser for NpmParser {
    fn supports(&self, kind: ManifestKind) -> bool {
        matches!(kind, ManifestKind::PackageJson)
    }

    fn parse(&self, kind: ManifestKind, content: &str) -> Result<Vec<DependencyRef>> {
        match kind {
            ManifestKind::PackageJson => parse_package_json(content),
            _ => Ok(Vec::new()),
        }
    }
}

const NPM_DEP_SECTIONS: &[&str] = &[
    "dependencies",
    "devDependencies",
    "optionalDependencies",
    "peerDependencies",
];

pub fn parse_package_json(content: &str) -> Result<Vec<DependencyRef>> {
    let data: Value = serde_json::from_str(content)?;
    let mut names = Vec::new();

    for section in NPM_DEP_SECTIONS {
        let Some(entries) = data.get(*section).and_then(Value::as_object) else {
            continue;
        };

        for dep in entries.keys() {
            let dep = dep.trim();
            if dep.is_empty() {
                continue;
            }
            names.push(dep.to_string());
        }
    }

    names.sort();
    names.dedup();
    Ok(names
        .into_iter()
        .map(|name| DependencyRef {
            name,
            ecosystem: Ecosystem::Npm,
            source_hint: None,
        })
        .collect())
}
