# Reference repo architecture patterns (DeepWiki-backed)

This file captures architecture patterns from these reference repos:

- `astral-sh/uv`
- `rust-lang/rust-analyzer`
- `ast-grep/ast-grep`
- `denoland/deno_graph`
- `rust-lang/cargo`

Use this as a practical checklist when implementing new `sorcy` features.

## Why this exists

We are intentionally avoiding unstable or archived primary references for implementation patterns.
This document favors mature, maintained Rust projects and distills patterns that fit `sorcy`'s current MVP direction.

## Pattern map for sorcy

### 1) Keep core logic separate from CLI and IO

**Reference pattern**
- `rust-analyzer` keeps protocol/transport edges separate from semantic core crates.
- `deno_graph` separates graph logic from content loading through trait boundaries.

**Apply in sorcy**
- Keep `sorcy-core` as the source of truth for scanning, materialization, and query APIs.
- Keep `sorcy-cli` as a thin adapter that parses args and serializes outputs.
- Put filesystem/network interactions behind small traits or modules at boundaries.

### 2) Model normalized domain records first

**Reference pattern**
- `deno_graph` uses normalized graph/module records.
- `rust-analyzer` uses stable internal IDs/data models and computes on top.

**Apply in sorcy**
- Continue using stable records (`ProjectScan`, `ResolutionRecord`, `ManagedRepo`, `ProjectMaterialization`) as the base contract.
- Add new features by extending query/read surfaces over existing records, not by bypassing them.

### 3) Use deterministic cache/index persistence

**Reference pattern**
- `cargo` and `uv` rely on deterministic cache paths and predictable local source state.

**Apply in sorcy**
- Keep deterministic cache layout: `repos/<host>/<owner>/<repo>`.
- Keep persisted metadata in `index.json`.
- Sort outputs for stable results across reruns.

### 4) Prefer sync orchestration at MVP stage

**Reference pattern**
- `cargo` source loading and git source preparation are organized as explicit readiness steps.
- `uv` isolates blocking git work cleanly and keeps boundaries clear.

**Apply in sorcy**
- Keep scan/materialize/query orchestration mostly synchronous.
- Use async only where required by external boundaries, not for internal composition.

### 5) Build safe local file access primitives

**Reference pattern**
- Mature tooling validates path and source boundaries before reading from cache/workspace.

**Apply in sorcy**
- Reject absolute paths and parent traversal for repo-file reads.
- Canonicalize root and target paths and enforce "inside repo root" checks.

### 6) Keep query APIs small and deterministic

**Reference pattern**
- `deno_graph` and `rust-analyzer` expose narrow query surfaces over normalized state.

**Apply in sorcy**
- Prefer small core APIs with clear contracts:
  - list materialized repos
  - lookup local repo for dependency
  - read repo file safely
  - deterministic file discovery with stable sort order

## Anti-patterns to avoid right now

- Mixing CLI formatting concerns into `sorcy-core` internals.
- Introducing a heavy graph or metadata DB before clone-backed source retrieval is fully useful.
- Adding broad async task meshes before there is a clear boundary-driven need.
- Making query outputs non-deterministic across reruns.
- Reading files from clone cache without path safety enforcement.

## Implementation checklist for next features

Before merging a new feature, verify:

1. Core-vs-CLI boundaries remain clean.
2. New logic builds on normalized internal records.
3. Cache state and query outputs are deterministic.
4. File/path access is constrained and safe.
5. Tests cover happy path + missing state + deterministic rerun behavior.

