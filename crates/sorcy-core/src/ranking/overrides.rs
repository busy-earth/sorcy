use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use super::RelevanceTier;

pub const RANK_OVERRIDES_FILE_NAME: &str = "sorcy-rank.toml";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RankOverrides {
    pub tiers: BTreeMap<String, RelevanceTier>,
}

impl RankOverrides {
    pub fn tier_for(&self, dependency_name: &str) -> Option<RelevanceTier> {
        self.tiers.get(dependency_name).copied().or_else(|| {
            self.tiers
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(dependency_name))
                .map(|(_, tier)| *tier)
        })
    }
}

#[derive(Debug, Deserialize)]
struct RankOverridesFile {
    #[serde(default)]
    tiers: BTreeMap<String, RelevanceTier>,
}

pub fn parse_rank_overrides(content: &str) -> Result<RankOverrides> {
    let parsed = toml::from_str::<RankOverridesFile>(content)
        .context("failed to parse sorcy-rank.toml overrides")?;
    Ok(RankOverrides {
        tiers: parsed.tiers,
    })
}

pub fn read_rank_overrides(project_root: &Path) -> Result<Option<RankOverrides>> {
    let path = project_root.join(RANK_OVERRIDES_FILE_NAME);
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("failed reading {}", path.as_path().display()))?;
    parse_rank_overrides(&content).map(Some)
}
