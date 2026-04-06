#!/usr/bin/env bash
set -euo pipefail

# Release script for piratetok-live-rs
# Usage: ./release.sh [patch|minor|major] [commit message]
# Default: patch, message prompted

BUMP="${1:-patch}"
MSG="${2:-}"
CARGO_TOML="Cargo.toml"

if [[ ! -f "$CARGO_TOML" ]]; then
    echo "error: run from the crate root (where Cargo.toml is)"
    exit 1
fi

# --- 1. Clean tree check ---
if [[ -n "$(git status --porcelain)" ]]; then
    echo "error: working tree is dirty — commit or stash first"
    git status --short
    exit 1
fi

# --- 2. Read current version ---
CURRENT=$(grep '^version' "$CARGO_TOML" | head -1 | sed 's/.*"\(.*\)"/\1/')
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT"

case "$BUMP" in
    major) MAJOR=$((MAJOR + 1)); MINOR=0; PATCH=0 ;;
    minor) MINOR=$((MINOR + 1)); PATCH=0 ;;
    patch) PATCH=$((PATCH + 1)) ;;
    *)
        echo "error: unknown bump type '$BUMP' — use patch, minor, or major"
        exit 1
        ;;
esac

NEW="${MAJOR}.${MINOR}.${PATCH}"
TAG="v${NEW}"

echo "bump: ${CURRENT} → ${NEW} (${BUMP})"
echo ""

# --- 3. Check tag doesn't exist ---
if git rev-parse "$TAG" >/dev/null 2>&1; then
    echo "error: tag $TAG already exists"
    exit 1
fi

# --- 4. Bump version in Cargo.toml ---
sed -i "s/^version = \"${CURRENT}\"/version = \"${NEW}\"/" "$CARGO_TOML"
echo "updated $CARGO_TOML"

# --- 5. Build ---
echo ""
echo "=== cargo build ==="
cargo build --release 2>&1
echo ""

# --- 6. Dry run publish (also validates crates.io auth) ---
echo "=== cargo publish --dry-run ==="
if ! cargo publish --dry-run 2>&1; then
    echo ""
    echo "error: dry-run failed — if auth issue, run 'cargo login' first"
    sed -i "s/^version = \"${NEW}\"/version = \"${CURRENT}\"/" "$CARGO_TOML"
    exit 1
fi
echo ""

# --- 7. Lock file update ---
cargo generate-lockfile 2>/dev/null || true

# --- 9. Commit message ---
if [[ -z "$MSG" ]]; then
    DEFAULT_MSG="release: ${TAG}"
    read -rp "commit message [${DEFAULT_MSG}]: " MSG
    MSG="${MSG:-$DEFAULT_MSG}"
fi

# --- 10. Commit + tag ---
git add "$CARGO_TOML" Cargo.lock 2>/dev/null || git add "$CARGO_TOML"
git commit -m "$MSG"
git tag -a "$TAG" -m "$MSG"

echo ""
echo "=== local release ready ==="
echo "  commit: $(git rev-parse --short HEAD)"
echo "  tag:    ${TAG}"
echo ""

# --- 11. Confirm push ---
read -rp "push to origin and publish to crates.io? [y/N] " CONFIRM
if [[ "$CONFIRM" != "y" && "$CONFIRM" != "Y" ]]; then
    echo "aborted — commit and tag are local only"
    echo "to undo: git reset --soft HEAD~1 && git tag -d ${TAG}"
    exit 0
fi

# --- 12. Push ---
BRANCH=$(git branch --show-current)
git push origin "$BRANCH"
git push origin "$TAG"

# --- 13. Publish ---
echo ""
echo "=== cargo publish ==="
cargo publish

echo ""
echo "done: ${TAG} published to crates.io"
echo "  https://crates.io/crates/piratetok-live-rs/${NEW}"
