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

# Extract current version from [workspace.package]
CURRENT_VERSION=$(sed -n '/^\[workspace\.package\]/,/^\[/{s/^version = "\(.*\)"/\1/p}' Cargo.toml)

# Update [workspace.package] version
sed -i '/^\[workspace\.package\]/,/^\[/{s/^version = ".*"/version = "'"$VERSION"'"/}' Cargo.toml

# Update intra-workspace dep versions in [workspace.dependencies]
sed -i '/^\[workspace\.dependencies\]/,/^\[/{s/version = "'"$CURRENT_VERSION"'"/version = "'"$VERSION"'"/g}' Cargo.toml

# Update pyproject.toml (only the [project] section)
sed -i '/^\[project\]/,/^\[/{s/^version = ".*"/version = "'"$VERSION"'"/}' pyproject.toml

# Update Cargo.lock to reflect new version
cargo update --workspace

# Regenerate README from doc comments (mirrors .github/workflows/readme.yml)
cargo doc2readme

git checkout -b "$BRANCH"
git add Cargo.toml pyproject.toml README.md Cargo.lock
git commit -m "chore(release): prepare for v${VERSION}"

echo "Done. Branch '$BRANCH' created with version bumped to $VERSION."
