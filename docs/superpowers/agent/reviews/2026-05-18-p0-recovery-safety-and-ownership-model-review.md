# Multi-Agent Review — P0 - Recovery Safety And Ownership Model

## Gate 1: Design Review (spec → plan)

### Round 1

| Role | Agent / Reviewer | Status |
|------|------------------|--------|
| Safety / Release | Heisenberg | completed |
| Product / Community | Nash | completed |
| Workflow / Harness | Mendel | completed |

#### Safety / Release

- **Findings:**
  1. Ownership inference is too weak for binary installs (`download_binary` without `install_dir_to`, `official_script`): the test "file exists at declared path" means any file at `~/.local/bin/rg` is claimed as owned. A sentinel file (e.g., `~/.local/share/dotman/installed/<tool>`) written during install would be stronger.
  2. Directory-symlink ownership test hardcodes `$HOME/.local/share/dotman/...` but `install_dir_to` is configurable via TOML. The spec and code must agree.
  3. Cleanup scan for link-conflict backups says "near linked dotfile targets" without bounds. Link.rs `unique_backup_path` places backups in the target's parent directory. Should be explicit: scan `parent_dir(target)` for each linked dotfile target.
  4. Backup naming pattern is imprecise. Actual pattern is `{target}.backup.{YYYYMMDDHHmmss}`, not `*.backup.<ts>`.
  5. Full Uninstall still requires manual `rm` — the inspect-first model is advisory, not enforced. Tedious manual removal of 20+ paths invites broad `rm` patterns again.
  6. `make uninstall` (Makefile:112) only removes `target/debug/dotman` and `target/release/dotman`. Release-installed binary at `~/.local/bin/dotman` is not handled.
  7. Source checkout ownership test is "directory present" — could match non-git or stale directories.
  8. `--json` output schema is not defined — shipping `--json` without a schema creates an implicit API contract.

- **Priority changes:**
  - Elevate: define scan scope for link-conflict backups explicitly.
  - Elevate: tighten binary install ownership test (at minimum, document the limitation).
  - Elevate: define `--json` schema before implementation.

- **Risks (new):**
  - False ownership for binaries not installed by dotman.
  - Unbounded cleanup scan for link-conflict backups.
  - Manual rm residual risk in Full Uninstall (users may bypass inspect-first).
  - `make uninstall` doesn't remove release binary.
  - Directory-symlink ownership test may disagree with `install_dir_to` paths.
  - JSON output ships without schema contract.

#### Product / Community

- **Findings:**
  1. `dotman status` naming aligns with ecosystem (`chezmoi status`, `yadm status`). Good UX choice.
  2. `--json` is well-motivated but underspecified — no schema defined.
  3. "Plausibly installed" is a fuzzy ownership test. Users from `brew`/`nix` expect precision. Should label ownership certainty: `(managed)` vs `(detected)`.
  4. Repo-required constraint for `dotman status` undermines recovery UX — managed state exists on machine even without the repo. Consider detecting `~/.local/share/dotman/dotfiles`.
  5. Documentation changes are appropriately scoped. Inspect-first section is clear.
  6. Missing in-tool prompts: `dotman cleanup` should suggest `dotman status` first.

- **Priority changes:**
  - Elevate: JSON schema definition.
  - Elevate: "Plausibly owned" UX clarity with tiered labels.

- **Risks:**
  - False-positives in ownership detection erode trust.
  - Repo-required constraint undermines recovery UX.
  - Missing community-facing install UX (in-tool discovery prompts).

#### Workflow / Harness

- **Findings:**
  1. Spec structure is complete — sufficient to write a plan from.
  2. Scope is well-calibrated: layers `dotman status` + ownership model atop P3 cleanup without expanding into deferred territory.
  3. Roadmap alignment is tight — addresses both risk-register items and all three P0 outcome components.
  4. P3 references are correct and properly positioned as prior art.
  5. Verification commands are underspecified: `make ci` is missing, no `cargo test status` or `cargo test recovery` for targeted regression.
  6. Plan and review artifacts don't exist yet (normal at this gate).
  7. P3 handoff has minor formatting artifacts.

- **Priority changes:** None.

- **Risks:**
  - Plan dependency gap: spec doesn't identify specific Rust modules.
  - Pre-existing test failure (1 failure) is not acknowledged.
  - Manual-only verification for status subcommand (no integration tests).

#### Round 1 Synthesis

**Consensus:**
- Spec direction is correct and addresses both risk-register items.
- JSON schema must be defined before implementation.
- Ownership inference is too weak for binary installs — needs tiered labels at minimum.
- `dotman status` repo-required constraint is a UX gap for recovery scenarios.
- Cleanup scan scope for link-conflict backups needs explicit bounds.

**Accepted changes:**
1. Add minimal JSON schema to spec (top-level sections, items with name/path/kind).
2. Add ownership certainty tiers: `(managed)` for verifiable ownership, `(detected)` for existence-only.
3. Precise scan scope for link backups: scan `parent_dir(target)` per `dotfiles.toml` entry.
4. Correct backup naming pattern to match `src/link.rs` actual: `*.backup.{YYYYMMDDHHmmss}`.
5. Document known limitations: weak binary ownership, repo-required constraint, manual rm residual risk.
6. Add `make uninstall` handling for release-installed binary at `~/.local/bin/dotman`.
7. Add `make ci` to verification commands in spec.
8. Note that `dotman cleanup` should suggest running `dotman status` first.

