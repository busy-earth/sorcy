from __future__ import annotations

import json
import re
from typing import Any, Callable
from urllib.error import HTTPError, URLError
from urllib.parse import urlparse
from urllib.request import Request, urlopen

_PREFERRED_URL_LABEL_RE = re.compile(r"(source|repository|repo|code|github)", re.IGNORECASE)
_PYPI_PROJECT_URL_RE = re.compile(r"^https?://pypi\.org/project/[^/]+/?$", re.IGNORECASE)
_SCP_STYLE_GIT_URL_RE = re.compile(r"^(?:[^@/]+@)?([A-Za-z0-9.-]+):([^?#]+)$")
_FORGE_PATH_CUT_MARKERS = (
    "/-/",
    "/tree/",
    "/blob/",
    "/src/",
    "/raw/",
    "/issues/",
    "/pull/",
    "/pulls/",
    "/merge_requests/",
    "/commit/",
    "/commits/",
    "/release/",
    "/releases/",
    "/wiki/",
)
_KNOWN_FORGE_HOSTS = {
    "github.com",
    "gitlab.com",
    "bitbucket.org",
    "codeberg.org",
    "git.sr.ht",
}
_KNOWN_FORGE_HOST_TOKENS = ("github", "gitlab", "bitbucket", "codeberg", "sourcehut", "gitea", "forgejo")


def _looks_like_forge_host(host: str) -> bool:
    lowered = host.lower()
    if lowered in _KNOWN_FORGE_HOSTS:
        return True
    return any(token in lowered for token in _KNOWN_FORGE_HOST_TOKENS)


def _extract_repo_path(path: str) -> str | None:
    trimmed = path
    for marker in _FORGE_PATH_CUT_MARKERS:
        if marker in trimmed:
            trimmed = trimmed.split(marker, 1)[0]
    trimmed = trimmed.strip().strip("/")
    if trimmed.endswith(".git"):
        trimmed = trimmed[:-4]
    parts = [part for part in trimmed.split("/") if part]
    if len(parts) < 2:
        return None
    return "/".join(parts)


def extract_source_repo(candidate_url: str, *, allow_unlisted_host: bool = False) -> tuple[str, str] | None:
    cleaned = candidate_url.strip()
    if not cleaned:
        return None
    if cleaned.startswith("git+"):
        cleaned = cleaned[4:]

    host = ""
    path = ""
    if "://" in cleaned:
        parsed = urlparse(cleaned)
        host = (parsed.hostname or "").lower()
        path = parsed.path
    else:
        match = _SCP_STYLE_GIT_URL_RE.match(cleaned)
        if not match:
            return None
        host = match.group(1).lower()
        path = match.group(2)

    if not host or host == "pypi.org":
        return None
    if not allow_unlisted_host and not _looks_like_forge_host(host):
        return None

    repo_path = _extract_repo_path(path)
    if not repo_path:
        return None

    repo = f"{host}/{repo_path}"
    return repo, f"https://{host}/{repo_path}"


def extract_github_repo(candidate_url: str) -> str | None:
    extracted = extract_source_repo(candidate_url, allow_unlisted_host=True)
    if not extracted:
        return None
    repo, _ = extracted
    if not repo.startswith("github.com/"):
        return None
    return repo.split("/", 1)[1]


def _project_url_candidates(info: dict[str, Any]) -> list[tuple[str, bool]]:
    preferred: list[tuple[str, bool]] = []
    fallback: list[tuple[str, bool]] = []
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
                preferred.append((cleaned, True))
            else:
                fallback.append((cleaned, False))

    for key in ("home_page", "project_url"):
        value = info.get(key)
        if not isinstance(value, str):
            continue
        cleaned = value.strip()
        if not cleaned or _PYPI_PROJECT_URL_RE.match(cleaned) or cleaned in seen:
            continue
        seen.add(cleaned)
        fallback.append((cleaned, False))

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


def resolve_source_repo(
    package_name: str, fetcher: Callable[[str], dict[str, Any] | None] = fetch_pypi_json
) -> tuple[str | None, str | None]:
    payload = fetcher(package_name)
    if not payload:
        return None, None

    info = payload.get("info", {})
    for candidate, preferred_label in _project_url_candidates(info):
        extracted = extract_source_repo(candidate, allow_unlisted_host=preferred_label)
        if extracted:
            return extracted
    return None, None


def resolve_github_repo(
    package_name: str, fetcher: Callable[[str], dict[str, Any] | None] = fetch_pypi_json
) -> tuple[str | None, str | None]:
    return resolve_source_repo(package_name, fetcher=fetcher)
