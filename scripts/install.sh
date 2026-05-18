#!/bin/sh
set -e

# dotman install script
# Downloads and installs the correct dotman binary for the current platform.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/tabsp/dotfiles/main/scripts/install.sh | sh
#
# Or with a custom base URL:
#   BASE_URL=https://example.com/releases sh install.sh

BASE_URL="${BASE_URL:-https://github.com/tabsp/dotfiles/releases/download}"
DOTFILES_DIR="${DOTFILES_DIR:-${HOME}/.local/share/dotman/dotfiles}"

# Detect OS and architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Darwin)
        case "$ARCH" in
            arm64) TARGET="aarch64-apple-darwin" ;;
            x86_64) TARGET="x86_64-apple-darwin" ;;
            *) echo "error: unsupported macOS architecture: $ARCH" >&2; exit 1 ;;
        esac
        ;;
    Linux)
        case "$ARCH" in
            x86_64) TARGET="x86_64-unknown-linux-gnu" ;;
            *) echo "error: unsupported Linux architecture: $ARCH" >&2; exit 1 ;;
        esac
        ;;
    *)
        echo "error: unsupported OS: $OS" >&2
        echo "dotman supports macOS and Linux. Windows is not supported." >&2
        exit 1
        ;;
esac

# Determine version
if [ -n "$DOTMAN_VERSION" ]; then
    VERSION="$DOTMAN_VERSION"
else
    VERSION="0.1.0"
fi

# Find checksum tool early; fail if unavailable
if command -v shasum >/dev/null 2>&1; then
    CHECKSUM_TOOL="shasum -a 256"
elif command -v sha256sum >/dev/null 2>&1; then
    CHECKSUM_TOOL="sha256sum"
else
    echo "error: shasum or sha256sum required for checksum verification" >&2
    exit 1
fi

ARCHIVE="dotman-${TARGET}-${VERSION}.tar.gz"
CHECKSUM="${ARCHIVE}.sha256"
URL="${BASE_URL}/v${VERSION}/${ARCHIVE}"
CHECKSUM_URL="${BASE_URL}/v${VERSION}/${CHECKSUM}"
DOTFILES_SOURCE_ARCHIVE="dotfiles-${VERSION}.tar.gz"
DOTFILES_SOURCE_URL="${DOTFILES_ARCHIVE_URL:-${BASE_URL}/v${VERSION}/${DOTFILES_SOURCE_ARCHIVE}}"
DOTFILES_SOURCE_CHECKSUM_URL="${BASE_URL}/v${VERSION}/${DOTFILES_SOURCE_ARCHIVE}.sha256"

INSTALL_DIR="${HOME}/.local/bin"
mkdir -p "$INSTALL_DIR"

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

echo "==> downloading dotman ${VERSION} for ${TARGET}..."

# Download binary archive and checksum
if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$URL" -o "$TMPDIR/$ARCHIVE"
    curl -fsSL "$CHECKSUM_URL" -o "$TMPDIR/$CHECKSUM"
elif command -v wget >/dev/null 2>&1; then
    wget -q "$URL" -O "$TMPDIR/$ARCHIVE"
    wget -q "$CHECKSUM_URL" -O "$TMPDIR/$CHECKSUM"
else
    echo "error: curl or wget required to download dotman" >&2
    exit 1
fi

# Verify binary checksum
(cd "$TMPDIR" && $CHECKSUM_TOOL -c "$CHECKSUM") || {
    echo "error: checksum verification failed for dotman binary" >&2
    exit 1
}
echo "==> binary checksum verified"

# Extract and install binary
tar -xzf "$TMPDIR/$ARCHIVE" -C "$TMPDIR"
cp "$TMPDIR/dotman" "$INSTALL_DIR/dotman"
chmod +x "$INSTALL_DIR/dotman"

# Download dotfiles source archive and checksum
echo "==> downloading dotfiles source ${VERSION}..."
if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$DOTFILES_SOURCE_URL" -o "$TMPDIR/$DOTFILES_SOURCE_ARCHIVE"
    curl -fsSL "$DOTFILES_SOURCE_CHECKSUM_URL" -o "$TMPDIR/$DOTFILES_SOURCE_ARCHIVE.sha256"
elif command -v wget >/dev/null 2>&1; then
    wget -q "$DOTFILES_SOURCE_URL" -O "$TMPDIR/$DOTFILES_SOURCE_ARCHIVE"
    wget -q "$DOTFILES_SOURCE_CHECKSUM_URL" -O "$TMPDIR/$DOTFILES_SOURCE_ARCHIVE.sha256"
fi

# Verify dotfiles source checksum
(cd "$TMPDIR" && $CHECKSUM_TOOL -c "$DOTFILES_SOURCE_ARCHIVE.sha256") || {
    echo "error: checksum verification failed for dotfiles source" >&2
    exit 1
}
echo "==> dotfiles source checksum verified"

# Extract and install dotfiles source
tar -xzf "$TMPDIR/$DOTFILES_SOURCE_ARCHIVE" -C "$TMPDIR"
SOURCE_ROOT="$(tar -tzf "$TMPDIR/$DOTFILES_SOURCE_ARCHIVE" | sed -n '1p' | cut -d/ -f1)"
if [ -z "$SOURCE_ROOT" ] || [ ! -d "$TMPDIR/$SOURCE_ROOT" ]; then
    echo "error: failed to locate dotfiles source root in archive" >&2
    exit 1
fi

rm -rf "$TMPDIR/dotfiles"
mv "$TMPDIR/$SOURCE_ROOT" "$TMPDIR/dotfiles"
mkdir -p "$(dirname "$DOTFILES_DIR")"
if [ -e "$DOTFILES_DIR" ]; then
    BACKUP_DIR="${DOTFILES_DIR}.backup.$(date +%Y%m%d%H%M%S)"
    mv "$DOTFILES_DIR" "$BACKUP_DIR"
    echo "==> existing dotfiles source moved to ${BACKUP_DIR}"
fi
mv "$TMPDIR/dotfiles" "$DOTFILES_DIR"

echo "==> dotman ${VERSION} installed to ${INSTALL_DIR}/dotman"
echo "==> dotfiles source installed to ${DOTFILES_DIR}"
echo ""
echo "Make sure ${INSTALL_DIR} is in your PATH:"
echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
echo ""
echo "Then run:"
echo "  cd ${DOTFILES_DIR} && dotman bootstrap"
