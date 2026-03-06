# sorcy

`sorcy` is a small CLI that reads Python dependencies from `pyproject.toml`, looks up their source repository metadata from PyPI, and writes a Markdown report with forge repository pointers.

## Why this exists

Coding agents and humans often need quick links to the actual source code of dependencies.  
Package manager output is useful, but it usually does not directly produce a clean Markdown list of repository targets for downstream tools.

## MVP scope (current)

- Input: `pyproject.toml`
- Dependencies scanned:
  - `[project.dependencies]`
  - `[project.optional-dependencies]` (unless `--no-optional`)
  - `[dependency-groups]` (unless `--no-groups`)
  - Poetry fallback sections (`[tool.poetry.dependencies]` and poetry groups)
- Resolver: PyPI JSON API metadata (`project_urls`, `home_page`, `project_url`)
- Output: Markdown table (`sorcy-dependencies.md` by default)
- Forge support: GitHub, GitLab, Bitbucket, Codeberg, SourceHut (`git.sr.ht`), plus easy extension for new forges

## Install (local dev)

```bash
python3 -m pip install -e .
```

## Usage

Run in a project directory (or pass a path):

```bash
python3 -m sorcy .
```

Write to a custom file:

```bash
python3 -m sorcy . -o dependency-sources.md
```

Ignore optional dependencies:

```bash
python3 -m sorcy . --no-optional
```

Ignore dependency groups:

```bash
python3 -m sorcy . --no-groups
```

## Output format

The report is Markdown and includes:

- dependency name
- source repo as `host/path` (if found)
- clickable source URL

Dependencies with no usable source metadata are marked as `_not found_`.

## Reliability and security notes

- `sorcy` only reads TOML and queries PyPI JSON over HTTPS.
- It does **not** execute dependency code.
- Network or metadata issues fail gracefully per dependency when possible.
- Name parsing normalizes dependency names to reduce duplicates.

## Test

```bash
python3 -m unittest discover -s tests -v
```

## About uv / linters

- `uv` is great for resolution/lock/install workflows, but it does not currently provide this exact Markdown source repo report out of the box.
- Linters are focused on code quality/style/static checks, not dependency source repo mapping.
- `sorcy` is intended to fill that narrow gap cleanly.

## Project values

- Forge-neutral by default. We support multiple forges and avoid hard-coding one vendor as "the only source of truth."
- Fast support for new forge hosts. Add host rules in `src/sorcy/source_resolver.py` so community migration can be supported quickly.
- Prefer open packaging metadata (PyPI JSON) as the primary signal.
- Contribution workflow and policy: see `CONTRIBUTING.md`.

## Roadmap

1. Add import scanning as an optional signal.
2. Expand to other language ecosystems.
3. Add transitive dependency source mapping.
4. Add machine-readable output mode (JSON) alongside Markdown.
