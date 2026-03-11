mod harness;
mod model;
mod scenarios;

pub use harness::{run_tiered_ranking_harness, StrategyName};
pub use model::{
    BenchmarkScenario, CandidateSignals, HarnessReport, RankingTier, RepoCandidate,
    ScenarioMetrics, ScenarioResult, StrategySummary,
};