**Rejected suggestions:** None — all feedback is aligned and actionable.

---

## Risk Register Updates

| Risk | Evidence | Linked Epic | Proposed Status |
|------|----------|-------------|-----------------|
| Recovery guidance encourages broad deletion outside known managed state. | `docs/recovery.md` Full Uninstall includes `rm "$HOME/.local/bin/"*`. | P0 - Recovery Safety And Ownership Model | Addressed by spec (remove broad deletion) |
| Cleanup and uninstall cannot be made safely automatic without ownership inventory. | Cleanup only scans `*.dotman-backup`/`*.dotman-staging`. | P0 - Recovery Safety And Ownership Model | Addressed by spec (`dotman status` ownership model) |
| `dotman status` claims ownership of binaries not installed by dotman (false ownership). | Ownership test for binary-only installers is file existence. | P0 - Recovery Safety And Ownership Model | new / open |
| Cleanup scan for link-conflict backups is unbounded. | Spec says "near linked dotfile targets" without explicit scan bounds. | P0 - Recovery Safety And Ownership Model | new / open |
| `make uninstall` does not remove the release-installed binary. | Makefile:112 only removes `target/*/dotman`. | P0 - Recovery Safety And Ownership Model | new / open |
| JSON output ships without schema. | Spec mentions `--json` with no schema definition. | P0 - Recovery Safety And Ownership Model | new / open |

## Coordinator Summary

All three reviewers confirm the spec's direction is correct: inspect-first model, read-only ownership inventory, and dangerous-broad-deletion removal. The accepted changes make the spec more precise without expanding scope. The plan phase can proceed after incorporating the accepted changes into the spec.

---

## Gate 2: Approach Review (spec + plan → implementation)

### Round 1

| Role | Agent / Reviewer | Status |
|------|------------------|--------|
| Safety / Release | Archimedes | completed |
| Product / Community | Ramanujan | completed |
| Workflow / Harness | Kepler | completed |

#### Safety / Release
- **Findings:**
  1. HIGH: `install_archive_dir` missing from plan's installer enumeration — tools installed via directory-symlink would not appear in status. Add to Task 1.
  2. MEDIUM: No path normalization for symlink matching — `read_link` returns absolute paths, `install_dir_to` may be relative.
  3. LOW: `*.backup.*` glob too broad — constrain to `*.backup.[0-9]*`.
  4. LOW: Error-path test coverage not specified in plan.
  5. `make uninstall` extension is safe ✓

- **Priority changes:** Elevate findings 1 and 2 (blockers for in_progress).

- **Risks:** False inventory from missing installer; false negatives from unnormalized paths.

#### Product / Community
- **Findings:**
  1. Plan faithfully maps spec UX to code — ownership tiers, JSON schema, inspect-first all covered.
  2. Module split (status.rs + recovery.rs) is sensible.
  3. Gap: plan misses "nothing to clean up → suggest `dotman status`" prompt.
  4. Error message for "not in repo" should mention `~/.local/share/dotman/dotfiles` fallback.
  5. Non-dotman installers silently omitted — correct, but note in plan.

- **Priority changes:** Two minor amendments (cleanup prompt, error message fallback).

- **Risks:** Pattern collision with `*.backup.*` in `~/.config/` (low, acceptable).

#### Workflow / Harness
- **Findings:**
  1. Plan is complete and implementable.
  2. Module structure follows existing patterns (doctor.rs as analog).
  3. Missing explicit `mod status;` in checklist.
  4. Verification commands sufficient and correctly scoped.
  5. Pre-existing test failure should be noted in plan.
  6. No integration tests (consistent with project pattern).

- **Priority changes:** Three low polish items (task ordering, mod declaration, pre-existing failure note).

- **Risks:** Implicit task ordering (low), pre-existing failure confusion (low).

#### Round 1 Synthesis

**Consensus:** Plan is ready for implementation. No blocking gaps. All three reviewers confirm the module structure, verification strategy, and spec-to-plan mapping are sound.

**Accepted changes:**
1. Add `install_archive_dir` to Task 1 installer enumeration (Safety HIGH).
2. Add path canonicalization to symlink matching (Safety MEDIUM).
3. Constrain link-backup pattern to `*.backup.[0-9]*` (Safety LOW).
4. Add error-path test bullets to Tasks 1 and 2 (Safety LOW).
5. Add "nothing to clean up → suggest `dotman status`" to Task 2 (Product).
6. Error message includes `~/.local/share/dotman/dotfiles` fallback path (Product).
7. Note pre-existing test failure in verification section (Workflow).
8. Add `mod status;` to `src/main.rs` checklist (Workflow).
9. Add explicit task dependency ordering note (Workflow).

**Rejected suggestions:** None.

---

## Gate 3: Code Review (implementation → done)

*To be filled after implementation.*

