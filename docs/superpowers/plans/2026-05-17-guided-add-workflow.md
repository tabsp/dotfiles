# Guided Add Workflow Implementation Plan

**Spec:** `docs/superpowers/specs/2026-05-17-guided-add-workflow-design.md`

**Goal:** Add `dotman add dep` and `dotman add config` interactive CLI
subcommands that build valid manifest entries without manual TOML editing.

**Architecture:** New `src/add.rs` module for the interactive flow + TOML
generation. `toml_edit` for format-preserving file appends. Existing
`config` and `check` modules for validation.

**Tech Stack:** Rust, `toml_edit`, `clap`, existing project modules.

---

## Existing Code Map

- `src/main.rs`: CLI command enum, entry point. Add `Add` variant.
- `src/config.rs`: `DepsManifest`, `DotfilesManifest`, `load_deps`, `load_dotfiles`.
- `src/check.rs`: `run_check` for manifest validation.
- `deps.toml`, `dotfiles.toml`: repo manifests.
- `tests/common/mod.rs`: `current_host_table`, `write_minimal_dotfiles` helpers.
- `tests/cli_check.rs`: pattern for CLI integration tests.

## Task 1: Add `toml_edit` dependency and scaffold module

**Files:**
- Modify: `Cargo.toml`
- New: `src/add.rs`

- [ ] Add `toml_edit = "0.22"` to `Cargo.toml` dependencies.
- [ ] Create `src/add.rs` with module declaration, stub functions for
  `run_add_dep` and `run_add_config`.
- [ ] Add `Add` subcommand to `Command` enum in `src/main.rs`:
  ```rust
  Add {
      #[command(subcommand)]
      command: AddCommand,
  },
  ```
- [ ] Define `AddCommand` enum with `Dep { dry_run: bool }` and
  `Config { dry_run: bool }` variants.
- [ ] Wire dispatch in `run()`.

## Task 2: Implement `dotman add dep` interactive flow

**Files:** `src/add.rs`

- [ ] `prompt_dep_name`: read existing deps, ask for name, validate uniqueness.
- [ ] `prompt_command`: ask CLI command, default to name, check uniqueness.
- [ ] `prompt_installer`: show numbered installer list, accept selection.
- [ ] `prompt_version`: accept `latest` or pinned version.
- [ ] `prompt_source`: optional HTTPS URL.
- [ ] `prompt_installer_params`: dispatcher per installer type, ask required params.
- [ ] `build_dep_toml`: generate TOML string for `[deps.<name>]` section +
  per-installer table + params on current host.
- [ ] `confirm_and_write`: print TOML, ask y/n, proceed to atomic write or
  dry-run exit.

## Task 3: Implement `dotman add config` interactive flow

**Files:** `src/add.rs`

- [ ] `prompt_config_source`: validate path, check uniqueness.
- [ ] `prompt_config_target`: validate `~` or `/` prefix, check uniqueness.
- [ ] `prompt_kind`: `file` or `dir`, offer to create source path.
- [ ] `prompt_platforms`: accept `all` or comma-separated platforms.
- [ ] `prompt_enabled`: default `true`.
- [ ] `prompt_notes`: optional description.
- [ ] `build_config_toml`: generate TOML for `[[files]]` entry.
- [ ] `create_source_path`: mkdir or touch based on kind, unless `--dry-run`.
- [ ] `confirm_and_write`: same flow as dep.

## Task 4: Implement atomic write with validation

**Files:** `src/add.rs`

- [ ] `append_to_deps_toml`: read existing, append with `toml_edit`, write to
  `.tmp`, validate, rename.
- [ ] `append_to_dotfiles_toml`: same for `[[files]]` array of tables.
- [ ] `validate_tmp_deps`: parse `.tmp` with `config::load_deps` +
  `check::run_check`.
- [ ] `validate_tmp_dotfiles`: parse `.tmp` with `config::load_dotfiles` +
  `check::run_check`.
- [ ] On validation failure: delete `.tmp`, report errors.

## Task 5: Write CLI integration tests

**Files:** New: `tests/cli_add.rs`

- [ ] `add_dep_dry_run_prints_toml_no_file_change`: pipe stdin answers,
  assert stdout contains expected TOML, assert deps.toml unchanged.
- [ ] `add_config_dry_run_prints_toml_no_file_change`: pipe stdin answers,
  assert stdout, assert dotfiles.toml unchanged.
- [ ] `add_dep_rejects_duplicate_command`: write deps.toml with existing cmd,
  pipe stdin, assert failure + stderr mentions duplicate.
- [ ] `add_config_rejects_duplicate_target`: write dotfiles.toml with existing
  target, pipe stdin, assert failure.
- [ ] `add_dep_valid_output_passes_check`: create valid dep via dry-run,
  manually write to temp, run `dotman check`.
- [ ] `add_config_valid_output_passes_check`: same for config.
- [ ] `add_config_creates_source_path`: interactive flow creates placeholder
  file/dir under config/.
- [ ] `add_dep_atomic_validation_failure_leaves_original`: create broken dep
  scenario, assert original deps.toml unchanged.

## Task 6: Wire up and verify end-to-end

**Files:** `src/add.rs`, `src/main.rs`

- [ ] Ensure all prompts use `std::io::stdin()` + `std::io::stdout()`.
- [ ] Run `make lint`, fix any formatting/clippy issues.
- [ ] Run `cargo test add` for targeted tests.
- [ ] Run `cargo test` for full suite.
- [ ] Run `make check` for manifest validation.
- [ ] Run `make ci` for full verification.

## Verification Commands

```sh
cargo test add              # targeted: new add tests
cargo test                  # full test suite
make check                  # manifest validation
make lint                   # rustfmt + clippy
make ci                     # full verification: lint -> check -> test
```

## Test Level

- CLI integration tests: `tests/cli_add.rs` (8+ test cases)
- Unit tests: inline in `src/add.rs` for TOML generation and param validation

## Regression Coverage Expectations

- All existing tests continue to pass (118 tests).
- `make check`, `make lint`, `make ci` pass without regressions.

## Expected Outcomes

1. `dotman add dep` interactively builds a valid `[deps.<name>]` entry in
   `deps.toml` that passes `dotman check`.
2. `dotman add config` interactively builds a valid `[[files]]` entry in
   `dotfiles.toml` and optionally creates the source path under `config/`.
3. `--dry-run` prints the to-be-written TOML and file operations without
   modifying any files.
4. Duplicate dep command names and duplicate config targets are rejected
   before any file modifications.
5. Existing TOML formatting, key order, and comments are preserved when
   appending new entries.
6. Failed writes leave the original manifest files intact (atomic write).
7. All new CLI integration tests pass alongside the existing test suite.
