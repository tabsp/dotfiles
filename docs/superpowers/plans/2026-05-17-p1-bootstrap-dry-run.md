# Bootstrap Dry Run Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** `docs/superpowers/specs/2026-05-17-p1-bootstrap-dry-run-design.md`

**Goal:** Add `--dry-run` flag to `bootstrap` command so users can preview the full setup process without installing packages or mutating the filesystem.

**Architecture:** Thread a `dry_run: bool` parameter through `run_bootstrap`. In dry-run mode, replace `deps::install_missing` with a preview loop that calls `is_installed` for each dependency. Pass `dry_run=true` to `link::run_link`. Skip doctor and hints.

**Tech Stack:** Rust 2024, existing `clap`, `link::run_link`.

---

## Existing Code Map

- `src/main.rs:30-42` (`Command` enum): add `dry_run` to `Bootstrap` variant.
- `src/main.rs:64` (`run`): pass `dry_run` to `run_bootstrap`.
- `src/main.rs:120-136` (`run_bootstrap`): add dry-run branch.
- `src/link.rs:20-30` (`run_link`): already supports dry_run.
- `src/deps.rs:5-38` (`install_missing`): reference for dep iteration.
- `src/installers.rs:12-41` (`is_installed`): used to check status per dep.

## Task 1: Add --dry-run to CLI and wire through run_bootstrap

**Files:**
- Modify: `src/main.rs`

- [ ] Add `#[arg(long)] dry_run: bool` to `Command::Bootstrap`.
- [ ] Pass to `run_bootstrap(dry_run)`.
- [ ] In dry-run: run check, preview deps, run link dry, skip doctor/hints.
- [ ] In non-dry-run: same as before.

## Verification Commands

- `cargo test`
- `cargo clippy`

## Expected Outcomes

- `dotman bootstrap --dry-run` prints preview of all steps.
- `dotman bootstrap` (without flag) works identically to before.
- Zero regressions in existing tests.
