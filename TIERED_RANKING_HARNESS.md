# Tiered relevance ranking harness (BE-14 MVP)

This document defines the tier model, scoring schema, benchmark scenarios, and current recommendation for dependency repo ranking.

## Why this harness exists

We want an objective way to decide:

1. which dependency repos are most relevant for a given action scope, and
2. when retrieval cost is worth paying.

The harness is deterministic and local. It does not call external services.

## Tier definitions

### 1) Core / milestone tier

- Scope: broad architecture or milestone planning.
- Goal: prioritize foundational framework repos.
- Cost sensitivity: low-to-medium.

### 2) Feature tier

- Scope: specific feature implementation or bug fix.
- Goal: prioritize repos tightly linked to that feature.
- Cost sensitivity: medium.

### 3) Task / chat / subagent tier

- Scope: short-lived execution context (single task/chat/subagent).
- Goal: prioritize direct evidence from files/symbols for the exact task.
- Cost sensitivity: highest.

## Scoring signals

Each candidate repo gets a weighted score from these normalized signals:

- `dependency_graph_proximity` (closer is better)
- `file_symbol_overlap` (higher overlap is better)
- `freshness` (more recent is better)
- `trust_safety` (higher trust is better)
- `retrieval_cost` (lower latency/tokens is better)

For the candidate strategy, low-trust repos are gated by tier-specific trust thresholds.

## Strategies compared

- `baseline_heuristic`: single fixed weighting, no trust gate, no explicit cost term.
- `tier_aware_balanced`: tier-dependent weighting + trust gate + cost-aware weighting.

## Benchmark scenarios (labeled)

The harness includes 4 deterministic scenarios with expected top-N outcomes:

1. `core_rag_foundation` (Core, top-3)
2. `feature_vectorstore_bug` (Feature, top-2)
3. `task_prompt_regression` (Task, top-2)
4. `rare_json_edge_case` (Task, top-1)

## How to run

```bash
cargo run -p sorcy -- --ranking-harness --pretty
```

## Current benchmark result snapshot

From the current harness run:

- Baseline avg precision@N: `0.625`
- Tier-aware avg precision@N: `1.000`
- Baseline avg opportunity cost: `0.561`
- Tier-aware avg opportunity cost: `0.711`

The tier-aware strategy improves relevance quality (precision/recall), with higher average retrieval cost.

## Default strategy recommendation

Use `tier_aware_balanced` as the default for the next implementation phase.

Reason: it materially improves precision/recall across benchmark scenarios, including safety-sensitive and rare-edge-case contexts. The cost increase is explicit in metrics and can be tuned later with stricter retrieval thresholds if needed.
