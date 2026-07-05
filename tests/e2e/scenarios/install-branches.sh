#!/bin/bash
# install-branches.sh — Test special install code paths.
# Requires network access (brew bootstrap, font download).
# Skip with SKIP_NETWORK_TESTS=1.
SCENARIO_NAME="install-branches"
FIXTURE_TYPE=install
# shellcheck source=tests/e2e/common.sh
source "$(dirname "$0")/../common.sh"
# shellcheck source=tests/e2e/setup.sh
source "$(dirname "$0")/../setup.sh"

if [[ "${SKIP_NETWORK_TESTS:-0}" == "1" ]]; then
    info "SKIP_NETWORK_TESTS=1 — skipping"
    exit 0
fi

info "Checking network..."
if ! curl -s --connect-timeout 5 https://github.com > /dev/null 2>&1; then
    info "No network — skipping install-branches tests"
    exit 0
fi

info "Running init against install fixture..."
set +e
INIT_OUT=$(dotman init "$FIXTURE_REPO" --profile e2e --branch main --path "$CHECKOUT_PATH" 2>&1)
INIT_EXIT=$?
set -e
assert_eq "$INIT_EXIT" "0" "init exits 0"
assert_contains "$INIT_OUT" "initialized" "init succeeds"

info "Verify plan has auto-install package manager action"
PLAN=$(dotman plan 2>&1)
assert_contains "$PLAN" "auto-install package manager" "plan has auto-install pkg manager"
assert_contains "$PLAN" "command -v brew" "plan has brew guard condition"

info "Verify plan has font install action"
assert_contains "$PLAN" "font-maple-mono-nf-cn" "plan has font install"

info "Running deploy (brew bootstrap + font install)..."
set +e
DEPLOY_OUTPUT=$(dotman deploy 2>&1)
DEPLOY_EXIT=$?
set -e
assert_eq "$DEPLOY_EXIT" "0" "first deploy (bootstrap + font) exits 0"
assert_contains "$DEPLOY_OUTPUT" "install-deploy-verified" "deploy marker shell ran"

info "Verify font installed"
FONT_FILE="$HOME/.local/share/fonts/MapleMono-NF-CN-Regular.ttf"
assert_file_exists "$FONT_FILE"

info "Verify brew is available after bootstrap"
BREW_PREFIX=/home/linuxbrew/.linuxbrew
if command -v brew > /dev/null 2>&1; then
    pass "brew is available on PATH after bootstrap"
elif [[ -x "$BREW_PREFIX/bin/brew" ]]; then
    pass "brew installed at $BREW_PREFIX/bin/brew (expected — not on PATH in subprocess)"
    export PATH="$BREW_PREFIX/bin:$PATH"
else
    fail "brew was not installed by bootstrap step"
fi

# ---- Phase 2: real brew package smoke test via dotman ----
# Export brew into PATH so dotman spawns can find it, then deploy a second
# fixture with only lazygit (auto_install_pkg_manager: false) to prove dotman's
# install dispatch works after bootstrap.
info "Second deploy: real brew package smoke via dotman..."
cat > "$CHECKOUT_PATH/dotman.yaml" << 'ENDYAML'
package_managers:
  ubuntu: brew
auto_install_pkg_manager: false

install:
  - lazygit

links: []
create: []
shell:
  - command: echo "smoke-deploy-verified"
    description: Smoke deploy marker
    optional: false
ENDYAML

set +e
# Run deploy from $HOME so profile config is used, not repo root dotman.yaml.
# Pass brew PATH so child processes can find the binary.
SMOKE_OUT=$(cd "$HOME" && HOME="$HOME" PATH="$BREW_PREFIX/bin:$PATH" "$DOTMAN" --headless deploy 2>&1)
SMOKE_EXIT=$?
set -e
assert_eq "$SMOKE_EXIT" "0" "smoke deploy (lazygit via dotman) exits 0"
assert_contains "$SMOKE_OUT" "smoke-deploy-verified" "smoke deploy marker shell ran"

info "Verify lazygit is executable..."
assert_exit_code 0 lazygit --version

summary
