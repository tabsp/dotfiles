# Managed Config Coverage Implementation Plan

**Spec:** `docs/superpowers/specs/2026-05-17-p3-managed-config-coverage-design.md`

**Goal:** Add dotfiles.toml entries for yazi, starship, lazygit with placeholder configs.

**Architecture:** Configuration-only. No Rust changes.

**Tech Stack:** TOML, Markdown.

---

## Existing Code Map

- `dotfiles.toml`: existing entries for nvim, fish, ghostty, tmux.
- `config/`: existing dirs for fish, ghostty, nvim + tmux.conf.
- `deps.toml`: dependency entries for all tools (reference for gap analysis).
- `README.md`: documentation index.

## Task 1: Add placeholder config files

**Files:**
- New: `config/yazi/yazi.toml`
- New: `config/starship.toml`
- New: `config/lazygit/config.yml`

- [ ] Create `config/yazi/yazi.toml` with placeholder comment.
- [ ] Create `config/starship.toml` with placeholder comment.
- [ ] Create `config/lazygit/config.yml` with placeholder comment.

## Task 2: Update dotfiles.toml

**Files:**
- Modify: `dotfiles.toml`

- [ ] Add entries for yazi (dir), starship (file), lazygit (dir).
- [ ] Add comment documenting tools without managed configs and why.

## Verification Commands

- `make check`
- `cargo test`
- `cargo clippy`
- `make lint`

## Test Level

- No new tests (config-only change).
- Existing tests: `cargo test` (all).

## Expected Outcomes

- 3 new config placeholders exist.
- `dotfiles.toml` covers 7 of 12 deps tools.
- Gap documentation present as comments.
- All existing tests pass.

## Regression Coverage Expectations

- Existing dotfiles.toml entries unchanged.
- `make bootstrap` continues to work.
- No Rust source changes.
