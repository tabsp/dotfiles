# Rust Env Manager Rebuild Design

## Goal

Replace the current environment manager with a small Rust backend while keeping
`make` as the public entry point and preserving only the core workflows:

- `make bootstrap`
- `make link`
- `make doctor`
- `make check`

The Rust program is an internal backend. It should not become a new
user-facing command for now.

## Scope

- Keep the existing dotfiles behavior unchanged.
- Migrate repository-owned source files into a unified `config/` tree in v1.
  This changes repository layout but not home-directory targets.
- Keep application configuration in the existing layout, including XDG
  directories under `.config/` and root-level dotfiles such as `.tmux.conf`.
- Keep managed-file selection in a separate manifest from dependency
  installation.
- Keep environment-manager configuration at the repository root.
- Keep one cross-platform dependency manifest and one managed-files manifest
  for macOS and Linux.
- Keep the backend data-driven and small.
- Keep installation explicit and fail-fast.
- Keep `doctor` helpful but non-destructive.
- Keep `check` read-only.

## Non-Goals

- No lock file in the first Rust version.
- No dependency install plan output in the first Rust version.
- No uninstall or cleanup command.
- No requirement that every installer support pinned versions in the first Rust
  version.
- No automatic rollback when dependency installation stops after a partial
  success.
- No quiet/verbose flag set in the first Rust version.

## Architecture

### Public Entry Point

`make` remains the documented entry point.

The Makefile should be a thin wrapper over the Rust backend. It should not
contain install logic itself, and `make bootstrap` should call the Rust backend
directly rather than routing through a shell wrapper.

For v1, `make` should build the backend as needed before invoking it.

Because `cargo` is required before the Rust backend can run, Makefile must
perform one minimal preflight outside Rust:

- check that `cargo` exists
- print a clear error if it is missing
- do not perform installer or manifest logic in Makefile

### Rust Backend

The Rust backend owns:

- manifest parsing
- platform and architecture resolution
- installer dispatch
- environment checks
- version comparison and warning reporting

The backend should remain data-driven. It should not grow a large
tool-specific control flow tree.

Build artifacts should use the standard Rust/Cargo layout under `target/`.

The Rust project should live at the repository root:

- `Cargo.toml` at the repository root
- `src/` at the repository root
- build output under `target/`

For v1, `make` should invoke the debug binary during development:

- `target/debug/dotman`

### Backend CLI

The Rust backend still needs a make-facing CLI because `make` is only a
wrapper. This is a stable contract for the repository and its Makefile in v1,
not a broader public compatibility promise. The v1 backend interface should be:

- `cargo run -- bootstrap`
- `cargo run -- link --conflict <fail|backup|overwrite>`
- `cargo run -- link --dry-run --conflict <fail|backup|overwrite>`
- `cargo run -- doctor`
- `cargo run -- check`

The compiled binary should expose the same subcommands when run directly.

`cargo run -- <subcommand>` is a development-equivalent entry point. Makefile
should call `target/debug/dotman` after building it.

Makefile variables map to backend flags:

- `CONFLICT=backup` -> `dotman link --conflict backup`
- `DRY_RUN=1` -> `dotman link --dry-run`
- unset `CONFLICT` defaults to `backup`

`bootstrap` invokes the same internal logic as `check`, dependency
installation, `link --conflict backup`, and `doctor`. It does not shell out to
`dotman check` or `dotman link`.

Exit code contract:

- `0` for success
- non-zero for hard failures
- `doctor` may print warnings and still exit `0`
- `doctor` exits non-zero when any hard failure is found

Output contract:

- progress messages go to stdout and start with `==>`
- warnings go to stderr and start with `warn:`
- hard failures go to stderr and start with `error:`

## Data Model

### Manifest

Use one cross-platform `deps.toml`.

This file is the environment-manager configuration. It should live at the
repository root and should not be mixed into the application config tree,
whether that config lives under `.config/` or as a root-level dotfile.

