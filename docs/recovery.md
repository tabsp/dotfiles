# Recovery

This document describes the state `dotman` manages and how to undo or clean
up its changes.

## Managed State

| State | Location | Created By |
|-------|----------|------------|
| Installed tools | `$HOME/.local/bin/<tool>` | `make bootstrap` (symlinks for directory installs) |
| Linked dotfiles | `$HOME/.config/...`, etc. | `make link` (symlinks to repo config) |
| Backup directories | `$HOME/.local/bin/<tool>.dotman-backup` | Conflict resolution during install |
| Staging directories | `$HOME/.local/bin/<tool>.dotman-staging` | Atomic install (cleaned on success, may persist on failure) |

## Cleaning Up Stale State

### List and remove backup / staging directories

```sh
# List stale directories
dotman cleanup

# Remove them
dotman cleanup --execute
```

### Remove installed tools

Installed tools are symlinks in `$HOME/.local/bin/`. To remove one:

```sh
rm "$HOME/.local/bin/<tool-name>"
```

To remove all dotman-managed tools, remove the symlinks that point into the
dotman-managed directories (check with `ls -la $HOME/.local/bin/`).

### Unlink dotfiles

Dotfiles are symlinks managed by `make link`. To remove them:

```sh
# Preview what would be linked
make link DRY_RUN=1

# Manually remove individual symlinks
rm "$HOME/.config/nvim"
```

## Full Uninstall

To completely remove `dotman` and all managed state:

```sh
# 1. Remove installed tools
rm "$HOME/.local/bin/"*

# 2. Remove linked dotfiles (replace with your actual paths)
rm "$HOME/.config/nvim"
rm "$HOME/.config/fish"

# 3. Clean up stale backup/staging dirs
dotman cleanup --execute

# 4. Remove dotman binary
make uninstall

# 5. Optionally, remove the repository
rm -rf /path/to/dotfiles
```

## Automatic Rollback

`dotman` does not automatically roll back a failed bootstrap. If
`make bootstrap` fails partway through:

1. Some dependencies may have been partially installed — re-running
   bootstrap will skip already-satisfied deps.
2. Staging directories may persist — run `dotman cleanup --execute`.
3. Backup directories preserve pre-existing state — rename
   `<tool>.dotman-backup` back to `<tool>` to restore.
