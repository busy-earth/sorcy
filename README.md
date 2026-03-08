# sorcy

`sorcy` is a Rust CLI that scans a repository for dependency manifests and outputs
JSON records with:

- dependency name
- source repository URL

The MVP stays intentionally small and stable.

## Workspace layout

`sorcy` is now a 2-crate Cargo workspace:

- `crates/sorcy-core`: scanning, parsing, resolving, normalization, and provenance-rich models.
- `crates/sorcy-cli`: thin CLI wrapper (`sorcy` binary).

The CLI output shape is unchanged:

```json
[
  {
    "dependency": "requests",
    "source_url": "https://github.com/psf/requests"
  }
]
```

Only dependencies with resolved source URLs are emitted in this compatibility JSON output.

## Install (quick)

macOS / Linux:

```bash
curl -LsSf https://raw.githubusercontent.com/busy-earth/sorcy/main/install.sh | sh
```

Windows (PowerShell):

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://raw.githubusercontent.com/busy-earth/sorcy/main/install.ps1 | iex"
```

Notes:

- Installer uses `cargo install --git ... --package sorcy`.
- Rust/Cargo must already be installed.
- Optional env vars:
  - `SORCY_VERSION` (install a specific git tag)
  - `SORCY_REPO_URL` (install from a fork)

## MVP scope

Current behavior:

1. Discover dependency manifests in a repo.
2. Parse dependencies across supported ecosystems.
3. Resolve source repository URLs from source hints or registry metadata.
4. Optionally materialize resolved source repositories into a managed hidden local clone cache.
5. Output JSON records (`dependency`, `source_url`).

What this step still does **not** do:

- build dependency graphs
- parse source files
- run background services
- run filesystem watchers
- include a normalized metadata store or graph database
- include Tree-sitter parsing/indexing

## Supported manifests

- Python:
  - `pyproject.toml`
  - `requirements*.txt`
- npm:
  - `package.json`
- Cargo:
  - `Cargo.toml`
- C/C++:
  - `vcpkg.json`
  - `vcpkg-configuration.json`
  - `conanfile.txt`
  - `conanfile.py`

## Resolver behavior

- Python: PyPI metadata (`project_urls`, `home_page`, `project_url`)
- npm: npm registry metadata (`repository`, `homepage`)
- Cargo: crates.io metadata (`repository`, `homepage`)
- C/C++: local source hints from manifest metadata where present

URL normalization and retry behavior are preserved.

## Build, run, test

Build all workspace members:

```bash
cargo build --workspace
```

Run CLI against current directory:

```bash
cargo run -p sorcy -- .
```

Pretty JSON:

```bash
cargo run -p sorcy -- . --pretty
```

Write to file:

```bash
cargo run -p sorcy -- . --output sorcy-sources.json --pretty
```

Materialize resolved repositories into the hidden local cache while keeping default JSON output:

```bash
cargo run -p sorcy -- . --materialize
```

Materialize and print rich JSON (scan + cache + per-resolution clone state):

```bash
cargo run -p sorcy -- . --materialize --materialize-rich --pretty
```

Run tests:

```bash
cargo test --workspace
```

Optional live smoke tests (opt-in, network + real repo clones):

```bash
SORCY_LIVE_TESTS=1 cargo test -p sorcy-core --test live_registry_optional -- --ignored --nocapture
```

Use this after larger feature work when you want extra confidence against real registries.

## Settings precedence

`sorcy` resolves settings in this order:

1. CLI arguments
2. environment variables
3. defaults

Environment variables:

- `SORCY_PYPI_BASE_URL`
- `SORCY_NPM_BASE_URL`
- `SORCY_CRATES_BASE_URL`
- `SORCY_HTTP_TIMEOUT_SECONDS`
- `SORCY_HTTP_RETRIES`
- `SORCY_HTTP_RETRY_BACKOFF_MS`
- `SORCY_REPO_CACHE_DIR`
- `SORCY_REPO_UPDATE_STRATEGY` (`missing-only` or `fetch-if-present`)

## Local clone cache

When materialization is enabled, `sorcy-core` clones resolved upstream repositories into a hidden
local cache directory:

- default: `$XDG_CACHE_HOME/sorcy` or `~/.cache/sorcy`
- override via `SORCY_REPO_CACHE_DIR` or CLI `--repo-cache-dir`

Cache layout:

- `repos/<host>/<owner>/<repo>` for deterministic human-inspectable clone paths
- `index.json` for persisted clone metadata (status, path, last materialization time, error text)

Default CLI behavior is unchanged unless `--materialize` is used.
