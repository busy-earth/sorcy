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
4. Output JSON records (`dependency`, `source_url`).

What this step still does **not** do:

- clone repositories
- build dependency graphs
- parse source files
- run background services

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

Run tests:

```bash
cargo test --workspace
```

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
