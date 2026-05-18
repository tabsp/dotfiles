# Narrow Diff And Reconcile Reporting Implementation Plan

**Spec:** `docs/superpowers/specs/2026-05-19-p2-narrow-diff-and-reconcile-design.md`

**Goal:** Extend `dotman diff` with `--narrow` and `--reconcile` flags.

**Architecture:** Modify `src/diff.rs` to add narrow filtering and reconcile
command generation. Update `src/main.rs` CLI dispatch. No new modules.

**Tech Stack:** Rust (existing diff module), Markdown.

---

## Existing Code Map

- `src/diff.rs`: `run_diff(json: bool)` — main diff entry point. Has `DiffOutput`,
  `ToolDiffEntry`, `DotfileDiffEntry`, `StaleEntry`, `DiffSummary` structs.
- `src/main.rs`: `Command::Diff { json: bool }` — CLI dispatch.
- `tests/cli_diff.rs`: integration tests for diff.

## Task 1: Add --narrow and --reconcile CLI flags

**Files:**
- Modify: `src/main.rs`
- Modify: `src/diff.rs`

- [ ] Add `narrow: bool` and `reconcile: bool` to `Command::Diff`.
- [ ] Update `run_diff` signature: `run_diff(json: bool, narrow: bool, reconcile: bool)`.
- [ ] Pass flags through from CLI.

## Task 2: Implement narrow filtering

**Files:**
- Modify: `src/diff.rs`

- [ ] When `narrow` is true, skip printing entries with `ok` status.
- [ ] Summary still shows full counts (all items), but detailed listing shows
  only non-ok items.
- [ ] Section headers still appear even if all items in that section are ok
  (show "(none)" or "(all ok)").

## Task 3: Implement reconcile command generation

**Files:**
- Modify: `src/diff.rs`

- [ ] Add `generate_reconcile_commands()` function that inspects diff results.
- [ ] Rules: missing tools → `dotman bootstrap`, missing/wrong_target dotfiles →
  `dotman link --force <name>`, stale backups/staging → `dotman cleanup`.
- [ ] Print `## Reconcile commands (advisory — review before running)` section.
- [ ] Commands prefixed with `# ` comment marker.
- [ ] All ok → "Nothing to reconcile."

## Task 4: Update JSON output

**Files:**
- Modify: `src/diff.rs`

- [ ] Add `narrow: bool` and `reconcile_commands: Vec<String>` to `DiffSummary`.
- [ ] Populate in JSON output.

## Task 5: Update tests

**Files:**
- Modify: `src/diff.rs` (unit tests)
- Modify: `src/diff.rs` (inline unit tests under `#[cfg(test)] mod tests`)

- [ ] Test `--narrow` skips ok entries.
- [ ] Test `--reconcile` generates correct advisory commands.
- [ ] Test `--narrow --reconcile` combination.
- [ ] Test all-ok diff prints "Nothing to reconcile."
- [ ] Test JSON includes narrow flag and reconcile commands.

## Verification Commands

- `cargo test diff` — existing + new diff tests pass.
- `cargo test` — all tests pass.
- `cargo clippy` — zero warnings.
- `make check` — manifest validation passes.

## Expected Outcomes

- `dotman diff` unchanged (default behavior preserved).
- `dotman diff --narrow` shows only non-ok items.
- `dotman diff --reconcile` appends advisory reconcile commands.
- `dotman diff --narrow --reconcile` combines both.
- JSON output includes `narrow` and `reconcile_commands` in summary.

## Test Level

- Unit tests: `src/diff.rs` for narrow filtering, reconcile generation.
- Unit tests: `src/diff.rs` inline tests for narrow filtering, reconcile generation, and exit code behavior.

## Regression Coverage Expectations

- Default `dotman diff` output unchanged.
- All existing diff tests pass.
- `dotman diff --json` still produces valid JSON.

## Machine State Safety

> **Required for implementation plans.**

- **Dry-run / preview path:** `dotman diff` is already read-only. `--narrow`
  and `--reconcile` do not modify state — they only filter output and print
  advisory text. The `# ` comment prefix prevents accidental paste-execution.
- **Failure-path tests:** Existing diff tests cover missing manifests,
  unparseable manifests, permission errors. New tests will cover reconcile
  edge cases (no changes needed, unknown dotfile names).
- **Recovery notes:** Not applicable — no state is modified.
- **Manual smoke checks:** `dotman diff --narrow` in repo dir, `dotman diff
  --reconcile` prints advisory commands, `dotman diff --narrow --reconcile`
  combines both.
- **Non-destructive scope:** All changes are read-only output additions.
  No destructive operations.
