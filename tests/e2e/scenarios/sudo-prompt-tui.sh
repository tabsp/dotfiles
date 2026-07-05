#!/bin/bash
# sudo-prompt-tui.sh — Interactive manual test for sudo password flow in TUI.
#
# Tests: ConfirmView → [sudo -v prompt] → RunView → ResultView
# Requires: TTY + sudo that needs a password (no NOPASSWD)

set -euo pipefail

SCENARIO_NAME="sudo-prompt-tui"

DOTMAN="${DOTMAN:-${DOTMAN_BIN:-target/debug/dotman}}"
export FIXTURE_TYPE=sudo

# shellcheck source=tests/e2e/common.sh
source "$(dirname "$0")/../common.sh"
# shellcheck source=tests/e2e/setup.sh
source "$(dirname "$0")/../setup.sh"

echo ""
echo "========================================="
echo " dotman SUDO TUI — Interactive Manual Test"
echo "========================================="

# ---- Prerequisites ----
info "Checking sudo requires password..."
set +e
sudo -n true 2>/dev/null
HAS_NOPASSWD=$?
set -e
if [[ $HAS_NOPASSWD -eq 0 ]]; then
    fail "sudo is passwordless — this test needs real sudo"
    exit 1
fi
pass "sudo requires password (correct)"

# ---- Phase 1: headless plan check ----
info "Init + plan check (headless)..."
INIT_OUT=$("$DOTMAN" init "$FIXTURE_REPO" --profile sudo-test --branch main --path "$CHECKOUT_PATH" 2>&1) || true
assert_contains "$INIT_OUT" "initialized" "init succeeded"

# Verify fixture config was cloned.
info "Verifying fixture config..."
assert_file_contains "$CHECKOUT_PATH/dotman.yaml" "ripgrep"
assert_file_contains "$CHECKOUT_PATH/dotman.yaml" "sudo-deploy-verified"

# Plan check: auto apt update + ripgrep install should be in the plan.
PLAN=$(cd "$CHECKOUT_PATH" && "$DOTMAN" plan 2>&1)
assert_contains "$PLAN" "auto apt update" "plan has auto apt update"
assert_contains "$PLAN" "ripgrep" "plan has ripgrep"

# ---- Phase 2: TUI with sudo ----
echo ""
echo -e "${YELLOW}>>> TUI starting. You will see PlanView.${NC}"
echo -e "${YELLOW}>>> Press 'Enter' to go to ConfirmView, then 'Enter/r' to confirm.${NC}"
echo -e "${YELLOW}>>> TUI exits briefly for sudo password. Type: dotman${NC}"
echo -e "${YELLOW}>>> TUI resumes to RunView. Press 'q' to abort if desired.${NC}"
echo ""
read -rp "Press Enter to start TUI deploy..."

# Run deploy from checkout path. dotman reads `dotman.yaml` from cwd first,
# so we must cd there to pick up the fixture config instead of profile config.
set +e
(cd "$CHECKOUT_PATH" && HOME="$HOME" "$DOTMAN" deploy)
TUI_EXIT=$?
set -e

echo ""
if [[ $TUI_EXIT -eq 0 ]]; then
    pass "TUI deploy exited 0"
else
    fail "TUI deploy exited $TUI_EXIT"
fi

# ---- Phase 3: verify ----
info "Verifying results..."

# Check history via the same home dir used during TUI run.
HISTORY=$(HOME="$HOME" "$DOTMAN" --headless history 2>&1 || true)
echo ""
echo "=== History ==="
echo "$HISTORY"
echo ""

if echo "$HISTORY" | grep -qiF "success"; then
    pass "latest run status is Success"
else
    fail "latest run not Success — check history above"
fi

# rg should now be installed system-wide (apt install).
if command -v rg > /dev/null 2>&1; then
    pass "rg is on PATH"
elif [[ -x /usr/bin/rg ]]; then
    pass "rg found at /usr/bin/rg"
else
    fail "rg not found — apt install may have failed"
fi

# Verify dotman profile history is written to the right place.
RUNS_DIR="$HOME/.local/share/dotman/runs"
info "Run files in $RUNS_DIR:"
ls -la "$RUNS_DIR" 2>/dev/null || echo "  (empty or missing)"

summary
