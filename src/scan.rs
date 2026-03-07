use std::path::Path;

use anyhow::Result;
use walkdir::WalkDir;

use crate::model::{ManifestFile, ManifestKind};

const SKIP_DIRS: &[&str] = &[".git", "target", "node_modules", ".venv", "venv"];

pub fn discover_manifests(root: &Path) -> Result<Vec<ManifestFile>> {
    let mut files = Vec::new();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| should_keep(entry.path()))
    {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }

        let Some(name) = entry.path().file_name().and_then(|x| x.to_str()) else {
            continue;
        };

        if let Some(kind) = detect_manifest_kind(name) {
            files.push(ManifestFile {
                path: entry.path().to_path_buf(),
                kind,
            });
        }
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn should_keep(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|x| x.to_str()) else {
        return true;
    };
    !SKIP_DIRS.contains(&name)
}

fn detect_manifest_kind(file_name: &str) -> Option<ManifestKind> {
    match file_name {
        "pyproject.toml" => Some(ManifestKind::PyProjectToml),
        "package.json" => Some(ManifestKind::PackageJson),
        "Cargo.toml" => Some(ManifestKind::CargoToml),
        "vcpkg.json" => Some(ManifestKind::VcpkgJson),
        "vcpkg-configuration.json" => Some(ManifestKind::VcpkgConfigurationJson),
        "conanfile.txt" => Some(ManifestKind::ConanfileTxt),
        "conanfile.py" => Some(ManifestKind::ConanfilePy),
        name if name.starts_with("requirements") && name.ends_with(".txt") => {
            Some(ManifestKind::RequirementsTxt)
        }
        _ => None,
    }
}
