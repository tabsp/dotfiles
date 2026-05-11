# Installer Extensions Design

## Goal

Add execution support for these installer kinds without changing the primary
design contract defined in
`docs/superpowers/specs/2026-05-10-rust-env-manager-rebuild-design.md`:

- `repo_package`
- `official_script`
- `download_binary`

This document is subordinate to the main spec. When the two documents differ,
the main spec wins.

## Scope

- Keep the existing `deps.toml` entry structure unchanged.
- Reuse the installer names and required `params` already defined in the main
  spec.
- Define only execution details and validation details that the main spec does
  not yet pin down.
- Keep bootstrap fail-fast and idempotent.

## Non-Goals

- No installer-specific upgrade logic in this slice.
- No download cache in this slice.
- No shell-pipe execution for `official_script`.
- No non-Debian repository models for `repo_package`.
- No archive formats beyond those already allowed by the main spec.
- No uninstall or cleanup command.

## Shared Rules

### Bootstrap Semantics

- Installers run only when the tool is not already installed.
- Installed-state checks remain the idempotency gate.
- If a tool is already installed, bootstrap skips it without comparing
  versions.

### Error Handling

- Installer failures are hard failures.
- Error messages must include:
  - dependency name
  - installer kind
  - key target fields for that installer
- Successful child-process stdout/stderr is suppressed.
- On failure, stderr is attached as raw context.
- If stderr is empty, stdout may be attached instead.
- Attached child output should be truncated to at most 8 KiB.

### URL Rules

Manifest-authored URLs must use `https://`.

For `download_binary`, redirects are allowed, but the final URL must also use
`https://`.

### Root Privileges

`apt` and `repo_package` execute through `sudo`.

- missing `sudo` => hard failure
- sudo authentication or command failure => hard failure
- no fallback to unprivileged execution

## `official_script`

### Model

`official_script` keeps the model already defined in the main spec:

- required: `script_url`
- optional: `args`
- optional: `install_to`

Trust in the upstream script itself remains an accepted v1 risk.

### Manifest Example

```toml
[deps.example.mac.arm64]
installer = "official_script"
version = "latest"
source = "https://example.com/install-docs"

[deps.example.mac.arm64.params]
script_url = "https://example.com/install.sh"
args = ["--yes"]
install_to = "~/.local/bin/example"
```

### Execution Rules

- fetch the script over HTTPS
- write it to a temporary file
- execute that temporary file explicitly
- never run the script through an inline pipe
- use the child exit code as the success signal
- do not parse human-readable stdout/stderr for success text

### Cleanup Rules

- the downloaded script file is a temporary artifact
- cleanup is attempted on both success and failure
- cleanup failure must not hide the original execution failure
- if execution succeeds but cleanup fails, emit `warn:`

### Installed-State Check

- if `install_to` is configured, it is the primary installed-state check
- otherwise, use `command` presence in `PATH`
- if `install_to` exists but is not a regular executable file, fail
- if `install_to` exists and is a regular executable file, treat the tool as
  installed
- if `install_to` is absent, the `command` fallback remains authoritative

`install_to` remains optional, matching the main spec.

## `download_binary`

### Model

`download_binary` keeps the parameter contract defined in the main spec:

- required: `url`
- required: `sha256`
- required: `archive_kind`
- required: `binary_path`
- required: `install_to`

The main spec also remains authoritative for version semantics:

- v1 requires an explicit concrete version
- `latest` is not supported for `download_binary`

### Manifest Example

```toml
[deps.example.linux.x86_64]
installer = "download_binary"
version = "1.2.3"
source = "https://example.com/releases"

[deps.example.linux.x86_64.params]
url = "https://example.com/releases/download/v1.2.3/example-linux-x86_64.tar.gz"
sha256 = "REQUIRED_SHA256"
archive_kind = "tar.gz"
binary_path = "example/bin/example"
install_to = "~/.local/bin/example"
```

### Supported Archive Kinds

The main spec is authoritative:

- `tar.gz`
- `tar.xz`
- `zip`
- `raw`

### Execution Flow

1. Check whether `install_to` exists.
2. If `install_to` exists and is a regular executable file, treat the tool as
   installed and skip.