The manifest should be organized like this:

- top-level `deps`
- each dependency declares a command name
- each dependency may declare a `version_check` block
- each dependency declares platform and architecture entries under
  `deps.<tool>.<platform>.<arch>`
- each platform/architecture entry declares an `installer`, `version`, and a
  `params` subtable
- each platform/architecture entry may declare `source` as a human-reference
  upstream URL

Example:

```toml
[deps.nvim]
command = "nvim"

[deps.nvim.version_check]
args = ["--version"]
regex = 'v?([0-9]+\.[0-9]+\.[0-9]+)'
stream = "stdout"

[deps.nvim.mac.arm64]
installer = "brew"
version = "latest"
source = "https://neovim.io/doc/install/"

[deps.nvim.mac.arm64.params]
package = "neovim"

[deps.nvim.linux.x86_64]
installer = "download_binary"
version = "0.10.4"
source = "https://github.com/neovim/neovim/releases"

[deps.nvim.linux.x86_64.params]
url = "https://github.com/neovim/neovim/releases/download/v0.10.4/nvim-linux64.tar.gz"
sha256 = "REQUIRED_SHA256"
archive_kind = "tar.gz"
binary_path = "nvim-linux64/bin/nvim"
install_to = "~/.local/bin/nvim"
```

### Repository Layout

The repository may keep source configuration under a shared `config/` tree for
clarity, but the home-directory targets should remain the standard application
paths. For example:

- `config/nvim` -> `~/.config/nvim`
- `config/fish` -> `~/.config/fish`
- `config/ghostty` -> `~/.config/ghostty`
- `config/tmux.conf` -> `~/.tmux.conf`

### Managed Files Manifest

Use a separate `dotfiles.toml` to describe which repository files are managed
and where they should be linked in the home directory.

The manifest should be organized like this:

- top-level `files`
- each entry describes one source path and one target path
- entries may point at source files under `config/` or at root-level dotfiles
- the manifest should not describe installer behavior
- source paths are relative to the repository root
- target paths may use `~` for the home directory
- one source path may be mapped to multiple targets only when the mapping is
  written out explicitly

Minimum fields for each managed file entry:

- `source`
- `target`

Optional fields:

- `kind`
- `enabled` (defaults to true)
- `platforms`
- `notes`

When present, `platforms` is an array of platform names. The valid values in v1
are:

- `mac`
- `linux`

`kind` is optional. When present, the only valid values in v1 are:

- `file`
- `dir`

When omitted, the backend should infer the kind from the source path in the
repository. No other `kind` values are supported in v1.

An entry is active for the current host only when:

- `enabled` is not `false`
- `platforms` is omitted, or it contains the current platform

Linking semantics in v1:

- `kind = "file"` creates one symlink at `target`
- `kind = "dir"` creates one symlink for the directory itself at `target`
- v1 does not recursively symlink directory contents one file at a time
- if `target` already exists and is not the expected symlink, `link` follows
  the active `CONFLICT` policy
- the default conflict policy is `backup`
- backup names use `<target>.backup.<YYYYMMDDHHMMSS>` and must be regenerated
  until unique if a collision occurs
- `overwrite` may remove an existing file, symlink, or directory at `target`
- conflict policy applies only to `target`, not to parent directories
- parent directory conflicts are always hard failures

Example:

```toml
[[files]]
source = "config/nvim"
target = "~/.config/nvim"
kind = "dir"
enabled = true

[[files]]
source = "config/tmux.conf"
target = "~/.tmux.conf"
kind = "file"
enabled = true

[[files]]
source = "config/tmux.conf"
target = "~/.config/tmux/tmux.conf"
kind = "file"
enabled = false
```

### Core Fields

Every install entry must include these core fields:

- `installer`
- `version`

`source` is a human-reference URL, not an execution input. Installer execution
must use `params`.

`source` is optional for every installer and omitted for `system`. When present,
it must use `https://`.

