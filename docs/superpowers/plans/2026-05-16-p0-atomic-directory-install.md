# Atomic Directory Install Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** `docs/superpowers/specs/2026-05-16-p0-atomic-directory-install-design.md`

**Goal:** Make `install_archive_dir` atomic by staging new directory content in a sibling temp directory and using `rename` to promote it into place, preserving the old install until the new one is verified.

**Architecture:** Modify `install_archive_dir` in `src/installers.rs` to use a staging-then-rename pattern. Keep `copy_dir_recursive` unchanged. Update `install_download_binary` to handle the new error flow correctly. Add unit tests for the new behavior.

**Tech Stack:** Rust 2024, existing `std::fs`, `tempfile` crate for sibling temp directories.

---

## Existing Code Map

- `src/installers.rs:310-383` (`install_archive_dir`): target function for atomic refactor.
- `src/installers.rs:385-423` (`copy_dir_recursive`): unchanged; handles per-file copy.
- `src/installers.rs:110-158` (`install_download_binary`): caller of `install_archive_dir`; verify error propagation unchanged.
- `tests/`: no existing tests for directory install. Add tests in `src/installers.rs` `#[cfg(test)]` module.

## Task 1: Add sibling tempdir helper and refactor install_archive_dir

**Files:**
- Modify: `src/installers.rs`

- [x] Add `use std::time::{SystemTime, UNIX_EPOCH}` import.
- [x] Add `sibling_tempdir` helper that creates a uniquely-named sibling temp directory.
- [x] Refactor `install_archive_dir` to stage-and-rename pattern:
  - Copy source into staging directory first.
  - Rename old install to `.old` backup.
  - Rename staging into place.
  - Clean up `.old` on success.

## Task 2: Add unit tests for atomic directory install

**Files:**
- Modify: `src/installers.rs`

- [x] Test first-time install (no existing `install_dir_to`).
- [x] Test upgrade (existing `install_dir_to` present, old dir renamed to `.old`).
- [x] Test staging failure preserves old install.
- [x] Test leftover `.old` backup cleanup before rename.
- [x] Test intermediate directory creation.

## Verification Commands

- `cargo test atomic_install`
- `cargo test`
- `cargo clippy`

## Expected Outcomes

- `install_archive_dir` uses atomic staging-then-rename instead of remove-then-copy.
- Old directory install remains usable if staging or rename fails.
- Symlink at `install_to` points to the new binary after successful upgrade.
- All 5 new tests pass, zero regressions across the full test suite.
- Clippy reports zero warnings.
