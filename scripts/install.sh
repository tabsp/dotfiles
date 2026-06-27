#!/usr/bin/env sh
set -eu

base_url=${DOTFILES_SITE_URL:-"https://dotfiles.tabsp.com"}
dotman_bin=${DOTMAN_BIN:-"$HOME/.local/bin/dotman"}
dotfiles_dir=${DOTFILES_DIR:-"$HOME/.local/share/tabsp-dotfiles"}
yes=0
tmp_dir=""
stage="starting"

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

# ── Guard: TTY required for interactive mode ──
allow_pipe=${DOTFILES_ALLOW_PIPE:-0}
if [ "$yes" -eq 0 ] && [ "$allow_pipe" -eq 0 ] && ! ([ -t 0 ] && [ -r /dev/tty ]); then
  printf 'error: TTY required for interactive install.\n' >&2
  printf 'Use --yes for unattended mode.\n' >&2
  exit 1
fi

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

# ── gum helpers ──
use_gum=0
if [ "$yes" -eq 0 ] && command -v gum >/dev/null 2>&1; then
  use_gum=1
  export GUM_CONFIRM_PROMPT_FOREGROUND="#89b4fa"
  export GUM_CONFIRM_SELECTED_FOREGROUND=0
  export GUM_CONFIRM_SELECTED_BACKGROUND=2
  export GUM_CONFIRM_UNSELECTED_FOREGROUND=7
  export GUM_CONFIRM_UNSELECTED_BACKGROUND=0
fi

gum_header() {
  if [ "$use_gum" -eq 1 ]; then
    gum style --foreground "#89b4fa" --bold "$@"
  else
    printf '%s\n' "$*"
  fi
}

gum_spin() {
  title=$1; shift
  if [ "$use_gum" -eq 1 ]; then
    gum spin --spinner dot --title "$title" -- sh -c "$*"
  else
    printf '%s...\n' "$title"
    eval "$*"
  fi
}

gum_confirm() {
  if [ "$yes" -eq 1 ]; then
    return 0
  fi
  if [ "$use_gum" -eq 1 ]; then
    gum confirm "$1"
  else
    printf '%s [y/N] ' "$1"
    answer=$(read_tty)
    case "$answer" in y | Y | yes | YES) return 0 ;; *) return 1 ;; esac
  fi
}

gum_warn() {
  if [ "$use_gum" -eq 1 ]; then
    gum style --foreground 3 "$@"
  else
    printf '%s\n' "$*"
  fi
}

gum_card() {
  if [ "$use_gum" -eq 1 ]; then
    gum style --border rounded --border-foreground "#89b4fa" --padding "1 2" "$@"
  else
    printf '%s\n' "$*"
  fi
}

need_command mktemp
need_command awk

gum_header "tabsp dotfiles"
if [ "$use_gum" -eq 1 ]; then
  gum style --foreground "#6c7086" "from $base_url"
  echo
else
  printf 'Installing tabsp dotfiles from %s\n' "$base_url"
  printf '  dotman:   %s\n' "$dotman_bin"
  printf '  dotfiles: %s\n' "$dotfiles_dir"
fi

bundle_next="$dotfiles_dir.next"
bundle_previous="$dotfiles_dir.previous"
source_checkout=0

looks_like_source_checkout() {
  path=$1

  [ -d "$path" ] || return 1
  [ -e "$path/.git" ] || return 1
  [ -f "$path/dotman.yaml" ] || return 1
  [ -f "$path/scripts/install.sh" ] || return 1
}

detect_install_mode() {
  if looks_like_source_checkout "$dotfiles_dir"; then
    source_checkout=1
    printf 'Detected source checkout at %s; skipping published bundle download.\n' "$dotfiles_dir"
    return 0
  fi

  if looks_like_source_checkout "$bundle_previous"; then
    cat >&2 <<EOF
error: refusing to remove source checkout backup: $bundle_previous

Move or rename that directory before running the installer again.
EOF
    exit 1
  fi
}

install_dotman_from_source() {
  if [ -x "$dotman_bin" ]; then
    current_version=$("$dotman_bin" --version 2>/dev/null | awk '{print $2}' || true)
    printf 'using existing dotman%s at %s\n' "${current_version:+ $current_version}" "$dotman_bin"
    return 0
  fi

  if ! command -v cargo >/dev/null 2>&1; then
    cat >&2 <<EOF
error: dotman is not installed at $dotman_bin and cargo is not available

Install Rust/Cargo or set DOTMAN_BIN to an existing dotman binary, then run the
installer again.
EOF
    exit 1
  fi

  stage="building dotman from source"
  mkdir -p "$(dirname -- "$dotman_bin")"
  (
    cd "$dotfiles_dir"
    cargo build --release --locked
  )
  cp "$dotfiles_dir/target/release/dotman" "$dotman_bin"
  chmod 755 "$dotman_bin"
  printf 'installed local dotman to %s\n' "$dotman_bin"
}