The `version` field may be `latest`, but the meaning depends on the installer:

- for package-manager installers, `latest` means "no pinned version requested"
- for `system`, `latest` means "only check that the command exists"
- for `download_binary`, v1 requires an explicit concrete version and SHA256
  rather than `latest`

### Parameters

Installer-specific settings belong in a `params` subtable.

Required `params` by installer in v1:

- `system`
  - no params
- `brew`
  - `package`
- `cask`
  - `package`
- `apt`
  - `package`
- `repo_package`
  - `package`
  - `repo_url`
  - `repo_key_url`
  - `repo_channel`
  - `repo_components`
- `official_script`
  - `script_url`
  - optional `args`
  - optional `install_to`
- `download_binary`
  - `url`
  - `sha256`
  - `archive_kind`
  - `binary_path`
  - `install_to`

`download_binary` parameter semantics:

- `archive_kind` is required and v1 supports only:
  - `tar.gz`
  - `tar.xz`
  - `zip`
  - `raw`
- `binary_path` is the path to the installed executable relative to the
  extracted archive root
- `install_to` is the final target path for that executable
- the backend expands `~` in `install_to`
- the backend creates parent directories for `install_to` when they do not
  exist
- `install_to` must be the final executable path, not a directory
- `install_to` must not point inside the repository

`official_script` parameter semantics:

- `install_to` is only an installed-state marker path
- the upstream script decides where it installs files
- when present, `install_to` must be the final executable path used for
  detection, not a directory
- when absent, installed-state detection falls back to `command`

### Platform And Architecture

Platform and architecture coverage is expressed only through actual install
entries under `deps.<tool>.<platform>.<arch>`. There is no separate
`supported_arches` field in v1.

Platform names in v1 are:

- `mac`
- `linux`

Architecture names in v1 are:

- `arm64`
- `x86_64`

Rust target values must be normalized before matching manifest keys:

- `std::env::consts::OS == "macos"` -> `mac`
- `std::env::consts::OS == "linux"` -> `linux`
- `std::env::consts::ARCH == "aarch64"` -> `arm64`
- `std::env::consts::ARCH == "x86_64"` -> `x86_64`

### Version Detection

Version detection is tool-specific data, not a hardcoded backend guess.

Each tool may declare:

- `command`
- `version_check.args`
- `version_check.regex`
- `version_check.stream`

Defaults:

- `args = ["--version"]`
- `stream = "stdout"`

`regex` is required when a tool declares `version_check`. If the block is
present without `regex`, `check` should fail.

The first capture group in `regex` is the version string. No other extraction
rule is supported in v1.

If a tool cannot report a version in a stable way, `doctor` should warn that
version detection is unavailable and continue.

If any current-host entry uses a concrete pinned version instead of `latest`,
the tool must declare `version_check`. Without it, `check` fails because
`doctor` cannot verify version drift.

### Platform Detection

The backend should detect:

- OS from Rust target information
- architecture from Rust target information
- Linux distribution from `/etc/os-release`

Supported hosts in v1:

- macOS
- Ubuntu
- Debian

Linux distributions outside Ubuntu/Debian should fail `check` and `bootstrap`
clearly.

## Installer Model

### Supported Installers

The backend should start with a small set of stable installers:

- `system`
- `brew`
- `cask`
- `apt`
- `repo_package`
- `official_script`
- `download_binary`

The installer value is the execution backend. The manifest does not require a
separate `kind` field.

### Installer Semantics

- `system`
  - check that the command exists
  - do not try to manage a version
- `brew`
  - use Homebrew formula installation
  - valid only for `mac` entries
  - install if missing
  - otherwise leave the installed version in place in v1
- `cask`
  - use Homebrew cask installation
  - valid only for `mac` entries
  - install if missing
  - otherwise leave the installed version in place in v1
