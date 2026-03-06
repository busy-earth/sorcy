from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Callable
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

import tomllib

_NAME_RE = re.compile(r"^\s*([A-Za-z0-9][A-Za-z0-9._-]*)")
_GITHUB_RE = re.compile(r"github\.com[:/]+([A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+)", re.IGNORECASE)
_PREFERRED_URL_LABEL_RE = re.compile(r"(source|repository|repo|code|github)", re.IGNORECASE)
_PYPI_PROJECT_URL_RE = re.compile(r"^https?://pypi\.org/project/[^/]+/?$", re.IGNORECASE)
@dataclass(frozen=True)
class ReportRow:
    dependency: str
    repository: str | None
    repository_url: str | None

def normalize_name(name: str) -> str:
    return re.sub(r"[-_.]+", "-", name).lower().strip()

def parse_requirement_name(spec: str) -> str | None:
    match = _NAME_RE.match(spec)
    return normalize_name(match.group(1)) if match else None

def collect_dependency_names(
    pyproject_data: dict[str, Any],
    *,
    include_optional: bool = True,
    include_groups: bool = True,
) -> list[str]:
    names: set[str] = set()
    project = pyproject_data.get("project", {})

    for spec in project.get("dependencies", []) if isinstance(project.get("dependencies"), list) else []:
        if isinstance(spec, str):
            name = parse_requirement_name(spec)
            if name:
                names.add(name)

    if include_optional:
        optional = project.get("optional-dependencies", {})
        if isinstance(optional, dict):
            for specs in optional.values():
                if isinstance(specs, list):
                    for spec in specs:
                        if isinstance(spec, str):
                            name = parse_requirement_name(spec)
                            if name:
                                names.add(name)

    if include_groups:
        groups = pyproject_data.get("dependency-groups", {})
        if isinstance(groups, dict):
            for specs in groups.values():
                if isinstance(specs, list):
                    for spec in specs:
                        if isinstance(spec, str):
                            name = parse_requirement_name(spec)
                            if name:
                                names.add(name)

    # Poetry compatibility for projects not using [project] yet.
    poetry = pyproject_data.get("tool", {}).get("poetry", {})
    if isinstance(poetry, dict):
        deps = poetry.get("dependencies", {})
        if isinstance(deps, dict):
            for dep_name in deps:
                if isinstance(dep_name, str) and dep_name.lower() != "python":
                    names.add(normalize_name(dep_name))

        if include_groups:
            poetry_groups = poetry.get("group", {})
            if isinstance(poetry_groups, dict):
                for group in poetry_groups.values():
                    if isinstance(group, dict):
                        gdeps = group.get("dependencies", {})
                        if isinstance(gdeps, dict):
                            for dep_name in gdeps:
                                if isinstance(dep_name, str):
                                    names.add(normalize_name(dep_name))

    return sorted(names)

def extract_github_repo(candidate_url: str) -> str | None:
    match = _GITHUB_RE.search(candidate_url.strip())
    if not match:
        return None
    repo = match.group(1).strip().rstrip("/")
    if repo.endswith(".git"):
        repo = repo[:-4]
    return repo if "/" in repo else None

def _project_url_candidates(info: dict[str, Any]) -> list[str]:
    preferred: list[str] = []
    fallback: list[str] = []
    seen: set[str] = set()

    project_urls = info.get("project_urls", {})
    if isinstance(project_urls, dict):
        for label, url in project_urls.items():
            if not isinstance(url, str):
                continue
            cleaned = url.strip()
            if not cleaned or _PYPI_PROJECT_URL_RE.match(cleaned) or cleaned in seen:
                continue
            seen.add(cleaned)
            if isinstance(label, str) and _PREFERRED_URL_LABEL_RE.search(label):
                preferred.append(cleaned)
            else:
                fallback.append(cleaned)

    for key in ("home_page", "project_url"):
        value = info.get(key)
        if not isinstance(value, str):
            continue
        cleaned = value.strip()
        if not cleaned or _PYPI_PROJECT_URL_RE.match(cleaned) or cleaned in seen:
            continue
        seen.add(cleaned)
        fallback.append(cleaned)

    return [*preferred, *fallback]

def fetch_pypi_json(package_name: str, timeout: int = 8) -> dict[str, Any] | None:
    request = Request(f"https://pypi.org/pypi/{package_name}/json", headers={"User-Agent": "sorcy/0.1"})
    try:
        with urlopen(request, timeout=timeout) as response:
            return json.loads(response.read().decode("utf-8"))
    except HTTPError as exc:
        if exc.code == 404:
            return None
        raise
    except (URLError, TimeoutError, json.JSONDecodeError):
        return None

def resolve_github_repo(
    package_name: str, fetcher: Callable[[str], dict[str, Any] | None] = fetch_pypi_json
) -> tuple[str | None, str | None]:
    payload = fetcher(package_name)
    if not payload:
        return None, None

    info = payload.get("info", {})
    for candidate in _project_url_candidates(info):
        repo = extract_github_repo(candidate)
        if repo:
            return repo, f"https://github.com/{repo}"
    return None, None

def render_markdown(pyproject_path: Path, rows: list[ReportRow]) -> str:
    stamp = datetime.now(timezone.utc).replace(microsecond=0).isoformat()
    lines = [
        "# sorcy dependency source report",
        "",
        f"- Project file: `{pyproject_path}`",
        f"- Generated: `{stamp}`",
        "",
        "| Dependency | GitHub repo | Source |",
        "|---|---|---|",
    ]
    for row in rows:
        if row.repository and row.repository_url:
            lines.append(f"| `{row.dependency}` | `{row.repository}` | [link]({row.repository_url}) |")
        else:
            lines.append(f"| `{row.dependency}` | _not found_ | - |")
    lines.append("")
    return "\n".join(lines)

def run(project_path: Path, output_path: Path, include_optional: bool, include_groups: bool) -> int:
    pyproject_path = project_path if project_path.is_file() else project_path / "pyproject.toml"
    if not pyproject_path.exists():
        raise FileNotFoundError(f"No pyproject.toml found at: {pyproject_path}")

    pyproject_data = tomllib.loads(pyproject_path.read_text(encoding="utf-8"))
    dependencies = collect_dependency_names(
        pyproject_data, include_optional=include_optional, include_groups=include_groups
    )
    rows = [ReportRow(dep, *resolve_github_repo(dep)) for dep in dependencies]

    output_path.write_text(render_markdown(pyproject_path, rows), encoding="utf-8")
    resolved = sum(1 for row in rows if row.repository)
    print(f"Wrote {output_path} ({resolved}/{len(rows)} dependencies resolved to GitHub).")
    return 0

def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="sorcy", description="Generate a Markdown report of dependency source repositories."
    )
    parser.add_argument("path", nargs="?", default=".", help="Project directory or pyproject.toml path.")
    parser.add_argument("-o", "--output", default="sorcy-dependencies.md", help="Markdown output path.")
    parser.add_argument("--no-optional", action="store_true", help="Ignore optional dependency groups.")
    parser.add_argument("--no-groups", action="store_true", help="Ignore dependency-groups from pyproject.")
    args = parser.parse_args(argv)

    try:
        return run(
            project_path=Path(args.path),
            output_path=Path(args.output),
            include_optional=not args.no_optional,
            include_groups=not args.no_groups,
        )
    except Exception as exc:  # noqa: BLE001 - CLI should print a single readable error.
        print(f"Error: {exc}", file=sys.stderr)
        return 1

if __name__ == "__main__":
    raise SystemExit(main())
