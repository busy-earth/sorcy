from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import tomllib

from .source_resolver import resolve_source_repo

_NAME_RE = re.compile(r"^\s*([A-Za-z0-9][A-Za-z0-9._-]*)")

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

def render_markdown(pyproject_path: Path, rows: list[ReportRow]) -> str:
    stamp = datetime.now(timezone.utc).replace(microsecond=0).isoformat()
    lines = [
        "# sorcy dependency source report",
        "",
        f"- Project file: `{pyproject_path}`",
        f"- Generated: `{stamp}`",
        "",
        "| Dependency | Source repo | Source |",
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
    rows = [ReportRow(dep, *resolve_source_repo(dep)) for dep in dependencies]

    output_path.write_text(render_markdown(pyproject_path, rows), encoding="utf-8")
    resolved = sum(1 for row in rows if row.repository)
    print(f"Wrote {output_path} ({resolved}/{len(rows)} dependencies resolved to source repos).")
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
