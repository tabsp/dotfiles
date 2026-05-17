# CI Automation Implementation Plan

**Spec:** `docs/superpowers/specs/2026-05-17-p2-ci-automation-design.md`

**Goal:** Add GitHub Actions CI workflow that runs `make ci` on push/PR.

**Architecture:** Create `.github/workflows/ci.yml` and `rust-toolchain.toml`.

**Tech Stack:** GitHub Actions, `actions-rust-lang/setup-rust-toolchain@v1`.

---

## Existing Code Map

- `Makefile`: `make ci` runs lint + check + test.
- `Cargo.toml`: Rust 2024 edition.
- No existing CI configuration.

## Task 1: Add CI workflow and toolchain pin

**Files:**
- New: `.github/workflows/ci.yml`
- New: `rust-toolchain.toml`

- [ ] Create `.github/workflows/ci.yml` with push/PR triggers on `main`.
- [ ] Use `ubuntu-latest` + `actions-rust-lang/setup-rust-toolchain@v1`.
- [ ] Run `make ci` as the verification step.
- [ ] Add `rust-toolchain.toml` pinning stable.

## Verification Commands

- `make ci`
- Manual: push to trigger CI

## Test Level

- Manual smoke test: push branch, observe CI run.

## Regression Coverage Expectations

- `make ci` must pass on Ubuntu.
