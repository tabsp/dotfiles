# Narrow Diff And Reconcile Reporting Design

## Goal

Extend `dotman diff` with narrow filtering options and reconcile advisory output,
so users can focus on what needs attention and see what commands would resolve
drift.

## Motivation

Today `dotman diff` shows everything (ok, missing, drifted, stale, etc.) in one
dump. Users reading a long list of "ok" entries struggle to find the few items
that need action. After identifying drift, users must manually construct
`dotman bootstrap`, `dotman link`, or `dotman cleanup` commands.

## Scope

- Add `--narrow` flag: show only non-ok entries (missing, drifted, wrong_target,
  stale, version_unknown).
- Add `--reconcile` flag: after the diff, print advisory commands that would
  bring state back to desired (e.g., `dotman bootstrap`, `dotman link --force`,
  `dotman cleanup`). Never execute — advisory only.
- When both `--narrow --reconcile` are used, show only non-ok items + reconcile
  commands.
- JSON output (`--json`): when `--narrow`, the data arrays (`installed_tools`,
  `linked_dotfiles`, etc.) are also filtered to non-ok entries. The summary
  metadata includes `narrow: bool` and `reconcile_commands: [String]` (only
  when `--reconcile` is active).

## Non-Goals

- Do not execute reconcile commands automatically.
- Do not add interactive prompt or `--yes` flag.
- Do not change default (no-flag) diff output.
- Do not add per-tool or per-dotfile filtering (deferred).

## Design

### `--narrow` Flag

When `--narrow` is set, skip printing items with `ok` status. Summary still
shows total counts but the detailed listing only shows actionable items.

```
dotman diff --narrow
```

Output:

```
==> Installed tools
  zoxide     ~/.local/bin/zoxide      missing
  delta      ~/.local/bin/delta       drifted (installed 0.18.2, expected 0.19.0)

==> Linked dotfiles
  wezterm    ~/.config/wezterm        missing

==> Backups
  bat.dotman-backup       ~/.local/bin/bat.dotman-backup         stale

3 ok, 1 missing, 1 drifted, 1 stale
```

### `--reconcile` Flag

When `--reconcile` is set, append a "Reconcile commands" section after the diff
with advisory shell commands. Each command is prefixed with `# ` (comment) to
prevent accidental paste-execution.

```
dotman diff --reconcile
```

Output (appended after normal diff):

```
==> Reconcile commands (advisory — review before running)
# Install missing tools
  dotman bootstrap

# Fix wrong dotfile targets
  dotman link --force wezterm

# Clean up stale backups and staging
  dotman cleanup
```

Reconcile logic:
- Any `missing` or `drifted` tool → suggest `dotman bootstrap`. If tools share
  the same category, batch into a single `dotman bootstrap` line.
- `version_unknown` tool → suggest manual version check (no automatic fix).
- Any `missing` or `wrong_target` dotfile → batch into a single
  `dotman link --force <name1> <name2> ...` line.
- Any `stale` backup or staging → suggest `dotman cleanup`.
- All ok → "Nothing to reconcile."

### JSON Changes

Add `narrow` (bool) and `reconcile_commands` (string array) to summary:

```json
{
  "summary": {
    "ok": 4, "missing": 1, "drifted": 1, "stale": 1,
    "wrong_target": 0, "version_unknown": 0,
    "narrow": false
  }
}
```

When `--reconcile` is active, `reconcile_commands` is included:
```json
{
  "summary": {
    "ok": 4, "missing": 1,
    "narrow": true,
    "reconcile_commands": ["dotman bootstrap", "dotman link --force wezterm"]
  }
}
```

### Implementation Strategy

- Add `--narrow` and `--reconcile` flags to `Command::Diff` in `src/main.rs`.
- Narrow: add a helper that filters entries by non-ok status before printing.
- Reconcile: add a helper that generates advisory commands from diff results.
- JSON: extend `DiffSummary` with `narrow` and `reconcile_commands` fields.

### Error Handling

- `--reconcile` sans `--narrow`: still append reconcile (makes sense — user
  wants full diff + fix suggestions).
- No reconcile commands generated (all ok): print "Nothing to reconcile."
- Unknown dotfile names in reconcile: use path basename as fallback.
- Exit codes: `--narrow` and `--reconcile` do not change exit code behavior.
  Exit 0 if all ok, exit 1 if any non-ok status (same as default diff).

### Verification Strategy

- `cargo test diff` — existing tests + new narrow/reconcile tests.
- `cargo test` — all tests pass.
- `cargo clippy` — zero warnings.
- `make check` — manifest validation.
- Manual: `dotman diff --narrow` in repo dir.
- Manual: `dotman diff --reconcile` prints advisory commands.
- Manual: `dotman diff --narrow --reconcile` combines both.
- Manual: `dotman diff --narrow --json` includes narrow flag in summary.

### Regression Coverage Expectations

- Default `dotman diff` output unchanged.
- Existing diff tests pass.
- `dotman diff --json` still produces valid JSON.
