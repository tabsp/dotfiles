# Doctor Summary And Machine Output Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** `docs/superpowers/specs/2026-05-17-p1-doctor-summary-machine-output-design.md`

**Goal:** Add a summary line and optional `--json` flag to the `doctor` command.

**Architecture:** Add `json: bool` parameter to `doctor::run_doctor`. In JSON mode, serialize results with `serde_json`. Add `#[arg(long)] json: bool` to `Command::Doctor`. Thread parameter through `run_doctor` in main.rs.

**Tech Stack:** Rust 2024, `serde_json` (already in dependencies).

---

## Existing Code Map

- `src/doctor.rs:9-88` (`run_doctor`): collect and print items.
- `src/main.rs:44` (`Command::Doctor`): add json flag.
- `src/main.rs:69` (`run`): pass json to `run_doctor`.
- `src/main.rs:111-119` (`run_doctor`): pass json to `doctor::run_doctor`.

## Task 1: Add summary and --json flag

**Files:**
- Modify: `src/doctor.rs`
- Modify: `src/main.rs`

- [ ] Add `json: bool` to `doctor::run_doctor` signature.
- [ ] Add summary line after printing items in human-readable mode.
- [ ] In JSON mode: build a serde struct, serialize to stdout, skip line-by-line output.
- [ ] Add `json: bool` to `Command::Doctor` and thread through.

## Verification Commands

- `cargo test doctor`
- `cargo test`
- `cargo clippy`

## Expected Outcomes

- `dotman doctor` prints summary line at the end.
- `dotman doctor --json` prints valid JSON to stdout.
- Exit code unchanged.
- Existing doctor tests still pass.
