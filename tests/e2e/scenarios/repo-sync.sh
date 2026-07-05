#!/bin/bash
# repo-sync.sh — Verify git sync updates checkout after fixture repo change.
SCENARIO_NAME="repo-sync"
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

info "Modifying fixture repo..."
FIXTURE_WORK="/tmp/fixture-sync-work"
rm -rf "$FIXTURE_WORK"
git clone "$FIXTURE_REPO" "$FIXTURE_WORK" > /dev/null 2>&1
echo '# updated by sync test' >> "$FIXTURE_WORK/config/fish/config.fish"
(
    cd "$FIXTURE_WORK"
    git add -A > /dev/null 2>&1
    git -c user.name="e2e-test" -c user.email="e2e@test" commit -m "fixture: sync test update" > /dev/null 2>&1
    git push origin main > /dev/null 2>&1
)
rm -rf "$FIXTURE_WORK"

info "dotman sync"
set +e
OUTPUT=$(dotman sync 2>&1)
SYNC_EXIT=$?
set -e
assert_eq "$SYNC_EXIT" "0" "sync exits 0"
assert_contains "$OUTPUT" "complete" "sync succeeds"

info "Verify checkout has updated content"
assert_file_contains "$CHECKOUT_PATH/config/fish/config.fish" "updated by sync test" "sync pulled new content"

info "dotman plan reflects new content"
PLAN=$(dotman plan 2>&1)
assert_contains "$PLAN" '"kind"' "plan still valid after sync"
# Verify plan is valid JSON (not an error message)
echo "$PLAN" | python3 -c "import sys,json; json.load(sys.stdin)" 2>/dev/null
assert_eq "$?" "0" "plan output is valid JSON"

summary
