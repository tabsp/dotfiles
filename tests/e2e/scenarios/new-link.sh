#!/bin/bash
# new-link.sh — Verify new-link command adds link and deploy picks it up.
SCENARIO_NAME="new-link"
FIXTURE_TYPE=runtime
# shellcheck source=tests/e2e/common.sh
source "$(dirname "$0")/../common.sh"
# shellcheck source=tests/e2e/setup.sh
source "$(dirname "$0")/../setup.sh"

info "Running init and first deploy..."
set +e
INIT_OUT=$(dotman init "$FIXTURE_REPO" --profile e2e --branch main --path "$CHECKOUT_PATH" 2>&1)
INIT_EXIT=$?
set -e
assert_eq "$INIT_EXIT" "0" "init exits 0"
assert_contains "$INIT_OUT" "initialized" "init succeeds"

set +e
DEPLOY_OUT=$(dotman deploy 2>&1)
DEPLOY_EXIT=$?
set -e
assert_eq "$DEPLOY_EXIT" "0" "first deploy exits 0"
assert_contains "$DEPLOY_OUT" "deployment-verified" "first deploy marker ran"

info "dotman new-link"
TARGET=\~/.config/test-link
OUTPUT=$(dotman_in_dir "$CHECKOUT_PATH" new-link "$TARGET" "config/fish" 2>&1)
assert_contains "$OUTPUT" "added link" "new-link succeeds"

info "Verify dotman.yaml has the new link"
assert_file_contains "$CHECKOUT_PATH/dotman.yaml" "test-link" "dotman.yaml contains test-link"

info "dotman plan includes new link"
PLAN=$(dotman plan 2>&1)
assert_contains "$PLAN" "test-link" "plan includes test-link"

info "dotman deploy picks up new link"
set +e
DEPLOY=$(dotman deploy 2>&1)
DEPLOY_EXIT=$?
set -e
assert_eq "$DEPLOY_EXIT" "0" "deploy with new link exits 0"
assert_contains "$DEPLOY" "finished" "deploy with new link succeeds"

info "Verify symlink exists"
assert_symlink_exists "$HOME/.config/test-link"

summary
