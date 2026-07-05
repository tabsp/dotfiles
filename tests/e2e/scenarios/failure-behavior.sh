#!/bin/bash
# failure-behavior.sh — Verify required/optional failure semantics.
SCENARIO_NAME="failure-behavior"
FIXTURE_TYPE=failure
# shellcheck source=tests/e2e/common.sh
source "$(dirname "$0")/../common.sh"
# shellcheck source=tests/e2e/setup.sh
source "$(dirname "$0")/../setup.sh"

info "Running init with failure fixture..."
set +e
INIT_OUT=$(dotman init "$FIXTURE_REPO" --profile e2e --branch main --path "$CHECKOUT_PATH" 2>&1)
INIT_EXIT=$?
set -e
assert_eq "$INIT_EXIT" "0" "init exits 0"
assert_contains "$INIT_OUT" "initialized" "init succeeds"

# The failure fixture has:
#   shell[0]: "echo deployment-verified" (optional: false)
#   shell[1]: scripts/required-fail.sh    (optional: false)
#   shell[2]: scripts/optional-fail.sh    (optional: true)

# ---- Phase 1: required failure must cause non-zero exit ----

info "dotman deploy (must fail due to required-fail.sh)"
set +e
DEPLOY_OUTPUT=$(dotman deploy 2>&1)
DEPLOY_EXIT=$?
set -e
assert_eq "$DEPLOY_EXIT" "1" "deploy with required failure exits non-zero"
assert_contains "$DEPLOY_OUTPUT" "required-fail" "deploy output mentions required-fail"

info "Verify run history records the failed deploy"
HISTORY=$(dotman history 2>&1)
assert_contains "$HISTORY" "deploy" "history has entries after failed deploy"

# ---- Phase 2: optional failure must not block deploy ----

info "Remove required-fail, keep optional-fail..."
python3 -c "
import yaml
with open('$CHECKOUT_PATH/dotman.yaml') as f:
    cfg = yaml.safe_load(f)
cfg['shell'] = [s for s in cfg['shell'] if 'required-fail' not in s.get('command', '')]
with open('$CHECKOUT_PATH/dotman.yaml', 'w') as f:
    yaml.dump(cfg, f, default_flow_style=False, allow_unicode=True)
"

info "dotman deploy (optional failure must not block)"
set +e
DEPLOY2=$(dotman deploy 2>&1)
DEPLOY2_EXIT=$?
set -e
info "Deploy exit: $DEPLOY2_EXIT"
assert_eq "$DEPLOY2_EXIT" "0" "deploy with only optional failure succeeds (exit 0)"
assert_contains "$DEPLOY2" "optional-fail" "deploy output mentions optional-fail"
assert_contains "$DEPLOY2" "deployment-verified" "deploy marker shell still ran after optional fail"

summary
