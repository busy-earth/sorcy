# Contributing to sorcy

Thanks for helping improve `sorcy`.

## Core principles

- Keep changes simple and reliable.
- Prefer official packaging metadata over scraping.
- Stay forge-neutral: do not assume one hosting provider is always correct.
- Add tests for every behavior change.

## Project structure

- `crates/sorcy-core/src/scan.rs`: manifest discovery
- `crates/sorcy-core/src/parse/*`: per-ecosystem parsers (via `ManifestParser` trait)
- `crates/sorcy-core/src/resolve.rs`: source URL resolution (via `SourceResolver` trait)
- `crates/sorcy-core/src/repo.rs`: managed local clone cache materialization (`RepoManager`)
- `crates/sorcy-core/src/settings.rs`: settings resolution (`CLI > env > defaults`)
- `crates/sorcy-core/src/model.rs`: internal normalized model and compatibility output type
- `crates/sorcy-core/src/lib.rs`: orchestration API (`scan_project*`, compatibility `run*`, `materialize_project*`)
- `crates/sorcy-cli/src/main.rs`: thin CLI wrapper and process behavior
- `crates/sorcy-core/tests/integration_mvp.rs`: end-to-end integration coverage

Keep `sorcy-core` as the source of truth for product behavior. Keep `sorcy-cli` thin.

## Materialization scope (current MVP step)

- Materialization is opt-in and should not change default compatibility JSON behavior.
- Managed clone cache is local and file-based (`index.json`), with deterministic paths.
- Do not add graph building, Tree-sitter parsing/indexing, or metadata database layers in this step.
- Prefer simple synchronous orchestration and small, testable interfaces.

## Adding support for a new forge host

Source host checks and URL normalization live in `crates/sorcy-core/src/resolve.rs`.

When adding a forge:

1. Update forge host matching constants.
2. Add URL normalization rules only when needed.
3. Add unit tests in `crates/sorcy-core/src/resolve.rs`.
4. Add an integration test path in `crates/sorcy-core/tests/integration_mvp.rs` if behavior changes end-to-end.
5. Keep existing output record shape stable.

## Installer scripts

- `install.sh` and `install.ps1` provide one-command installation for MVP usage.
- Keep them simple, auditable, and shell-safe.
- If behavior changes, update README install examples in the same PR.
- Do not add new `rustup` or `cargo install` invocations outside of `mise.toml` for dev toolchain setup.

## Running tests

Bootstrap local tools once:

```bash
./bin/mise install
```

After bootstrap, use `mise` directly for routine commands:

```bash
mise run test
```

### Optional live registry + materialization smoke tests

These tests are intentionally opt-in and are meant for bigger feature milestones.
They hit real registries (PyPI, npm, crates.io), resolve real dependencies, and clone real repos.

```bash
mise x -- bash -lc 'SORCY_LIVE_TESTS=1 cargo test -p sorcy-core --test live_registry_optional -- --ignored --nocapture'
```

Notes:
- Not run in normal local loops or default CI yet.
- Use when you want extra confidence before opening/finalizing a larger PR.

## Note about `AGENTS.md`

`AGENTS.md` is for coding-agent environment behavior.  
Project mission and contributor policy should live in `README.md` and `CONTRIBUTING.md`.

## Contributor command reference (mise wrappers)

Bootstrap local tools once:

```bash
./bin/mise install
```

After bootstrap, use `mise` directly for day-to-day contributor workflows:

Build all workspace members:

```bash
mise run build
```

Run CLI against current directory:

```bash
mise x -- cargo run -p sorcy -- .
```

Pretty JSON:

```bash
mise x -- cargo run -p sorcy -- . --pretty
```

Write to file:

```bash
mise x -- cargo run -p sorcy -- . --output sorcy-sources.json --pretty
```

Materialize resolved repositories into the hidden local cache while keeping default JSON output:

```bash
mise x -- cargo run -p sorcy -- . --materialize
```

Materialize and print rich JSON (scan + cache + per-resolution clone state):

```bash
mise x -- cargo run -p sorcy -- . --materialize --materialize-rich --pretty
```

Run tests:

```bash
mise run test
```

Optional live smoke tests (opt-in, network + real repo clones):

```bash
mise x -- bash -lc 'SORCY_LIVE_TESTS=1 cargo test -p sorcy-core --test live_registry_optional -- --ignored --nocapture'
```

Local update flow (includes `self-update`) remains available:

```bash
mise run update
```

`mise run ci` remains build + test only and does not run `update` or `self-update`.
