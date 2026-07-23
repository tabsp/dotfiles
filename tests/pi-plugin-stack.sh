#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STACK="$ROOT_DIR/bin/pi-plugin-stack"
CATALOG="$ROOT_DIR/config/pi/plugins.json"
TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/pi-plugin-stack-test.XXXXXX")"
trap 'rm -rf "$TEST_ROOT"' EXIT

mkdir -p "$TEST_ROOT/bin" "$TEST_ROOT/pi"

cat >"$TEST_ROOT/bin/pi" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

[[ "${1:-}" == install && -n "${2:-}" ]]
source="$2"
if [[ "$source" == "${FAKE_PI_FAIL_SOURCE:-}" ]]; then
  exit 42
fi

spec="${source#npm:}"
version="${spec##*@}"
name="${spec%@*}"
manifest_dir="$PI_CODING_AGENT_DIR/npm/node_modules/$name"
mkdir -p "$manifest_dir"
printf '{"version":"%s"}\n' "$version" >"$manifest_dir/package.json"
while IFS= read -r resource; do
  [[ -n "$resource" ]] || continue
  mkdir -p "$manifest_dir/$(dirname "$resource")"
  : >"$manifest_dir/$resource"
done < <(
  jq -r --arg source "$source" '
    .packages[]
    | select(type == "object" and .source == $source)
    | ((.extensions // []) + (.skills // []))[]
  ' "$PI_PLUGIN_CATALOG"
)

settings="$PI_CODING_AGENT_DIR/settings.json"
if [[ ! -f "$settings" ]]; then
  printf '{}\n' >"$settings"
fi
candidate="$(mktemp "$PI_CODING_AGENT_DIR/settings.json.fake.XXXXXX")"
jq --arg source "$source" '.packages = ((.packages // []) + [$source])' \
  "$settings" >"$candidate"
mv "$candidate" "$settings"
EOF
chmod +x "$TEST_ROOT/bin/pi"

jq -n '{
  defaultProvider: "volcengine",
  defaultModel: "glm-5.2",
  customSetting: {preserve: true},
  packages: ["npm:old-plugin@1.0.0"]
}' >"$TEST_ROOT/pi/settings.json"

export PATH="$TEST_ROOT/bin:$PATH"
export PI_CODING_AGENT_DIR="$TEST_ROOT/pi"
export PI_PLUGIN_CATALOG="$CATALOG"

"$STACK" list >/dev/null
"$STACK" install --dry-run >/dev/null
"$STACK" install >/dev/null
"$STACK" check >/dev/null

jq -e --slurpfile catalog "$CATALOG" '
  .defaultProvider == "volcengine"
  and .defaultModel == "glm-5.2"
  and .customSetting == {preserve: true}
  and .packages == $catalog[0].packages
' "$TEST_ROOT/pi/settings.json" >/dev/null

rm "$TEST_ROOT/pi/npm/node_modules/pi-web-access/skills/librarian/SKILL.md"
if "$STACK" check >/dev/null 2>&1; then
  printf 'missing configured plugin resource was accepted\n' >&2
  exit 1
fi
mkdir -p "$TEST_ROOT/pi/npm/node_modules/pi-web-access/skills/librarian"
: >"$TEST_ROOT/pi/npm/node_modules/pi-web-access/skills/librarian/SKILL.md"

jq '.packages[0] = "npm:pi-permission-system@^0.8.0"' \
  "$CATALOG" >"$TEST_ROOT/non-exact.json"
if PI_PLUGIN_CATALOG="$TEST_ROOT/non-exact.json" "$STACK" list >/dev/null 2>&1; then
  printf 'non-exact plugin version was accepted\n' >&2
  exit 1
fi

cp "$TEST_ROOT/pi/settings.json" "$TEST_ROOT/settings.before-failure.json"
export FAKE_PI_FAIL_SOURCE="npm:pi-subagents@0.35.1"
if "$STACK" install >/dev/null 2>&1; then
  printf 'simulated plugin installation failure unexpectedly succeeded\n' >&2
  exit 1
fi
cmp "$TEST_ROOT/settings.before-failure.json" "$TEST_ROOT/pi/settings.json"

printf 'pi-plugin-stack tests passed\n'
