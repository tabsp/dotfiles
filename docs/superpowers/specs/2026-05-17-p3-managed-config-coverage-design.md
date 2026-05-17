# Managed Config Coverage Design

## Goal

Add managed config entries in `dotfiles.toml` for installed tools that have
meaningful config files but aren't yet tracked, closing the gap between
dependency installation and configuration coverage.

## Scope

- Audit `deps.toml` vs `dotfiles.toml` to find tools with deps but no config.
- Add `dotfiles.toml` entries for tools with configurable dotfiles: `yazi`,
  `starship`, `lazygit`.
- Create minimal placeholder config files where they don't exist.
- Document tools that intentionally have no managed config (env-var-driven
  tools like `zoxide`, `eza`, `fzf`).

## Non-Goals

- Do not add config entries for system tools (`git`, `make`) — their configs
  are system-scoped, not dotfile-scoped.
- Do not create full configuration content — placeholders only.

## Design

### Gap Analysis

| Tool | deps.toml | dotfiles.toml | Config Type | Action |
|------|-----------|---------------|-------------|--------|
| nvim | ✓ | ✓ | dir | covered |
| fish | ✓ | ✓ | dir | covered |
| ghostty | ✓ | ✓ | dir | covered |
| tmux | ✓ | ✓ | file | covered |
| yazi | ✓ | ✗ | dir | add |
| starship | ✓ | ✗ | file | add |
| lazygit | ✓ | ✗ | dir | add |
| zoxide | ✓ | ✗ | env vars | document |
| eza | ✓ | ✗ | env vars | document |
| fzf | ✓ | ✗ | env vars | document |
| git | ✓ | ✗ | system | document |
| make | ✓ | ✗ | system | document |

### Changes

1. Create `config/yazi/` with a minimal `yazi.toml` placeholder.
2. Create `config/starship.toml` with a placeholder comment.
3. Create `config/lazygit/` with a minimal `config.yml` placeholder.
4. Add entries to `dotfiles.toml`.
5. Add a comment in `dotfiles.toml` listing tools not covered and why.

## Error Handling

- No runtime error codes needed (config-only changes).
- Missing config files cause `make link` to report a conflict/warning (existing
  behavior).

## Verification Strategy

- `make check` — manifest validation passes.
- `cargo test` — all existing tests pass.
- `cargo clippy` — zero warnings.
- `make link DRY_RUN=1` — shows new entries without applying them.

## Regression Coverage Expectations

- Existing dotfiles.toml entries are unchanged.
- `make bootstrap` continues to work.
- No Rust source changes.
