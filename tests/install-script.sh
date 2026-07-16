#!/bin/sh

set -eu

root=$(mktemp -d "${TMPDIR:-/tmp}/dotman-installer-test.XXXXXX")
cleanup() {
  rm -rf "$root"
}
trap cleanup EXIT HUP INT TERM

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64 | Darwin-aarch64) target="aarch64-apple-darwin" ;;
  Darwin-x86_64 | Darwin-amd64) target="x86_64-apple-darwin" ;;
  Linux-arm64 | Linux-aarch64) target="aarch64-unknown-linux-gnu" ;;
  Linux-x86_64 | Linux-amd64) target="x86_64-unknown-linux-gnu" ;;
  *) echo "unsupported test platform" >&2; exit 1 ;;
esac

release_dir="$root/releases/latest/download"
install_dir="$root/bin"
package_dir="$root/package"
artifact="dotman-$target.tar.gz"
mkdir -p "$release_dir" "$package_dir"

cat >"$package_dir/dotman" <<'EOF'
#!/bin/sh
echo "dotman 9.9.9"
EOF
chmod 755 "$package_dir/dotman"
tar -C "$package_dir" -czf "$release_dir/$artifact" dotman

if command -v sha256sum >/dev/null 2>&1; then
  sha256sum "$release_dir/$artifact" >"$release_dir/$artifact.sha256"
else
  shasum -a 256 "$release_dir/$artifact" >"$release_dir/$artifact.sha256"
fi

DOTMAN_DOWNLOAD_ROOT="file://$root/releases" \
  DOTMAN_INSTALL_DIR="$install_dir" \
  sh scripts/install.sh >/dev/null

test -x "$install_dir/dotman"
test "$("$install_dir/dotman" --version)" = "dotman 9.9.9"

printf '%064d  %s\n' 0 "$artifact" >"$release_dir/$artifact.sha256"
if DOTMAN_DOWNLOAD_ROOT="file://$root/releases" \
  DOTMAN_INSTALL_DIR="$install_dir" \
  sh scripts/install.sh >/dev/null 2>&1; then
  echo "installer accepted a bad checksum" >&2
  exit 1
fi

test "$("$install_dir/dotman" --version)" = "dotman 9.9.9"

checksum_dir="$root/checksums"
formula="$root/Formula/dotman.rb"
mkdir -p "$checksum_dir"
for formula_target in \
  aarch64-apple-darwin \
  x86_64-apple-darwin \
  aarch64-unknown-linux-gnu \
  x86_64-unknown-linux-gnu; do
  printf '%064d  dotman-%s.tar.gz\n' 1 "$formula_target" \
    >"$checksum_dir/dotman-$formula_target.tar.gz.sha256"
done

DOTMAN_CHECKSUM_DIR="$checksum_dir" \
  sh scripts/update-homebrew-formula.sh v9.9.9 "$formula" >/dev/null
grep -q 'version "9.9.9"' "$formula"
grep -q 'on_macos do' "$formula"
grep -q 'on_linux do' "$formula"
if command -v ruby >/dev/null 2>&1; then
  ruby -c "$formula" >/dev/null
fi

echo "installer script tests passed"
