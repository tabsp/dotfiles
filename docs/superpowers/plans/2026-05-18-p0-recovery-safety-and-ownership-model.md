# Recovery Safety And Ownership Model Implementation Plan

**Spec:** `docs/superpowers/specs/2026-05-18-p0-recovery-safety-and-ownership-model-design.md`

**Goal:** Add `dotman status` subcommand with ownership model, harden `dotman
cleanup`, extend `make uninstall`, and rewrite recovery docs to be
inspect-first.

**Architecture:** New read-only `src/status.rs` module for ownership inventory.
`src/recovery.rs` extended to scan link-conflict backups and suggest `dotman
status`. Makefile extended for release binary uninstall. No new persistent
state — ownership is inferred from filesystem and manifests.

**Task ordering:** Tasks must be completed in order. Task 2 depends on Task 1
(status command must exist before cleanup can reference it). Task 4 requires
Tasks 1–3 complete (docs describe the new behavior).

**Tech Stack:** Rust (std::fs, serde for JSON), Make, Markdown.

---

## Existing Code Map

- `src/main.rs`: CLI command enum, main dispatch. Module declarations at top
  (`mod recovery;`, `mod doctor;`, etc.).
- `src/recovery.rs`: `run_cleanup(execute)` — scans `~/.local/bin/` for
  `*.dotman-backup` and `*.dotman-staging`.
- `src/installers.rs`: `install_archive_dir` creates directory-symlink tools,
  `.dotman-backup`, and `.dotman-staging` directories via `sibling_tempdir`.
- `src/link.rs`: `unique_backup_path` creates `{target}.backup.{YYYYMMDDHHmmss}`
  backups in the target's parent directory.
- `src/config.rs`: loads `deps.toml` and `dotfiles.toml`.
- `src/path.rs`: `expand_home` helper.
- `src/doctor.rs`: inspects installed tools and linked files — closest analog
  to the new `src/status.rs`.
- `Makefile:112-123`: current `uninstall` target.
- `docs/recovery.md`: current recovery documentation (has dangerous broad
  deletion patterns).
- `README.md`: documentation index and commands list.

## Task 1: Add dotman status subcommand

**Files:**
- New: `src/status.rs`
- Modify: `src/main.rs`

- [ ] Add `mod status;` to module declarations in `src/main.rs`.
- [ ] Create `src/status.rs` with `run_status(json: bool)` function.
- [ ] Load deps.toml and dotfiles.toml (reuse config module).
- [ ] For each dep entry, determine owned tools:
  - `download_binary` with `install_dir_to`: resolve symlink at `install_to`,
    canonicalize both sides, check if target matches canonical `install_dir_to`.
    Certainty: `managed`.
  - `install_archive_dir`: same symlink-resolution + canonicalization logic
    as `download_binary` with `install_dir_to`. Certainty: `managed`.
  - `download_binary` without `install_dir_to`: check file exists at
    `install_to`. Certainty: `detected`.
  - `official_script` with `install_to`: check file exists at path.
    Certainty: `detected`.
  - `official_script` without `install_to`: check `command` on PATH via
    `which`. Certainty: `detected`.
  - Other installers (brew, apt, system, repo_package): silently omitted
    from status output (not owned by dotman).
- [ ] For each dotfile entry, check if symlink at target points into repo
  config dir. Certainty: `managed`.
- [ ] Scan `~/.local/bin/` for `*.dotman-backup` and `*.dotman-staging` dirs
  (reuse recovery.rs pattern). Certainty: `managed`.
- [ ] For each linked dotfile target, scan its parent dir for
  `{target_name}.backup.[0-9]*` entries (timestamp-suffixed backups from
  `unique_backup_path`). Certainty: `managed`.
- [ ] Check `~/.local/share/dotman/dotfiles` for source checkout (verify
  `.git` exists). Certainty: `detected`.
- [ ] Print human-readable sections with ownership tiers.
- [ ] With `--json`, output structured JSON matching the spec schema.
- [ ] Error: not in a dotfiles repo → "not in a dotfiles repo. The release
  installer clones the repo to ~/.local/share/dotman/dotfiles — run dotman
  status from there." exit 1.