- `apt`
  - use apt for bootstrap/system packages
  - valid only for `linux` entries on Ubuntu/Debian
  - requires root or `sudo`
  - detect installed state through the native package database for the current
    platform implementation
  - install if missing
  - otherwise leave the installed version in place in v1
- `repo_package`
  - add an apt repository and install from it
  - this installer is only in scope for Ubuntu/Debian in v1
  - it requires root or `sudo`
  - repository configuration is a persistent system change in v1 and is not
    cleaned up automatically
- `official_script`
  - fetch the upstream script over HTTPS to a temporary file
  - run it explicitly, never through an inline pipe
  - trust in the script itself is an accepted v1 risk
  - if `install_to` is present, use it as the primary installed-state check
  - otherwise use `command` as the installed-state check
- `download_binary`
  - download a specific binary artifact
  - verify it with SHA256 from the manifest
  - unpack it and install it at the requested target path
  - detect installed state through `install_to`, then validate `command`

The backend should fail clearly if a tool references an installer that is not
implemented.

All installers must be idempotent:

- if the tool is missing, install it
- if the tool is present and v1 does not support in-place upgrades for that
  installer, leave it in place
- if the current host has no matching manifest entry, fail clearly

## Command Semantics

### `make bootstrap`

- build the Rust backend if needed
- run `check`
- install dependencies
- link dotfiles
- run `doctor`

This is the primary first-run command.

### `make link`

- link dotfiles only
- run `check` first
- support `CONFLICT=fail|backup|overwrite`
- support `DRY_RUN=1`
- fail fast on link errors

### `make doctor`

- validate manifest structure first
- verify the real machine state
- check that required commands exist
- check that the key symlinks point to this repository
- check that commands can report a version
- if a pinned version differs from the manifest expectation, warn but do not
  fail
- if the manifest uses `latest`, print the discovered version without treating
  it as drift

### `make check`

- read-only validation
- parse the manifest
- validate basic field presence and structure
- validate that `deps.toml` and `dotfiles.toml` both exist
- validate that `cargo` exists
- validate that the current host has a supported platform and architecture
- validate that active file mappings exist for the current host
- validate that dependency entries for the current host are structurally valid
- do not modify the machine
- do not produce an install plan

## Execution Flow

### Dependency Install Flow

1. Read `deps.toml`.
2. Determine the current platform and architecture.
3. Select tools that have exactly one matching entry for the current host.
4. Resolve the installer.
5. Use the installer-specific `params`.
6. Detect installed state through the installer's native check.
7. Install sequentially in manifest order only for missing tools.
8. Re-check the installed command when that installer manages the command.
9. Stop immediately on the first hard failure.

### Link Flow

1. Read `dotfiles.toml`.
2. Filter entries where `enabled != false`.
3. Filter entries by `platforms` when present.
4. Validate the source path exists in the repository and matches `kind` when
   `kind` is declared.
5. Create missing parent directories for the target.
6. Apply the `CONFLICT` policy to the target only:
   - `fail`
   - `backup`
   - `overwrite`
7. Create a single symlink per active file entry.
8. Stop immediately on the first hard failure.

### Link Dry-Run Flow

1. Run the same structural validation as `check` for `dotfiles.toml`.
2. Filter active entries using `enabled` and `platforms`.
3. Evaluate each entry against the current `CONFLICT` policy.
4. Group output in this order:
   - `would fail`
   - `would overwrite`
   - `would backup`
   - `would link`
5. Within each group, keep manifest order.
6. Print source path, target path, action, and conflict reason when relevant.
7. Print a final summary line:
   - `dry-run: success`
   - `dry-run: would fail`
8. Return non-zero if real execution would fail.

### Doctor Flow

1. Read the manifest.
2. Resolve the current platform and architecture.
3. Stop immediately if manifest validation fails.
4. Check that expected commands exist for matching dependency entries.
5. Check that active symlink targets from `dotfiles.toml` exist and point to
   the expected source in the repository.
