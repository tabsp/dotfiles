# Multi-Agent Review — P1 - Managed State Inventory And Status

## Gate 1: Design Review (spec → plan)

### Round 1

| Role | Agent / Reviewer | Status |
|------|------------------|--------|
| Safety / Release | Herschel | completed |
| Product / Community | Darwin | completed |
| Workflow / Harness | Hume | completed |

#### Safety / Release
- **Findings:**
  1. Tool diff must scope to DownloadBinary + OfficialScript only — P0 ownership model doesn't track Brew/System/Apt install paths. Spec/plan both assumed all deps have paths.
  2. No host detection in plan — will report Linux deps as missing on macOS. Add `detect_host()` + `entries_for_host()`.
  3. Version check failure undefined — need `version_unknown` fallback status.
  4. Version commands have no timeout (pre-existing risk, noted for CI).
  5. Dotfile "extra" detection: spec lists it, plan defers — clarify.
  6. `stale` items trigger exit 1 — document this.

- **Priority changes:** Elevate host detection and tool scope (P1). Elevate version-check error handling (P2).

- **Risks:** False missing reports without host filtering, hanging version commands in CI.

#### Product / Community
- **Findings:**
  1. `dotman diff` is the right name (separate from `dotman status`).
  2. Human output is scannable, follows conventions.
  3. JSON `detail` string should be structured (`installed_version`/`expected_version`).
  4. `wrong_target` should include `actual_target` detail.
  5. Deferred `extra` creates spec/plan mismatch — remove from scope or document.

- **Priority changes:** P1 — struct fields for version/target. P2 — `--quiet`, `--category`.

- **Risks:** `extra` status confusion, overlap with `dotman status` on backup reporting.

#### Workflow / Harness
- **Findings:**
  1. `status.rs` helpers are private — can't reuse without making `pub(crate)`.
  2. Roadmap still says `proposed`, state is `in_progress` — phase mismatch.
  3. Handoff file missing.
  4. Review doc had all roles "skipped" — now filled with real findings.

- **Priority changes:** Fix harness issues (blocking). Add `pub(crate)` step to plan.

- **Risks:** Private helpers cause mid-implementation refactoring.

#### Round 1 Synthesis

**Consensus:**
- Tool diff must scope to DownloadBinary + OfficialScript only.
- Host detection + `entries_for_host()` filtering required.
- Version check failure needs `version_unknown` fallback.
- JSON should use structured fields (not freeform strings).
- `extra` detection deferred to future iteration.
- `status.rs` helpers need `pub(crate)` access.

**Accepted changes:**
1. Scope tool diff to DownloadBinary/OfficialScript (Safety, Product, Workflow).
2. Add host detection + `entries_for_host()` filtering (Safety, Workflow).
3. Add `version_unknown` status for failed version checks (Safety).
4. Structured JSON: `installed_version`/`expected_version` for drifted, `actual_target` for wrong_target (Product).
5. Remove `extra` from this iteration's scope (all three).
6. Document exit 1 includes stale items (Safety, Product).
7. Make status.rs helpers `pub(crate)` (Workflow).
8. Fix harness issues: roadmap status, handoff (Workflow).

**Rejected suggestions:**
- `--quiet` and `--category` flags: defer to future iteration (out of scope).
- Version command timeout: note as known limitation, not blocking.

---

## Gate 2: Approach Review (spec + plan → implementation)

*To be filled.*

---

## Gate 3: Code Review (implementation → done)

*To be filled.*

## Risk Register Updates

| Risk | Linked Epic | Status |
|------|-------------|--------|
| Diff silently skips brew/system/apt deps | P1 - Managed State Inventory And Status | Mitigated: doc scoped to DownloadBinary/OfficialScript only |
| No host filtering causes false missing reports | P1 - Managed State Inventory And Status | Mitigated: host detection added to plan |
| Hanging version commands block CI | P1 - Managed State Inventory And Status | Accepted as known limitation |

## Coordinator Summary

Spec and plan updated with all accepted changes. Ready for implementation.
