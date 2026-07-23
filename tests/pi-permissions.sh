#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

node --input-type=module - "$ROOT_DIR/config/pi/agent/pi-permissions.jsonc" <<'EOF'
import fs from "node:fs";

const config = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
const entries = Object.entries(config.bash);

function compile(pattern) {
  let escaped = pattern
    .replaceAll("\\", "/")
    .replace(/[.+^${}()|[\]\\]/g, "\\$&")
    .replace(/\*/g, ".*")
    .replace(/\?/g, ".");
  if (escaped.endsWith(" .*")) {
    escaped = `${escaped.slice(0, -3)}( .*)?`;
  }
  return new RegExp(`^${escaped}$`, "s");
}

const rules = entries.map(([pattern, state]) => ({
  pattern,
  state,
  regex: compile(pattern),
}));

function resolve(command) {
  for (let index = rules.length - 1; index >= 0; index -= 1) {
    if (rules[index].regex.test(command)) {
      return rules[index];
    }
  }
  return { state: config.defaultPolicy.bash };
}

const cases = [
  ["rg TODO src", "allow"],
  ["git diff --check", "allow"],
  ["cargo test --workspace", "allow"],
  ["make config-check", "allow"],
  ["npm run lint", "allow"],
  ["rg --pre cat TODO", "ask"],
  ["sed -i '' file", "ask"],
  ["git diff --output=patch", "ask"],
  ["rg TODO; rm -rf build", "ask"],
  ["cargo test && cargo publish", "ask"],
  ["rg TODO\nrm -rf build", "ask"],
  ["make deploy", "ask"],
  ["rm -rf build", "ask"],
];

for (const [command, expected] of cases) {
  const actual = resolve(command);
  if (actual.state !== expected) {
    throw new Error(
      `${JSON.stringify(command)}: expected ${expected}, got ${actual.state} via ${actual.pattern ?? "default"}`,
    );
  }
}

console.log("Pi permission policy tests passed");
EOF
