use crate::model::Ecosystem;

use super::RelevanceTier;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LowValueSeed {
    pub dependency_name: &'static str,
    pub tier: RelevanceTier,
}

impl LowValueSeed {
    const fn new(dependency_name: &'static str, tier: RelevanceTier) -> Self {
        Self {
            dependency_name,
            tier,
        }
    }
}

const PYTHON_LOW_VALUE_SEEDS: &[LowValueSeed] = &[
    LowValueSeed::new("typing-extensions", RelevanceTier::Distant),
    LowValueSeed::new("python-dateutil", RelevanceTier::Distant),
    LowValueSeed::new("six", RelevanceTier::Void),
];

const NPM_LOW_VALUE_SEEDS: &[LowValueSeed] = &[
    LowValueSeed::new("left-pad", RelevanceTier::Void),
    LowValueSeed::new("is-typedarray", RelevanceTier::Void),
    LowValueSeed::new("inherits", RelevanceTier::Distant),
];

const CARGO_LOW_VALUE_SEEDS: &[LowValueSeed] = &[
    LowValueSeed::new("libc", RelevanceTier::Distant),
    LowValueSeed::new("itoa", RelevanceTier::Distant),
    LowValueSeed::new("ryu", RelevanceTier::Distant),
    LowValueSeed::new("lazy_static", RelevanceTier::Void),
];

const CPP_LOW_VALUE_SEEDS: &[LowValueSeed] = &[];

pub fn low_value_seeds_for_ecosystem(ecosystem: &Ecosystem) -> &'static [LowValueSeed] {
    match ecosystem {
        Ecosystem::Python => PYTHON_LOW_VALUE_SEEDS,
        Ecosystem::Npm => NPM_LOW_VALUE_SEEDS,
        Ecosystem::Cargo => CARGO_LOW_VALUE_SEEDS,
        Ecosystem::Cpp => CPP_LOW_VALUE_SEEDS,
    }
}

pub fn classify_seeded_tier(ecosystem: &Ecosystem, dependency_name: &str) -> Option<RelevanceTier> {
    low_value_seeds_for_ecosystem(ecosystem)
        .iter()
        .find(|seed| seed.dependency_name.eq_ignore_ascii_case(dependency_name))
        .map(|seed| seed.tier)
}