3. If `install_to` exists but is not a regular executable file, fail.
4. If `install_to` does not exist, continue to download and install:
   - download to a temporary directory
   - verify the download with manifest SHA256
   - if `archive_kind = "raw"`, treat the downloaded file as the binary payload
   - otherwise unpack it in-process with Rust libraries
   - resolve `binary_path` relative to the unpack root
   - copy the final binary to `install_to`
   - set installed permissions to `0o755`
   - best-effort remove the temporary directory

### Cleanup Rules

- temporary download and unpack directories are process artifacts
- cleanup is attempted on both success and failure
- cleanup failure must not hide the original install failure
- if install succeeds but cleanup fails, emit `warn:`

### Implementation Rules

- use Rust libraries for archive extraction
- do not require system `tar`, `unzip`, or similar tools
- follow redirects only when the final target still uses `https://`

## `repo_package`

### Platform Scope

`repo_package` is supported only for:

- `linux.arm64`
- `linux.x86_64`

And only when distro detection resolves to:

- `ubuntu`
- `debian`

Any other platform or distro is a hard failure.

### Model

`repo_package` keeps the parameter contract defined in the main spec:

- `package`
- `repo_url`
- `repo_key_url`
- `repo_channel`
- `repo_components`

This extension spec does not add new required manifest fields beyond the main
spec.

### Derived Paths And Generated Content

Because the main spec does not carry explicit `keyring_path` or
`sources_path` fields, v1 derives them from the manifest data:

- keyring path:
  - `/usr/share/keyrings/<package>-dotman.gpg`
- sources path:
  - `/etc/apt/sources.list.d/<package>-dotman.list`

The source entry written to `sources_path` is:

```text
deb [signed-by=/usr/share/keyrings/<package>-dotman.gpg] <repo_url> <repo_channel> <repo_components...>
```

These derived paths are part of the installer contract for this slice.

### Manifest Example

```toml
[deps.example.linux.x86_64]
installer = "repo_package"
version = "latest"
source = "https://example.com/docs/install"

[deps.example.linux.x86_64.params]
package = "example"
repo_url = "https://packages.example.com/apt"
repo_key_url = "https://packages.example.com/apt/key.gpg"
repo_channel = "stable"
repo_components = ["main"]
```

### Execution Flow

1. Check whether the package is already installed.
2. If it is already installed, skip without mutating repository config.
3. Validate distro support.
4. Download and prepare repository key material from `repo_key_url`.
5. Materialize repository configuration from `repo_url`, `repo_channel`, and
   `repo_components`.
6. Compare the desired repository content against the current derived
   `sources_path`, and rewrite only when content differs.
7. Compare the desired keyring bytes against the current derived keyring path,
   and rewrite only when content differs.
8. Run `sudo apt-get update`.
9. Run `sudo apt-get install -y <package>`.

### Key Handling

- download `repo_key_url`
- accept armored or binary key material
- if armored, convert it with `gpg --dearmor`
- run `gpg --dearmor` as the current user and capture the converted bytes
- write the final keyring file by materializing content in a temporary file and
  then copying it into place with a `sudo`-executed system command
- if conversion is required and `gpg` is unavailable, fail
- do not use `apt-key`

### Persistent State

Repository configuration is a persistent system change in v1 and is not
cleaned up automatically.

## Validation Additions

### `official_script`

- `script_url` is required
- `script_url` must use `https://`
- `args`, when present, must be an array of strings
- `install_to`, when present, must be a string path

### `download_binary`

- `url`, `sha256`, `archive_kind`, `binary_path`, and `install_to` are all
  required
- `url` must use `https://`
- `archive_kind` must be one of the values allowed by the main spec
- `install_to` must not point inside the repository

### `repo_package`

- all required fields from the main spec must exist
- `repo_url` and `repo_key_url` must use `https://`
- `repo_components` must be a non-empty string array
- installer must only appear on supported Linux entries

## Testing Requirements

The first implementation pass should add:

- manifest validation tests for all three installers
- `download_binary` tests covering:
  - raw payload install
  - archive install
  - sha256 verification
  - redirect handling
  - existing non-executable `install_to` failure
- `official_script` tests covering:
  - download-to-temp execution
  - exit-code failure handling
  - `install_to`-based installed-state
  - `command` fallback installed-state
- `repo_package` tests covering:
  - installed-state precheck
  - source-content generation
  - key preparation logic
  - apt command construction

System-mutating end-to-end tests for `repo_package` may remain deferred if they
require privileged Linux fixtures. In that case, the implementation must still
ship unit coverage for generated repository content, key handling, and command
construction.
