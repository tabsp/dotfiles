#!/bin/bash
# profile-lifecycle.sh — Test profile add/list/remove commands.
SCENARIO_NAME="profile-lifecycle"
# shellcheck source=tests/e2e/common.sh
source "$(dirname "$0")/../common.sh"

info "Testing profile lifecycle..."

# List (should work without init)
OUTPUT=$(dotman profile list 2>&1) || true
assert_contains "$OUTPUT" "profile" "profile list produces output"

# Add a profile
OUTPUT=$(dotman profile add test https://example.com/test.git 2>&1)
assert_contains "$OUTPUT" "added" "profile add succeeds"

# List should show the new profile
OUTPUT=$(dotman profile list 2>&1)
assert_contains "$OUTPUT" "test" "profile list contains 'test'"

# Duplicate add should fail
set +e
OUTPUT=$(dotman profile add test https://example.com/test.git 2>&1)
ADD_EXIT=$?
set -e
assert_eq "$ADD_EXIT" "1" "duplicate profile add fails"

# Remove
OUTPUT=$(dotman profile remove test 2>&1)
assert_contains "$OUTPUT" "removed" "profile remove succeeds"

# Remove non-existent should fail
set +e
OUTPUT=$(dotman profile remove missing 2>&1)
RM_EXIT=$?
set -e
assert_eq "$RM_EXIT" "1" "removing non-existent profile fails"

# Re-add after remove
OUTPUT=$(dotman profile add test2 https://example.com/test2.git 2>&1)
assert_contains "$OUTPUT" "added" "profile re-add succeeds"

# Cleanup
dotman profile remove test2 2>&1 || true

summary
