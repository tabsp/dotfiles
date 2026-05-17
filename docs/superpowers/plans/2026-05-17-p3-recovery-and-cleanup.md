# Recovery And Cleanup Implementation Plan

**Spec:** `docs/superpowers/specs/2026-05-17-p3-recovery-and-cleanup-design.md`

**Goal:** Add `dotman cleanup` subcommand, `make uninstall` target, and recovery documentation.

**Architecture:** New `src/recovery.rs` module + Makefile target + documentation. Minimal Rust changes: scan known paths, list/remove stale dirs.

**Tech Stack:** Rust (std::fs), Make, Markdown.

---

## Existing Code Map

- `src/main.rs`: CLI command enum.
- `src/installers.rs`: creates `<name>.dotman-backup` and `<name>.dotman-staging` directories.
- `src/link.rs`: creates backups of conflicting files.
- `Makefile`: existing targets for build, bootstrap, link, etc.
- `README.md`: documentation index.
- `src/path.rs`: `expand_home` helper.

## Task 1: Add dotman cleanup subcommand

**Files:**
- New: `src/recovery.rs`
- Modify: `src/main.rs`

- [ ] Create `src/recovery.rs` with `run_cleanup(execute: bool)` function.
- [ ] Scan `$HOME/.local/bin/` for `*.dotman-backup` and `*.dotman-staging` dirs.
- [ ] Print list of stale dirs. With `--execute`, remove them.
- [ ] Add `Command::Cleanup { execute: bool }` to CLI.

## Task 2: Add make uninstall target

**Files:**
- Modify: `Makefile`

- [ ] Add `make uninstall` target that removes `dotman` binary.
- [ ] Print list of remaining managed state locations.

## Task 3: Document recovery procedures

**Files:**
- New: `docs/recovery.md`
- Modify: `README.md`

- [ ] Write recovery guide covering: managed state locations, backup cleanup, dotfile unlinking, full uninstall.
- [ ] Link from README.

## Verification Commands

- `cargo test`
- `cargo clippy`
- `make lint`
- `make check`
- Manual: `dotman cleanup` (no stale dirs)
- Manual: `make uninstall` (removes binary)

## Test Level

- Unit tests: `cargo test recovery` — scan dir listing, empty dir handling.
- No CLI integration tests for cleanup (requires filesystem state).

## Expected Outcomes

- `dotman cleanup` lists and optionally removes stale backup/staging dirs.
- `make uninstall` removes dotman binary and prints managed state info.
- `docs/recovery.md` covers all recovery procedures.
- All existing tests pass.

## Regression Coverage Expectations

- Existing install and link workflows unchanged.
- `make bootstrap` continues to work.
- No changes to error codes.
