use std::cmp::Ordering;
use std::collections::HashSet;

use super::model::{HarnessReport, RankingTier, ScenarioMetrics, ScenarioResult, StrategySummary};
use super::scenarios::default_scenarios;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategyName {
    BaselineHeuristic,
    TierAwareBalanced,
}

impl StrategyName {
    fn as_str(self) -> &'static str {
        match self {
            Self::BaselineHeuristic => "baseline_heuristic",
            Self::TierAwareBalanced => "tier_aware_balanced",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct SignalWeights {
    graph: f64,
    overlap: f64,
    freshness: f64,
    trust: f64,
    cost: f64,
}

#[derive(Debug, Clone)]
struct ScoredRepo {
    repo_id: String,
    score: f64,
    retrieval_latency_ms: u32,
    retrieval_tokens: u32,
}

pub fn run_tiered_ranking_harness() -> HarnessReport {
    let strategies = [
        StrategyName::BaselineHeuristic,
        StrategyName::TierAwareBalanced,
    ];
    let scenarios = default_scenarios();
    let mut scenario_results = Vec::new();
    let mut strategy_summaries = Vec::new();

    for strategy in strategies {
        let mut strategy_results = Vec::new();
        for scenario in &scenarios {
            let result = evaluate_scenario(strategy, scenario);
            scenario_results.push(result.clone());
            strategy_results.push(result);
        }
        strategy_summaries.push(summarize_strategy(strategy, &strategy_results));
    }

    let recommended = strategy_summaries
        .iter()
        .max_by(|left, right| compare_summary(left, right))
        .map(|x| x.strategy.clone())
        .unwrap_or_else(|| StrategyName::TierAwareBalanced.as_str().to_string());

    let baseline = strategy_summaries
        .iter()
        .find(|x| x.strategy == StrategyName::BaselineHeuristic.as_str());
    let candidate = strategy_summaries
        .iter()
        .find(|x| x.strategy == StrategyName::TierAwareBalanced.as_str());
    let recommendation_reason = match (baseline, candidate) {
        (Some(base), Some(next)) if recommended == StrategyName::TierAwareBalanced.as_str() => {
            format!(
                "tier_aware_balanced is recommended because avg precision@N improved from {:.3} to {:.3}. \
avg opportunity cost changed from {:.3} to {:.3}; this trade-off is acceptable for improving relevance quality in the next phase.",
                base.avg_precision_at_n,
                next.avg_precision_at_n,
                base.avg_opportunity_cost,
                next.avg_opportunity_cost
            )
        }
        (Some(base), Some(next)) => format!(
            "{recommended} is recommended because it balanced precision and cost better than tier_aware_balanced (baseline precision {:.3}, candidate precision {:.3}).",
            base.avg_precision_at_n, next.avg_precision_at_n
        ),
        _ => format!("{recommended} is recommended based on benchmark aggregate metrics."),
    };

    HarnessReport {
        schema_version: "be14-tiered-ranking-v1".to_string(),
        scoring_signals: vec![
            "dependency_graph_proximity".to_string(),
            "file_symbol_overlap".to_string(),
            "freshness".to_string(),
            "trust_safety".to_string(),
            "retrieval_cost".to_string(),
        ],
        scenario_count: scenarios.len(),
        scenario_results,
        strategy_summaries,
        recommended_default_strategy: recommended,
        recommendation_reason,
    }
}

fn evaluate_scenario(
    strategy: StrategyName,
    scenario: &super::model::BenchmarkScenario,
) -> ScenarioResult {
    let max_latency_ms = scenario
        .candidates
        .iter()
        .map(|x| x.signals.retrieval_latency_ms)
        .max()
        .unwrap_or(1) as f64;
    let max_tokens = scenario
        .candidates
        .iter()
        .map(|x| x.signals.retrieval_tokens)
        .max()
        .unwrap_or(1) as f64;
    let weights = weights_for(strategy, scenario.tier);

    let mut ranked = scenario
        .candidates
        .iter()
        .map(|candidate| {
            let score = weighted_score(
                strategy,
                scenario.tier,
                &candidate.signals,
                max_latency_ms,
                max_tokens,
                weights,
            );
            ScoredRepo {
                repo_id: candidate.repo_id.clone(),
                score,
                retrieval_latency_ms: candidate.signals.retrieval_latency_ms,
                retrieval_tokens: candidate.signals.retrieval_tokens,
            }
        })
        .collect::<Vec<_>>();
    ranked.sort_by(compare_ranked_repo);

    let ranked_repos = ranked.iter().map(|x| x.repo_id.clone()).collect::<Vec<_>>();
    let mut retrieved = ranked
        .iter()
        .filter(|x| x.score >= min_retrieval_score(strategy, scenario.tier))
        .take(scenario.top_n)
        .cloned()
        .collect::<Vec<_>>();
    if retrieved.is_empty() {
        retrieved = ranked.iter().take(scenario.top_n).cloned().collect();
    }
    let retrieved_repos = retrieved
        .iter()
        .map(|x| x.repo_id.clone())
        .collect::<Vec<_>>();

    let expected = scenario
        .expected_top_repos
        .iter()
        .cloned()
        .collect::<HashSet<_>>();
    let hits = retrieved_repos
        .iter()
        .filter(|repo| expected.contains(*repo))
        .count();
    let precision_at_n = hits as f64 / scenario.top_n.max(1) as f64;
    let recall_at_n = hits as f64 / scenario.expected_top_repos.len().max(1) as f64;
    let avg_retrieval_latency_ms = average_u32(retrieved.iter().map(|x| x.retrieval_latency_ms));
    let avg_retrieval_tokens = average_u32(retrieved.iter().map(|x| x.retrieval_tokens));
    let avg_opportunity_cost = average_f64(retrieved.iter().map(|x| {
        let latency = x.retrieval_latency_ms as f64 / max_latency_ms;
        let tokens = x.retrieval_tokens as f64 / max_tokens;
        ((latency + tokens) / 2.0).clamp(0.0, 1.0)
    }));

    ScenarioResult {
        strategy: strategy.as_str().to_string(),
        scenario_id: scenario.scenario_id.clone(),
        tier: scenario.tier,
        top_n: scenario.top_n,
        expected_top_repos: scenario.expected_top_repos.clone(),
        ranked_repos,
        retrieved_repos,
        metrics: ScenarioMetrics {
            precision_at_n,
            recall_at_n,
            avg_retrieval_latency_ms,
            avg_retrieval_tokens,
            avg_opportunity_cost,
        },
    }
}

fn weighted_score(
    strategy: StrategyName,
    tier: RankingTier,
    signals: &super::model::CandidateSignals,
    max_latency_ms: f64,
    max_tokens: f64,
    weights: SignalWeights,
) -> f64 {
    let graph_score = 1.0 - (signals.dependency_graph_distance.min(3) as f64 / 3.0);
    let overlap_score = clamp_01(signals.file_symbol_overlap);
    let freshness_score = 1.0 - (signals.freshness_days.min(180) as f64 / 180.0);
    let trust_score = clamp_01(signals.trust_score);
    let latency_score = 1.0 - (signals.retrieval_latency_ms as f64 / max_latency_ms.max(1.0));
    let token_score = 1.0 - (signals.retrieval_tokens as f64 / max_tokens.max(1.0));
    let cost_score = ((latency_score + token_score) / 2.0).clamp(0.0, 1.0);

    let mut score = graph_score * weights.graph
        + overlap_score * weights.overlap
        + freshness_score * weights.freshness
        + trust_score * weights.trust
        + cost_score * weights.cost;

    // Tier-aware strategy rejects unsafe/low-trust repositories.
    if strategy == StrategyName::TierAwareBalanced && trust_score < trust_threshold_for_tier(tier) {
        score *= 0.05;
    }
    score
}

fn weights_for(strategy: StrategyName, tier: RankingTier) -> SignalWeights {
    match strategy {
        StrategyName::BaselineHeuristic => SignalWeights {
            graph: 0.45,
            overlap: 0.20,
            freshness: 0.20,
            trust: 0.15,
            cost: 0.00,
        },
        StrategyName::TierAwareBalanced => match tier {
            RankingTier::CoreMilestone => SignalWeights {
                graph: 0.40,
                overlap: 0.25,
                freshness: 0.15,
                trust: 0.15,
                cost: 0.05,
            },
            RankingTier::Feature => SignalWeights {
                graph: 0.30,
                overlap: 0.35,
                freshness: 0.10,
                trust: 0.15,
                cost: 0.10,
            },
            RankingTier::TaskChatSubagent => SignalWeights {
                graph: 0.20,
                overlap: 0.45,
                freshness: 0.05,
                trust: 0.15,
                cost: 0.15,
            },
        },
    }
}

fn trust_threshold_for_tier(tier: RankingTier) -> f64 {
    match tier {
        RankingTier::CoreMilestone => 0.70,
        RankingTier::Feature => 0.65,
        RankingTier::TaskChatSubagent => 0.60,
    }
}

fn min_retrieval_score(strategy: StrategyName, tier: RankingTier) -> f64 {
    match strategy {
        StrategyName::BaselineHeuristic => 0.35,
        StrategyName::TierAwareBalanced => match tier {
            RankingTier::CoreMilestone => 0.40,
            RankingTier::Feature => 0.45,
            RankingTier::TaskChatSubagent => 0.50,
        },
    }
}

fn summarize_strategy(
    strategy: StrategyName,
    scenario_results: &[ScenarioResult],
) -> StrategySummary {
    StrategySummary {
        strategy: strategy.as_str().to_string(),
        avg_precision_at_n: average_f64(scenario_results.iter().map(|x| x.metrics.precision_at_n)),
        avg_recall_at_n: average_f64(scenario_results.iter().map(|x| x.metrics.recall_at_n)),
        avg_retrieval_latency_ms: average_f64(
            scenario_results
                .iter()
                .map(|x| x.metrics.avg_retrieval_latency_ms),
        ),
        avg_retrieval_tokens: average_f64(
            scenario_results
                .iter()
                .map(|x| x.metrics.avg_retrieval_tokens),
        ),
        avg_opportunity_cost: average_f64(
            scenario_results
                .iter()
                .map(|x| x.metrics.avg_opportunity_cost),
        ),
    }
}

fn compare_summary(left: &StrategySummary, right: &StrategySummary) -> Ordering {
    left.avg_precision_at_n
        .partial_cmp(&right.avg_precision_at_n)
        .unwrap_or(Ordering::Equal)
        .then_with(|| {
            right
                .avg_opportunity_cost
                .partial_cmp(&left.avg_opportunity_cost)
                .unwrap_or(Ordering::Equal)
        })
}

fn compare_ranked_repo(left: &ScoredRepo, right: &ScoredRepo) -> Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.repo_id.cmp(&right.repo_id))
}

fn average_u32(values: impl Iterator<Item = u32>) -> f64 {
    average_f64(values.map(|x| x as f64))
}

fn average_f64(values: impl Iterator<Item = f64>) -> f64 {
    let mut total = 0.0;
    let mut count = 0_u32;
    for value in values {
        total += value;
        count += 1;
    }
    if count == 0 {
        0.0
    } else {
        total / count as f64
    }
}

fn clamp_01(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::run_tiered_ranking_harness;

    #[test]
    fn harness_is_deterministic() {
        let first = serde_json::to_string(&run_tiered_ranking_harness()).expect("serialize report");
        let second =
            serde_json::to_string(&run_tiered_ranking_harness()).expect("serialize report");
        assert_eq!(first, second);
    }

    #[test]
    fn candidate_strategy_has_better_precision_than_baseline() {
        let report = run_tiered_ranking_harness();
        let baseline = report
            .strategy_summaries
            .iter()
            .find(|x| x.strategy == "baseline_heuristic")
            .expect("baseline summary");
        let candidate = report
            .strategy_summaries
            .iter()
            .find(|x| x.strategy == "tier_aware_balanced")
            .expect("candidate summary");
        assert!(candidate.avg_precision_at_n >= baseline.avg_precision_at_n);
    }
}
