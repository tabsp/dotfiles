# Multi-Agent Review Protocol Design

## Goal

Formalize the multi-agent review pattern used in the roadmap planning cycle into
a documented, reusable protocol. All roadmap epics (P0, P1, P2, P3) and roadmap refreshes must
pass a structured multi-agent review before implementation begins. The harness enforces this for every roadmap work item via `AGENT_REVIEW_MISSING` checks on advance to `in_progress` and `verifying` phases.

## Scope

- Document the three reviewer roles: Safety/Release, Product/Community,
  Workflow/Harness.
- Define read-only constraints for reviewer agents.
- Specify structured output format (findings, priority changes, risks).
- Define coordinator synthesis rules (consensus, disagreements, accepted and
  rejected changes, risk register updates).
- Update the multi-agent review template at
  `docs/superpowers/agent/templates/multi-agent-review.md`.
- Document when multi-agent review is required (trigger criteria).
- Add a section to `docs/superpowers/agent/templates/roadmap-review.md`
  referencing the multi-agent review protocol.

## Non-Goals

- Building a new orchestration framework or agent-to-agent communication layer.
- Running reviewer agents in production automatically without human oversight.
- Creating new MCP tools or plugin infrastructure.
- Enforcing review gates programmatically in the agent harness.


## Design

### Reviewer Roles

| Role | Focus | Constraints |
|------|-------|------------|
| Safety / Release | Trust boundaries, machine state, recovery, release risk | Read-only; isolated context |
| Product / Community | User value, comparable tools, productization timing | Read-only; isolated context |
| Workflow / Harness | Roadmap scheduling, specs/plans, handoffs, agent process | Read-only; isolated context |

### Trigger Criteria

Multi-agent review is required for:

- All roadmap epics (P0, P1, P2, P3) across all three gates (design, approach, code).
- Roadmap refreshes (Roadmap Planning Review).
- Any epic that modifies agent harness safety gates.

P1/P2/P3 epics may use lighter review rounds (fewer findings, shorter synthesis)
but must still produce a review document with all three roles addressed.

### Review Protocol

1. **Coordinator** spawns three reviewer agents, each with the review template
   and the target spec/plan/roadmap.
2. Each **reviewer** fills their section of the template with findings,
   priority changes, and risks. Reviewers are read-only and must not modify
   files.
3. **Coordinator** collects outputs and synthesizes:
   - **Stable Consensus**: findings all reviewers agree on.
   - **Disagreements**: conflicting views with coordinator's decision and
     rationale.
   - **Accepted Changes**: what will be incorporated.
   - **Rejected Suggestions**: what was considered but not adopted, with
     reasons.
   - **Risk Register Updates**: new or modified risks.
4. **Coordinator** writes the final review document and presents findings to
   the user for confirmation before any file edits.

### Template Updates

The existing `multi-agent-review.md` template already has the skeleton. It
needs:
- Clearer role descriptions.
- Explicit read-only constraint language.
- Coordinator synthesis instructions.

## Error Handling

- If a reviewer agent produces incomplete output, coordinator requests
  resubmission or fills gaps manually.
- If consensus cannot be reached, coordinator documents the disagreement and
  makes a final call with rationale.
- If the user rejects the review findings, the cycle restarts with adjusted
  scope.

## Verification Strategy

- `make agent-check` passes for the current epic.
- Review template renders correctly when filled by hand.
- AGENTS.md references the protocol under the Multi-Agent Review section.

## Regression Coverage Expectations

- The existing review template at
  `docs/superpowers/agent/templates/multi-agent-review.md` must not lose
  existing sections.
- AGENTS.md must continue to reference multi-agent review for safety-sensitive
  epics.
- The protocol must remain documentation-only; no new harness enforcement
  should prevent existing workflows.
