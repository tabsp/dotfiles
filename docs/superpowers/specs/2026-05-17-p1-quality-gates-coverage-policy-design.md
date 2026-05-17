# Quality Gates And Coverage Policy Design

## Goal

Define explicit quality expectations per roadmap epic: test level, required
verification commands, and regression coverage. Enforce these through template
requirements and agent-check validation.

## Scope

- Add `## Test Level` and `## Regression Coverage Expectations` sections to
  the plan template.
- Add `## Regression Coverage Expectations` section to the spec template.
- Update `agent-check` plan validation to require the new sections.
- Update `agent-check` spec validation to require the new section.
- Existing specs and plans are not backfilled (they will be updated as each
  epic is revisited).

## Non-Goals

- Do not add code coverage tooling or measurement infrastructure.
- Do not backfill existing specs and plans.
- Do not change how tests are run or what `cargo test` validates.

## Design

### Template changes

**Spec template** (`docs/superpowers/agent/templates/spec.md`):
Add after `## Verification Strategy`:

```
## Regression Coverage Expectations

- Which behaviors must NOT regress (e.g., "path traversal rejection",
  "checksum mismatch detection").
```

**Plan template** (`docs/superpowers/agent/templates/plan.md`):
Add after `## Verification Commands`:

```
## Test Level

- Unit tests: `cargo test <module>`
- Integration tests: `cargo test --test <name>`
- Manual smoke test: `cargo run -- <command>`

## Regression Coverage Expectations

- Behaviors that must remain passing.
```

### agent-check validation

Add to `validate_spec_structure`:
- Require `## Regression Coverage Expectations` heading.

Add to `validate_plan_structure`:
- Require `## Test Level` heading.
- Require `## Regression Coverage Expectations` heading.

### Error codes

- `AGENT_MISSING_SPEC_SECTION: missing required section ## Regression Coverage Expectations`
- `AGENT_MISSING_PLAN_SECTION: missing ## Test Level`
- `AGENT_MISSING_PLAN_SECTION: missing ## Regression Coverage Expectations`

## Error Handling

- Missing sections fail `agent-check` with clear error codes.
- Empty sections (heading present but no content) are not enforced — the
  harness validates presence of the heading, not content quality.

## Verification Strategy

- `cargo test agent` — agent module tests pass
- `cargo test` — full suite
- `cargo clippy` — zero warnings
