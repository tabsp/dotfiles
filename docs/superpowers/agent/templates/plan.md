# {{EPIC}} Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** `{{SPEC_PATH}}`

**Goal:** Describe the concrete implementation outcome.

**Architecture:** Describe the implementation approach.

**Tech Stack:** Rust 2024, Makefile, existing project test stack.

---

## Existing Code Map

- Describe relevant files.

## Placeholder Task: First Verifiable Change

**Files:**
- Modify: `path/to/file`
- Test: `tests/path.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn names_expected_behavior() {
    assert!(true);
}
```

## Verification Commands

- `cargo test <test_name>`
- `cargo test`
- `make check`

## Test Level

- Unit tests: `cargo test <module>`
- Integration tests: `cargo test --test <name>`
- Manual smoke test: `cargo run -- <command>`

## Regression Coverage Expectations

- Behaviors that must remain passing.

## Expected Outcomes

- Describe the observable state after the plan is implemented.

## Machine State Safety

> **Required for implementation plans.** Skip this section for documentation-only
> or governance epics with no code or machine state changes.

- **Dry-run / preview path:** Describe how to preview changes without applying
  (e.g., `--dry-run`, `DRY_RUN=1`, `dotman status`).
- **Failure-path tests:** List specific tests that cover failure scenarios
  (e.g., permission errors, missing files, partial state).
- **Recovery notes:** Describe rollback or recovery steps if something goes
  wrong (e.g., restore from `.dotman-backup`, re-run bootstrap).
- **Manual smoke checks:** List manual verification steps beyond automated
  tests (e.g., `dotman cleanup` on a clean system, `make smoke-test`).
- **Non-destructive scope:** Confirm no destructive operations run without
  explicit confirmation or `--execute` flag.

## Machine State Safety

> **Required for implementation plans.** Skip this section for documentation-only
> or governance epics with no code or machine state changes.

- **Dry-run / preview path:** Describe how to preview changes without applying
  (e.g., `--dry-run`, `DRY_RUN=1`, `dotman status`).
- **Failure-path tests:** List specific tests that cover failure scenarios
  (e.g., permission errors, missing files, partial state).
- **Recovery notes:** Describe rollback or recovery steps if something goes
  wrong (e.g., restore from `.dotman-backup`, re-run bootstrap).
- **Manual smoke checks:** List manual verification steps beyond automated
  tests (e.g., `dotman cleanup` on a clean system, `make smoke-test`).
- **Non-destructive scope:** Confirm no destructive operations run without
  explicit confirmation or `--execute` flag.
