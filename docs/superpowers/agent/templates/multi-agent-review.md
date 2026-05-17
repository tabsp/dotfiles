# Multi-Agent Review - YYYY-MM-DD

## Protocol Rules

This review follows the Multi-Agent Review Protocol. Reviewers are **read-only
specialists** with isolated context. They gather evidence and discover risks,
but do not vote. The coordinator is responsible for final judgment.

- Reviewers must not modify files.
- Reviewers operate in isolated context (no shared state with other reviewers).
- Each reviewer fills only their assigned role section.
- The coordinator synthesizes all reviewer outputs, resolves disagreements,
  and presents findings to the user for confirmation before any file edits.

## Trigger

Why multi-agent review is required for this work. Reference the trigger
criteria (roadmap refresh, safety-sensitive P0 epic, agent harness safety gate
change).

## Scope

Files, docs, epics, or subsystems reviewed.

## Reviewer Roles

Three fixed roles, each with read-only constraints and isolated context:

| Role | Focus | Constraints |
|------|-------|-------------|
| Safety / Release | Trust boundaries, machine state, recovery, release risk, install/uninstall paths, archive extraction, symlink safety | Read-only; reviews security and machine-state impact only |
| Product / Community | User value, comparable tools, productization timing, documentation quality, community expectations | Read-only; reviews user-facing impact and product direction only |
| Workflow / Harness | Roadmap scheduling, specs/plans, handoffs, agent process, CI, verification completeness | Read-only; reviews process and agent workflow quality only |

| Role | Agent / Reviewer | Status |
|------|------------------|--------|
| Safety / Release | | pending |
| Product / Community | | pending |
| Workflow / Harness | | pending |

## Reviewer Findings

### Safety / Release

- Findings:
- Priority changes:
- Risks:

### Product / Community

- Findings:
- Priority changes:
- Risks:

### Workflow / Harness

- Findings:
- Priority changes:
- Risks:

## Stable Consensus

What all or most reviewers agree on. If no consensus is reached on a point,
list it under Disagreements instead.

## Disagreements

| Topic | Positions | Coordinator decision | Rationale |
|-------|-----------|----------------------|-----------|

The coordinator resolves disagreements with explicit rationale. Disagreement
resolution is not a compromise; the coordinator chooses the best path based on
project priorities and safety rules.

## Accepted Roadmap Changes

- Change and reason.

## Rejected Or Deferred Suggestions

- Suggestion and reason for rejection/deferral.

## Risk Register Updates

| Risk | Evidence | Linked Epic | Proposed Status |
|------|----------|-------------|-----------------|

New risks discovered during review. Existing risks that should be updated.

## Coordinator Summary

Final judgment after synthesizing reviewer outputs. Must include:

1. Summary of consensus findings.
2. Summary of resolved disagreements with rationale.
3. List of accepted changes.
4. List of rejected suggestions with reasons.
5. Updated risk register entries.
6. Next action (start implementation, revise spec, or request re-review).
