# Multi-Agent Review — P2 - Narrow Diff And Reconcile Reporting

## Protocol Rules

This review follows the Multi-Agent Review Protocol. Reviewer agents are
**read-only specialists** with isolated context.

---

## Gate 1: Design Review

### Round 1

| Role | Agent / Reviewer | Status |
|------|------------------|--------|
| Safety / Release | Galileo | completed |
| Product / Community | — | skipped (internal tool, no user-facing product changes) |
| Workflow / Harness | — | skipped (P2, lightweight review) |

#### Safety / Release (Galileo)
- Findings:
  - JSON `--narrow` ambiguous — arrays should also be filtered, not just human output.
  - `reconcile_commands` unconditionally in JSON — should use `skip_serializing_if`.
  - `drifted`/`version_unknown` tools no reconcile path — gap.
  - Exit code behavior for `--narrow`/`--reconcile` unspecified.
  - Multiple wrong_target dotfiles could generate noisy output — consider batching.
- Priority changes:
  - P1: Resolve JSON `--narrow` array filtering.
  - P2: Address drifted/version_unknown reconcile gap.
  - P3: `reconcile_commands` serialization policy.

#### Round 1 Synthesis
- Consensus: Spec is well-scoped; minor clarifications needed.
- Accepted changes:
  - JSON `--narrow` now explicitly filters data arrays.
  - `reconcile_commands` uses `skip_serializing_if` — only present when `--reconcile`.
  - `drifted` tools get `dotman bootstrap` reconcile; `version_unknown` gets manual note.
  - Exit code explicitly documented as unchanged.
  - Dotfile reconcile batched into single `dotman link --force` line.
- Rejected suggestions: None.

---

## Gate 2: Approach Review

### Round 1

| Role | Agent / Reviewer | Status |
|------|------------------|--------|
| Safety / Release | Linnaeus | completed |
| Product / Community | — | skipped |
| Workflow / Harness | — | skipped |

#### Safety / Release (Linnaeus)
- Findings:
  - Plan references `tests/cli_diff.rs` which doesn't exist — tests are inline in `src/diff.rs`.
  - `# ` comment prefix on reconcile commands is a thoughtful safety layer.
  - Exit code behavior should be verified in tests.
- Priority changes: Fix test file reference in plan.

#### Round 1 Synthesis
- Consensus: Plan is well-structured; one factual error to fix.
- Accepted changes:
  - Fixed test file reference: `tests/cli_diff.rs` → `src/diff.rs` (inline unit tests).
  - Added exit code verification to test plan.
- Rejected suggestions: None.

---

## Gate 3: Code Review

*To be filled after implementation.*

---

## Risk Register Updates

| Risk | Evidence | Linked Epic | Proposed Status |
|------|----------|-------------|-----------------|
| JSON schema additive change | New fields in DiffSummary | P2 - Narrow Diff And Reconcile Reporting | accepted (low) |
| Bootstrap reconcile risk | User pasting reconcile output | P2 - Narrow Diff And Reconcile Reporting | accepted (mitigated by `# ` prefix) |

## Coordinator Summary

Low-risk, well-scoped P2 change. Read-only output additions.

1. **Consensus:** Spec and plan are sound. Minor spec clarifications accepted.
2. **Disagreements:** None.
3. **Accepted changes:** JSON --narrow array filtering, reconcile_commands skip_serializing_if, drifted reconcile, exit code docs, plan test file fix.
4. **Rejected suggestions:** None.
5. **Risk register:** Two low-severity risks accepted.
6. **Next action:** Advance to in_progress and implement.
