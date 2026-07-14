#!/bin/bash
# production-manual.sh — Full production E2E with REAL dotman.yaml, config/, bin/.
#
# This is a MANUAL test entrypoint — run inside the Docker container:
#   docker run --rm -it dotman-e2e-production tests/e2e/scenarios/production-manual.sh
#
# What it does:
#   1. Creates a fixture git repo from /app (real production config, no test fixture).
#   2. Runs: init → plan → (manual) TUI deploy → history → verification.
#   3. Verifies run records exist and key links/dirs are present.
#
# Non-goals:
#   - NOT for CI.
#   - Does NOT require all production tools to install successfully in Docker/Linux.
#   - User may cancel unsuitable items in TUI.

set -euo pipefail

SCENARIO_NAME="production-manual"

DOTMAN="${DOTMAN:-${DOTMAN_BIN:-target/release/dotman}}"

# shellcheck source=tests/e2e/common.sh
source "$(dirname "$0")/../common.sh"

# ---- Isolation ----

HOME=/tmp/dotman-home
export HOME

rm -rf "$HOME"
mkdir -p "$HOME/.config/dotman"
mkdir -p "$HOME/.local/share/dotman"
mkdir -p "$HOME/.local/bin"

FIXTURE_REPO=/tmp/production-fixture-repo.git
FIXTURE_WORK=/tmp/production-fixture-work
CHECKOUT_PATH="$HOME/.local/share/dotman/repos/production"

echo ""
echo "========================================="
echo " dotman PRODUCTION E2E — Manual Test"
echo "========================================="
echo " HOME:           $HOME"
echo " FIXTURE_REPO:   $FIXTURE_REPO"
echo " CHECKOUT_PATH:  $CHECKOUT_PATH"
echo " DOTMAN:         $DOTMAN"
echo "========================================="
echo ""

# ---- Step 1: Build production fixture repo from /app ----

info "Step 1: Building production fixture repo from /app..."

rm -rf "$FIXTURE_REPO" "$FIXTURE_WORK"

# 1a. Create bare repo and clone into empty work tree first.
git init --bare --initial-branch=main "$FIXTURE_REPO" > /dev/null 2>&1
git clone "$FIXTURE_REPO" "$FIXTURE_WORK" > /dev/null 2>&1

# 1b. Copy production config from /app into the clone, excluding build artifacts.
rsync -a /app/ "$FIXTURE_WORK/" \
    --exclude .git \
    --exclude target \
    --exclude tests \
    --exclude docs \
    --exclude nvim.log \
    --exclude .claude \
    --exclude config/fish/fish_variables \
    --exclude config/fish/completions \
    --exclude config/fish/conf.d \
    --exclude config/fish/functions \
    --exclude config/fish/local.d \
    --exclude config/fish/themes

# 1c. Verify essential production files are present.
info "Verifying fixture content..."
assert_file_exists "$FIXTURE_WORK/dotman.yaml"
assert_dir_exists "$FIXTURE_WORK/config"
assert_dir_exists "$FIXTURE_WORK/bin"

# 1d. Show what we're working with.
info "Production config includes:"
find "$FIXTURE_WORK/config" -mindepth 1 -maxdepth 1 -type d -printf '  config/%f/\n' 2>/dev/null || true
find "$FIXTURE_WORK/bin" -mindepth 1 -maxdepth 1 -type f -printf '  bin/%f\n' 2>/dev/null || true

# 1e. Commit and push.
(
    cd "$FIXTURE_WORK"
    git add -A > /dev/null 2>&1
    git -c user.name="production-e2e" -c user.email="e2e@production" \
        commit -m "production fixture from real dotman.yaml" > /dev/null 2>&1
    git push origin main > /dev/null 2>&1
)

pass "Production fixture repo created at $FIXTURE_REPO"

# ---- Step 2: Init ----

echo ""
info "Step 2: Running dotman init..."

INIT_OUT=$("$DOTMAN" init "$FIXTURE_REPO" \
    --profile production-e2e \
    --branch main \
    --path "$CHECKOUT_PATH" 2>&1) || true

