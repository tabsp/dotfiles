#!/bin/bash
# deploy-runtime.sh — Full init → plan → deploy → config verification.
# Uses the happy-path fixture (no network, no fail scripts).
SCENARIO_NAME="deploy-runtime"
FIXTURE_TYPE=runtime
# shellcheck source=tests/e2e/common.sh
source "$(dirname "$0")/../common.sh"
# shellcheck source=tests/e2e/setup.sh
source "$(dirname "$0")/../setup.sh"

info "dotman init — headless"
OUTPUT=$(dotman init "$FIXTURE_REPO" --profile e2e --branch main --path "$CHECKOUT_PATH" 2>&1)
assert_contains "$OUTPUT" "initialized" "dotman init succeeds"

info "Verify profile config"
assert_file_exists "$HOME/.config/dotman/config.toml"
assert_file_contains "$HOME/.config/dotman/config.toml" "e2e" "profile config has e2e profile"

info "dotman profile list"
OUTPUT=$(dotman profile list 2>&1)
assert_contains "$OUTPUT" "e2e" "profile list shows e2e profile"

info "dotman status"
OUTPUT=$(dotman status 2>&1)
assert_contains "$OUTPUT" "checkout" "status shows checkout info"

info "dotman doctor"
OUTPUT=$(dotman doctor 2>&1)
assert_contains "$OUTPUT" "git" "doctor checks git"

info "dotman plan --headless"
PLAN=$(dotman plan 2>&1)
# serde_json pretty output contains these action kind discriminants.
assert_contains "$PLAN" '"link"' "plan contains link action"
assert_contains "$PLAN" '"create"' "plan contains create action"
assert_contains "$PLAN" '"shell"' "plan contains shell action"
# Verify the plan is valid JSON (not an error message).
echo "$PLAN" | python3 -c "import sys,json; json.load(sys.stdin)" 2>/dev/null
assert_eq "$?" "0" "plan output is valid JSON"

info "dotman deploy --headless"
set +e
DEPLOY_OUTPUT=$(dotman deploy 2>&1)
DEPLOY_EXIT=$?
set -e
assert_eq "$DEPLOY_EXIT" "0" "deploy exits 0"
assert_contains "$DEPLOY_OUTPUT" "deployment-verified" "deploy runs verification shell"

info "Verify link results"
assert_symlink_exists "$HOME/.config/fish"
assert_symlink_exists "$HOME/.config/nvim"
assert_symlink_exists "$HOME/.tmux.conf"
assert_symlink_exists "$HOME/.local/bin/hello"

info "Verify create results"
assert_dir_exists "$HOME/.config/fish/local.d"

info "Verify fish config is parseable"
assert_exit_code 0 fish -n "$HOME/.config/fish/config.fish"

info "Verify nvim config can headless load"
assert_exit_code 0 nvim --headless -u "$HOME/.config/nvim/init.lua" +qa

info "Verify tmux config is parseable"
assert_exit_code 0 tmux -f "$HOME/.tmux.conf" start-server \; kill-server

info "Verify bin script is linked and executable"
assert_exit_code 0 "$HOME/.local/bin/hello"

info "Verify idempotent deploy"
set +e
DEPLOY2=$(dotman deploy 2>&1)
DEPLOY2_EXIT=$?
set -e
assert_eq "$DEPLOY2_EXIT" "0" "second deploy exits 0"
assert_contains "$DEPLOY2" "finished" "second deploy succeeds (idempotent)"

info "dotman history --headless"
HISTORY=$(dotman history 2>&1)
assert_contains "$HISTORY" "deploy" "history shows deploy runs"

summary
