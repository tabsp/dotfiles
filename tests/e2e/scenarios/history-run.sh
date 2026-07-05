#!/bin/bash
# history-run.sh — Verify history listing and run detail display.
SCENARIO_NAME="history-run"
FIXTURE_TYPE=runtime
# shellcheck source=tests/e2e/common.sh
source "$(dirname "$0")/../common.sh"
# shellcheck source=tests/e2e/setup.sh
source "$(dirname "$0")/../setup.sh"

info "Running init and deploy..."
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

info "dotman history --headless"
HISTORY=$(dotman history 2>&1)
assert_contains "$HISTORY" "deploy" "history shows deploy mode"
assert_not_contains "$HISTORY" "no runs yet" "history is not empty"

# Extract a run ID (ULID: 26 uppercase alphanumeric chars).
RUN_ID=$(echo "$HISTORY" | grep -oE '[0-9A-HJKMNP-TV-Z]{26}' | head -1)
if [[ -z "$RUN_ID" ]]; then
    RUN_ID=$(echo "$HISTORY" | head -2 | tail -1 | awk '{print $1}')
fi
info "Extracted run ID: $RUN_ID"
if [[ -n "$RUN_ID" ]]; then
    pass "a run ID was extracted from history"
else
    fail "no run ID found in history output"
fi

if [[ -n "$RUN_ID" ]]; then
    info "dotman run $RUN_ID --headless"
    set +e
    RUN_OUTPUT=$(dotman run "$RUN_ID" 2>&1)
    RUN_EXIT=$?
    set -e
    assert_eq "$RUN_EXIT" "0" "run replay exits 0"
    assert_contains "$RUN_OUTPUT" "Run:" "run output shows run id"
    assert_contains "$RUN_OUTPUT" "items:" "run output shows item count"
    assert_contains "$RUN_OUTPUT" "status:" "run output shows status"
fi

summary
