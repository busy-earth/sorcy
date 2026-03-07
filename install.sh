#!/usr/bin/env sh
set -eu

if ! command -v cargo >/dev/null 2>&1; then
  echo "Error: cargo is required to install sorcy." >&2
  echo "Install Rust from https://rustup.rs and run this script again." >&2
  exit 1
fi

REPO_URL="${SORCY_REPO_URL:-https://github.com/busy-earth/sorcy}"
VERSION_TAG="${SORCY_VERSION:-}"

if [ -n "${VERSION_TAG}" ]; then
  echo "Installing sorcy from ${REPO_URL} (tag: ${VERSION_TAG})..."
  cargo install --locked --git "${REPO_URL}" --tag "${VERSION_TAG}" --package sorcy
else
  echo "Installing sorcy from ${REPO_URL} (default branch)..."
  cargo install --locked --git "${REPO_URL}" --package sorcy
fi

echo "sorcy installed. Make sure ~/.cargo/bin is in your PATH."
