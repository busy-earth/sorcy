pub mod cli;
pub mod model;
pub mod parse;
pub mod resolve;
pub mod scan;

use std::path::Path;

use anyhow::Result;
use model::SourceRecord;
use parse::parse_dependencies;
use resolve::{RegistryConfig, RegistryResolver, SourceResolver};
use scan::discover_manifests;

pub fn run_with_config(root: &Path, config: RegistryConfig) -> Result<Vec<SourceRecord>> {
    let resolver = RegistryResolver::new(config)?;
    run_with_resolver(root, &resolver)
}

pub fn run_with_resolver(root: &Path, resolver: &impl SourceResolver) -> Result<Vec<SourceRecord>> {
    let manifests = discover_manifests(root)?;
    let dependencies = parse_dependencies(&manifests)?;

    let mut records = Vec::new();
    for item in dependencies {
        if let Some(source_url) = resolver.resolve(&item.dependency) {
            records.push(SourceRecord {
                dependency: item.dependency.name,
                source_url,
            });
        }
    }

    records.sort();
    records.dedup();
    Ok(records)
}

pub fn run(root: &Path) -> Result<Vec<SourceRecord>> {
    run_with_config(root, RegistryConfig::default())
}
