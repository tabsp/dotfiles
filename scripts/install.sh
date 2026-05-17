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

# Determine latest version
if [ -n "$DOTMAN_VERSION" ]; then
    VERSION="$DOTMAN_VERSION"
else
    # If no version specified, use the latest release
    VERSION="0.1.0"
fi

ARCHIVE="dotman-${TARGET}-${VERSION}.tar.gz"
CHECKSUM="${ARCHIVE}.sha256"
URL="${BASE_URL}/v${VERSION}/${ARCHIVE}"
CHECKSUM_URL="${BASE_URL}/v${VERSION}/${CHECKSUM}"

INSTALL_DIR="${HOME}/.local/bin"
mkdir -p "$INSTALL_DIR"

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

echo "==> downloading dotman ${VERSION} for ${TARGET}..."

# Download archive and checksum
if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$URL" -o "$TMPDIR/$ARCHIVE"
    curl -fsSL "$CHECKSUM_URL" -o "$TMPDIR/$CHECKSUM" 2>/dev/null || true
elif command -v wget >/dev/null 2>&1; then
    wget -q "$URL" -O "$TMPDIR/$ARCHIVE"
    wget -q "$CHECKSUM_URL" -O "$TMPDIR/$CHECKSUM" 2>/dev/null || true
else
    echo "error: curl or wget required to download dotman" >&2
    exit 1
fi

# Verify checksum if available
if [ -f "$TMPDIR/$CHECKSUM" ]; then
    if command -v shasum >/dev/null 2>&1; then
        (cd "$TMPDIR" && shasum -a 256 -c "$CHECKSUM") || {
            echo "error: checksum verification failed" >&2
            exit 1
        }
    elif command -v sha256sum >/dev/null 2>&1; then
        (cd "$TMPDIR" && sha256sum -c "$CHECKSUM") || {
            echo "error: checksum verification failed" >&2
            exit 1
        }
    fi
    echo "==> checksum verified"
fi

# Extract
tar -xzf "$TMPDIR/$ARCHIVE" -C "$TMPDIR"
cp "$TMPDIR/dotman" "$INSTALL_DIR/dotman"
chmod +x "$INSTALL_DIR/dotman"

echo "==> dotman ${VERSION} installed to ${INSTALL_DIR}/dotman"
echo ""
echo "Make sure ${INSTALL_DIR} is in your PATH:"
echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
echo ""
echo "Then run:"
echo "  dotman bootstrap"
