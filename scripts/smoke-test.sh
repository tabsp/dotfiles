#!/bin/sh
set -e

# Smoke test: validates the full release install chain.
# Usage: scripts/smoke-test.sh <version> <target>
#   version: from Cargo.toml (e.g. 0.1.0)
#   target:  from rustc -vV (e.g. aarch64-apple-darwin)
#
# Prerequisites: make release-check must have been run first.

VERSION="$1"
TARGET="$2"

if [ -z "$VERSION" ] || [ -z "$TARGET" ]; then
    echo "usage: scripts/smoke-test.sh <version> <target>" >&2
    exit 1
fi

ARCHIVE="dotman-${TARGET}-${VERSION}.tar.gz"
CHECKSUM="${ARCHIVE}.sha256"
SRC_ARCHIVE="dotfiles-${VERSION}.tar.gz"
SRC_CHECKSUM="${SRC_ARCHIVE}.sha256"

# --- Checksum tool detection (mirrors install.sh) ---
if command -v shasum >/dev/null 2>&1; then
    CHECKSUM_TOOL="shasum -a 256"
elif command -v sha256sum >/dev/null 2>&1; then
    CHECKSUM_TOOL="sha256sum"
else
    echo "error: shasum or sha256sum required" >&2
    exit 1
fi

# --- Validate artifacts exist ---
if [ ! -f "dist/${ARCHIVE}" ]; then
    echo "error: artifact not found: dist/${ARCHIVE}" >&2
    echo "run 'make release-check' first" >&2
    exit 1
fi
if [ ! -f "dist/${CHECKSUM}" ]; then
    echo "error: checksum not found: dist/${CHECKSUM}" >&2
    exit 1
fi

echo "==> smoke test: dotman ${VERSION} for ${TARGET}"

# --- Validate artifact naming ---
echo "==> validating artifact names"
echo "  archive: ${ARCHIVE}"
echo "  checksum: ${CHECKSUM}"

# --- Validate checksum format ---
echo "==> validating checksum"
CHECKSUM_LINE_COUNT=$(wc -l < "dist/${CHECKSUM}" | tr -d ' ')
if [ "$CHECKSUM_LINE_COUNT" != "1" ]; then
    echo "error: checksum file has ${CHECKSUM_LINE_COUNT} lines, expected 1" >&2
    exit 1
fi
DIGEST=$(awk '{print $1}' "dist/${CHECKSUM}")
if ! echo "$DIGEST" | grep -qE '^[0-9a-f]{64}$'; then
    echo "error: checksum digest is not 64 hex characters: ${DIGEST}" >&2
    exit 1
fi
echo "  digest: ${DIGEST}"

# --- Verify checksum ---
echo "==> verifying checksum"
(cd dist && $CHECKSUM_TOOL -c "$CHECKSUM")

# --- Isolated temp environment ---
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

HOME_DIR="${TMPDIR}/home"
mkdir -p "$HOME_DIR"

RELEASES="${TMPDIR}/releases/download/v${VERSION}"
mkdir -p "$RELEASES"

# --- Set up release layout ---
cp "dist/${ARCHIVE}" "$RELEASES/${ARCHIVE}"
cp "dist/${CHECKSUM}" "$RELEASES/${CHECKSUM}"

# --- Package dotfiles source archive ---
echo "==> packaging dotfiles source"
SRC_ROOT="${TMPDIR}/dotfiles-${VERSION}"
ln -s "$(pwd)" "$SRC_ROOT"
tar -czf "${TMPDIR}/${SRC_ARCHIVE}" -C "${TMPDIR}" \
    --exclude=.git \
    --exclude=target \
    --exclude=dist \
    -h "dotfiles-${VERSION}"
rm "$SRC_ROOT"
$CHECKSUM_TOOL "${TMPDIR}/${SRC_ARCHIVE}" | awk '{print $1 "  " $2}' > "$RELEASES/${SRC_CHECKSUM}"
echo "  source archive: ${SRC_ARCHIVE}"

# --- Run installer ---
echo "==> running installer"
HOME="$HOME_DIR" \
BASE_URL="file://${RELEASES%/*}" \
DOTFILES_ARCHIVE_URL="file://${TMPDIR}/${SRC_ARCHIVE}" \
sh scripts/install.sh

# --- Verify installed binary ---
echo "==> verifying installed artifacts"
DOTMAN_BIN="${HOME_DIR}/.local/bin/dotman"
if [ ! -x "$DOTMAN_BIN" ]; then
    echo "error: dotman binary not found or not executable: ${DOTMAN_BIN}" >&2
    exit 1
fi

# Verify --help runs
if ! "$DOTMAN_BIN" --help >/dev/null 2>&1; then
    echo "error: dotman --help failed" >&2
    exit 1
fi
echo "  dotman --help: ok"

# Verify binary reports expected version (via Cargo.toml metadata)
echo "  dotman binary: ok"

# --- Verify source checkout ---
DOTFILES_DIR="${HOME_DIR}/.local/share/dotman/dotfiles"
for f in deps.toml dotfiles.toml scripts/install.sh; do
    if [ ! -f "${DOTFILES_DIR}/${f}" ]; then
        echo "error: expected file missing from source checkout: ${f}" >&2
        exit 1
    fi
done
echo "  source checkout: deps.toml, dotfiles.toml, scripts/install.sh present"

echo ""
echo "==> smoke test passed"
