# Manifest Schema Evolution Implementation Plan

**Spec:** `docs/superpowers/specs/2026-05-17-p2-manifest-schema-evolution-design.md`

**Goal:** Create `docs/manifest-schema.md` documenting deps.toml and dotfiles.toml schemas.

**Architecture:** Single Markdown reference file. Link from README.md.

**Tech Stack:** Markdown.

---

## Existing Code Map

- `src/config.rs`: Rust structs that define the deserialization format.
- `src/check.rs`: Validation rules applied by `make check`.
- `deps.toml`: Example manifest in repo root.
- `dotfiles.toml`: Example manifest in repo root.

## Task 1: Create manifest schema documentation

**Files:**
- New: `docs/manifest-schema.md`
- Modify: `README.md`

- [ ] Document deps.toml: top-level, Dependency, VersionCheck, InstallEntry, Installer types, per-installer params.
- [ ] Document dotfiles.toml: FileEntry fields.
- [ ] Document field types, defaults, validation rules.
- [ ] Link from README.md.

## Verification Commands

- `make check`
- `cargo test`

## Test Level

- Manual review

## Regression Coverage Expectations

- `make check` and `cargo test` continue to pass.
