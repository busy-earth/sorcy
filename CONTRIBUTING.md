# Contributing to sorcy

Thanks for helping improve `sorcy`.

## Core principles

- Keep changes simple and reliable.
- Prefer official packaging metadata over scraping.
- Stay forge-neutral: do not assume one hosting provider is always correct.
- Add tests for every behavior change.

## Project structure

- `src/scan.rs`: manifest discovery
- `src/parse/*`: per-ecosystem manifest readers (via `ManifestParser` trait)
- `src/resolve.rs`: source URL resolution (via `SourceResolver` trait)
- `src/settings.rs`: settings resolution (`CLI > env > defaults`)
- `src/lib.rs`: end-to-end orchestration
- `src/cli.rs`: CLI behavior
- `tests/integration_mvp.rs`: end-to-end tests

## Adding support for a new forge host

Source host checks and URL normalization live in `src/resolve.rs`.

When adding a forge:

1. Update forge host matching constants.
2. Add URL normalization rules only when needed.
3. Add unit tests in `src/resolve.rs`.
4. Add an integration test path in `tests/integration_mvp.rs` if behavior changes end-to-end.
5. Keep existing output record shape stable.

## Installer scripts

- `install.sh` and `install.ps1` provide one-command installation for MVP usage.
- Keep them simple, auditable, and shell-safe.
- If behavior changes, update README install examples in the same PR.

## Running tests

```bash
cargo test
```

## Note about `AGENTS.md`

`AGENTS.md` is for coding-agent environment behavior.  
Project mission and contributor policy should live in `README.md` and `CONTRIBUTING.md`.