echo "$INIT_OUT"
assert_contains "$INIT_OUT" "initialized" "dotman init succeeded"

# Verify init results.
info "Verifying init results..."
assert_file_exists "$HOME/.config/dotman/config.toml"
assert_file_exists "$CHECKOUT_PATH/dotman.yaml"

# Verify real config directories are in the checkout.
assert_dir_exists "$CHECKOUT_PATH/config/fish"
assert_dir_exists "$CHECKOUT_PATH/config/nvim"
assert_file_exists "$CHECKOUT_PATH/bin/tmux-status"

pass "Init verification complete"

# ---- Step 3: Plan ----

echo ""
info "Step 3: Running dotman plan..."

PLAN=$(cd "$CHECKOUT_PATH" && "$DOTMAN" --headless plan 2>&1) || true
echo "$PLAN"

# Verify plan contains expected production items.
info "Verifying plan content..."
assert_contains "$PLAN" "fish" "plan includes fish"
assert_contains "$PLAN" "tmux" "plan includes tmux"
assert_contains "$PLAN" "neovim" "plan includes neovim"
assert_contains "$PLAN" "ripgrep" "plan includes ripgrep"
assert_contains "$PLAN" "Sync fish plugins" "plan includes Sync fish plugins"
assert_contains "$PLAN" "Update tealdeer pages" "plan includes Update tealdeer pages"

# Plan should be valid JSON.
if echo "$PLAN" | python3 -c "import sys,json; json.load(sys.stdin)" 2>/dev/null; then
    pass "plan output is valid JSON"
else
    fail "plan output is not valid JSON"
fi

pass "Plan verification complete"

# ---- Step 4: Deploy ----

if [[ "${PRODUCTION_E2E_SMOKE:-0}" == "1" ]]; then
    echo ""
    info "Step 4: Running non-interactive smoke deploy..."
    info "Real production init/plan already passed; smoke deploy uses cheap link/create actions only."

    cat > "$CHECKOUT_PATH/dotman.yaml" <<'YAML'
package_managers: {}
auto_install_pkg_manager: false
install: []
links:
  ~/.config/fish: config/fish
create:
  - ~/.config/fish-local
shell: []
YAML

    SMOKE_OUT=$(cd "$CHECKOUT_PATH" && HOME="$HOME" "$DOTMAN" --headless deploy 2>&1) || true
    echo "$SMOKE_OUT"
    assert_contains "$SMOKE_OUT" "finished" "smoke deploy produced a run"
else
    # ---- Step 4: TUI Deploy (MANUAL) ----

    echo ""
    echo "========================================="
    echo -e "${YELLOW}>>> MANUAL TUI DEPLOY <<<${NC}"
    echo "========================================="
    echo ""
    echo -e "You are about to enter the TUI deploy interface."
    echo -e "The plan is generated from your REAL production dotman.yaml."
    echo ""
    echo -e "${YELLOW}Instructions:${NC}"
    echo -e "  1. Review the production plan in the TUI."
    echo -e "  2. ${YELLOW}Cancel items unsuitable for this Docker run${NC}"
    echo -e "     (e.g. GUI apps/fonts if you only want a fast smoke test)."
    echo -e "  3. Press 'r' or Enter to confirm and run."
    echo -e "  4. When sudo/brew prompts for password, ${YELLOW}type: dotman${NC}"
    echo -e "  5. After results, press 'q' to return and exit."
    echo ""
    echo -e "${YELLOW}Known caveats:${NC}"
    echo -e "  - ghostty: supported on Linux, but installing GUI dependencies can take time; cancel it for smoke tests"
    echo -e "  - font-maple-mono-nf-cn: font install may fail in Docker → cancel it"
    echo -e "  - Linuxbrew bootstrap: needs network, takes time"
    echo -e "  - fisher update / tldr --update: optional, may fail"
    echo ""
    read -rp "Press Enter to start TUI deploy..."

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
fi

# ---- Step 5: Post-deploy Verification ----

echo ""
info "Step 5: Verifying deploy results..."

# --- Mandatory checks ---

