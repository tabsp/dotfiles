#!/usr/bin/env sh
set -eu

base_url=${DOTFILES_SITE_URL:-"https://dotfiles.tabsp.com"}
dotman_bin=${DOTMAN_BIN:-"$HOME/.local/bin/dotman"}
dotfiles_dir=${DOTFILES_DIR:-"$HOME/.local/share/tabsp-dotfiles"}
yes=0

usage() {
  cat <<EOF
Usage: install.sh [--yes]

Install or update dotman and the tabsp dotfiles bundle.

Options:
  --yes    Run bootstrap/deploy after dry-run succeeds.
  --help   Show this help.
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --yes | -y)
      yes=1
      ;;
    --help | -h)
      usage
      exit 0
      ;;
    *)
      printf 'error: unknown option: %s\n' "$1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

need_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'error: required command not found: %s\n' "$1" >&2
    exit 1
  fi
}

read_tty() {
  if [ ! -r /dev/tty ]; then
    return 0
  fi

  (
    IFS= read -r line </dev/tty && printf '%s' "$line"
  ) 2>/dev/null || true
}

json_string() {
  key=$1
  sed -n "s/.*\"$key\"[[:space:]]*:[[:space:]]*\"\\([^\"]*\\)\".*/\\1/p" "$manifest" | head -n 1
}

download() {
  url=$1
  output=$2
  printf 'download: %s\n' "$url"
  curl -fsSL "$url" -o "$output"
}

sha256_file() {
  file=$1
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | awk '{print $1}'
  else
    shasum -a 256 "$file" | awk '{print $1}'
  fi
}

detect_target() {
  os=$(uname -s)
  arch=$(uname -m)

  case "$os:$arch" in
    Darwin:arm64) printf 'aarch64-apple-darwin' ;;
    Darwin:x86_64) printf 'x86_64-apple-darwin' ;;
    Linux:x86_64) printf 'x86_64-unknown-linux-gnu' ;;
    Linux:aarch64 | Linux:arm64) printf 'aarch64-unknown-linux-gnu' ;;
    *)
      printf 'error: unsupported platform: %s %s\n' "$os" "$arch" >&2
      exit 1
      ;;
  esac
}

need_command curl
need_command tar
need_command mktemp
need_command sed
need_command awk

ensure_brew() {
  if command -v brew >/dev/null 2>&1; then
    return 0
  fi

  os=$(uname -s)

  printf '\nHomebrew is required but not found.\n'
  printf 'Install it with:\n'
  printf '  /bin/bash -c "%s"\n' "\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

  if [ "$yes" -eq 0 ]; then
    printf '\nInstall Homebrew automatically now? [y/N] '
    answer=$(read_tty)
    case "$answer" in
      y | Y | yes | YES) ;;
      *)
        printf 'Skipping Homebrew installation. Bootstrap steps that depend on brew will fail.\n'
        return 0
        ;;
    esac
  fi

  printf 'Installing Homebrew...\n'
  NONINTERACTIVE=1 /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

  if [ "$os" = "Darwin" ]; then
    if [ -x /opt/homebrew/bin/brew ]; then
      eval "$(/opt/homebrew/bin/brew shellenv)"
    elif [ -x /usr/local/bin/brew ]; then
      eval "$(/usr/local/bin/brew shellenv)"
    fi
  elif [ "$os" = "Linux" ]; then
    if [ -x /home/linuxbrew/.linuxbrew/bin/brew ]; then
      eval "$(/home/linuxbrew/.linuxbrew/bin/brew shellenv)"
    elif [ -x "$HOME/.linuxbrew/bin/brew" ]; then
      eval "$("$HOME/.linuxbrew/bin/brew" shellenv)"
    fi
  fi

  if ! command -v brew >/dev/null 2>&1; then
    printf 'error: Homebrew installation completed but brew is still not in PATH\n' >&2
    exit 1
  fi

  printf 'Homebrew installed successfully.\n'
}

