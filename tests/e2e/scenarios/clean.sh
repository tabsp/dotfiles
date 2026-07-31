#!/bin/bash
# clean.sh — Verify clean semantics against the clean-only fixture:
#   - missing target:            no-op
#   - plain file (no force):     preserved
#   - dangling symlink:          removed
#   - plain file (force: true):  backed up next to it, then removed
#   - sentinel outside targets:  untouched
#   - second deploy:             idempotent
SCENARIO_NAME="clean"
FIXTURE_TYPE=clean
# shellcheck source=tests/e2e/common.sh
source "$(dirname "$0")/../common.sh"
# shellcheck source=tests/e2e/setup.sh
source "$(dirname "$0")/../setup.sh"

CLEAN_DIR="$HOME/.local/clean-fixture"
mkdir -p "$CLEAN_DIR"

info "Preparing clean fixture targets..."
printf 'keep-me\n' > "$CLEAN_DIR/plain-file"
printf 'force-me\n' > "$CLEAN_DIR/forced-file"
ln -s "$CLEAN_DIR/nonexistent-target" "$CLEAN_DIR/dangling-link"
printf 'sentinel\n' > "$CLEAN_DIR/sentinel"

info "dotman init (clean fixture)"
set +e
INIT_OUT=$(dotman init "$FIXTURE_REPO" --profile e2e --branch main --path "$CHECKOUT_PATH" 2>&1)
INIT_EXIT=$?
set -e
assert_eq "$INIT_EXIT" "0" "init exits 0"
assert_contains "$INIT_OUT" "initialized" "init succeeds"

info "dotman plan --headless includes clean actions"
PLAN=$(dotman plan 2>&1)
assert_contains "$PLAN" '"clean"' "plan contains clean action"
# Plan JSON keeps the literal `~` form; execution expands it.
# shellcheck disable=SC2088 # the literal tilde is what the plan JSON contains
assert_contains "$PLAN" "~/.local/clean-fixture/forced-file" "plan lists forced-file target"
assert_contains "$PLAN" '"force": true' "plan marks the forced clean with force: true"

info "dotman deploy --headless"
set +e
DEPLOY_OUTPUT=$(dotman deploy 2>&1)
DEPLOY_EXIT=$?
set -e
assert_eq "$DEPLOY_EXIT" "0" "deploy exits 0"
assert_contains "$DEPLOY_OUTPUT" "remove symlink" "deploy reports dangling symlink removal"
assert_contains "$DEPLOY_OUTPUT" "backup to" "deploy reports forced-file backup"

info "Verify clean results"
assert_not_exists "$CLEAN_DIR/missing-target" "missing target stays absent"
assert_file_exists "$CLEAN_DIR/plain-file" "plain file is preserved"
assert_eq "$(cat "$CLEAN_DIR/plain-file")" "keep-me" "plain file content preserved"
assert_not_exists "$CLEAN_DIR/dangling-link" "dangling symlink removed"
assert_not_exists "$CLEAN_DIR/forced-file" "forced file removed"
BACKUP="$(find "$CLEAN_DIR" -maxdepth 1 -name 'forced-file.backup.*' -print -quit 2>/dev/null || true)"
if [[ -z "$BACKUP" ]]; then
    fail "no backup found for forced-file (expected forced-file.backup.* in $CLEAN_DIR)"
    exit 1
fi
assert_file_exists "$BACKUP" "forced file backup exists next to it"
assert_eq "$(cat "$BACKUP")" "force-me" "forced file content preserved in backup"
assert_file_exists "$CLEAN_DIR/sentinel" "sentinel untouched"

info "dotman deploy again (idempotent)"
set +e
dotman deploy >/dev/null 2>&1
DEPLOY2_EXIT=$?
set -e
assert_eq "$DEPLOY2_EXIT" "0" "second deploy exits 0"
assert_not_exists "$CLEAN_DIR/forced-file" "forced file still absent after second deploy"
assert_file_exists "$CLEAN_DIR/plain-file" "plain file still preserved after second deploy"
assert_file_exists "$CLEAN_DIR/sentinel" "sentinel untouched after second deploy"

summary
