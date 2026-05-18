# Handoff And Plan Quality Gates Implementation Plan

**Spec:** `docs/superpowers/specs/2026-05-18-p1-handoff-and-plan-quality-gates-design.md`

**Goal:** Strengthen plan and handoff quality gates for machine-state safety.

**Architecture:** Update plan template, add plan validation check, update
handoff template.

---

## Existing Code Map

- `docs/superpowers/agent/templates/plan.md`: current plan template.
- `docs/superpowers/agent/templates/handoff.md`: current handoff template.
- `src/agent.rs`: `validate_plan_structure()` at line 999, handoff validation.

## Task 1: Update plan template

**Files:**
- Modify: `docs/superpowers/agent/templates/plan.md`

- [ ] Add "## Machine State Safety" section to template with dry-run, failure-path, recovery, smoke check, non-destructive scope bullets.

## Task 2: Add plan validation check

**Files:**
- Modify: `src/agent.rs`

- [ ] Add `validate_plan_machine_state_safety()` function.
- [ ] If plan has task headings + checkboxes, require "## Machine State Safety".
- [ ] If plan has no tasks (docs-only), skip check.
- [ ] Add unit test: plan with tasks fails without machine state safety.
- [ ] Add unit test: docs-only plan skips machine state safety check.

## Task 3: Update handoff template

**Files:**
- Modify: `docs/superpowers/agent/templates/handoff.md`

- [ ] Add "## Machine State Verification" section.

## Verification Commands

- `cargo test agent` — existing + new agent tests pass.
- `cargo test` — all tests pass.
- `cargo clippy` — zero warnings.
- `make check` — manifest validation passes.

## Expected Outcomes

- Plans with tasks must include machine state safety section.
- Docs-only plans skip machine state safety check.
- Handoff template includes machine state verification.
- Existing agent tests pass.

## Test Level

- Unit tests in `src/agent.rs` for new validation logic.
- Existing agent tests for regression.

## Regression Coverage Expectations

- Existing plan and handoff validation unchanged.
- Existing plan template sections preserved.
