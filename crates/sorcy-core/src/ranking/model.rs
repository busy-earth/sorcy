use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RankingTier {
    CoreMilestone,
    Feature,
    TaskChatSubagent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateSignals {
    pub dependency_graph_distance: u8,
    pub file_symbol_overlap: f64,
    pub freshness_days: u16,
    pub trust_score: f64,
    pub retrieval_latency_ms: u32,
    pub retrieval_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoCandidate {
    pub repo_id: String,
    pub dependency_name: String,
    pub signals: CandidateSignals,
}

impl RepoCandidate {
    pub fn new(
        repo_id: &str,
        dependency_name: &str,
        dependency_graph_distance: u8,
        file_symbol_overlap: f64,
        freshness_days: u16,
        trust_score: f64,
        retrieval_latency_ms: u32,
        retrieval_tokens: u32,
    ) -> Self {
        Self {
            repo_id: repo_id.to_string(),
            dependency_name: dependency_name.to_string(),
            signals: CandidateSignals {
                dependency_graph_distance,
                file_symbol_overlap,
                freshness_days,
                trust_score,
                retrieval_latency_ms,
                retrieval_tokens,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkScenario {
    pub scenario_id: String,
    pub description: String,
    pub tier: RankingTier,
    pub top_n: usize,
    pub expected_top_repos: Vec<String>,
    pub candidates: Vec<RepoCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioMetrics {
    pub precision_at_n: f64,
    pub recall_at_n: f64,
    pub avg_retrieval_latency_ms: f64,
    pub avg_retrieval_tokens: f64,
    pub avg_opportunity_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResult {
    pub strategy: String,
    pub scenario_id: String,
    pub tier: RankingTier,
    pub top_n: usize,
    pub expected_top_repos: Vec<String>,
    pub ranked_repos: Vec<String>,
    pub retrieved_repos: Vec<String>,
    pub metrics: ScenarioMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategySummary {
    pub strategy: String,
    pub avg_precision_at_n: f64,
    pub avg_recall_at_n: f64,
    pub avg_retrieval_latency_ms: f64,
    pub avg_retrieval_tokens: f64,
    pub avg_opportunity_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessReport {
    pub schema_version: String,
    pub scoring_signals: Vec<String>,
    pub scenario_count: usize,
    pub scenario_results: Vec<ScenarioResult>,
    pub strategy_summaries: Vec<StrategySummary>,
    pub recommended_default_strategy: String,
    pub recommendation_reason: String,
}
