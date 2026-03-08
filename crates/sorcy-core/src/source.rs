use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context, Result};
use regex::Regex;
use walkdir::WalkDir;

use crate::model::{ManagedRepo, ManagedRepoStatus, ProjectMaterialization};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializedRepoLookup {
    pub dependency_name: String,
    pub local_path: Option<PathBuf>,
    pub status: Option<ManagedRepoStatus>,
    pub source_url: Option<String>,
    pub is_materialized: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FindFilesQuery {
    pub glob_pattern: Option<String>,
    pub path_contains: Option<String>,
    pub extension: Option<String>,
    pub max_results: Option<usize>,
}

pub fn list_materialized_repos(
    project_materialization: &ProjectMaterialization,
) -> Vec<MaterializedRepoLookup> {
    let mut entries = project_materialization
        .materialized_resolutions
        .iter()
        .map(|item| {
            let managed = item.managed_repo.as_ref();
            MaterializedRepoLookup {
                dependency_name: item.resolution.dependency_name.clone(),
                local_path: managed.map(|repo| repo.local_path.clone()),
                status: managed.map(|repo| repo.status),
                source_url: item
                    .resolution
                    .source_repo
                    .as_ref()
                    .map(|repo| repo.normalized_source_url.clone()),
                is_materialized: managed
                    .map(|repo| is_usable_materialized_repo(repo) && repo.local_path.exists())
                    .unwrap_or(false),
            }
        })
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| {
        a.dependency_name
            .cmp(&b.dependency_name)
            .then_with(|| a.source_url.cmp(&b.source_url))
            .then_with(|| a.local_path.cmp(&b.local_path))
    });
    entries
}

pub fn get_local_repo_for_dependency<'a>(
    project_materialization: &'a ProjectMaterialization,
    dep_name: &str,
) -> Option<&'a Path> {
    let mut selected: Option<&Path> = None;
    for item in &project_materialization.materialized_resolutions {
        if item.resolution.dependency_name != dep_name {
            continue;
        }
        let Some(repo) = item.managed_repo.as_ref() else {
            continue;
        };
        if !is_usable_materialized_repo(repo) || !repo.local_path.exists() {
            continue;
        }
        let candidate = repo.local_path.as_path();
        if selected.is_none_or(|current| candidate < current) {
            selected = Some(candidate);
        }
    }
    selected
}

pub fn read_repo_file(local_repo_path: &Path, relative_path: &Path) -> Result<String> {
    ensure_relative_safe_path(relative_path)?;
    let canonical_root = canonical_repo_root(local_repo_path)?;
    let requested_path = canonical_root.join(relative_path);
    let canonical_file = fs::canonicalize(&requested_path).with_context(|| {
        format!(
            "failed to resolve file path {}",
            requested_path.as_path().display()
        )
    })?;
    if !canonical_file.starts_with(&canonical_root) {
        bail!(
            "refusing to read path outside repository root: {}",
            relative_path.display()
        );
    }
    if !canonical_file.is_file() {
        bail!("path is not a file: {}", relative_path.display());
    }
    fs::read_to_string(&canonical_file).with_context(|| {
        format!(
            "failed to read repo file {}",
            canonical_file.as_path().display()
        )
    })
}

pub fn find_files(local_repo_path: &Path, query: &FindFilesQuery) -> Result<Vec<PathBuf>> {
    let canonical_root = canonical_repo_root(local_repo_path)?;
    if query.max_results == Some(0) {
        return Ok(Vec::new());
    }

    let path_contains = query
        .path_contains
        .as_ref()
        .map(|value| value.to_lowercase());
    let extension = query
        .extension
        .as_ref()
        .map(|value| value.trim_start_matches('.').to_lowercase());
    let glob_regex = query
        .glob_pattern
        .as_ref()
        .map(|pattern| compile_glob_pattern(pattern))
        .transpose()?;

    let mut matches = Vec::new();
    for entry in WalkDir::new(&canonical_root) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }

        let absolute_path = entry.path();
        let relative_path = absolute_path
            .strip_prefix(&canonical_root)
            .with_context(|| {
                format!(
                    "failed to compute relative path for {}",
                    absolute_path.display()
                )
            })?
            .to_path_buf();
        let relative_display = relative_path.to_string_lossy();

        if let Some(filter) = path_contains.as_ref() {
            if !relative_display.to_lowercase().contains(filter) {
                continue;
            }
        }

        if let Some(ext) = extension.as_ref() {
            let current_extension = relative_path
                .extension()
                .and_then(|value| value.to_str())
                .map(|value| value.to_lowercase());
            if current_extension.as_deref() != Some(ext.as_str()) {
                continue;
            }
        }

        if let Some(regex) = glob_regex.as_ref() {
            if !regex.is_match(relative_display.as_ref()) {
                continue;
            }
        }

        matches.push(relative_path);
    }

    matches.sort();
    if let Some(limit) = query.max_results {
        matches.truncate(limit);
    }
    Ok(matches)
}

fn is_usable_materialized_repo(repo: &ManagedRepo) -> bool {
    matches!(
        repo.status,
        ManagedRepoStatus::Cloned | ManagedRepoStatus::Updated | ManagedRepoStatus::Unchanged
    )
}

fn canonical_repo_root(local_repo_path: &Path) -> Result<PathBuf> {
    let canonical_root = fs::canonicalize(local_repo_path).with_context(|| {
        format!(
            "failed to resolve local repository path {}",
            local_repo_path.display()
        )
    })?;
    if !canonical_root.is_dir() {
        bail!(
            "local repository path is not a directory: {}",
            local_repo_path.display()
        );
    }
    Ok(canonical_root)
}

fn ensure_relative_safe_path(relative_path: &Path) -> Result<()> {
    if relative_path.is_absolute() {
        bail!(
            "relative_path must not be absolute: {}",
            relative_path.display()
        );
    }
    for component in relative_path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                bail!("relative_path is not safe: {}", relative_path.display());
            }
        }
    }
    Ok(())
}

fn compile_glob_pattern(pattern: &str) -> Result<Regex> {
    if pattern.is_empty() {
        bail!("glob_pattern must not be empty");
    }

    let mut regex = String::from("^");
    for ch in pattern.chars() {
        match ch {
            '*' => regex.push_str(".*"),
            '?' => regex.push('.'),
            other => regex.push_str(&regex::escape(&other.to_string())),
        }
    }
    regex.push('$');

    Regex::new(&regex).with_context(|| format!("invalid glob pattern: {pattern}"))
}
