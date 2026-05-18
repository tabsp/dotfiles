# Manifest Compatibility Guardrails Design

## Goal

Add a lightweight `schema_version` field to dotman manifests and compatibility
tests that ensure manifest parsing remains backward-compatible, so manifest
evolution stays boring and predictable.

## Motivation

Today `deps.toml` and `dotfiles.toml` have no version marker. If manifest
structure changes (new required fields, renamed keys, changed types), there's
no way to detect incompatibility or give users a clear error. The P2 roadmap
item calls for "compatibility tests, deprecation rules, and schema version
decision points."

## Scope

- Add optional `schema_version = 1` field to `DepsManifest` and `DotfilesManifest`.
- When `schema_version` is missing, default to 1 (current format).
- When `schema_version` > supported, reject with clear error:
  `manifest requires schema version N but dotman supports up to 1`.
- Add compatibility tests that parse manifests at version 1 and verify
  all current fields round-trip correctly.
- Document deprecation rules: fields can be added (optional), never removed
  or renamed without a schema version bump and migration period.

## Non-Goals

- Do not implement automatic migration between schema versions.
- Do not add a `schema_version` field to agent state or other config files.
- Do not change manifest format — `schema_version` is purely additive.
- Do not create a migration tool or migration guide (deferred).

## Design

### Schema Version Field

```toml
# deps.toml
schema_version = 1

[deps.bat]
command = "bat"
# ...existing fields...
```

```toml
# dotfiles.toml
schema_version = 1

[[files]]
source = "config/nvim"
target = "~/.config/nvim"
```

### Parsing Rules

- `schema_version` is optional. If absent, default to 1.
- If present and > 1, return error: `"manifest requires schema version N but dotman supports up to 1. Upgrade dotman or use an older manifest."`
- If present and == 1, parse normally.
- If present and < 1 (invalid), return error: `"manifest schema version N is not supported (minimum: 1)."`

### Deprecation Rules (Documentation)

Add a `docs/manifest-schema.md` documenting:

1. Current schema version: 1.
2. Rules for evolving the manifest:
   - New optional fields: add with `#[serde(default)]`, no version bump needed.
   - New required fields: requires schema version bump and migration period.
   - Renamed fields: requires schema version bump + old name support for one version.
   - Removed fields: requires schema version bump + deprecation warning for one version.
3. Decision tree for when to bump schema version.

### Compatibility Tests

Add `cargo test manifest_compat` tests that:
- Parse a `deps.toml` with `schema_version = 1` and all field types.
- Parse a `dotfiles.toml` with `schema_version = 1`.
- Parse manifests without `schema_version` (defaults to 1).
- Reject `schema_version = 99` with clear error.
- Reject `schema_version = 0` with clear error.

### Implementation Strategy

- Add `schema_version: Option<u32>` to `DepsManifest` and `DotfilesManifest`
  with `#[serde(default)]`.
- After deserialization, validate schema version:
  - `None` → set to 1.
  - `Some(v)` where `v > 1` → error.
  - `Some(v)` where `v < 1` → error.
- Validation happens in `load_deps()` and `load_dotfiles()`.

### Error Handling

- Bad schema version: `"manifest requires schema version N but dotman supports up to 1."`
- Missing `schema_version`: treated as version 1 (backward compatible).

### Verification Strategy

- `cargo test manifest_compat` — new compatibility tests.
- `cargo test` — all tests pass.
- `cargo clippy` — zero warnings.
- `make check` — manifest validation.

### Regression Coverage Expectations

- Existing manifests (without `schema_version`) continue to parse correctly.
- All existing tests pass.
- `dotman check`, `dotman diff`, `dotman status` work unchanged on existing repos.