- [ ] Add `Command::Status { json: bool }` to CLI in `src/main.rs`.
- [ ] Add unit tests: owned tool detection, backup pattern matching, JSON
  output structure, error paths (no repo, missing toml, no managed state).

## Task 2: Harden dotman cleanup

**Files:**
- Modify: `src/recovery.rs`

- [ ] Extend `run_cleanup` to also scan link-conflict backups:
  - Load `dotfiles.toml` to get linked target paths.
  - For each target, scan parent directory for `*.backup.[0-9]*` entries
    (timestamp-suffixed backups from `unique_backup_path`).
- [ ] Add ownership category labels to stale item listings.
- [ ] If stale items found: suggest running `dotman status` for full
  inventory.
- [ ] If no stale items found: print "nothing to clean up" and suggest
  `dotman status` for a full inventory.
- [ ] Add unit tests for link-conflict backup detection, empty-dir "suggest
  status" message.
- [ ] Add unit tests for error paths (unreadable dirs).
- [ ] Ensure existing cleanup tests still pass unchanged.

## Task 3: Extend make uninstall

**Files:**
- Modify: `Makefile`

- [ ] Add removal of `~/.local/bin/dotman` (release-installed binary) if it
  exists.
- [ ] Print note about remaining managed state and reference
  `docs/recovery.md`.
- [ ] Keep existing behavior for `target/debug/dotman` and
  `target/release/dotman`.

## Task 4: Rewrite recovery documentation

**Files:**
- Modify: `docs/recovery.md`
- Modify: `README.md`

- [ ] Replace `rm "$HOME/.local/bin/"*` with `dotman status` output +
  targeted removal instructions.
- [ ] Add "Inspect First" section at top: run `dotman status` before any
  destructive action.
- [ ] Rewrite "Full Uninstall" section: status → targeted removal →
  uninstall binary.
- [ ] Document repo-required limitation and `~/.local/share/dotman/dotfiles`
  fallback.
- [ ] Add note that `(detected)` tools need manual verification before rm.
- [ ] In README: add `dotman status` and `dotman status --json` to commands
  list, update recovery section.

## Verification Commands

- `cargo test status` — unit tests for status subcommand.
- `cargo test recovery` — cleanup tests (existing + new).
- `cargo test` — all tests pass (note: 1 pre-existing failure not related to
  this epic; do not treat as a regression).
- `cargo clippy` — zero warnings.
- `make check` — manifest validation passes.
- `make ci` — full local verification.
- `make agent-check` — run before claiming completion of any phase.

## Expected Outcomes

- `dotman status` prints read-only inventory with ownership tiers
  `(managed)` / `(detected)`.
- `dotman status --json` outputs valid JSON matching the spec schema.
- `dotman cleanup` scans link-conflict backups in addition to
  backup/staging dirs, suggests `dotman status`.
- `dotman cleanup` with nothing to clean also suggests `dotman status`.
- `make uninstall` removes release-installed binary at `~/.local/bin/dotman`.
- `docs/recovery.md` no longer contains broad `rm *` patterns.
- All existing tests pass; new tests cover status, extended cleanup, and
  error paths.

## Regression Coverage Expectations

- Existing install, link, bootstrap, doctor, cleanup workflows unchanged.
- No changes to existing error codes or subcommand interfaces.
- Existing `cargo test recovery` tests pass unchanged.
- `make bootstrap`, `make link` continue to work.

## Test Level

- **Unit tests (src/status.rs):** 7 tests covering JSON serialization/deserialization, backup dir scanning, staging dir scanning, non-dotman file filtering, source checkout detection, and no-repo error path with fallback message.
- **Unit tests (src/recovery.rs):** 7 tests covering existing cleanup patterns (empty dir, backup dirs, staging dirs, file filtering) plus 3 new tests for link-conflict backup detection (timestamp pattern matching, non-timestamp suffix rejection, unrelated file filtering).
- **CLI integration tests:** No new CLI integration tests added. `dotman status` and hardened `dotman cleanup` are covered by unit tests and manual verification. This is consistent with existing project patterns (e.g., `dotman doctor` has no CLI integration tests).
