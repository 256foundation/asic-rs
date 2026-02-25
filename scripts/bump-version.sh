#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-}"

if [[ -z "$VERSION" ]]; then
    echo "Usage: $0 <version>  (e.g. $0 0.2.7)" >&2
    exit 1
fi

if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Error: version must be in X.Y.Z format, got: $VERSION" >&2
    exit 1
fi

BRANCH="release-v${VERSION}"

# Update Cargo.toml (only the [package] section)
sed -i '/^\[package\]/,/^\[/{s/^version = ".*"/version = "'"$VERSION"'"/}' Cargo.toml

# Update pyproject.toml (only the [project] section)
sed -i '/^\[project\]/,/^\[/{s/^version = ".*"/version = "'"$VERSION"'"/}' pyproject.toml

# Regenerate README from doc comments
if ! cargo doc2readme 2>/dev/null; then
    echo "Warning: cargo doc2readme failed or not installed; README not updated" >&2
fi

git checkout -b "$BRANCH"
git add Cargo.toml pyproject.toml README.md
git commit -m "chore(release): prepare for v${VERSION}"

echo "Done. Branch '$BRANCH' created with version bumped to $VERSION."
