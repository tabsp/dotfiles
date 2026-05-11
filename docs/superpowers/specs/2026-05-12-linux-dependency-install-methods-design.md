# Linux Dependency Install Methods Design

## Goal

Define the Linux install method for each managed dependency before changing
`deps.toml`.

The decisions here are scoped to Ubuntu first. Debian support for dependencies
that require PPAs or Ubuntu-specific repositories is deferred and should be
handled separately.

## Non-Goals

- Do not implement dependency installation in this spec.
- Do not introduce automatic latest-version resolution for binary downloads.
- Do not add dependency upgrade planning.
- Do not add fallback installers for Debian in this version.
- Do not install optional enhancement dependencies for tools such as `yazi`.

## General Policy

Use the simplest reliable installer per dependency:

- foundational system tools should use `apt`
- tools with official or accepted Ubuntu PPAs should use those PPAs
- fast-moving CLI tools should use official GitHub release binaries when a
  suitable Linux artifact exists
- official install scripts are acceptable when the upstream script is the
  documented install path and can be checked with `install_to`

`download_binary` dependencies should use explicit fixed versions and SHA256
checksums. Do not implement automatic latest-release resolution in this change.
Each dependency with a fixed version must also declare `version_check`, as
required by the main Rust env manager spec, so `check` and `doctor` can verify
version drift.

`official_script` dependencies may use `version = "latest"` when the upstream
script is intentionally tracking the latest release.

## PATH Requirements

All user-level binary install targets in this spec should install into
`~/.local/bin` unless a tool has a documented reason to use another directory.
The tracked fish configuration should add `$HOME/.local/bin` to `PATH` when the
directory exists, so these tools should be visible in new interactive shells:

- `download_binary` tools such as `nvim`, `yazi`, `eza`, `lazygit`, and `fzf`
- `official_script` tools such as `zoxide` and `starship`

Use a `$HOME`-based path with an existence check:

```fish
set -l local_bin "$HOME/.local/bin"
if test -d $local_bin
    fish_add_path $local_bin
end
```

The fish configuration should not create `$HOME/.local/bin`. Directory creation
belongs to installers:

- `download_binary` creates the parent directory for `install_to`
- `official_script` relies on the upstream script or installer-specific
  handling

Rust itself is a bootstrap prerequisite for building `dotman`, not a dependency
installed by `dotman`. The tracked shell configuration should add
`~/.cargo/bin` to `PATH` so `cargo`, `rustc`, and `rustup` installed by rustup
are visible without relying on ignored machine-local fish universal variables.
Use a `$HOME`-based path with an existence check, matching the portable style
used for other user-local paths:

```fish
set -l cargo_bin "$HOME/.cargo/bin"
if test -d $cargo_bin
    fish_add_path $cargo_bin
end
```

Package-manager installed tools use system paths that are already expected to be
on `PATH`:

- `apt` and `ppa` packages install commands under system binary directories such
  as `/usr/bin`
- Homebrew on macOS is covered by the existing `/opt/homebrew/bin` and
  `/opt/homebrew/sbin` fish path setup

## Installer Model Decisions

### Ubuntu PPA Installer

Ubuntu PPA support should be represented by a dedicated installer named `ppa`.
Do not overload `apt`, because `apt` means installing an already configured
package. Do not overload `repo_package`, because `repo_package` models explicit
keyring and sources-list materialization, while PPAs are configured through
Ubuntu's `add-apt-repository` workflow.

The `ppa` installer is outside the original seven-installer v1 set and should be
implemented as an explicit extension before `fish` and `ghostty` are moved to
PPA-based installation.

Required `ppa` params:

- `ppa`
- `package`

Optional `ppa` params:

- `bootstrap_package`, default `software-properties-common`

Execution semantics:

1. Validate the host is Ubuntu.
2. Ensure `software-properties-common` is installed, because minimal Ubuntu
   systems may not include `add-apt-repository`.
   - Check installed state with `dpkg -s <bootstrap_package>`.
   - If missing, install with `sudo apt-get install -y <bootstrap_package>`.
3. Run `sudo add-apt-repository -y <ppa>`.
4. Run `sudo apt-get update`.
5. Run `sudo apt-get install -y <package>`.

Installed-state detection should use the native package database for `package`,
matching the existing `apt` behavior.

The `ppa` installer must check `host.distro == "ubuntu"` directly. It must not
use the broader `distro_supported()` helper, because that helper also accepts
Debian.

### Distro-Specific Entries

Do not change the top-level manifest shape from
`deps.<tool>.linux.<arch>` in this change. Distro specificity should be expressed
inside the install entry with an optional `distros` array:

```toml
[deps.fish.linux.x86_64]
installer = "ppa"
version = "latest"
distros = ["ubuntu"]
```

`check` and dependency installation should treat a current-host entry as
matching only when:

- platform matches
- architecture matches
- `distros` is absent, or the detected Linux distro is listed

If `distros = []` is present, the entry matches no distro. `check` should treat
that as a valid but inactive entry, which means the dependency may fail with "no
current-host entry" for every Linux distro unless another matching entry exists.

`distros` is valid only for Linux entries. If a non-Linux entry declares
`distros`, `check` should reject it as a manifest error.

If no matching entry remains for the current distro, `check` should fail clearly
with a message that the dependency has no current-host entry for the detected
distro. The error should include the detected distro value, for example:

