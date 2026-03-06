# Contributing to sorcy

Thanks for helping improve `sorcy`.

## Core principles

- Keep changes simple and reliable.
- Prefer official packaging metadata over scraping.
- Stay forge-neutral: do not assume one hosting provider is always correct.
- Add tests for every behavior change.

## Adding support for a new forge

Source resolution lives in `src/sorcy/source_resolver.py`.

When adding a forge:

1. Add host matching logic in `_KNOWN_FORGE_HOSTS` or `_KNOWN_FORGE_HOST_TOKENS`.
2. Add URL parsing rules only if needed.
3. Add unit tests in `tests/test_cli.py` for:
   - direct extraction (`extract_source_repo`)
   - end-to-end resolution (`resolve_source_repo`)
4. Keep backward compatibility when possible.

## Note about `AGENTS.md`

`AGENTS.md` is for coding-agent environment behavior.  
Project mission and contributor policy should live in `README.md` and `CONTRIBUTING.md`.
