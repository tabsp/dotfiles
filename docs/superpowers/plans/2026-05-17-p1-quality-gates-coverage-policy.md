# Quality Gates And Coverage Policy Implementation Plan

**Spec:** `docs/superpowers/specs/2026-05-17-p1-quality-gates-coverage-policy-design.md`

**Goal:** Add Test Level and Regression Coverage Expectations sections to spec/plan templates, and enforce them in agent-check.

**Architecture:** Update template Markdown files, extend agent-check validation in `src/agent.rs`.

**Tech Stack:** Rust 2024, existing agent module.

---

## Existing Code Map

- `docs/superpowers/agent/templates/spec.md`: spec template.
- `docs/superpowers/agent/templates/plan.md`: plan template.
- `src/agent.rs:940-979` (`validate_spec_structure`): spec heading validation.
- `src/agent.rs:980-1060` (`validate_plan_structure`): plan heading validation.

## Task 1: Update templates and agent-check validation

**Files:**
- Modify: `docs/superpowers/agent/templates/spec.md`
- Modify: `docs/superpowers/agent/templates/plan.md`
- Modify: `src/agent.rs`

- [ ] Add `## Regression Coverage Expectations` to spec template.
- [ ] Add `## Test Level` and `## Regression Coverage Expectations` to plan template.
- [ ] Add new required headings to `validate_spec_structure`.
- [ ] Add new required headings to `validate_plan_structure`.

## Verification Commands

- `cargo test agent`
- `cargo test`
- `cargo clippy`

## Test Level

- Unit tests: `cargo test agent`
- Integration tests: `cargo test --test cli_agent`

## Regression Coverage Expectations

- `agent-check` still rejects specs/plans missing core sections (Goal, Scope, etc.).
- Template rendering produces valid Markdown.
