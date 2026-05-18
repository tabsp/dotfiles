# Security And Project Governance Baseline Implementation Plan

**Spec:** `docs/superpowers/specs/2026-05-18-p1-security-and-project-governance-baseline-design.md`

**Goal:** Add SECURITY.md, LICENSE, release checklist, and trust boundary
documentation.

---

## Task 1: Create governance files

- [ ] `SECURITY.md`: supported versions, reporting, trust boundaries.
- [ ] `LICENSE`: MIT license.
- [ ] `docs/release-checklist.md`: pre-release verification steps.
- [ ] `docs/trust-boundaries.md`: trust model per installer type.

## Task 2: Update risk register

- [ ] Close `official_script` trust boundary risk item in roadmap.

## Verification

- `cargo test` passes (no code changes).
- `make check` passes.

## Test Level

No code changes. Documentation review only.

## Regression Coverage Expectations

No code changes. All existing tests pass.

---

## Existing Code Map

No code changes. Files added:
- `SECURITY.md` (new)
- `LICENSE` (new)
- `docs/release-checklist.md` (new)
- `docs/trust-boundaries.md` (new)
- `docs/roadmap.md` (risk register item)

**Files:** SECURITY.md, LICENSE, docs/release-checklist.md, docs/trust-boundaries.md

## Verification Commands

- `cargo test` — no code changes; all existing tests pass.
- `make check` — manifest validation passes.
- Manual review of new documentation files.

## Expected Outcomes

- SECURITY.md with supported versions, reporting, and trust boundaries.
- MIT LICENSE file.
- Release checklist for maintainers.
- Trust boundary documentation per installer type.
- Risk register item for `official_script` closed.
