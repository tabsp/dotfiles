# Managed State Inventory Diff Implementation Plan

**Spec:** `docs/superpowers/specs/2026-05-18-p1-managed-state-inventory-and-status-design.md`

**Goal:** Add `dotman diff` subcommand that compares manifests to machine state,
showing what's missing, drifted, wrong, or stale.

**Architecture:** New `src/diff.rs` module. Makes `src/status.rs` collection
helpers `pub(crate)` where needed. Reads manifests and compares against
filesystem with host-aware filtering. Read-only, no side effects.

**Task ordering:** Tasks run sequentially.

**Tech Stack:** Rust (std::fs, serde for JSON), Markdown.

---

## Existing Code Map

- `src/status.rs`: P0 ownership model — `collect_tools` (private),
  `collect_dotfiles` (private), `collect_backups_and_staging` (private),
  `collect_source_checkout` (private). Will make `pub(crate)` where needed.
- `src/main.rs`: CLI dispatch — add `Command::Diff` variant.
- `src/config.rs`: manifest loading, `Dependency::entries_for_host()`.
- `src/doctor.rs`: `read_version()` for version comparison.
- `src/path.rs`: `expand_home`, `paths_match`.
- `src/platform.rs`: `detect_host()`.

## Task 1: Add dotman diff subcommand

**Files:**
- New: `src/diff.rs`
- Modify: `src/main.rs`
- Modify: `src/status.rs` (make helpers `pub(crate)`)

- [ ] Add `mod diff;` to `src/main.rs`.
- [ ] Make `collect_backups_and_staging` and `collect_source_checkout`
  `pub(crate)` in `src/status.rs`.
- [ ] Create `src/diff.rs` with `run_diff(json: bool)`.
- [ ] Detect host via `platform::detect_host()`.
- [ ] Load deps.toml and dotfiles.toml.
- [ ] Tool diff: iterate deps.toml entries filtered by `entries_for_host()`:
  - `DownloadBinary` / `OfficialScript` only (others skipped per P0 model).
  - `ok`: tool exists at install path with matching version.
  - `missing`: tool not found at install path.
  - `drifted`: tool exists but version differs from expected.
  - `version_unknown`: version_check failed or not available.
- [ ] Dotfile diff: iterate dotfiles.toml entries:
  - `ok`: symlink exists and points to correct repo path.
  - `missing`: no symlink at target.
  - `wrong_target`: symlink exists but points elsewhere (include actual target).
  - No `extra` detection in this iteration (deferred).
- [ ] Backup/staging: reuse `status::collect_backups_and_staging()`,
  report as `stale`.
- [ ] Source checkout: reuse `status::collect_source_checkout()`.
- [ ] Human-readable output with status labels and summary count.
- [ ] `--json` output with structured `installed_version`/`expected_version`
  for drifted, `actual_target` for wrong_target.
- [ ] Exit 0 if all ok, exit 1 if any non-ok (including stale).
- [ ] Add `Command::Diff { json: bool }` to CLI.
- [ ] Add unit tests: ok, missing, drifted, version_unknown, wrong_target,
  stale, exit codes.

## Task 2: Documentation

**Files:**
- Modify: `README.md`

- [ ] Add `dotman diff` and `dotman diff --json` to commands list.
- [ ] Note that exit code 1 includes stale items.

## Verification Commands

- `cargo test diff` — unit tests for diff logic.
- `cargo test` — all tests pass.
- `cargo clippy` — zero warnings.
- `make check` — manifest validation passes.
- `make agent-check` — harness validation.

## Expected Outcomes

- `dotman diff` prints comparison of manifests to machine state with host
  filtering.
- `dotman diff --json` outputs valid JSON with structured version/target fields.
- Exit code reflects whether all state matches manifests.
- All existing tests pass; new tests cover diff logic.

## Regression Coverage Expectations

- `dotman status` unchanged.
- `dotman doctor` unchanged.
- `dotman cleanup` unchanged.

## Test Level

- Unit tests (src/diff.rs): diff logic for each status type, exit codes,
  host filtering.
- No CLI integration tests (consistent with existing patterns).

## Pre-existing test baseline

- 151 tests pass, 0 failures, clippy clean (1 pre-existing warning in agent.rs).
