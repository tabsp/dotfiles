# Manifest Compatibility Guardrails Implementation Plan

**Spec:** `docs/superpowers/specs/2026-05-19-p2-manifest-compatibility-guardrails-design.md`

**Goal:** Add `schema_version` field to manifests with compatibility validation.

**Architecture:** Modify `src/config.rs` to add `schema_version` field and
validation. Add `docs/manifest-schema.md` for deprecation rules. No new modules.

**Tech Stack:** Rust, serde, TOML.

---

## Existing Code Map

- `src/config.rs`: `DepsManifest`, `DotfilesManifest` structs, `load_deps()`,
  `load_dotfiles()` functions.

## Task 1: Add schema_version to manifest structs

**Files:**
- Modify: `src/config.rs`

- [ ] Add `schema_version: Option<u32>` to `DepsManifest` with `#[serde(default)]`.
- [ ] Add `schema_version: Option<u32>` to `DotfilesManifest` with `#[serde(default)]`.
- [ ] Add `validate_schema_version(v: Option<u32>) -> Result<u32, String>` function.

## Task 2: Validate schema version on load

**Files:**
- Modify: `src/config.rs`

- [ ] In `load_deps()`, call `validate_schema_version` after deserialization.
- [ ] In `load_dotfiles()`, call `validate_schema_version` after deserialization.
- [ ] Error messages: `"manifest requires schema version N but dotman supports up to 1."`

## Task 3: Add compatibility tests

**Files:**
- Modify: `src/config.rs` (inline tests)

- [ ] Test: manifest with `schema_version = 1` parses correctly.
- [ ] Test: manifest without `schema_version` defaults to 1.
- [ ] Test: `schema_version = 99` is rejected.
- [ ] Test: `schema_version = 0` is rejected.

## Task 4: Document deprecation rules

**Files:**
- New: `docs/manifest-schema.md`

- [ ] Document schema version 1 fields and structure.
- [ ] Document evolution rules (optional fields, required fields, renamed, removed).
- [ ] Document decision tree for version bumps.

## Verification Commands

- `cargo test config` — existing + new compatibility tests.
- `cargo test` — all tests pass.
- `cargo clippy` — zero warnings.
- `make check` — manifest validation passes.

## Expected Outcomes

- Existing manifests (no `schema_version`) parse unchanged.
- Manifests with `schema_version = 1` parse correctly.
- Manifests with unsupported version are rejected with clear error.
- `docs/manifest-schema.md` documents compatibility rules.

## Test Level

- Unit tests: `src/config.rs` inline tests.

## Regression Coverage Expectations

- All existing config tests pass.
- `dotman check` works on existing repos without changes.
- `dotman diff`, `dotman status` unchanged.

## Machine State Safety

- **Dry-run / preview path:** `dotman check` on current repo will validate
  manifest compatibility without changes.
- **Failure-path tests:** Tests cover unsupported version rejection and
  missing version default.
- **Recovery notes:** Not applicable — no destructive changes.
- **Manual smoke checks:** `dotman check` on existing repo passes.
- **Non-destructive scope:** Read-only validation. No manifest modification.
