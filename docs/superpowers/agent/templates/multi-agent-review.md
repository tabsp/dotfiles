# Multi-Agent Review - {{EPIC}}

## Protocol Rules

This review follows the Multi-Agent Review Protocol. Reviewer agents are
**read-only specialists** with isolated context. They gather evidence and
discover risks, but do not vote. The coordinator is responsible for final
judgment.

- Reviewers must not modify files.
- Reviewers operate in isolated context (no shared state with other reviewers).
- Each reviewer fills only their assigned role section.
- The coordinator synthesizes all reviewer outputs, resolves disagreements,
  and presents findings to the user for confirmation before any file edits.
- Reviews may run multiple rounds per gate. Record each round in the
  corresponding section below.

## Reviewer Roles

Three fixed roles, each with read-only constraints and isolated context:

| Role | Focus | Constraints |
|------|-------|-------------|
| Safety / Release | Trust boundaries, machine state, recovery, release risk, install/uninstall paths, archive extraction, symlink safety | Read-only; reviews security and machine-state impact only |
| Product / Community | User value, comparable tools, productization timing, documentation quality, community expectations | Read-only; reviews user-facing impact and product direction only |
| Workflow / Harness | Roadmap scheduling, specs/plans, handoffs, agent process, CI, verification completeness | Read-only; reviews process and agent workflow quality only |

---

## Gate 1: Design Review (spec → plan)

Review the spec document before the plan is written.

### Round 1

| Role | Agent / Reviewer | Status |
|------|------------------|--------|
| Safety / Release | | pending |
| Product / Community | | pending |
| Workflow / Harness | | pending |

#### Safety / Release
- Findings:
- Priority changes:
- Risks:

#### Product / Community
- Findings:
- Priority changes:
- Risks:

#### Workflow / Harness
- Findings:
- Priority changes:
- Risks:

#### Round 1 Synthesis
- Consensus:
- Disagreements:
- Accepted changes:
- Rejected suggestions:

### Round 2 (if needed)
...

---

## Gate 2: Approach Review (plan → implementation)

Review the spec + plan before code is written.

### Round 1

| Role | Agent / Reviewer | Status |
|------|------------------|--------|
| Safety / Release | | pending |
| Product / Community | | pending |
| Workflow / Harness | | pending |

#### Safety / Release
- Findings:
- Priority changes:
- Risks:

#### Product / Community
- Findings:
- Priority changes:
- Risks:

#### Workflow / Harness
- Findings:
- Priority changes:
- Risks:

#### Round 1 Synthesis
- Consensus:
- Disagreements:
- Accepted changes:
- Rejected suggestions:

### Round 2 (if needed)
...

---

## Gate 3: Code Review (implementation → done)

Review the git diff, code changes, and test coverage before finishing.

### Round 1

| Role | Agent / Reviewer | Status |
|------|------------------|--------|
| Safety / Release | | pending |
| Product / Community | | pending |
| Workflow / Harness | | pending |

#### Safety / Release
- Code review findings:
- Error handling coverage:
- Test coverage gaps:
- Risks:

#### Product / Community
- Code review findings:
- UX / API surface:
- Documentation changes:
- Risks:

#### Workflow / Harness
- Code review findings:
- CI / verification impact:
- Handoff completeness:
- Risks:

#### Round 1 Synthesis
- Consensus:
- Disagreements:
- Accepted changes:
- Rejected suggestions:

### Round 2 (if needed)
...

---

## Risk Register Updates

| Risk | Evidence | Linked Epic | Proposed Status |
|------|----------|-------------|-----------------|

## Coordinator Summary

Final judgment after synthesizing all gate reviews. Must include:

1. Summary of consensus findings across all gates.
2. Summary of resolved disagreements with rationale.
3. List of accepted changes.
4. List of rejected suggestions with reasons.
5. Updated risk register entries.
6. Next action (start next gate, revise artifacts, or proceed to finish).
