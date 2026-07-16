#!/bin/sh

set -eu

repository="${DOTMAN_REPOSITORY:-tabsp/dotfiles}"
version="${DOTMAN_VERSION:-latest}"
install_dir="${DOTMAN_INSTALL_DIR:-$HOME/.local/bin}"
download_root="${DOTMAN_DOWNLOAD_ROOT:-https://github.com/$repository/releases}"

fail() {
  echo "dotman installer: $*" >&2
  exit 1
}

valid_version() {
  candidate=${1#v}
  major=${candidate%%.*}
  remainder=${candidate#*.}
  [ "$remainder" != "$candidate" ] || return 1
  minor=${remainder%%.*}
  patch=${remainder#*.}
  [ "$patch" != "$remainder" ] || return 1
  case "$patch" in
    *.*) return 1 ;;
  esac
  for part in "$major" "$minor" "$patch"; do
    case "$part" in
      '' | *[!0-9]*) return 1 ;;
    esac
  done
}

command -v curl >/dev/null 2>&1 || fail "curl is required"
command -v tar >/dev/null 2>&1 || fail "tar is required"

case "$version" in
  latest) release_base="$download_root/latest/download" ;;
  *)
    valid_version "$version" || fail "invalid version '$version' (expected latest or vX.Y.Z)"
    case "$version" in
      v*) release_tag=$version ;;
      *) release_tag="v$version" ;;
    esac
    release_base="$download_root/download/$release_tag"
    ;;
esac

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64 | Darwin-aarch64) target="aarch64-apple-darwin" ;;
  Darwin-x86_64 | Darwin-amd64) target="x86_64-apple-darwin" ;;
  Linux-arm64 | Linux-aarch64) target="aarch64-unknown-linux-gnu" ;;
  Linux-x86_64 | Linux-amd64) target="x86_64-unknown-linux-gnu" ;;
  *) fail "unsupported platform: $(uname -s)-$(uname -m)" ;;
esac

artifact="dotman-$target.tar.gz"
tmp_dir=$(mktemp -d "${TMPDIR:-/tmp}/dotman-install.XXXXXX")
stage=""
cleanup() {
  rm -rf "$tmp_dir"
  if [ -n "$stage" ]; then
    rm -f "$stage"
  fi
}
trap cleanup EXIT HUP INT TERM

echo "Downloading $artifact ($version)..."
curl -fsSL "$release_base/$artifact" -o "$tmp_dir/$artifact"
curl -fsSL "$release_base/$artifact.sha256" -o "$tmp_dir/$artifact.sha256"

expected=$(awk 'NR == 1 { print $1 }' "$tmp_dir/$artifact.sha256")
case "$expected" in
  *[!0-9a-fA-F]* | '') fail "release checksum is invalid" ;;
esac
[ "${#expected}" -eq 64 ] || fail "release checksum is invalid"

if command -v sha256sum >/dev/null 2>&1; then
  actual=$(sha256sum "$tmp_dir/$artifact" | awk '{ print $1 }')
elif command -v shasum >/dev/null 2>&1; then
  actual=$(shasum -a 256 "$tmp_dir/$artifact" | awk '{ print $1 }')
else
  fail "sha256sum or shasum is required"
fi

[ "$actual" = "$expected" ] || fail "checksum verification failed for $artifact"

tar -xzf "$tmp_dir/$artifact" -C "$tmp_dir" dotman
[ -s "$tmp_dir/dotman" ] || fail "release archive does not contain a dotman binary"

mkdir -p "$install_dir"
stage=$(mktemp "$install_dir/.dotman.XXXXXX")
cp "$tmp_dir/dotman" "$stage"
chmod 755 "$stage"
mv -f "$stage" "$install_dir/dotman"
stage=""

echo "Installed $install_dir/dotman"
"$install_dir/dotman" --version

case ":${PATH:-}:" in
  *":$install_dir:"*) ;;
  *) echo "Add $install_dir to PATH before running dotman." ;;
esac
