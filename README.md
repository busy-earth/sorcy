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

Windows:

```bash
cargo install --locked --git https://github.com/busy-earth/sorcy sorcy
```

Notes:

- Installer uses `cargo install --git ... sorcy`.
- Rust/Cargo must already be installed.
- Ensure `~/.cargo/bin` is on your `PATH` so `sorcy` is available.
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

## Usage

After install, run `sorcy` directly (from `~/.cargo/bin` on `PATH`).

Default scan (current directory):

```bash
sorcy .
```

Pretty JSON output:

```bash
sorcy . --pretty
```

Write to file:

```bash
sorcy . --output sorcy-sources.json --pretty
```

Materialize resolved repos into local cache:

```bash
sorcy . --materialize
```

Materialize with rich JSON output:

```bash
sorcy . --materialize --materialize-rich --pretty
```

Install the sorcy-rank skill (project-local):

```bash
sorcy install-skill
```

Install the sorcy-rank skill (global):

```bash
sorcy install-skill --global
```

All CLI flags (scan mode):

| Flag | Description |
| -- | -- |
| `--output <path>` | Write output to file instead of stdout |
| `--pretty` | Pretty-print JSON output |
| `--materialize` | Clone resolved repos into local cache |
| `--materialize-rich` | Rich JSON with scan + cache + clone state (requires `--materialize`) |
| `--pypi-base-url` | Override PyPI registry URL |
| `--npm-base-url` | Override npm registry URL |
| `--crates-base-url` | Override [crates.io](https://crates.io) registry URL |
| `--http-timeout-seconds` | HTTP request timeout |
| `--http-retries` | Number of HTTP retries |
| `--http-retry-backoff-ms` | Backoff between retries (ms) |
| `--repo-cache-dir` | Override local clone cache directory |
| `--repo-update-strategy` | `missing-only` (default) or `fetch-if-present` |

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