detect_install_mode

cleanup() {
  status=$?

  if [ -n "$tmp_dir" ]; then
    rm -rf "$tmp_dir"
  fi

  if [ "$status" -eq 130 ] || [ "$status" -eq 143 ]; then
    rm -rf "$bundle_next"
    if [ ! -d "$dotfiles_dir" ] && [ -d "$bundle_previous" ]; then
      mv "$bundle_previous" "$dotfiles_dir"
    fi
    printf '\nInstall interrupted during: %s\n' "$stage" >&2
    printf 'Cleaned temporary files. Existing dotfiles were left in place when possible.\n' >&2
    if [ -d "$bundle_previous" ]; then
      printf 'Previous bundle backup is available at: %s\n' "$bundle_previous" >&2
    fi
    printf 'Run the installer again to resume.\n' >&2
  fi
}

interrupt() {
  trap - INT TERM
  exit 130
}

trap cleanup EXIT
trap interrupt INT TERM

ensure_brew() {
  if command -v brew >/dev/null 2>&1; then
    return 0
  fi

  os=$(uname -s)

  gum_warn "Homebrew is required but not found."

  if ! gum_confirm "Install Homebrew automatically?"; then
    printf 'Skipping Homebrew installation. Bootstrap steps that depend on brew will fail.\n'
    return 0
  fi

  gum_spin "Installing Homebrew..." \
    'NONINTERACTIVE=1 /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"'

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

  gum_warn "Fish shell is required but not found."

  if ! gum_confirm "Install fish via Homebrew?"; then
    printf 'Skipping fish installation.\n'
    return 0
  fi

  gum_spin "Installing fish..." \
    "brew install fish"

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

  if ! gum_confirm "Add it automatically? (may require password)"; then
    printf 'Skipping shell registration. Run this later:\n'
    printf '  grep -Fx %s /etc/shells || printf "%%s\\n" %s | sudo tee -a /etc/shells\n' "$shell_path" "$shell_path"
    return 1
  fi

  if [ -w /etc/shells ]; then
    if printf '%s\n' "$shell_path" >>/etc/shells; then
      printf 'Added %s to /etc/shells.\n' "$shell_path"
      return 0
    fi
  elif command -v sudo >/dev/null 2>&1; then
    if [ "$yes" -eq 1 ]; then
      sudo_command="sudo -n"
    else
      sudo_command="sudo"
    fi

    if printf '%s\n' "$shell_path" | $sudo_command tee -a /etc/shells >/dev/null; then
      printf 'Added %s to /etc/shells.\n' "$shell_path"
      return 0
    fi
  fi

  printf 'Could not update /etc/shells. Run this later:\n'
  printf '  grep -Fx %s /etc/shells || printf "%%s\\n" %s | sudo tee -a /etc/shells\n' "$shell_path" "$shell_path"
  return 1
}

print_fish_login_commands() {
  shell_path=$1
  user_name=$(id -un)

  printf '     sudo grep -Fx %s /etc/shells || printf "%%s\\n" %s | sudo tee -a /etc/shells\n' "$shell_path" "$shell_path"
  case "$(uname -s)" in
    Darwin)
      printf '     chsh -s %s\n' "$shell_path"
      ;;
    *)
      printf '     sudo chsh -s %s %s\n' "$shell_path" "$user_name"
      ;;
  esac
}

current_login_shell() {
  current_shell=$(getent passwd "$(id -un)" 2>/dev/null | cut -d: -f7)
  if [ -z "$current_shell" ]; then
    current_shell=$(dscl . -read ~/ UserShell 2>/dev/null | awk '{print $NF}' || printf '')
  fi
  if [ -z "$current_shell" ]; then
    current_shell=${SHELL:-}
  fi

  printf '%s' "$current_shell"
}

login_shell_is() {
  [ "$(current_login_shell)" = "$1" ]
}

change_login_shell() {
  shell_path=$1
  user_name=$(id -un)

  if [ "$yes" -eq 1 ]; then
    if chsh -s "$shell_path" </dev/null 2>/dev/null && login_shell_is "$shell_path"; then
      return 0
    fi

    if command -v sudo >/dev/null 2>&1; then
      sudo -n chsh -s "$shell_path" "$user_name" 2>/dev/null || true
      login_shell_is "$shell_path"
      return $?
    fi

    return 1
  fi

  case "$(uname -s)" in
    Darwin)
      chsh -s "$shell_path" || true
      ;;
    *)
      if command -v sudo >/dev/null 2>&1; then
        sudo chsh -s "$shell_path" "$user_name" || true
      else
        chsh -s "$shell_path" || true
      fi
      ;;
  esac

  if login_shell_is "$shell_path"; then
    return 0
  fi

  if [ "$(uname -s)" = "Darwin" ] && command -v sudo >/dev/null 2>&1; then
    sudo chsh -s "$shell_path" "$user_name" || true
    login_shell_is "$shell_path"
    return $?
  fi

  return 1
}

