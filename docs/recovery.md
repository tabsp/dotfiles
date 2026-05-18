# Recovery

This document describes the state `dotman` manages and how to undo or clean
up its changes.

## Inspect First

Before taking any destructive action, run `dotman status` to see everything
dotman manages:

```sh
dotman status
```

This prints a read-only inventory of installed tools, linked dotfiles, backups,
staging leftovers, and the source checkout. Each item is labeled `(managed)`
(dotman can verify ownership) or `(detected)` (exists at the expected path but
dotman cannot cryptographically verify it was installed by dotman). Always
verify `(detected)` items manually before removing them.

For machine-readable output:

```sh
dotman status --json
```

> **Note:** `dotman status` requires a dotfiles repo with `deps.toml` and
> `dotfiles.toml`. If you deleted the repo, the release installer clones it to
> `~/.local/share/dotman/dotfiles` — run `dotman status` from there.

## Managed State

| State | Location | Created By |
|-------|----------|------------|
| Installed tools | `$HOME/.local/bin/<tool>` | `make bootstrap` |
| Linked dotfiles | `$HOME/.config/...`, etc. | `make link` (symlinks to repo config) |
| Backup directories | `$HOME/.local/bin/<tool>.dotman-backup` | Conflict resolution during install |
| Link-conflict backups | `<target>.backup.<YYYYMMDDHHmmss>` | Conflict resolution during link |
| Staging directories | `$HOME/.local/bin/<tool>.dotman-staging` | Atomic install (cleaned on success, may persist on failure) |

## Cleaning Up Stale State

### Inspect first

```sh
dotman status
```

### List and remove backup / staging directories

```sh
# List stale directories
dotman cleanup

# Remove them
dotman cleanup --execute
```

### Remove installed tools

Run `dotman status` to see which tools are installed and their paths. Remove
them individually:

```sh
# For directory-symlink tools (managed)
rm "$HOME/.local/bin/<tool-name>"

# For binary tools (detected) — verify first with dotman status
rm "$HOME/.local/bin/<tool-name>"
```

### Unlink dotfiles

Dotfiles are symlinks managed by `make link`. To remove them:

```sh
# See what's linked
dotman status

# Manually remove individual symlinks
rm "$HOME/.config/nvim"
```

## Full Uninstall

To completely remove `dotman` and all managed state:

```sh
# 1. Inspect what dotman manages
dotman status

# 2. Remove tools one at a time (verify (detected) items first)
rm "$HOME/.local/bin/<tool>"

# 3. Remove linked dotfiles
rm "$HOME/.config/nvim"
rm "$HOME/.config/fish"

# 4. Clean up stale backup/staging dirs
dotman cleanup --execute

# 5. Remove dotman binary
make uninstall

# 6. Optionally, remove the repository
rm -rf /path/to/dotfiles
```

> ⚠️  Do not run broad deletion commands like `rm "$HOME/.local/bin/"*`.
> `$HOME/.local/bin/` may contain tools installed by other package managers or
> manually. Always inspect with `dotman status` and remove items individually.

## Automatic Rollback

`dotman` does not automatically roll back a failed bootstrap. If
`make bootstrap` fails partway through:

1. Some dependencies may have been partially installed — re-running
   bootstrap will skip already-satisfied deps.
2. Staging directories may persist — run `dotman cleanup --execute`.
3. Backup directories preserve pre-existing state — rename
   `<tool>.dotman-backup` back to `<tool>` to restore.
