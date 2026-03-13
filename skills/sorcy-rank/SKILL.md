---
name: sorcy-rank
description: Use this skill when an agent needs to prioritize which depos to search
  first for this project. Invoke after running sorcy scan output to generate or
  refresh SORCY_RANKINGS.md with Orbit/Transit/Distant/Void tiers.
allowed-tools:
  - Read
  - Write
  - Bash
compatibility:
  - cursor
  - claude-code
  - codex
metadata:
  author: busy-earth
  version: 1.0.0
  tags:
    - sorcy
    - rank
    - depos
license: MIT
---

# sorcy-rank

This skill ranks depos into relevance tiers for agent context prioritization.

## Input contract

The skill consumes the JSON array emitted by `sorcy .` (refs output).
Each element has exactly two fields:
- `dependency` — string, the package name
- `source_url` — string, normalized GitHub/forge URL

Treat this as immutable structured input. Do not infer or supplement fields.

## Tier system

- **Orbit**: Architecturally central. Search every session.
- **Transit**: Used regularly. Worth indexing.
- **Distant**: Foundational primitive. Rarely worth source search.
- **Void**: No source value. Omit from search candidates.

## Required behavior

1. Treat `sorcy-rank.toml` overrides as authoritative.
   - Never reclassify a depo pinned in `[tiers]`.
2. Pre-classify obvious low-value deps using the built-in seed list.
3. Classify remaining depos using the combined metadata and ecosystem context.
4. Write the result to `SORCY_RANKINGS.md` in this same folder.
5. Keep this `SKILL.md` file unchanged.

## `sorcy-rank.toml` format

```toml
[tiers]
tokio = "Orbit"
libc = "Void"
```

## Output contract

Write `SORCY_RANKINGS.md` to this same skill folder.
The file MUST contain a rankings table with exactly these columns:

| depo | host | owner | repo | tier | override |
|------|------|-------|------|------|----------|

- `depo` — full normalized_source_url
- `host` — hostname only (e.g. `github.com`)
- `owner` — org or user segment
- `repo` — repository name segment
- `tier` — one of: `Orbit`, `Transit`, `Distant`, `Void`
- `override` — `true` if pinned in `sorcy-rank.toml`, `false` otherwise

The table is the machine-readable contract. Human-readable tier sections
may follow the table but the table must come first and must be complete.
Do not omit any resolved depo from the table regardless of tier.
