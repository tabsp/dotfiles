# Managed State Inventory Diff Design

## Goal

Add a read-only `dotman diff` subcommand that compares manifest-defined desired
state against current machine state, showing what's missing, what's drifted, and
what's stale. This is the inspectability prerequisite for future reconcile,
cleanup, adopt, and unmanage work.

## Motivation

The roadmap says: "Dotman can explain what it considers managed and how current
machine state differs from manifests." Today `dotman status` (P0) shows current
state but doesn't compare it to manifests. `dotman doctor` reports health but
mixes errors and warnings without a structured diff view.

## Scope

- Add `dotman diff` subcommand: read-only comparison of manifests to machine.
- Diff categories: installed tools (DownloadBinary + OfficialScript only),
  linked dotfiles, backup/staging directories, source checkout.
- Output: human-readable sections and structured JSON.
- Each diff item has a status: `ok`, `missing`, `drifted`, `wrong_target`,
  `stale`, `version_unknown`.
- Host-aware filtering: only diff entries applicable to the current platform.

## Non-Goals

- Do not diff Brew/System/Apt/RepoPackage/Ppa tools (managed externally, no
  install paths in deps.toml; deferred to future ownership work).
- Do not detect `extra` tools or dotfiles (requires scanning install/target
  dirs for items not in manifests; deferred to future iteration).
- Do not implement reconcile or automatic fix (deferred to P2).
- Do not modify files or machine state.
- Do not change `dotman status` or `dotman doctor` behavior.

## Design

### Diff Categories

| Category | Checks | Statuses |
|----------|--------|----------|
| Installed tools | Dep entries (DownloadBinary, OfficialScript) vs actual files at install paths, with host filtering | `ok`, `missing`, `drifted`, `version_unknown` |
| Linked dotfiles | dotfiles.toml entries vs actual symlinks | `ok`, `missing`, `wrong_target` |
| Backups | `*.dotman-backup` directories | `stale` |
| Staging leftovers | `*.dotman-staging` directories | `stale` |
| Source checkout | `~/.local/share/dotman/dotfiles` | `ok`, `missing`, `not_git` |

Brew, System, Apt, RepoPackage, and Ppa installers are excluded from diff
because dotman has no defined install path for them (they're managed by external
package managers). This matches the P0 ownership model.

### `dotman diff` Subcommand

```
dotman diff             # human-readable diff
dotman diff --json      # machine-readable
```

Human-readable output:

```
==> Installed tools
  bat        ~/.local/bin/bat         ok
  fd         ~/.local/bin/fd          ok
  ripgrep    ~/.local/bin/rg          ok
  zoxide     ~/.local/bin/zoxide      missing
  delta      ~/.local/bin/delta       drifted (installed 0.18.2, expected 0.19.0)

==> Linked dotfiles
  nvim       ~/.config/nvim           ok
  fish       ~/.config/fish           ok
  wezterm    ~/.config/wezterm        missing

==> Backups
  bat.dotman-backup       ~/.local/bin/bat.dotman-backup         stale

==> Staging leftovers
  (none)

==> Source checkout
  ~/.local/share/dotman/dotfiles       ok

3 ok, 1 missing, 1 drifted, 1 stale
```

Exit codes: 0 if all `ok`, 1 if any non-ok status. `stale` items (backups,
staging leftovers) also trigger exit 1 — they represent state that should be
cleaned up.

#### JSON Schema

```json
{
  "installed_tools": [
    {"name": "bat", "path": "/home/user/.local/bin/bat", "status": "ok"},
    {"name": "zoxide", "path": "/home/user/.local/bin/zoxide", "status": "missing"},
    {"name": "delta", "path": "/home/user/.local/bin/delta", "status": "drifted", "installed_version": "0.18.2", "expected_version": "0.19.0"}
  ],
  "linked_dotfiles": [
    {"name": "nvim", "path": "/home/user/.config/nvim", "status": "ok"},
    {"name": "fish", "path": "/home/user/.config/fish", "status": "wrong_target", "actual_target": "/wrong/path"}
  ],
  "backups": [
    {"path": "/home/user/.local/bin/bat.dotman-backup", "status": "stale"}
  ],
  "staging_leftovers": [],
  "source_checkout": {"path": "/home/user/.local/share/dotman/dotfiles", "status": "ok"},
  "summary": {"ok": 4, "missing": 1, "drifted": 1, "stale": 1, "wrong_target": 0, "version_unknown": 0}
}
```

### Implementation Strategy

New module: `src/diff.rs`. Makes helper functions in `src/status.rs` accessible
(`pub(crate)`) or duplicates where simpler.

Key logic:
- Tool diff: detect host via `platform::detect_host()`, filter deps with
  `entries_for_host()`, check `DownloadBinary`/`OfficialScript` entries
  at install paths.
- Version drift: use `doctor::read_version()` when `version_check` is available.
  If version check fails, report `version_unknown` (not an error, not a blocker).
- Dotfile diff: iterate `dotfiles.toml` entries, check symlinks.
- Backup/staging: reuse `status.rs` scan logic.
- Source checkout: reuse `status.rs` check.

### Error Handling

- Not in a dotfiles repo: same error as `dotman status` with fallback path.
- Missing/unparseable manifests: error, exit 1.
- No deps/dotfiles entries at all: "no managed state defined," exit 0.
- Version check failure: item status `version_unknown`, does not block the diff.
- Known limitation: version commands have no timeout (same as `dotman doctor`).

### Verification Strategy

- `cargo test diff` — unit tests for diff logic (ok, missing, drifted,
  version_unknown, wrong_target, stale).
- `cargo test` — all existing tests pass.
- `cargo clippy` — zero warnings.
- `make check` — manifest validation passes.
- Manual: `dotman diff` in repo dir prints diff inventory.
- Manual: `dotman diff --json` outputs valid JSON.

### Regression Coverage Expectations

- `dotman status` unchanged.
- `dotman doctor` unchanged.
- `dotman cleanup` unchanged.
- All existing tests pass.