ensure_fish() {
  if command -v fish >/dev/null 2>&1; then
    return 0
  fi

  if ! command -v brew >/dev/null 2>&1; then
    printf 'Homebrew is not available; skipping fish installation.\n'
    return 0
  fi

  printf '\nFish shell is required but not found.\n'

  if [ "$yes" -eq 0 ]; then
    printf 'Install fish via Homebrew? [y/N] '
    answer=$(read_tty)
    case "$answer" in
      y | Y | yes | YES) ;;
      *)
        printf 'Skipping fish installation.\n'
        return 0
        ;;
    esac
  fi

  printf 'Installing fish...\n'
  brew install fish

  if ! command -v fish >/dev/null 2>&1; then
    printf 'error: fish installation completed but fish is still not in PATH\n' >&2
    exit 1
  fi

  printf 'Fish installed successfully.\n'
}

ensure_shell_registered() {
  shell_path=$1

  if [ -r /etc/shells ] && grep -Fx "$shell_path" /etc/shells >/dev/null 2>&1; then
    return 0
  fi

  printf '\n%s is not listed in /etc/shells.\n' "$shell_path"

  if [ "$yes" -eq 0 ]; then
    printf 'Add it automatically now? (may require password) [y/N] '
    answer=$(read_tty)
    case "$answer" in
      y | Y | yes | YES) ;;
      *)
        printf 'Skipping shell registration. Run this later:\n'
        printf '  grep -Fx %s /etc/shells || printf "%%s\\n" %s | sudo tee -a /etc/shells\n' "$shell_path" "$shell_path"
        return 1
        ;;
    esac
  fi

  if [ -w /etc/shells ]; then
    if printf '%s\n' "$shell_path" >>/etc/shells; then
      printf 'Added %s to /etc/shells.\n' "$shell_path"
      return 0
    fi
  elif command -v sudo >/dev/null 2>&1; then
    if printf '%s\n' "$shell_path" | sudo tee -a /etc/shells >/dev/null; then
      printf 'Added %s to /etc/shells.\n' "$shell_path"
      return 0
    fi
  fi

  printf 'Could not update /etc/shells. Run this later:\n'
  printf '  grep -Fx %s /etc/shells || printf "%%s\\n" %s | sudo tee -a /etc/shells\n' "$shell_path" "$shell_path"
  return 1
}

ensure_fish_login() {
  if ! command -v fish >/dev/null 2>&1; then
    return 0
  fi

  fish_path=$(command -v fish)

  current_shell=$(getent passwd "$(id -un)" 2>/dev/null | cut -d: -f7)
  if [ -z "$current_shell" ]; then
    current_shell=$(dscl . -read ~/ UserShell 2>/dev/null | awk '{print $NF}' || printf '')
  fi
  if [ -z "$current_shell" ]; then
    current_shell=${SHELL:-}
  fi

  if [ "$current_shell" = "$fish_path" ]; then
    return 0
  fi

  if [ "$yes" -eq 0 ]; then
    printf '\nCurrent default shell is %s, not fish.\n' "${current_shell:-unknown}"
    printf 'Change default shell to fish? (requires password) [y/N] '
    answer=$(read_tty)
    case "$answer" in
      y | Y | yes | YES) ;;
      *)
        printf 'Skipping shell change. Run this later:\n'
        printf '  chsh -s %s\n' "$fish_path"
        return 0
        ;;
    esac
  fi

  if ! ensure_shell_registered "$fish_path"; then
    printf 'Skipping shell change until fish is listed in /etc/shells.\n'
    return 0
  fi

  if chsh -s "$fish_path" 2>/dev/null; then
    printf 'Default shell changed to fish.\n'
    printf '\nRun this to switch your current session:\n'
    printf '  exec fish -l\n'
  else
    printf 'chsh failed (may require password). Run this manually:\n'
    printf '  chsh -s %s\n' "$fish_path"
  fi
}

tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT INT TERM

manifest="$tmp_dir/manifest.json"
download "$base_url/manifest.json" "$manifest"

target=$(detect_target)
bundle_url=$(json_string bundle_url)
bundle_sha256=$(json_string bundle_sha256)
dotman_version=$(json_string dotman_version)
release_base_url=$(json_string dotman_release_base_url)
asset_template=$(json_string dotman_asset_template)

