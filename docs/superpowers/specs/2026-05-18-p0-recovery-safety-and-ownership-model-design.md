# Recovery Safety And Ownership Model Design

## Goal

Give users a read-only ownership model for all dotman-managed state so that
recovery, cleanup, and uninstall workflows follow an inspect-first pattern. No
automatic uninstall, adopt, unmanage, or broad-deletion guidance ships without
the ownership model in place.

## Motivation

Two risk-register items drive this work:

| Risk | Evidence |
|------|----------|
| Recovery guidance encourages broad deletion outside known managed state. | `docs/recovery.md` "Full Uninstall" section includes `rm "$HOME/.local/bin/"*`. |
| Cleanup and uninstall cannot be made safely automatic without ownership inventory. | `dotman cleanup` only scans for `*.dotman-backup` and `*.dotman-staging` directories. |

The P0 outcome from the roadmap: "recovery guidance follows an inspect-first
model, dangerous broad deletion examples are removed or replaced, and a
read-only ownership model is specified before any automatic uninstall, adopt,
unmanage, or corrective cleanup work."

## Scope

- Define dotman's ownership categories and where managed state lives.
- Add `dotman status` subcommand that prints a read-only inventory of all
  managed state (installed tools, linked dotfiles, stale backup/staging dirs,
  source checkout).
- Replace dangerous broad-deletion guidance in `docs/recovery.md` with
  inspect-first procedures.
- Harden `dotman cleanup` to integrate with the ownership model (scan all
  owned categories, not just backup/staging pattern matches).
- Add `dotman cleanup` suggestion to run `dotman status` first for inspect-first
  workflow.
- Extend `make uninstall` to also remove release-installed binary at
  `~/.local/bin/dotman`.

## Non-Goals

- Do not implement automatic uninstall.
- Do not implement adopt/unmanage (deferred in roadmap).
- Do not add a persistent state database (ownership is derived from filesystem
  inspection and manifest parsing, same as doctor/link today).
- Do not change install or link behavior.

## Design

### Ownership Categories

Dotman manages state in these categories:

| Category | Location | Ownership Test | Certainty |
|----------|----------|----------------|-----------|
| Installed tools (directory-symlink) | `$HOME/.local/bin/<tool>` | Symlink whose target is the `install_dir_to` path from deps.toml | `(managed)` |
| Installed tools (binary) | `$HOME/.local/bin/<tool>` | Regular file at the declared `install_to` path from deps.toml | `(detected)` |
| Linked dotfiles | `$HOME/.config/...`, etc. | Symlink whose target is under the dotfiles repo `config/` directory | `(managed)` |
| Backup directories | `<install-dir>.dotman-backup` | Directory name matches `*.dotman-backup` under `$HOME/.local/bin/` | `(managed)` |
| Link-conflict backups | `<target>.backup.<YYYYMMDDHHmmss>` | File/dir matching `*.backup.*` in the parent directory of each linked dotfile target | `(managed)` |
| Staging leftovers | `<install-dir>.dotman-staging` | Directory name matches `*.dotman-staging` under `$HOME/.local/bin/` | `(managed)` |
| Source checkout | `$HOME/.local/share/dotman/dotfiles` | Directory present at the path (check for `.git` to confirm git repo) | `(detected)` |

**Ownership certainty tiers:**
- `(managed)`: dotman can verify ownership (symlink resolves to expected path,
  name matches dotman convention, or file metadata confirms dotman origin).
- `(detected)`: file exists at the expected path but dotman cannot
  cryptographically verify it was installed by dotman. The user must confirm
  before taking destructive action.

**Known limitations:**
- Binary installs (`download_binary` without `install_dir_to`, `official_script`)
  are labeled `(detected)` because dotman cannot distinguish a dotman-installed
  binary from one placed manually or by another tool. Future work may add
  install sentinels (`~/.local/share/dotman/installed/<tool>`) to strengthen
  this.
- `dotman status` requires a dotfiles repo with `deps.toml` and `dotfiles.toml`
  to resolve ownership. If the repo is deleted, `dotman status` cannot run.
  The release installer clones the repo to `~/.local/share/dotman/dotfiles`;
  users can run status from there. This limitation is documented in
  `docs/recovery.md`.

### `dotman status` Subcommand

Read-only inventory. No side effects.

```
dotman status           # human-readable summary
dotman status --json    # machine-readable
```

Output sections (human-readable):

