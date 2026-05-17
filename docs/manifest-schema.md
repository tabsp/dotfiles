# Manifest Schema

This document describes the schema for `deps.toml` (dependency installation) and
`dotfiles.toml` (dotfile symlinks). It serves as the authoritative reference for
field types, defaults, and validation rules enforced by `make check`.

---

## deps.toml

### Top-level

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `[deps.<name>]` | table | yes | One section per dependency. `<name>` is an arbitrary identifier (e.g., `git`, `nvim`). |

### Dependency (`[deps.<name>]`)

| Key | Type | Required | Default | Description |
|-----|------|----------|---------|-------------|
| `command` | string | yes | — | CLI command checked via `which`. |
| `version_check` | table | no | none | Version verification (see below). |
| `default` | table | no | none | Fallback entry inherited by all arch entries. |
| `mac` | table | no | `{}` | Per-architecture entries for macOS. |
| `linux` | table | no | `{}` | Per-architecture entries for Linux. |

Architecture keys under `mac` and `linux`:

| Key | Platform |
|-----|----------|
| `arm64` | Apple Silicon / Linux aarch64 |
| `x86_64` | Intel Mac / Linux amd64 |

### Default inheritance

When a `default` section is present, per-arch entries inherit its fields:
- `installer` and `version` are used as fallbacks if absent in the per-arch entry.
- `params` are merged: default params + per-arch params, with per-arch winning.
- `source` and `distros` are taken from the default if absent in per-arch.
- If no per-arch entry exists for a platform+arch, the default is used directly.
- If no default exists, behavior is unchanged.

### VersionCheck (`[deps.<name>.version_check]`)

| Key | Type | Required | Default | Description |
|-----|------|----------|---------|-------------|
| `args` | string[] | no | `["--version"]` | CLI arguments for version query. |
| `regex` | string | yes | — | Regex with one capture group extracting the version. |
| `stream` | string | no | `"stdout"` | `"stdout"` or `"stderr"` — which output stream to parse. |

### InstallEntry (`[deps.<name>.<platform>.<arch>]`)

| Key | Type | Required | Default | Description |
|-----|------|----------|---------|-------------|
| `installer` | string | yes | — | One of the installer types below. |
| `version` | string | yes | — | Expected version, or `"latest"` to skip version check. |
| `source` | string | no | none | URL of the download or repository page (documentation only). |
| `distros` | string[] | no | none | Restrict entry to specific Linux distros. If omitted, matches all. |
| `params` | table | no | `{}` | Installer-specific parameters (see below). |

### Installer types

| Value | Description |
|-------|-------------|
| `system` | Assumed pre-installed. No install action. |
| `brew` | macOS Homebrew formula. |
| `cask` | macOS Homebrew cask. |
| `apt` | Debian/Ubuntu apt package. |
| `repo_package` | Third-party apt repository + package. |
| `ppa` | Ubuntu PPA + package. |
| `official_script` | Official install script fetched via HTTPS. |
| `download_binary` | Binary archive downloaded via HTTPS with SHA-256 verification. |

### Per-installer params

Each installer type expects specific params under `[deps.<name>.<platform>.<arch>.params]`.

#### system, brew, cask, apt

No params required.

#### repo_package

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `package` | string | yes | Package name to install. |
| `repo_url` | string | yes | HTTPS URL of the repository. |
| `repo_key_url` | string | yes | HTTPS URL of the repository signing key. |
| `repo_channel` | string | yes | Repository channel (e.g., `stable`, `main`). |
| `repo_components` | string[] | yes | Repository components (e.g., `["main"]`). |

#### ppa

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `ppa` | string | yes | PPA name (e.g., `ppa:fish-shell/release-4`). |
| `package` | string | yes | Package name to install. |
| `bootstrap_package` | string | no | Bootstrap dependency (default: `software-properties-common`). |

#### official_script

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `script_url` | string | yes | HTTPS URL of the install script. |
| `args` | string[] | no | Extra arguments passed to the script. |
| `install_to` | string | no | Path to check for existing installation. Must be under `~/.local`. |

#### download_binary

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `url` | string | yes | HTTPS download URL. |
| `sha256` | string | yes | Expected SHA-256 hex digest. |
| `archive_kind` | string | yes | One of: `raw`, `tar.gz`, `tar.xz`, `zip`. |
| `binary_path` | string | yes | Path within archive to the binary. |
| `install_to` | string | yes | Destination path for the binary or symlink. Must be under `~/.local`. |
| `install_dir_from` | string | no | Subdirectory within archive to copy (paired with `install_dir_to`). |
| `install_dir_to` | string | no | Destination directory for extracted content. Must be under `~/.local`. |

`install_dir_from` and `install_dir_to` must both be present or both absent.

### Validation rules (`make check`)

- `url`, `script_url`, `repo_url`, `repo_key_url` must start with `https://`.
- `install_to` and `install_dir_to` must be under `~/.local`.
- `archive_kind` must be one of the supported values.
- `distros` is only valid on Linux entries.
- Per-installer required params are enforced.

---

## dotfiles.toml

### Top-level

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `[[files]]` | array of tables | yes | One entry per dotfile to symlink. |

### FileEntry (`[[files]]`)

| Key | Type | Required | Default | Description |
|-----|------|----------|---------|-------------|
| `source` | string | yes | — | Path relative to repo root (e.g., `config/nvim`). |
| `target` | string | yes | — | Absolute or `~`-prefixed destination path. |
| `kind` | string | no | `"file"` | `"file"` or `"dir"` — hint for link creation. |
| `enabled` | boolean | no | `true` | Set to `false` to skip this file. |
| `platforms` | string[] | no | `[]` | Restrict to specific platforms (`mac`, `linux`). Empty = all platforms. |
| `notes` | string | no | none | Human-readable description. |

### Validation rules (`make check`)

- `source` must be a path within the repository.
- `target` must start with `~` or `/`.