ensure_fish_login() {
  if ! command -v fish >/dev/null 2>&1; then
    return 0
  fi

  fish_path=$(command -v fish)

  current_shell=$(current_login_shell)

  if [ "$current_shell" = "$fish_path" ]; then
    return 0
  fi

  printf '\nCurrent default shell is %s, not fish.\n' "${current_shell:-unknown}"

  if ! gum_confirm "Change default shell to fish? (requires password)"; then
    printf 'Skipping shell change. Fish activation details will be shown at the end.\n'
    return 0
  fi

  if ! ensure_shell_registered "$fish_path"; then
    printf 'Skipping shell change until fish is listed in /etc/shells. Details will be shown at the end.\n'
    return 0
  fi

  if change_login_shell "$fish_path"; then
    printf 'Default shell changed to fish. Activation details will be shown at the end.\n'
  else
    printf 'chsh failed (may require password). Details will be shown at the end.\n'
  fi
}

print_next_step() {
  printf '  %s. %s\n' "$step_no" "$1"
  step_no=$((step_no + 1))
}

print_final_summary() {
  install_status=$1
  apply_status=$2

  printf '\n%s\n' "$install_status"
  printf 'Installed dotman:   %s\n' "$dotman_bin"
  printf 'Installed dotfiles: %s\n' "$dotfiles_dir"

  printf '\nNext steps:\n'
  step_no=1

  dotman_command=$dotman_bin
  case ":$PATH:" in
    *":$HOME/.local/bin:"*)
      dotman_command=dotman
      ;;
    *)
      print_next_step "Add dotman to PATH:"
      current_shell_name=${SHELL:-}
      if [ "${current_shell_name##*/}" = "fish" ]; then
        printf '     fish_add_path "$HOME/.local/bin"\n'
      else
        printf '     export PATH="$HOME/.local/bin:$PATH"\n'
      fi
      ;;
  esac

  if command -v fish >/dev/null 2>&1; then
    fish_path=$(command -v fish)
    current_shell=$(current_login_shell)

    if [ "$current_shell" = "$fish_path" ]; then
      print_next_step "Open a new login session to start fish automatically."
      printf '     Current default shell: %s\n' "$fish_path"
    else
      print_next_step "Configure fish as the default login shell:"
      print_fish_login_commands "$fish_path"
    fi

    print_next_step "Switch this terminal to fish now (optional):"
    printf '     exec %s -l\n' "$fish_path"
  fi

  if [ "$apply_status" = "stopped" ]; then
    print_next_step "Apply bootstrap and deploy:"
    printf '     cd %s\n' "$dotfiles_dir"
    printf '     %s bootstrap\n' "$dotman_command"
    printf '     %s deploy\n' "$dotman_command"
  else
    printf '\nLater, re-apply dotfiles with:\n'
    printf '  %s bootstrap\n' "$dotman_command"
    printf '  %s deploy\n' "$dotman_command"
  fi
}

stage="creating temporary workspace"
tmp_dir=$(mktemp -d)

if [ "$source_checkout" -eq 1 ]; then
  install_dotman_from_source
else
  need_command curl
  need_command tar
  need_command sed

  manifest="$tmp_dir/manifest.json"
  stage="downloading manifest"
  gum_spin "Downloading manifest" \
    "curl -fsSL '$base_url/manifest.json' -o '$manifest'"

  stage="reading manifest"
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
    stage="installing dotman"
    dotman_archive="$tmp_dir/$asset_name"
    dotman_extract_dir="$tmp_dir/dotman"
    mkdir -p "$dotman_extract_dir"
    gum_spin "Installing dotman $dotman_version" \
      "curl -fsSL '$dotman_url' -o '$dotman_archive' &&
       tar -xzf '$dotman_archive' -C '$dotman_extract_dir' &&
       cp '$dotman_extract_dir/dotman' '$dotman_bin' &&
       chmod 755 '$dotman_bin'"
    printf 'installed dotman to %s\n' "$dotman_bin"
  fi

  bundle_archive="$tmp_dir/dotfiles-bundle.tar.gz"

  stage="downloading dotfiles bundle"
  gum_spin "Downloading dotfiles bundle" \
    "curl -fsSL '$bundle_url' -o '$bundle_archive'"
  if [ -n "$bundle_sha256" ]; then
    stage="verifying dotfiles bundle"
    actual_sha256=$(sha256_file "$bundle_archive")
    if [ "$actual_sha256" != "$bundle_sha256" ]; then
      printf 'error: bundle checksum mismatch\n' >&2
      printf 'expected: %s\nactual:   %s\n' "$bundle_sha256" "$actual_sha256" >&2
      exit 1
    fi
  fi

  stage="extracting dotfiles bundle"
  gum_spin "Extracting dotfiles bundle" \
    "rm -rf '$bundle_next' && mkdir -p '$bundle_next' && tar -xzf '$bundle_archive' -C '$bundle_next'"

  stage="installing dotfiles bundle"
  rm -rf "$bundle_previous"
  if [ -d "$dotfiles_dir" ]; then
    mv "$dotfiles_dir" "$bundle_previous"
  fi
  mv "$bundle_next" "$dotfiles_dir"
  printf 'installed dotfiles bundle to %s\n' "$dotfiles_dir"
