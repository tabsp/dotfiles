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
DOTFILES_ARCHIVE_URL="${DOTFILES_ARCHIVE_URL:-https://github.com/tabsp/dotfiles/archive/refs/tags/v${VERSION}.tar.gz}"

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

echo "==> downloading dotfiles source ${VERSION}..."
if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$DOTFILES_ARCHIVE_URL" -o "$TMPDIR/dotfiles-source.tar.gz"
elif command -v wget >/dev/null 2>&1; then
    wget -q "$DOTFILES_ARCHIVE_URL" -O "$TMPDIR/dotfiles-source.tar.gz"
else
    echo "error: curl or wget required to download dotfiles source" >&2
    exit 1
fi

tar -xzf "$TMPDIR/dotfiles-source.tar.gz" -C "$TMPDIR"
SOURCE_ROOT="$(tar -tzf "$TMPDIR/dotfiles-source.tar.gz" | sed -n '1p' | cut -d/ -f1)"
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
