# Roadmap Refresh And Agent Queue Reset Design

## Goal

Archive completed foundation work, rebuild the Next Queue with only actionable
future items, add a risk register for cross-cutting concerns, and ensure the
agent harness can schedule new epics after queue exhaustion.

## Scope

- Move all `done` Next Queue items to a `Completed Foundation` archive section.
- Populate a fresh Next Queue with only `proposed` future work, ordered by
  priority and dependency.
- Add a `Risk Register` section with residual risks and cross-cutting concerns.
- Update `Agent Scheduling Rules` to document queue exhaustion behavior.
- Fix the parser in `src/agent.rs` to recognize `## Next Queue` (done pre-spec).

## Non-Goals

- Starting implementation of any next-phase epic.
- Modifying the agent harness workflow beyond the parser fix.
- Changing `dotman` runtime behavior.
- Updating README, install scripts, or CI configuration.
- Adding new roadmap items beyond what the Planning Review proposed.

## Design

### Roadmap structure after refresh

```
## Direction
## Priority Rules
## Status Values
## Agent Scheduling Rules
## Next Queue
  ### P0 - Multi-Agent Review Protocol (proposed)
  ### P0 - Release Installer Trust Boundary Hardening (proposed)
  ### P1 - ...
  ### P2 - ...
## Risk Register
  | Risk | Evidence | Suggested Owner | Status |
## Deferred / Non-Goals
## Completed Foundation
  | Priority | Epic | Category | Outcome | Handoff |
## Roadmap Planning Review
```

### Completed Foundation format

Table with columns: Priority, Epic, Category, Outcome summary, Handoff path.

### Risk Register format

Table with columns: Risk, Evidence, Suggested Owner, Status.

### Next Queue population

Based on Roadmap Planning Review 2026-05-18:

- P0 - Multi-Agent Review Protocol
- P0 - Release Installer Trust Boundary Hardening
- P1 - Recovery Ownership Inventory
- P1 - Config Diff And State Compare
- P1 - Error Context Propagation
- P1 - Pipeline Dry-Run And Preview
- P2 - Manifest Schema Migration Tool
- P2 - User Config Defaults

## Error Handling

- If a handoff link is missing for a `done` item, list it with `(missing)`.
- If the spec/plan paths are malformed in the roadmap, the parser will flag
  this at `make agent-check` time.
- The Roadmap Planning Review section documents the escape hatch for queue
  exhaustion; no runtime error here.

## Verification Strategy

- `make agent-next` returns the first P0 proposed item.
- `make agent-start` can lock a proposed item.
- `make agent-check` passes for the locked epic.
- `cargo test agent` passes (parser tests).
- `git diff docs/roadmap.md` shows only structural changes, no content loss.

## Regression Coverage Expectations

- The `## Next Queue` parser must still parse `## Active Queue` for backward
  compatibility with older roadmap formats.
- `make agent-next` must never return a `done` or `deferred` item.
- No completed epic handoff links or verification records may be dropped.
- The risk register must not duplicate full handoff content.
