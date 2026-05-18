# Release Installer Trust Boundary Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** `docs/superpowers/specs/2026-05-17-p0-release-installer-trust-boundary-hardening-design.md`

**Goal:** Make checksum verification mandatory in `scripts/install.sh` for both
the dotman binary and dotfiles source archive. Fail on missing checksum,
missing tool, or mismatch.

**Architecture:** Shell script changes only. No Rust code changes. Add failure
path tests in `tests/install_script.rs`.

**Tech Stack:** POSIX sh, curl/wget, shasum/sha256sum, Rust test harness.

---

## Existing Code Map

- `scripts/install.sh`: the installer script to harden.
- `tests/install_script.rs`: existing integration test; add failure path tests.

## Task: Harden Installer Trust Boundary

**Files:**
- Modify: `scripts/install.sh`
- Test: `tests/install_script.rs`

- [ ] **Step 1: Make checksum download mandatory**

Remove `|| true` from curl/wget checksum download commands. Fail if checksum
can't be fetched.

- [ ] **Step 2: Add checksum tool detection and mandatory verification**

Detect shasum/sha256sum early. Fail if neither is available. Always verify.

- [ ] **Step 3: Add dotfiles source checksum verification**

Download and verify checksum for the dotfiles source archive.

- [ ] **Step 4: Add failure path tests**

Add integration tests for: missing checksum, checksum mismatch, missing tool.

- [ ] **Step 5: Verify and advance**

Run `cargo test install_script`, `cargo clippy`, `make check`.

## Verification Commands

- `cargo test install_script`
- `cargo test`
- `make check`

## Test Level

- Integration tests: `cargo test --test install_script`

## Regression Coverage Expectations

- Existing `install_script_installs_dotfiles_source_and_prints_bootstrap_directory` must pass.
- `set -e` behavior must not be weakened.

## Expected Outcomes

- Installer fails with clear error if checksum can't be downloaded.
- Installer fails if shasum/sha256sum is unavailable.
- Installer fails on checksum mismatch.
- Dotfiles source archive is checksum-verified.
- All existing and new tests pass.
