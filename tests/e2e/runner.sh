#!/bin/bash
# runner.sh — Docker entrypoint for dotman e2e tests.
#
# Runs all scenario scripts sequentially, reports pass/fail counts.
# Set SKIP_NETWORK_TESTS=1 to skip scenarios requiring network access.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Build dotman if not already built.
DOTMAN="${DOTMAN_BIN:-/app/target/release/dotman}"
export DOTMAN_BIN="$DOTMAN"

if [[ ! -x "$DOTMAN" ]]; then
    echo "Building dotman..."
    cd /app && cargo build --release
fi

echo ""
echo "========================================="
echo " dotman e2e test suite"
echo "========================================="
echo " dotman: $DOTMAN"
echo " HOME:   ${HOME:-/tmp/dotman-home}"
echo "========================================="
echo ""

export HOME="${DOTMAN_HOME:-/tmp/dotman-home}"

SCENARIOS=(
    profile-lifecycle
    deploy-runtime
    repo-sync
    history-run
    new-link
    failure-behavior
)

if [[ "${SKIP_NETWORK_TESTS:-0}" != "1" ]]; then
    SCENARIOS+=(install-branches)
else
    echo "[SKIP] install-branches (network tests disabled)"
fi

TOTAL_PASS=0
TOTAL_FAIL=0

for scenario in "${SCENARIOS[@]}"; do
    SCENARIO_PATH="$SCRIPT_DIR/scenarios/${scenario}.sh"
    if [[ ! -f "$SCENARIO_PATH" ]]; then
        echo "=== SKIP: $scenario (not found) ==="
        continue
    fi
    echo ""
    echo "=== Running: $scenario ==="
    if bash "$SCENARIO_PATH"; then
        TOTAL_PASS=$((TOTAL_PASS + 1))
        echo "=== PASS: $scenario ==="
    else
        TOTAL_FAIL=$((TOTAL_FAIL + 1))
        echo "=== FAIL: $scenario ==="
    fi
done

echo ""
echo "========================================="
echo " Results: $TOTAL_PASS passed, $TOTAL_FAIL failed"
echo "========================================="

if [[ $TOTAL_FAIL -eq 0 ]]; then
    echo "All e2e tests passed."
    exit 0
else
    echo "Some e2e tests failed."
    exit 1
fi
