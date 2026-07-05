#!/bin/bash
# common.sh — Shared assertion helpers and utilities for e2e scenarios.
#
# Source this file at the beginning of each scenario:
#   source "$(dirname "$0")/../common.sh"

set -euo pipefail

# ---- Color output ----
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

PASS_COUNT=0
FAIL_COUNT=0

pass() {
    echo -e "${GREEN}[PASS]${NC} $*"
    PASS_COUNT=$((PASS_COUNT + 1))
}

fail() {
    echo -e "${RED}[FAIL]${NC} $*" >&2
    FAIL_COUNT=$((FAIL_COUNT + 1))
}

info() {
    echo -e "${YELLOW}[INFO]${NC} $*"
}

# ---- Dotman binary ----
DOTMAN="${DOTMAN:-${DOTMAN_BIN:-target/release/dotman}}"

dotman() {
    # Always run from $HOME so the active profile (not cwd dotman.yaml) is used.
    # This prevents repo root /app/dotman.yaml from shadowing the fixture profile.
    (
        cd "$HOME"
        HOME="$HOME" "$DOTMAN" --headless "$@"
    )
}

# dotman_in_dir <dir> [args...] — run dotman with CWD=<dir>.
# Use for commands that must operate on a specific directory (e.g. new-link
# reads dotman.yaml from cwd).
dotman_in_dir() {
    local dir="$1"
    shift
    (
        cd "$dir"
        HOME="$HOME" "$DOTMAN" --headless "$@"
    )
}

# ---- Assertions ----

# assert_contains <haystack> <needle> [message]
assert_contains() {
    local haystack="$1"
    local needle="$2"
    local msg="${3:-expected \"$needle\" in output}"
    if echo "$haystack" | grep -qF "$needle"; then
        pass "$msg"
    else
        fail "$msg — output did not contain '$needle'"
        return 1
    fi
}

# assert_not_contains <haystack> <needle> [message]
assert_not_contains() {
    local haystack="$1"
    local needle="$2"
    local msg="${3:-expected \"$needle\" NOT in output}"
    if ! echo "$haystack" | grep -qF "$needle"; then
        pass "$msg"
    else
        fail "$msg — output contains '$needle'"
        return 1
    fi
}

# assert_eq <actual> <expected> [message]
assert_eq() {
    local actual="$1"
    local expected="$2"
    local msg="${3:-expected \"$expected\"}"
    if [[ "$actual" == "$expected" ]]; then
        pass "$msg"
    else
        fail "$msg — got '$actual'"
        return 1
    fi
}

# assert_file_exists <path>
assert_file_exists() {
    local path="${1/#\~/$HOME}"
    local msg="file exists: $path"
    if [[ -e "$path" ]]; then
        pass "$msg"
    else
        fail "$msg"
        return 1
    fi
}

# assert_symlink_exists <path> [expected_target]
assert_symlink_exists() {
    local path="${1/#\~/$HOME}"
    local expected="${2:-}"
    local msg="symlink exists: $path"
    if [[ -L "$path" ]]; then
        if [[ -n "$expected" ]]; then
            local actual
            actual="$(readlink "$path")"
            if [[ "$actual" == "$expected" ]]; then
                pass "symlink $path -> $expected"
            else
                fail "symlink $path points to '$actual', expected '$expected'"
                return 1
            fi
        else
            pass "$msg"
        fi
    else
        fail "$msg"
        return 1
    fi
}

# assert_dir_exists <path>
assert_dir_exists() {
    local path="${1/#\~/$HOME}"
    local msg="directory exists: $path"
    if [[ -d "$path" ]]; then
        pass "$msg"
    else
        fail "$msg"
        return 1
    fi
}

# assert_exit_code <expected_code> <command...>
# Runs the command and checks its exit code.
assert_exit_code() {
    local expected="$1"
    shift
    set +e
    "$@"
    local actual=$?
    set -e
    local msg="exit code $expected for: $*"
    if [[ $actual -eq $expected ]]; then
        pass "$msg"
    else
        fail "$msg — got $actual"
        return 1
    fi
}

# assert_file_contains <file> <needle> [message]
assert_file_contains() {
    local file="${1/#\~/$HOME}"
    local needle="$2"
    local msg="${3:-file $file contains \"$needle\"}"
    if [[ -f "$file" ]] && grep -qF "$needle" "$file"; then
        pass "$msg"
    else
        fail "$msg"
        return 1
    fi
}

# summary — call at end of scenario to report.
summary() {
    echo ""
    if [[ $FAIL_COUNT -eq 0 ]]; then
        echo -e "${GREEN}Scenario: $SCENARIO_NAME — $PASS_COUNT passed${NC}"
        return 0
    else
        echo -e "${RED}Scenario: $SCENARIO_NAME — $PASS_COUNT passed, $FAIL_COUNT failed${NC}"
        return 1
    fi
}
