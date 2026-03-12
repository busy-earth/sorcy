mod overrides;
mod seeds;

use serde::{Deserialize, Serialize};

pub use overrides::{
    parse_rank_overrides, read_rank_overrides, RankOverrides, RANK_OVERRIDES_FILE_NAME,
};
pub use seeds::{classify_seeded_tier, low_value_seeds_for_ecosystem, LowValueSeed};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RelevanceTier {
    Orbit,
    Transit,
    Distant,
    Void,
}