fi

stage="installing Homebrew"
ensure_brew
stage="installing fish"
ensure_fish
stage="configuring fish login shell"
ensure_fish_login

stage="previewing bootstrap and deploy"
echo
(
  cd "$dotfiles_dir"
  "$dotman_bin" --color always bootstrap --dry-run 2>&1
) >/tmp/dotfiles-bootstrap.log
(
  cd "$dotfiles_dir"
  "$dotman_bin" --color always deploy --dry-run 2>&1
) >/tmp/dotfiles-deploy.log

bootstrap_summary=$(grep -E '[0-9]+ links ok' /tmp/dotfiles-bootstrap.log | tail -1 || true)
deploy_summary=$(grep -E '[0-9]+ links ok' /tmp/dotfiles-deploy.log | tail -1 || true)
bootstrap_warn=$(grep -oE '[0-9]+ warnings' /tmp/dotfiles-bootstrap.log | tail -1 || true)
deploy_warn=$(grep -oE '[0-9]+ warnings' /tmp/dotfiles-deploy.log | tail -1 || true)

if [ "$use_gum" -eq 1 ]; then
  gum style --foreground "#89b4fa" --bold "Preview"
  echo
  gum style --bold --foreground "#cba6f7" "# Bootstrap"
  if [ -n "$bootstrap_summary" ]; then
    gum style --foreground "#6c7086" "  $bootstrap_summary"
  fi
  if [ -n "$bootstrap_warn" ]; then
    gum style --foreground "#f9e2af" "  $bootstrap_warn"
  fi
  echo
  gum style --bold --foreground "#f5c2e7" "# Deploy"
  if [ -n "$deploy_summary" ]; then
    gum style --foreground "#6c7086" "  $deploy_summary"
  fi
  if [ -n "$deploy_warn" ]; then
    gum style --foreground "#f9e2af" "  $deploy_warn"
  fi
  echo
  if gum confirm --default=false "Show full details?"; then
    {
      gum style --foreground "#cba6f7" --bold "# Bootstrap"
      cat /tmp/dotfiles-bootstrap.log
      echo
      gum style --foreground "#6c7086" "────────────────────────────────────────────"
      echo
      gum style --foreground "#f5c2e7" --bold "# Deploy"
      cat /tmp/dotfiles-deploy.log
    } | ${PAGER:-less -r}
  fi
else
  cat /tmp/dotfiles-bootstrap.log
  echo
  cat /tmp/dotfiles-deploy.log
fi
rm -f /tmp/dotfiles-bootstrap.log /tmp/dotfiles-deploy.log
echo

if ! gum_confirm "Apply these changes now?"; then
  if [ "$use_gum" -eq 1 ]; then
    { printf 'Stopped before applying changes.\n\n'; printf 'dotman:   %s\n' "$dotman_bin"
      printf 'dotfiles: %s\n' "$dotfiles_dir"
      if command -v fish >/dev/null 2>&1; then
        printf '\nStart fish: exec %s -l\n' "$(command -v fish)"; fi; } | gum_card
  else
    print_final_summary "Stopped before applying changes." "stopped"
  fi
  exit 0
fi

stage="applying bootstrap and deploy"
(
  cd "$dotfiles_dir"
  "$dotman_bin" bootstrap
  "$dotman_bin" deploy
)

stage="done"
if [ "$use_gum" -eq 1 ]; then
  { printf 'Done.\n\n'; printf 'dotman:   %s\n' "$dotman_bin"
    printf 'dotfiles: %s\n' "$dotfiles_dir"
    if command -v fish >/dev/null 2>&1; then
      printf '\nStart fish now:\n  exec %s -l\n' "$(command -v fish)"; fi; } | gum_card
else
  print_final_summary "Done." "applied"
fi
