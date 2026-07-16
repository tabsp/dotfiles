#!/bin/sh

set -eu

repository="${DOTMAN_REPOSITORY:-tabsp/dotfiles}"
tag=${1:-}
output=${2:-}
checksum_dir="${DOTMAN_CHECKSUM_DIR:-}"

fail() {
  echo "formula updater: $*" >&2
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

case "$tag" in
  v*) valid_version "$tag" || fail "usage: $0 vX.Y.Z path/to/Formula/dotman.rb" ;;
  *) fail "usage: $0 vX.Y.Z path/to/Formula/dotman.rb" ;;
esac
[ -n "$output" ] || fail "usage: $0 vX.Y.Z path/to/Formula/dotman.rb"

version=${tag#v}

checksum() {
  target=$1
  asset="dotman-$target.tar.gz.sha256"
  if [ -n "$checksum_dir" ]; then
    source="$checksum_dir/$asset"
    [ -f "$source" ] || fail "missing checksum file: $source"
    value=$(awk 'NR == 1 { print $1 }' "$source")
  else
    command -v curl >/dev/null 2>&1 || fail "curl is required"
    value=$(curl -fsSL "https://github.com/$repository/releases/download/$tag/$asset" | awk 'NR == 1 { print $1 }')
  fi
  case "$value" in
    *[!0-9a-fA-F]* | '') fail "invalid checksum for $target" ;;
  esac
  [ "${#value}" -eq 64 ] || fail "invalid checksum for $target"
  printf '%s\n' "$value"
}

macos_arm=$(checksum aarch64-apple-darwin)
macos_intel=$(checksum x86_64-apple-darwin)
linux_arm=$(checksum aarch64-unknown-linux-gnu)
linux_intel=$(checksum x86_64-unknown-linux-gnu)

mkdir -p "$(dirname "$output")"
cat >"$output" <<EOF
class Dotman < Formula
  desc "TUI dotfiles deployer for macOS and Linux"
  homepage "https://github.com/$repository"
  version "$version"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/$repository/releases/download/$tag/dotman-aarch64-apple-darwin.tar.gz"
      sha256 "$macos_arm"
    end

    on_intel do
      url "https://github.com/$repository/releases/download/$tag/dotman-x86_64-apple-darwin.tar.gz"
      sha256 "$macos_intel"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/$repository/releases/download/$tag/dotman-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "$linux_arm"
    end

    on_intel do
      url "https://github.com/$repository/releases/download/$tag/dotman-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "$linux_intel"
    end
  end

  def install
    bin.install "dotman"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/dotman --version")
  end
end
EOF

echo "Updated $output for dotman $version"
