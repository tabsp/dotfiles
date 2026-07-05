#!/bin/bash
# setup.sh — Creates the fixture git repo and isolated $HOME for e2e tests.
#
# Source this file at the beginning of scenarios that need the fixture:
#   source "$(dirname "$0")/../setup.sh"
#
# Sets these variables:
#   $HOME          -> /tmp/dotman-home (isolated)
#   $FIXTURE_REPO  -> path to bare fixture git repo
#   $CHECKOUT_PATH -> where dotman init will clone to
#   $FIXTURE_DIR   -> path to fixture template files
#
# Scenarios can set FIXTURE_TYPE before sourcing:
#   FIXTURE_TYPE=runtime   dotman.yaml (happy path, no network, no fail scripts)
#   FIXTURE_TYPE=install   dotman-install.yaml (brew bootstrap + font)
#   FIXTURE_TYPE=failure   dotman-failure.yaml (has required/optional fail scripts)
#   FIXTURE_TYPE=sudo      dotman-sudo.yaml (apt installs requiring sudo)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
E2E_DIR="$(dirname "$SCRIPT_DIR")"
FIXTURE_DIR="$E2E_DIR/fixture"

# Each scenario is a separate bash process, so setup always runs fresh.
# Create a clean isolated HOME for every scenario.
export HOME="${DOTMAN_HOME:-/tmp/dotman-home}"
rm -rf "$HOME"
mkdir -p "$HOME/.config/dotman"
mkdir -p "$HOME/.local/share/dotman"
mkdir -p "$HOME/.local/bin"
mkdir -p "$HOME/.config/fish"

FIXTURE_REPO="/tmp/fixture-repo.git"
export CHECKOUT_PATH="$HOME/.local/share/dotman/repos/e2e"
FIXTURE_WORK="/tmp/fixture-work"

info_msg() {
    echo -e "\033[1;33m[SETUP]\033[0m $*"
}

# Choose which dotman.yaml to use based on FIXTURE_TYPE.
case "${FIXTURE_TYPE:-runtime}" in
    install) CONFIG_FILE="dotman-install.yaml" ;;
    failure) CONFIG_FILE="dotman-failure.yaml" ;;
    sudo)    CONFIG_FILE="dotman-sudo.yaml" ;;
    *)       CONFIG_FILE="dotman.yaml" ;;
esac

info_msg "Creating fixture git repo (FIXTURE_TYPE=${FIXTURE_TYPE:-runtime}) at $FIXTURE_REPO"

rm -rf "$FIXTURE_REPO" "$FIXTURE_WORK"
git init --bare --initial-branch=main "$FIXTURE_REPO" > /dev/null 2>&1
git clone "$FIXTURE_REPO" "$FIXTURE_WORK" > /dev/null 2>&1

# Copy the appropriate dotman.yaml as dotman.yaml in the work tree.
cp "$FIXTURE_DIR/$CONFIG_FILE" "$FIXTURE_WORK/dotman.yaml"
cp -r "$FIXTURE_DIR/config" "$FIXTURE_WORK/"
cp -r "$FIXTURE_DIR/bin" "$FIXTURE_WORK/"
cp -r "$FIXTURE_DIR/scripts" "$FIXTURE_WORK/"

chmod +x "$FIXTURE_WORK/bin/hello"
chmod +x "$FIXTURE_WORK/scripts/required-fail.sh"
chmod +x "$FIXTURE_WORK/scripts/optional-fail.sh"

(
    cd "$FIXTURE_WORK"
    git add -A > /dev/null 2>&1
    git -c user.name="e2e-test" -c user.email="e2e@test" commit -m "fixture: initial commit" > /dev/null 2>&1
    git push origin main > /dev/null 2>&1
)

rm -rf "$FIXTURE_WORK"
info_msg "Fixture repo ready: $FIXTURE_REPO (config: $CONFIG_FILE)"
info_msg "Isolated HOME: $HOME"
