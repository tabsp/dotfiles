# Recovery And Cleanup Design

## Goal

Provide documented recovery boundaries and explicit cleanup workflows for state
that `dotman` manages, so users can inspect and undo managed changes.

## Scope

- Document what state `dotman` manages and where: installed binaries, linked
  files, backup directories, temporary extraction staging.
- Add `dotman cleanup` subcommand that lists and optionally removes stale
  backup directories and temporary staging leftovers.
- Add `make uninstall` target that removes `dotman` binary and known managed
  state.
- Add `docs/recovery.md` with recovery procedures.

## Non-Goals

- Do not implement automatic rollback of bootstrap.
- Do not track per-file ownership (no state database).
- Do not remove user data outside known `dotman` paths.

## Design

### Managed State

| State | Location | Managed? |
|-------|----------|----------|
| Installed directory-symlink tools | `$HOME/.local/bin/` | Yes (dotman creates) |
| Linked dotfiles | `$HOME/.config/...`, etc. | Yes (dotman symlinks) |
| Backup directories | `<install-path>.dotman-backup` | Yes (dotman creates) |
| Temporary staging | `<install-path>.dotman-staging` | Yes (dotman creates, cleans after success) |
| Downloaded archives cache | `$TMPDIR/dotman-*` | No (ephemeral) |

### `dotman cleanup`

Scans `$HOME/.local/bin/` for directories matching `*.dotman-backup` and
`*.dotman-staging`. Lists them. With `--execute`, removes them.

```
dotman cleanup           # list stale backup/staging dirs
dotman cleanup --execute # remove them
```

### `make uninstall`

Removes `dotman` binary and prompts about managed state:
```
make uninstall           # remove dotman binary, list remaining managed state
```

### Documentation

`docs/recovery.md` covers:
- What dotman manages and where
- How to list and remove backups
- How to unlink dotfiles
- How to remove installed tools
- How to fully uninstall dotman

## Error Handling

- `cleanup` with no stale dirs found: prints "nothing to clean up", exits 0.
- `cleanup --execute` with permission errors: reports each failure, continues.
- `make uninstall` if dotman binary not found: warns, continues.

## Verification Strategy

- `cargo test` — all existing tests pass.
- `cargo clippy` — zero warnings.
- Manual test: `dotman cleanup` on a clean system prints "nothing to clean up".
- Manual test: `make uninstall` removes the dotman binary.

## Regression Coverage Expectations

- Existing install and link workflows are unchanged.
- `make bootstrap`, `make link` continue to work.
- No changes to existing error codes.