# 5a. History is not empty.
HISTORY=$(HOME="$HOME" "$DOTMAN" --headless history 2>&1) || true
echo ""
echo "=== dotman history ==="
echo "$HISTORY"
echo ""

if [[ -n "$HISTORY" ]] && echo "$HISTORY" | grep -qiE 'success|failed|deploy'; then
    pass "history is non-empty and contains run record"
else
    fail "history is empty or missing run record"
fi

# 5b. Run JSON files exist (exclude latest.json which is a symlink).
RUNS_DIR="$HOME/.local/share/dotman/runs"
RUN_COUNT=$(find "$RUNS_DIR" -maxdepth 1 -type f -name '*.json' ! -name latest.json 2>/dev/null | wc -l)
if [[ -d "$RUNS_DIR" ]] && [[ "$RUN_COUNT" -gt 0 ]]; then
    pass "run JSON files exist in $RUNS_DIR ($RUN_COUNT files)"
    echo ""
    info "Run files:"
    ls -la "$RUNS_DIR"/
else
    fail "no run JSON files found in $RUNS_DIR"
fi

# 5c. latest.json exists.
LATEST="$RUNS_DIR/latest.json"
assert_file_exists "$LATEST"

# 5d. Verify run items exist in latest run.
#     Run schema: { items: [{ name, status, output, ... }] }
if [[ -f "$LATEST" ]]; then
    if python3 -c "
import json, sys
with open('$LATEST') as f:
    data = json.load(f)
items = data.get('items', [])
if not items:
    print('NO_ITEMS')
    sys.exit(1)
print(f'Found {len(items)} items')
" 2>/dev/null; then
        pass "run items exist in latest.json"
    else
        fail "no run items in latest.json — TUI may not have executed anything"
    fi
else
    fail "latest.json missing — cannot verify run results"
fi

# 5e. Verify at least one symlink or create directory exists on filesystem.
#     This is a concrete check that deploy actually did something.
LINK_OR_CREATE_COUNT=0
for p in \
    "$HOME/.config/fish" \
    "$HOME/.config/nvim" \
    "$HOME/.tmux.conf" \
    "$HOME/.local/bin/tmux-status"; do
    if [[ -L "$p" ]] || [[ -f "$p" ]]; then
        LINK_OR_CREATE_COUNT=$((LINK_OR_CREATE_COUNT + 1))
    fi
done
if [[ -d "$HOME/.config/fish/local.d" ]]; then
    LINK_OR_CREATE_COUNT=$((LINK_OR_CREATE_COUNT + 1))
fi

if [[ "$LINK_OR_CREATE_COUNT" -gt 0 ]]; then
    pass "at least one link/create target exists on filesystem ($LINK_OR_CREATE_COUNT found)"
else
    fail "no link/create targets found on filesystem — deploy may have done nothing"
fi

# --- Optional / informational checks ---

echo ""
info "Optional checks (informational only):"

# Config symlinks.
for path_label in \
    "$HOME/.config/fish" \
    "$HOME/.config/nvim" \
    "$HOME/.tmux.conf" \
    "$HOME/.local/bin/tmux-status"; do
    label="${path_label#"$HOME"}"
    if [[ -L "$path_label" ]]; then
        pass "$label symlink exists"
    else
        info "$label symlink not found (may have been cancelled)"
    fi
done

# Create dirs.
if [[ -d "$HOME/.config/fish/local.d" ]]; then
    pass ".config/fish/local.d directory exists"
else
    info ".config/fish/local.d not found"
fi

# Tools on PATH (some may have been cancelled or failed — purely informational).
for tool in fish tmux rg nvim fd bat eza; do
    if command -v "$tool" > /dev/null 2>&1; then
        pass "$tool is on PATH"
    else
        info "$tool not on PATH (cancelled, failed, or not applicable)"
    fi
done

# ---- Summary ----

echo ""
echo "========================================="
echo " Production E2E Summary"
echo "========================================="
echo " HOME:           $HOME"
echo " CHECKOUT_PATH:  $CHECKOUT_PATH"
echo " RUNS_DIR:       $RUNS_DIR"
echo "========================================="

summary
