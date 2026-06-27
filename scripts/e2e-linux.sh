#!/usr/bin/env bash
set -euo pipefail

script_dir=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_dir=$(CDPATH= cd -- "$script_dir/.." && pwd)

mode=local
keep=0
inspect=0
interactive_install=0
verbose=0

usage() {
  cat <<'EOF'
Usage: scripts/e2e-linux.sh [--local|--production] [--interactive-install] [--keep] [--inspect] [--verbose]

Run a real Linux install test inside Docker.

Modes:
  --local       Build dotman and the dotfiles bundle from the current worktree.
  --production  Install from DOTFILES_SITE_URL, defaulting to production.

Options:
  --interactive-install  Exercise install.sh prompts through a pseudo-terminal.
  --keep        Keep the temporary Docker work directory after the run.
  --inspect     Open an interactive tester shell after E2E completes.
  --verbose     Enable shell tracing inside this wrapper and the container.
  --help        Show this help.

Environment:
  E2E_DOCKER_IMAGE   Docker base image. Default: ubuntu:24.04
  E2E_PLATFORM       Optional Docker platform, for example linux/amd64.
  E2E_PORT           Local HTTP port inside the container. Default: 8765
  DOTFILES_SITE_URL  Production site URL when --production is used.
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --local)
      mode=local
      ;;
    --production)
      mode=production
      ;;
    --interactive-install)
      interactive_install=1
      ;;
    --keep)
      keep=1
      ;;
    --inspect)
      inspect=1
      ;;
    --verbose)
      verbose=1
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

if [ "$verbose" -eq 1 ]; then
  set -x
fi

if ! command -v docker >/dev/null 2>&1; then
  printf 'error: docker is required for Linux E2E\n' >&2
  exit 1
fi