```
==> Installed tools (5)
  bat        ~/.local/bin/bat         (directory symlink, managed)
  fd         ~/.local/bin/fd          (directory symlink, managed)
  ripgrep    ~/.local/bin/rg          (directory symlink, managed)
  delta      ~/.local/bin/delta       (directory symlink, managed)
  zoxide     ~/.local/bin/zoxide      (binary, detected)

==> Linked dotfiles (12)
  nvim       ~/.config/nvim           -> ~/dev/dotfiles/config/nvim         (managed)
  fish       ~/.config/fish           -> ~/dev/dotfiles/config/fish         (managed)

==> Backups (2)
  bat.dotman-backup       ~/.local/bin/bat.dotman-backup       (managed)
  nvim.backup.20260518000000  ~/.config/nvim.backup.20260518000000  (managed)

==> Staging leftovers (0)
  (none)

==> Source checkout
  ~/.local/share/dotman/dotfiles (detected)
```

#### JSON Schema

```json
{
  "installed_tools": [
    {
      "name": "bat",
      "path": "/home/user/.local/bin/bat",
      "kind": "directory_symlink",
      "certainty": "managed"
    }
  ],
  "linked_dotfiles": [
    {
      "name": "nvim",
      "path": "/home/user/.config/nvim",
      "target": "/home/user/dev/dotfiles/config/nvim",
      "certainty": "managed"
    }
  ],
  "backups": [
    {
      "path": "/home/user/.local/bin/bat.dotman-backup",
      "kind": "install_backup",
      "certainty": "managed"
    },
    {
      "path": "/home/user/.config/nvim.backup.20260518000000",
      "kind": "link_backup",
      "certainty": "managed"
    }
  ],
  "staging_leftovers": [
    {
      "path": "/home/user/.local/bin/tool.dotman-staging",
      "certainty": "managed"
    }
  ],
  "source_checkout": {
    "path": "/home/user/.local/share/dotman/dotfiles",
    "certainty": "detected",
    "is_git_repo": true
  }
}
```

Schema stability: fields may be added in future versions; existing fields will
not be removed or renamed without a migration window. Top-level keys match
the human-readable sections.

### `dotman cleanup` Hardening

Current behavior: scans `~/.local/bin/` for `*.dotman-backup` and
`*.dotman-staging`.

Hardened behavior:
- Scan `$HOME/.local/bin/` for install backup/staging directories
  (`*.dotman-backup`, `*.dotman-staging`).
- Scan the parent directory of each linked dotfile target (from `dotfiles.toml`)
  for link-conflict backups matching `*.backup.*`. This matches
  `src/link.rs:unique_backup_path` which places backups at
  `{target}.backup.{YYYYMMDDHHmmss}` in the target's parent directory.
- Print which ownership category each stale item belongs to.
- Suggest running `dotman status` first if any stale items found.
- No other behavior change (still requires `--execute` to delete).

### `make uninstall` Extension

Current: removes `target/debug/dotman` and `target/release/dotman`.

Extended: also removes `~/.local/bin/dotman` (the release-installed binary path)
if it exists. Prints a note about remaining managed state and references
`docs/recovery.md`.

### Documentation Changes

**`docs/recovery.md`:**
- Replace `rm "$HOME/.local/bin/"*` with `dotman status` output and targeted
  removal instructions.
- Add an "Inspect First" section at the top that says: run `dotman status`
  to see everything dotman manages before taking any destructive action.
- Rewrite "Full Uninstall" to be inspect-first: status → targeted removal →
  uninstall binary.
- Document the repo-required limitation: if the dotfiles repo is deleted,
  use `~/.local/share/dotman/dotfiles` (cloned by installer) or re-clone.
- Add note that `(detected)` tools should be manually verified before removal.

**`README.md`:**
- Add `dotman status` to the commands list.
- Update recovery section to mention `dotman status` as the first step.
- Add `dotman status --json` usage example.

### Error Handling

- `dotman status` when not in a dotfiles repo: prints "not in a dotfiles repo"
  and exits 1.
- `dotman status` when deps.toml/dotfiles.toml are missing or unparseable:
  prints error and exits 1.
- `dotman status` with no managed state found: prints "no managed state found,"
  exits 0.
- `dotman cleanup` behavior unchanged for existing backup/staging scanning.
- `dotman cleanup` with no stale dirs: prints "nothing to clean up,"
  suggests `dotman status` for a full inventory.
- `make uninstall` when `~/.local/bin/dotman` not found: skips, prints note.

### Verification Strategy

- `make ci` — lint, check, test all pass.
- `cargo test status` — unit tests for status subcommand.
- `cargo test recovery` — existing cleanup tests still pass.
- `cargo clippy` — zero warnings.
- `make check` — manifest validation passes.
- Manual: `dotman status` in repo dir prints inventory.
- Manual: `dotman status --json` outputs valid JSON matching schema.
- Manual: `dotman cleanup` still works, suggests `dotman status`.

### Regression Coverage Expectations

- Existing install, link, bootstrap, doctor, cleanup workflows unchanged.
- No changes to existing error codes or subcommand interfaces.
- Recovery documentation no longer contains broad `rm *` patterns.
- `make uninstall` still removes build artifacts; now also handles release
  binary.
- Existing cleanup tests (`cargo test recovery`) must pass unchanged.
