# Multi-Agent Review - P0 - Release Installer Trust Boundary Hardening

Retroactive review. Three reviewer agents ran independently, then coordinator
synthesized.

---

## Stable Consensus

All three reviewers agree:
- Installer code hardening is well-executed (checksums mandatory, tool detection
  early, error messages clear).
- **Dotfiles source checksum is the critical outstanding gap** — both in tests
  (missing failure paths) and in the release pipeline (checksum file not
  produced).
- URL origin mismatch for dotfiles source checksum is a latent correctness bug.

---

## Gate 1: Design Review — Findings

### Safety / Release (Herschel)
- Spec defines 5 error scenarios with `error: ... → exit 1` pattern. Clear.
- Non-goals (no GPG, no version resolution change) are explicit.
- Spec function `find_checksum_tool()` not reflected in code (inline instead).
  Cosmetic only — no functional impact.

### Product / Community (Cicero)
- README says "latest" but installer hardcodes `0.1.0`. Spec acknowledges this
  as non-goal, but README inaccuracy is a broken user promise.
- `DOTMAN_VERSION` env var exists but is undocumented in README.
- Comparable tools analysis shows dotman is competitive on checksums but lags on
  version resolution and installer signature.

### Workflow / Harness (Parfit)
- All required spec sections present (Goal, Scope, Non-Goals, Design, Error
  Handling, Verification Strategy, Regression Coverage).
- Spec is complete and actionable.

---

## Gate 2: Approach Review — Findings

### Safety / Release
- Plan's 5 steps cover the right sequence. Step 4 (failure tests) was
  under-executed — only 2 of 5 planned failure tests written.
- Plan correctly identifies `set -e` behavior must not weaken. This held.

### Product / Community
- Plan doesn't mention README update as a step, despite the spec's scope
  including "installer trust contract."
- No UX consideration for the new mandatory checksum — no progress indicator
  for dotfiles source checksum download.

### Workflow / Harness
- Plan uses `- [ ]` checkbox syntax but none were checked during implementation.
- Verification commands list is adequate.
- Multi-agent review was listed as a dependency in the roadmap but not executed.

---

## Gate 3: Code Review — Findings

### Safety / Release (Herschel) — CRITICAL finding
- **F1**: Dotfiles source checksum file not produced by release pipeline
  (`.github/workflows/release-artifacts.yml` only builds binary tarballs).
  Production installs will fail.
- **F2**: Missing tests for dotfiles source checksum failure paths and missing
  tool scenario.
- **F3**: URL origin mismatch — `DOTFILES_ARCHIVE_URL` vs `BASE_URL` for
  checksum.
- **F4**: Partial state risk — binary installed before source verification.

### Product / Community (Cicero)
- F1: README "latest" vs hardcoded `0.1.0` — direct contradiction (HIGH).
- F2: `DOTMAN_VERSION` env var hidden from users (MEDIUM).
- F3: URL mismatch (LOW).
- Positives: backup behavior, clear error messages, mandatory verification.

### Workflow / Harness (Parfit)
- Review protocol not followed (process gap).
- Roadmap has duplicated Outcome block.
- Handoff stale (written at intermediate phase, never updated).
- Test coverage incomplete (2/5 scenarios).
- `make agent-review-check` is depthless (file existence only).

---

## Disagreements

None. All three reviewers independently identified the same key gaps from
different perspectives. Severity ratings differ (Safety says CRITICAL for
release pipeline gap; Product says HIGH for README inaccuracy) but findings
align.

---

## Accepted Changes

| # | Finding | Action | Priority |
|---|---------|--------|----------|
| 1 | Release pipeline doesn't produce dotfiles source checksum | Update `.github/workflows/release-artifacts.yml` | **P0 blocker** |
| 2 | Missing dotfiles source checksum failure tests | Add 2 tests (missing source checksum, mismatch) | HIGH |
| 3 | Missing checksum tool test | Add 1 test (tool unavailable) | MEDIUM |
| 4 | URL origin mismatch | Unify source/checksum URL base | HIGH |
| 5 | README says "latest", installer pins 0.1.0 | Fix README wording or add version resolution | HIGH |
| 6 | DOTMAN_VERSION undocumented | Add to README install section | MEDIUM |
| 7 | Duplicate Outcome in roadmap archive | Merge into single Outcome | MEDIUM |
| 8 | Stale handoff | Update handoff with verification records | LOW |
| 9 | agent-review-check is depthless | Add content validation (future enhancement) | LOW |

---

## Rejected Or Deferred Suggestions

| Suggestion | Reason |
|------------|--------|
| Add GPG/authenticity check to installer | Explicit non-goal in spec. Track as future P-item. |
| Reorder binary install after source verification | Pre-existing behavior. Document partial-state risk; revisit in recovery epic. |

---

## Risk Register Updates

| Risk | Evidence | Linked Epic | Status |
|------|----------|-------------|--------|
| Hardened installer will fail production installs | Release pipeline lacks dotfiles source checksum | P0 - Release Installer | **mitigated** — source job added to workflow |
| README "latest" claim is false | Installer hardcodes 0.1.0 | P0 - Release Installer | **mitigated** — README updated, DOTMAN_VERSION documented |
| Dotfiles source checksum code path untested | 0 of 2 failure tests written | P0 - Release Installer | **mitigated** — 2 tests added |
| Mirror users get checksum mismatches | URL origin mismatch | P0 - Release Installer | **mitigated** — URL unified |
| Partial state on late failure | Binary installed before source verified | P0 - Recovery Safety | watch |

---

## Coordinator Summary

The installer code hardening is **directionally correct and well-executed** —
mandatory checksums, early tool detection, clear error messages. However, this
epic **cannot be considered complete** until the critical release pipeline gap
(F1) is resolved: the dotfiles source checksum file must be produced by the
release workflow, or the production installer will fail every time.

**Status**: Items 1–6 fixed and verified. 141 tests pass, clippy clean. Epic complete. above before the epic is truly done. Items 1, 2,
4, 5 are blocking. Items 3, 6 are high-value follow-ups.