image=${E2E_DOCKER_IMAGE:-ubuntu:24.04}
port=${E2E_PORT:-8765}
production_site_url=${DOTFILES_SITE_URL:-https://dotfiles.tabsp.com}
tmp_dir=$(mktemp -d "${TMPDIR:-/tmp}/dotman-linux-e2e.XXXXXX")
container_name=""

print_manual_inspect_command() {
  printf '  docker exec -it %s sudo -H -u tester env '"'"'PATH=/home/tester/.local/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:$PATH'"'"' bash -lc '"'"'cd "$HOME/.local/share/tabsp-dotfiles" && exec /home/linuxbrew/.linuxbrew/bin/fish -l'"'"'\n' "$container_name"
}

cleanup() {
  if [ -n "$container_name" ]; then
    if [ "$keep" -eq 1 ]; then
      printf 'kept E2E container: %s\n' "$container_name"
      printf 'inspect again with:\n'
      print_manual_inspect_command
    else
      docker rm -f "$container_name" >/dev/null 2>&1 || true
    fi
  fi

  if [ "$keep" -eq 1 ]; then
    printf 'kept E2E work directory: %s\n' "$tmp_dir"
  else
    rm -rf "$tmp_dir"
  fi
}
trap cleanup EXIT

docker_args=()
if [ -n "${E2E_PLATFORM:-}" ]; then
  docker_args+=(--platform "$E2E_PLATFORM")
fi

docker_args+=(
  -e "E2E_MODE=$mode"
  -e "E2E_PORT=$port"
  -e "E2E_INTERACTIVE_INSTALL=$interactive_install"
  -e "E2E_VERBOSE=$verbose"
  -e "DOTFILES_SITE_URL=$production_site_url"
  -v "$tmp_dir:/work"
)

if [ "$mode" = "local" ]; then
  docker_args+=(-v "$repo_dir:/repo:ro")
fi

printf 'running %s Linux E2E in %s\n' "$mode" "$image"

container_script="$tmp_dir/container-e2e.sh"
cat >"$container_script" <<'CONTAINER'
set -euo pipefail

if [ "${E2E_VERBOSE:-0}" = "1" ]; then
  set -x
fi

export DEBIAN_FRONTEND=noninteractive

apt-get update
apt-get install -y --no-install-recommends \
  bash \
  build-essential \
  ca-certificates \
  curl \
  file \
  fontconfig \
  git \
  procps \
  python3 \
  sudo \
  tar \
  unzip \
  xz-utils
if [ "${E2E_INTERACTIVE_INSTALL:-0}" = "1" ]; then
  apt-get install -y --no-install-recommends expect
fi
rm -rf /var/lib/apt/lists/*

if ! id tester >/dev/null 2>&1; then
  useradd -m -s /bin/bash tester
fi
printf 'tester ALL=(ALL) NOPASSWD:ALL\n' >/etc/sudoers.d/tester
chmod 0440 /etc/sudoers.d/tester
chown -R tester:tester /work

detect_target() {
  os=$(uname -s)
  arch=$(uname -m)

  case "$os:$arch" in
    Linux:x86_64) printf 'x86_64-unknown-linux-gnu' ;;
    Linux:aarch64 | Linux:arm64) printf 'aarch64-unknown-linux-gnu' ;;
    *)
      printf 'error: unsupported E2E platform: %s %s\n' "$os" "$arch" >&2
      exit 1
      ;;
  esac
}

run_as_tester() {
  sudo -H -u tester env \
    HOME=/home/tester \
    USER=tester \
    PATH="/home/tester/.cargo/bin:/home/tester/.local/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin" \
    bash -lc "$1"
}

if [ "${E2E_MODE:-local}" = "local" ]; then
  target=$(detect_target)
  site_dir=/work/site
  mkdir -p "$site_dir/bundle" "$site_dir/release"
  cp /repo/scripts/install.sh "$site_dir/install.sh"
  chmod 755 "$site_dir/install.sh"

  run_as_tester 'curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable'
  run_as_tester 'cd /repo && cargo build --release --locked --target-dir /work/target'

  tar -czf "$site_dir/release/dotman-$target.tar.gz" -C /work/target/release dotman

  (
    cd /repo
    tar -czf "$site_dir/bundle/latest.tar.gz" \
      dotman.yaml \
      dotman.bootstrap.yaml \
      config \
      bin \
      packages \
      README.md \
      README.zh-CN.md \
      docs/new-machine.md
  )

  bundle_sha256=$(sha256sum "$site_dir/bundle/latest.tar.gz" | awk '{print $1}')
  package_version=$(sed -n 's/^version = "\(.*\)"/\1/p' /repo/Cargo.toml | head -n 1)
  generated_at=$(date -u '+%Y-%m-%dT%H:%M:%SZ')
  base_url="http://127.0.0.1:${E2E_PORT}"

  cat >"$site_dir/manifest.json" <<EOF
{
  "schema": 1,
  "generated_at": "$generated_at",
  "bundle_url": "$base_url/bundle/latest.tar.gz",
  "bundle_sha256": "$bundle_sha256",
  "dotman_version": "$package_version",
  "dotman_release_base_url": "$base_url/release",
  "dotman_asset_template": "dotman-{target}.tar.gz",
  "bundle": {
    "version": "local-worktree",
    "url": "$base_url/bundle/latest.tar.gz",
    "sha256": "$bundle_sha256"
  },
  "dotman": {
    "version": "$package_version",
    "release_base_url": "$base_url/release",
    "asset_template": "dotman-{target}.tar.gz"
  }
}
EOF

  python3 -m http.server "$E2E_PORT" --bind 127.0.0.1 --directory "$site_dir" >/work/http.log 2>&1 &
  http_pid=$!
  trap 'kill "$http_pid" 2>/dev/null || true' EXIT

  for _ in $(seq 1 50); do
    if curl -fsSL "$base_url/manifest.json" >/dev/null 2>&1; then
      break
    fi
    sleep 0.2
  done

  curl -fsSL "$base_url/manifest.json" >/dev/null
  install_url="$base_url/install.sh"
else
  install_url="${DOTFILES_SITE_URL%/}/install.sh"
fi

if [ "${E2E_INTERACTIVE_INSTALL:-0}" = "1" ]; then
  cat >/work/install-interactive.expect <<EOF
set timeout -1
spawn sudo -H -u tester env HOME=/home/tester USER=tester PATH=/home/tester/.cargo/bin:/home/tester/.local/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin DOTFILES_SITE_URL=${install_url%/install.sh} bash -lc {set -o pipefail; curl -fsSL '$install_url' | sh}
expect {
  -re {Install Homebrew automatically now\\? \\[y/N\\]} { send "y\r"; exp_continue }
  -re {Install fish via Homebrew\\? \\[y/N\\]} { send "y\r"; exp_continue }
  -re {Add it automatically now\\? .*\\[y/N\\]} { send "y\r"; exp_continue }
  -re {Change default shell to fish\\? .*\\[y/N\\]} { send "y\r"; exp_continue }
  -re {Dry-run complete\\. Apply these changes now\\? \\[y/N\\]} { send "y\r"; exp_continue }
  eof
}
catch wait result
exit [lindex \$result 3]
EOF
  expect /work/install-interactive.expect
else
  run_as_tester "set -o pipefail; curl -fsSL '$install_url' | DOTFILES_SITE_URL='${install_url%/install.sh}' sh -s -- --yes"
fi

run_as_tester '
  set -euo pipefail
  export PATH="$HOME/.local/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:$PATH"
  verify_log=/work/verify.log

  test -x "$HOME/.local/bin/dotman"
  dotman --version >"$verify_log"
  test -d "$HOME/.local/share/tabsp-dotfiles"
  test -f "$HOME/.local/share/tabsp-dotfiles/dotman.yaml"
  test -d "$HOME/.local/share/tabsp-dotfiles/config"
  command -v brew >/dev/null
  command -v fish >/dev/null
  fish_path=$(command -v fish)
  test "$(getent passwd tester | cut -d: -f7)" = "$fish_path"

  cd "$HOME/.local/share/tabsp-dotfiles"
  if ! {
    dotman bootstrap --dry-run
    dotman deploy --dry-run
  } >>"$verify_log" 2>&1; then
    cat "$verify_log"
    exit 1
  fi

  test -e "$HOME/.config/fish"
  test -e "$HOME/.config/nvim"
  test -e "$HOME/.tmux.conf"
'

printf 'Linux E2E completed successfully.\n'
CONTAINER
chmod 755 "$container_script"

if [ "$inspect" -eq 1 ]; then
  container_name="dotman-linux-e2e-$(date +%s)-$$"
  printf 'starting inspectable container: %s\n' "$container_name"
  docker run -d --name "$container_name" "${docker_args[@]}" "$image" sleep infinity >/dev/null
  set +e
  docker exec -i "$container_name" bash /work/container-e2e.sh
  e2e_status=$?
  set -e
else
  docker run --rm -i "${docker_args[@]}" "$image" bash /work/container-e2e.sh
  e2e_status=0
fi

if [ "$inspect" -eq 1 ]; then
  if [ "$e2e_status" -ne 0 ]; then
    printf '\nE2E failed with status %s. Opening the container for inspection.\n' "$e2e_status"
  fi
  if [ -t 0 ] && [ -t 1 ]; then
    printf '\nEntering E2E container as tester. Exit the shell to finish.\n'
    docker exec -it "$container_name" sudo -H -u tester env \
      HOME=/home/tester \
      USER=tester \
      PATH="/home/tester/.local/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin" \
      bash -lc 'cd "$HOME/.local/share/tabsp-dotfiles" && if [ -x /home/linuxbrew/.linuxbrew/bin/fish ]; then exec /home/linuxbrew/.linuxbrew/bin/fish -l; else exec bash -l; fi'
  else
    printf '\nNo interactive TTY detected. Inspect manually with:\n'
    print_manual_inspect_command
  fi
fi

exit "$e2e_status"
