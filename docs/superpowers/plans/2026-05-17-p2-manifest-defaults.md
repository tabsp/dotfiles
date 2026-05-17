# Manifest Defaults Implementation Plan

**Spec:** `docs/superpowers/specs/2026-05-17-p2-manifest-defaults-design.md`

**Goal:** Add `default` section to deps.toml entries so common fields are declared once and inherited by per-arch entries.

**Architecture:** Add `default: Option<InstallEntry>` to `Dependency`. Modify `entries_for` to merge defaults. Update check validation and schema docs.

**Tech Stack:** Rust 2024, existing config module.

---

## Existing Code Map

- `src/config.rs:9-22` (`Dependency` struct): add `default` field.
- `src/config.rs:24-36` (`entries_for`, `entries_for_host`): add merge logic.
- `src/check.rs:149-189` (download_binary validation): validate default entries too.
- `docs/manifest-schema.md`: document the new section.

## Task 1: Implement default entry merging

**Files:**
- Modify: `src/config.rs`
- Modify: `src/check.rs`
- Modify: `docs/manifest-schema.md`

- [ ] Add `default: Option<InstallEntry>` to `Dependency`.
- [ ] Add merge function that combines default + per-arch entry.
- [ ] Update `entries_for` to return merged entries.
- [ ] Update `make check` to validate default install entries.
- [ ] Add unit tests for merge behavior.

## Verification Commands

- `cargo test config`
- `cargo test`
- `cargo clippy`

## Test Level

- Unit tests: `cargo test config`

## Regression Coverage Expectations

- Existing deps.toml (no default sections) works unchanged.
- Default + per-arch merge produces correct final entry.
- Default-only entry (no per-arch) works as fallback.