if [ -z "$bundle_url" ] || [ -z "$release_base_url" ] || [ -z "$asset_template" ]; then
  printf 'error: manifest is missing required fields\n' >&2
  exit 1
fi

asset_name=$(printf '%s' "$asset_template" | sed "s/{target}/$target/g")
dotman_url="$release_base_url/$asset_name"

current_version=""
if [ -x "$dotman_bin" ]; then
  current_version=$("$dotman_bin" --version 2>/dev/null | awk '{print $2}' || true)
fi

mkdir -p "$(dirname -- "$dotman_bin")"

if [ -n "$current_version" ] && [ "$current_version" = "$dotman_version" ]; then
  printf 'dotman %s already installed at %s\n' "$current_version" "$dotman_bin"
else
  dotman_archive="$tmp_dir/$asset_name"
  dotman_extract_dir="$tmp_dir/dotman"
  mkdir -p "$dotman_extract_dir"
  download "$dotman_url" "$dotman_archive"
  tar -xzf "$dotman_archive" -C "$dotman_extract_dir"
  if [ ! -f "$dotman_extract_dir/dotman" ]; then
    printf 'error: dotman archive did not contain ./dotman\n' >&2
    exit 1
  fi
  cp "$dotman_extract_dir/dotman" "$dotman_bin"
  chmod 755 "$dotman_bin"
  printf 'installed dotman to %s\n' "$dotman_bin"
fi

bundle_archive="$tmp_dir/dotfiles-bundle.tar.gz"
bundle_next="$dotfiles_dir.next"
bundle_previous="$dotfiles_dir.previous"

download "$bundle_url" "$bundle_archive"
if [ -n "$bundle_sha256" ]; then
  actual_sha256=$(sha256_file "$bundle_archive")
  if [ "$actual_sha256" != "$bundle_sha256" ]; then
    printf 'error: bundle checksum mismatch\n' >&2
    printf 'expected: %s\nactual:   %s\n' "$bundle_sha256" "$actual_sha256" >&2
    exit 1
  fi
fi

rm -rf "$bundle_next"
mkdir -p "$bundle_next"
tar -xzf "$bundle_archive" -C "$bundle_next"

rm -rf "$bundle_previous"
if [ -d "$dotfiles_dir" ]; then
  mv "$dotfiles_dir" "$bundle_previous"
fi
mv "$bundle_next" "$dotfiles_dir"
printf 'installed dotfiles bundle to %s\n' "$dotfiles_dir"

ensure_brew
ensure_fish
ensure_fish_login

printf '\nPreviewing bootstrap and deploy...\n'
(
  cd "$dotfiles_dir"
  "$dotman_bin" bootstrap --dry-run
  "$dotman_bin" deploy --dry-run
)

if [ "$yes" -eq 0 ]; then
  printf '\nDry-run complete. Apply these changes now? [y/N] '
  answer=$(read_tty)
  case "$answer" in
    y | Y | yes | YES) ;;
    *)
      printf 'stopped before applying changes. Run this to apply later:\n'
      printf '  cd %s && %s bootstrap && %s deploy\n' "$dotfiles_dir" "$dotman_bin" "$dotman_bin"
      exit 0
      ;;
  esac
fi

(
  cd "$dotfiles_dir"
  "$dotman_bin" bootstrap
  "$dotman_bin" deploy
)

case ":$PATH:" in
  *":$HOME/.local/bin:"*) ;;
  *)
    printf '\nNote: %s is not in PATH for this shell.\n' "$HOME/.local/bin"
    current_shell_name=${SHELL:-}
    if [ "${current_shell_name##*/}" = "fish" ]; then
      printf 'For fish, add it with:\n'
      printf '  fish_add_path "$HOME/.local/bin"\n'
    else
      printf 'For POSIX shells, add something like:\n'
      printf '  export PATH="$HOME/.local/bin:$PATH"\n'
    fi
    ;;
esac

printf '\nDone. Future runs can use:\n'
printf '  dotman bootstrap\n'
printf '  dotman deploy\n'