6. Use `version_check` to read and normalize version output only after the
   command exists.
7. Treat version-check execution or parse failure as a hard error when
   `version_check` is declared.
8. Warn when pinned versions differ from the manifest, but do not fail.
9. When the manifest uses `latest`, report the discovered version without
   treating it as drift.
10. Print successful checks as well as errors and warnings.

### Check Flow

1. Read the manifest.
2. Validate required fields and TOML structure in both manifests.
3. Validate that every referenced installer is known.
4. Validate that `deps.<tool>.command` is globally unique.
5. Validate that only `mac` and `linux` are used as platform keys.
6. Validate that only `arm64` and `x86_64` are used as architecture keys.
7. Validate platform and architecture coverage for the current host.
8. Validate that detected Linux distributions are Ubuntu or Debian.
9. Validate that any current-host dependency has exactly one matching entry.
10. Validate that required installer parameters are present.
11. Validate each installer is compatible with the platform entry where it is
    declared.
12. Validate URL-bearing fields use `https://`.
13. Validate `dotfiles.toml` structure and managed-file paths.
14. Validate active file targets are unique for the current host.
15. Validate `(source, target)` pairs are unique.
16. Validate `source` paths stay inside the repository and exist.
17. Validate `target` paths are absolute or `~`-based, contain no environment
    variables, and do not point inside the repository.
18. Validate `install_to` paths are absolute or `~`-based, contain no
    environment variables, and do not point inside the repository.
19. Validate active file mappings exist for the current host.
20. Aggregate structural errors instead of stopping at the first one.
21. Do not mutate the filesystem or install software.

## Error Handling

- Missing manifest fields should fail `check`, `link`, and `bootstrap`.
- Unsupported platform or architecture should fail clearly.
- Unknown installer should fail clearly.
- Missing root or `sudo` access for `apt` or `repo_package` should fail clearly.
- `bootstrap` should stop on the first real failure.
- `link` should stop on the first real failure.
- dependency installation may leave the machine in a partially applied state.
  This is an accepted v1 limitation.
- `doctor` should treat these as hard failures:
  - missing command
  - missing active target symlink
  - target exists but is not a symlink
  - target symlink points to the wrong source
  - declared `version_check` cannot execute or cannot parse a version
- `doctor` should report pinned-version drift as a warning, not a failure.
- `latest` should suppress drift comparison and only report the discovered
  version.
- `check` should aggregate structural errors.

## Testing Strategy

The first Rust version should be covered by:

- unit tests for manifest parsing and installer selection
- unit tests for `check` structural validation and error aggregation
- unit tests for `link` conflict policy handling
- unit tests for `dry-run` grouping and exit status
- unit tests for platform and architecture resolution
- tests for command and version normalization
- tests for `doctor` warning behavior
- tests for fail-fast bootstrap behavior
- a Linux container smoke test
- a macOS local smoke test

## Migration Shape

The migration should be incremental. The target state described above is the
end state for v1. The steps below describe how to reach it from the current
dotfiles-only baseline:

1. introduce the Rust backend
2. keep the `make` entry points
3. move `link`, `doctor`, and `check` behind Rust
4. move `bootstrap` behind Rust after build-and-run is stable
5. remove legacy backend behavior only after the Rust path is stable

Stability for this migration means:

- `make check` passes on macOS and Ubuntu/Debian
- `make link` applies managed files correctly on both platforms
- `make doctor` reports commands, symlinks, and versions consistently
- the Rust backend can replace the old path without changing dotfiles behavior

Rollback path:

- keep the current clean dotfiles-only baseline as the rollback point until the
  Rust backend satisfies the stability criteria
- do not delete fallback history until after the first stable Rust cut

## Open Decisions

- whether future versions should add a dedicated dependency upgrade command
- whether future versions should support additional repository-backed package
  managers beyond Ubuntu/Debian
- whether future versions should support platform-specific `version_check`
  overrides
