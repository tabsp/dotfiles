# Handoff And Plan Quality Gates Design

## Goal

Strengthen plan and handoff quality gates so that machine-state-changing epics
explicitly document safety requirements, and handoffs become reliable audit
artifacts.

## Scope

- Add "## Machine State Safety" section to the plan template, required for
  epics that include tasks (machine-state or code changes).
- Add plan validation check for machine-state safety section presence.
- Update handoff template to include machine-state verification evidence.
- Fix stale phase handling in handoff content.

## Non-Goals

- Do not add a work-kind classification system (machine-state vs docs).
- Do not change the existing required plan sections.
- Do not break existing handoff validation.

## Design

### Plan Template: Machine State Safety

New required section in the plan template:

```
## Machine State Safety

- **Dry-run / preview path:** Describe how to preview changes without applying.
- **Failure-path tests:** List tests that cover failure scenarios.
- **Recovery notes:** Describe rollback or recovery steps if something goes wrong.
- **Manual smoke checks:** List manual verification steps.
- **Non-destructive scope:** Confirm no destructive operations run without confirmation.
```

This section is required when the plan contains at least one task heading
(`## Task` or `### Task`) and checkbox items, which indicates it's an
implementation plan rather than a documentation-only plan.

### Plan Validation

Add `validate_plan_machine_state_safety()` to agent checks:

- If plan has task headings + checkboxes → require "## Machine State Safety"
  section.
- If plan has no tasks (docs-only) → skip this check.
- Fire `AGENT_MISSING_PLAN_SECTION: missing ## Machine State Safety` if required
  but absent.

### Handoff Template

Add "## Machine State Verification" section:

```
## Machine State Verification

- Dry-run tested: yes/no
- Failure paths covered: yes/no
- Recovery notes documented: yes/no
- Manual smoke checks passed: yes/no
```

### Stale Phase Handling

The handoff `Phase` section is rendered from the template with `{{PHASE}}`.
When phase advances, the handoff content becomes stale. The `validate_handoff`
function already checks phase mismatch — ensure this is consistently enforced
before finish.

### Verification Strategy

- `cargo test agent` — existing agent tests pass.
- Add new test: plan with tasks requires machine state safety section.
- Add new test: docs-only plan skips machine state safety check.
- `cargo clippy` — zero warnings.
- `make check` — manifest validation passes.

### Error Handling

- `AGENT_MISSING_PLAN_SECTION` for missing machine state safety.
- No change to existing error codes.

### Regression Coverage Expectations

- Existing plan and handoff validation unchanged.
- Existing agent tests pass.
