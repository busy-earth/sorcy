use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use walkdir::WalkDir;

pub const SORCY_RANK_SKILL_NAME: &str = "sorcy-rank";
pub const SKILL_INSTRUCTIONS_FILE_NAME: &str = "SKILL.md";
pub const SKILL_RANKINGS_FILE_NAME: &str = "SORCY_RANKINGS.md";
pub const PROJECT_SKILLS_DIR: &str = ".claude/skills";
pub const GLOBAL_SKILLS_DIR: &str = ".claude/skills";
pub const SKILLS_DIR_OVERRIDE_ENV: &str = "SORCY_SKILLS_DIR";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillInstallScope {
    ProjectLocal,
    Global,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledSkill {
    pub source_dir: PathBuf,
    pub target_dir: PathBuf,
}

pub fn install_sorcy_rank_skill(
    project_root: &Path,
    scope: SkillInstallScope,
) -> Result<InstalledSkill> {
    install_sorcy_rank_skill_with_root_override(project_root, scope, None)
}

pub fn install_sorcy_rank_skill_with_root_override(
    project_root: &Path,
    scope: SkillInstallScope,
    install_root_override: Option<&Path>,
) -> Result<InstalledSkill> {
    let source_dir = locate_sorcy_rank_skill_source_dir()?;
    let install_root = if let Some(path) = install_root_override {
        path.to_path_buf()
    } else {
        match scope {
            SkillInstallScope::ProjectLocal => project_root.join(PROJECT_SKILLS_DIR),
            SkillInstallScope::Global => global_skill_root()?,
        }
    };
    install_sorcy_rank_skill_from_source(&source_dir, &install_root)
}

pub fn install_sorcy_rank_skill_from_source(
    source_skill_dir: &Path,
    install_root: &Path,
) -> Result<InstalledSkill> {
    let source_skill_file = source_skill_dir.join(SKILL_INSTRUCTIONS_FILE_NAME);
    let source_rankings_file = source_skill_dir.join(SKILL_RANKINGS_FILE_NAME);
    if !source_skill_file.is_file() {
        bail!(
            "source skill is missing {} at {}",
            SKILL_INSTRUCTIONS_FILE_NAME,
            source_skill_file.display()
        );
    }
    if !source_rankings_file.is_file() {
        bail!(
            "source skill is missing {} at {}",
            SKILL_RANKINGS_FILE_NAME,
            source_rankings_file.display()
        );
    }

    let target_dir = install_root.join(SORCY_RANK_SKILL_NAME);
    copy_skill_tree(source_skill_dir, &target_dir)?;

    Ok(InstalledSkill {
        source_dir: source_skill_dir.to_path_buf(),
        target_dir,
    })
}

fn copy_skill_tree(source_skill_dir: &Path, target_skill_dir: &Path) -> Result<()> {
    fs::create_dir_all(target_skill_dir)
        .with_context(|| format!("failed creating {}", target_skill_dir.display()))?;

    for entry in WalkDir::new(source_skill_dir) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }

        let source_path = entry.path();
        let relative_path = source_path
            .strip_prefix(source_skill_dir)
            .with_context(|| {
                format!(
                    "failed to strip source prefix for {}",
                    source_path.display()
                )
            })?;
        let target_path = target_skill_dir.join(relative_path);
        if target_path
            .file_name()
            .is_some_and(|name| name == SKILL_RANKINGS_FILE_NAME)
            && target_path.exists()
        {
            continue;
        }

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed creating {}", parent.display()))?;
        }
        fs::copy(source_path, &target_path).with_context(|| {
            format!(
                "failed copying {} to {}",
                source_path.display(),
                target_path.display()
            )
        })?;
    }
    Ok(())
}

fn locate_sorcy_rank_skill_source_dir() -> Result<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(override_dir) = env::var_os(SKILLS_DIR_OVERRIDE_ENV) {
        candidates.push(PathBuf::from(override_dir).join(SORCY_RANK_SKILL_NAME));
    }
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("skills").join(SORCY_RANK_SKILL_NAME));
            candidates.push(parent.join("../skills").join(SORCY_RANK_SKILL_NAME));
            candidates.push(
                parent
                    .join("../share/sorcy/skills")
                    .join(SORCY_RANK_SKILL_NAME),
            );
        }
    }

    for candidate in candidates {
        if candidate.join(SKILL_INSTRUCTIONS_FILE_NAME).is_file()
            && candidate.join(SKILL_RANKINGS_FILE_NAME).is_file()
        {
            return Ok(candidate);
        }
    }
    bail!("unable to locate sorcy-rank source skill folder")
}

fn global_skill_root() -> Result<PathBuf> {
    let home = home_dir().context("HOME is not set")?;
    Ok(home.join(GLOBAL_SKILLS_DIR))
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from))
}
