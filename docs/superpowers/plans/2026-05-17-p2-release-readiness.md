# Release Readiness Implementation Plan

**Spec:** `docs/superpowers/specs/2026-05-17-p2-release-readiness-design.md`

**Goal:** Define release contract: versioning, artifact naming, changelog, and backward compatibility policy.

**Architecture:** Documentation-only. No runtime code changes.

**Tech Stack:** Markdown.

---

## Existing Code Map

- `Cargo.toml`: version 0.1.0, package metadata.
- `README.md`: documentation index.
- `docs/roadmap.md`: P2 Release Readiness entry.
- Existing `deps.toml`, `dotfiles.toml`: schemas subject to compatibility policy.
- `src/main.rs`: CLI flags subject to compatibility policy.
- `src/agent.rs`: AGENT_* error codes subject to compatibility policy.

## Task 1: Create release policy document

**Files:**
- New: `docs/release-policy.md`

- [ ] Write semver policy with breaking-change criteria.
- [ ] Define artifact naming convention.
- [ ] Define backward compatibility guarantees for manifests, CLI, error codes.

## Task 2: Create changelog

**Files:**
- New: `CHANGELOG.md`

- [ ] Create `CHANGELOG.md` following Keep a Changelog format.
- [ ] Add initial entries for completed epics from this session.

## Task 3: Link from README

**Files:**
- Modify: `README.md`

- [ ] Add `docs/release-policy.md` and `CHANGELOG.md` links to documentation index.

## Verification Commands

- `cargo test`
- `cargo clippy`
- `make lint`
- `make check`

## Test Level

- No new tests (documentation-only change).
- Unit tests: `cargo test` (existing).
- CLI integration tests: `cargo test` (existing).

## Expected Outcomes

- `docs/release-policy.md` exists with all four policy areas.
- `CHANGELOG.md` exists with initial entries.
- Both linked from `README.md`.
- All existing tests pass.
- `cargo clippy` zero warnings.

## Regression Coverage Expectations

- All 101+ existing tests continue to pass.
- `Cargo.toml` version unchanged (0.1.0).
- No runtime behavior changes.