```text
dependency fish has no current-host entry for distro debian
```

This lets Ubuntu use PPA entries while Debian support is deferred without adding
a new manifest nesting level such as `linux.ubuntu.x86_64`.

## Required Backend Change

`official_script.args` should support expanding arguments that start with `~/`
before they are passed to the downloaded script.

This is needed for tools such as `starship`, whose official installer supports
installing to a user directory with:

```sh
sh install.sh -y -b ~/.local/bin
```

The backend should expand only leading `~/` in argument values. It should not
perform arbitrary shell expansion.

## Dependency Decisions

### `git`

- Linux installer: `apt`
- Version: `latest`
- Package: `git`
- Applies to: Ubuntu and Debian

Rationale: foundational tool; apt version is sufficient.

### `make`

- Add to `deps.toml`
- Linux installer: `apt`
- Version: `latest`
- Package: `make`
- Applies to: Ubuntu and Debian
- macOS: do not manage in deps for now

Rationale: project entry point; apt installation is simple and stable.

`make` in `deps.toml` is a steady-state presence guarantee, not the bootstrap
entry mechanism. A user still needs `make` available before running Makefile
targets. Once `dotman` is running, the `make` dependency can verify or install
`make` on Linux systems where it is missing.

### `tmux`

- Linux installer: `apt`
- Version: `latest`
- Package: `tmux`
- Applies to: Ubuntu and Debian

Rationale: foundational terminal tool; apt version is sufficient for v1.

### `fish`

- Linux installer: `ppa`
- PPA: `ppa:fish-shell/release-4`
- Version: `latest`
- Package: `fish`
- Applies to: Ubuntu only
- Required manifest filter: `distros = ["ubuntu"]`
- Debian: deferred

Reference command:

```sh
sudo apt-add-repository ppa:fish-shell/release-4
sudo apt update
sudo apt install fish
```

### `ghostty`

- Linux installer: `ppa`
- PPA: `ppa:mkasberg/ghostty-ubuntu`
- Version: `latest`
- Package: `ghostty`
- Applies to: Ubuntu only
- Required manifest filter: `distros = ["ubuntu"]`
- Debian: deferred

Reference command:

```sh
sudo add-apt-repository ppa:mkasberg/ghostty-ubuntu
sudo apt update
sudo apt install ghostty
```

### `nvim`

- Add to `deps.toml`
- Linux installer: `download_binary`
- Source: official Neovim GitHub release
- Version: fixed explicit version
- SHA256: required
- Applies to: Linux architectures with official release artifacts
- macOS installer: `brew`

Do not use Ubuntu/Debian apt for `nvim`.

### `yazi`

- Add to `deps.toml`
- Linux installer: `download_binary`
- Source: official Yazi GitHub release
- Version: fixed explicit version
- SHA256: required
- Install only the main `yazi` program in this version
- macOS installer: `brew`

Do not install optional enhancement dependencies in this change.

### `zoxide`

- Add to `deps.toml`
- Linux installer: `official_script`
- Script URL:
  `https://raw.githubusercontent.com/ajeetdsouza/zoxide/main/install.sh`
- Version: `latest`
- Args: none
- Installed-state marker: `~/.local/bin/zoxide`
- macOS installer: `brew`

The upstream install script defaults to installing the binary under
`$HOME/.local/bin`, so no args are needed.

### `starship`

- Add to `deps.toml`
- Linux installer: `official_script`
- Script URL: `https://starship.rs/install.sh`
- Version: `latest`
- Args: `["-y", "-b", "~/.local/bin"]`
- Installed-state marker: `~/.local/bin/starship`
- macOS installer: `brew`

Requires the backend `official_script.args` `~/` expansion described above.

### `eza`

- Add to `deps.toml`
- Linux installer: `download_binary`
- Source: official eza GitHub release
- Version: fixed explicit version
- SHA256: required
- macOS installer: `brew`

Do not use the documented `deb.gierens.de` apt repository in this version
because its repo URL is HTTP and conflicts with the repository HTTPS policy.
Do not use `cargo install` as the default installer.

### `lazygit`

- Add to `deps.toml`
- Linux installer: `download_binary`
- Source: official lazygit GitHub release
- Version: fixed explicit version
- SHA256: required
- macOS installer: `brew`

### `fzf`

- Add to `deps.toml`
- Linux installer: `download_binary`
- Source: official fzf GitHub release
- Version: fixed explicit version
- SHA256: required
- macOS installer: `brew`

Only install the main `fzf` binary in this version. Shell integration remains
configuration-owned.

## Verification

Before implementation, create an implementation plan that identifies:

- exact pinned versions for `nvim`, `yazi`, `eza`, `lazygit`, and `fzf`
- `version_check` args, stream, and regex for every dependency that uses a fixed
  version
- exact Linux artifact names for `x86_64` and `arm64`
- SHA256 checksums for each pinned artifact
- `ppa` installer implementation details and tests
- `distros` entry filtering implementation details and tests
- `InstallEntry` structure update for the optional `distros` field
- `Installer` enum update for the `Ppa` variant
- `check` logic for rejecting `distros` on non-Linux entries
- `check` and bootstrap/install selection logic for distro-filtered entries
- how unsupported Debian entries fail clearly

Implementation should not start until the plan has been reviewed.
