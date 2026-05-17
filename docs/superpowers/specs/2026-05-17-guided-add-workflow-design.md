# Guided Add Workflow Design

## Goal

Provide interactive CLI commands (`dotman add dep`, `dotman add config`) that
guide users through adding valid entries to `deps.toml` and `dotfiles.toml`,
eliminating manual TOML editing and schema memorization.

## Scope

### In scope (v1)

- `dotman add dep`: interactive Q&A → append valid `[deps.<name>]` entry to `deps.toml`.
- `dotman add config`: interactive Q&A → append valid `[[files]]` entry to `dotfiles.toml`,
  optionally create source placeholder under `config/`.
- `--dry-run` flag for both subcommands: print to-be-written TOML and file
  operations without touching disk.
- Deduplication: reject duplicate dep command names and duplicate config target
  paths before writing.
- Atomic write: stage to `.tmp`, validate in-memory, rename on success, discard on failure.
- TOML preservation: use `toml_edit` to append entries without disturbing
  existing formatting, key order, or comments.
- After successful write, print a hint to run `dotman check`.


## Non-Goals

- Do not implement `dotman add tool` (combined dep + config).
- Do not call any network APIs (no GitHub release auto-detection).
- Do not provide a non-interactive / flag-driven mode.
- Do not support editing or removing existing entries.
- Do not guide users through `version_check` configuration.
- Do not add a TUI or curses interface; plain stdin/stdout only.

## Design

### Command structure

```
dotman add dep [--dry-run]
dotman add config [--dry-run]
```

Both are added as new subcommands under the top-level `Command` enum in `main.rs`.

### Dependency resolution

Add `toml_edit` as a direct dependency (already a transitive dep via `toml`).
Use it for format-preserving TOML appends. Continue using `toml` +
`config::load_deps` / `config::load_dotfiles` for validation.

### Module layout

```
src/add.rs          # Interactive prompting + TOML generation logic
src/add_dep.rs      # dep-specific question flow
src/add_config.rs   # config-specific question flow
```

Or keep it in a single `src/add.rs` if the flow is compact enough. Decision
deferred to implementation.

### Interactive flow: `dotman add dep`

1. **Name**: Ask for dependency name (identifier, e.g., `ripgrep`).
   - Validate: not empty, no whitespace, no `[` `]` `.` characters.
   - Check: name not already in `deps.toml`.

2. **Command**: Ask for CLI command checked by `which`.
   - Default: same as name.
   - Check: command not already claimed by another dep in `deps.toml`.

3. **Installer type**: Present numbered list of supported installers:
   - `system`, `brew`, `cask`, `apt`, `repo_package`, `ppa`,
     `official_script`, `download_binary`.
   - Validate selection.

4. **Version**: Ask for version string.
   - Accept `latest` (no version_check required).
   - If pinned (e.g., `0.12.2`), note that `version_check` can be added
     manually later.

5. **Source** (optional): Ask for project URL (e.g., `https://github.com/…`).
   - Validate: must start with `https://`.

6. **Installer-specific params**: Based on installer type, ask for each
   required param:

   | Installer        | Required params                                   |
   |------------------|--------------------------------------------------|
   | `system`         | none                                             |
   | `brew`           | `package`                                        |
   | `cask`           | `package`                                        |
   | `apt`            | `package`                                        |
   | `repo_package`   | `package`, `repo_url`, `repo_key_url`, `repo_channel`, `repo_components` |
   | `ppa`            | `ppa`, `package`                                 |
   | `official_script`| `script_url`, `install_to`                       |
   | `download_binary`| `url`, `sha256`, `archive_kind`, `binary_path`, `install_to`; optionally `install_dir_from` + `install_dir_to` |

   For each param:
   - Show description and example.
   - Validate format (e.g., URLs start with `https://`, paths under `~/.local`).

7. **Summary**: Print generated TOML snippet and ask for confirmation.
   - If `--dry-run`: exit without writing.
   - Otherwise: proceed to atomic write.

### Interactive flow: `dotman add config`

1. **Source**: Ask for source path relative to repo (e.g., `config/ripgreprc`).
   - Validate: no `..`, no absolute paths, no `$`.
   - Check: not already in `dotfiles.toml`.

2. **Target**: Ask for target path (e.g., `~/.ripgreprc`).
   - Validate: must start with `~` or `/`.
   - Check: target not already in active entries.

3. **Kind**: Ask `file` or `dir` (default: `file`).
   - If source doesn't exist yet, offer to create it (empty file or directory).
   - `--dry-run` skips creation.

4. **Platforms** (optional): Ask for platform restriction.
   - `all` (default), `mac`, `linux`, or comma-separated list.
   - Validate against known platforms.

5. **Enabled**: default `true`, ask `y/n`.

6. **Notes** (optional): human-readable description.

7. **Summary**: Print generated TOML snippet and optional file creation plan.
   - Confirm or `--dry-run` exit.

### Atomic write strategy

1. Read existing file content as string.
2. Append new TOML entry using `toml_edit` (or string concatenation for
   `dotfiles.toml` array-of-tables).
3. Write merged content to `<file>.tmp`.
4. Validate `.tmp` with `config::load_*` + `check::run_check`.
5. If valid: `std::fs::rename(.tmp, original)`. Print success.
6. If invalid: delete `.tmp`, report errors, leave original untouched.

For `dotfiles.toml`, the `[[files]]` array of tables requires appending to the
end of the document. `toml_edit` can handle this by manipulating the document
AST.

### Deduplication

Before prompting, parse existing manifests and check:
- Dep command name isn't already claimed.
- Config source path isn't already used.
- Config target path isn't already in active entries.

Report conflicts immediately and exit (no changes).

### Error Handling

- Invalid user input → re-prompt with explanation (max 3 attempts, then exit).
- Manifest parse failure → report and exit (can't safely append to broken file).
- Validation failure after append → report errors, delete `.tmp`, exit.
- File I/O errors → report and exit.

## Verification Strategy

### Test Level

CLI integration tests (`tests/cli_add.rs`) exercising:
- `dotman add dep --dry-run` produces expected TOML on stdout, no file changes.
- `dotman add config --dry-run` produces expected TOML on stdout, no file changes.
- Duplicate dep command rejected, no file changes.
- Duplicate config target rejected, no file changes.
- After adding a dep, `dotman check` passes.
- After adding a config, `dotman check` passes.
- Atomic write: invalid input leaves original file intact.

### Unit tests

- TOML generation produces valid `DepsManifest` / `DotfilesManifest` on parse.
- Param validation rejects malformed URLs, paths, etc.

### Verification Commands

```sh
cargo test add            # targeted tests
cargo test                # full suite
make check                # manifest validation
make lint                 # formatting + clippy
make ci                   # full verification
```

## Regression Coverage Expectations

- All existing CLI integration tests (`tests/cli_check.rs`,
  `tests/cli_link.rs`, `tests/cli_doctor.rs`, `tests/cli_shell.rs`,
  `tests/cli_agent.rs`) continue to pass.
- All existing unit tests in `src/config.rs`, `src/check.rs`,
  `src/main.rs` continue to pass.
- `make check`, `make lint`, `make ci` pass without regressions.
- Existing `deps.toml` and `dotfiles.toml` remain parseable after the
  new module is added (no code changes to config/check modules that
  would break existing manifests).
