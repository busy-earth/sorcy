#!/usr/bin/env bash
set -euo pipefail

tool="${1:-}"
if [[ -z "$tool" ]]; then
  echo "usage: $0 <tool>" >&2
  exit 2
fi

if ! command -v mise >/dev/null 2>&1; then
  echo "mise not found on PATH" >&2
  exit 3
fi

version="$(mise latest "$tool" | tail -n1 | tr -d '[:space:]')"

if [[ -z "$version" ]]; then
  echo "could not resolve latest version for: $tool" >&2
  exit 4
fi

printf '%s\n' "$version"
