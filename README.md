# dotfiles

Personal dotfiles managed by `dotman`, a safety-first bootstrap manager for
macOS and Linux.


## Install

### Quick install (recommended)

Download and install `dotman` without cloning the repo:

```sh
curl -fsSL https://raw.githubusercontent.com/tabsp/dotfiles/main/scripts/install.sh | sh
```

Prerequisites: `curl`, `tar`, and either `shasum` (macOS) or `sha256sum` (Linux).
No Rust toolchain required.

To install a specific version, set the `DOTMAN_VERSION` environment variable:

```sh
curl -fsSL https://raw.githubusercontent.com/tabsp/dotfiles/main/scripts/install.sh | DOTMAN_VERSION=0.2.0 sh
```

If `DOTMAN_VERSION` is not set, the installer defaults to `0.1.0`.
All downloads are checksum-verified before installation.

The installer also downloads the matching dotfiles source into
`~/.local/share/dotman/dotfiles`. Run `dotman bootstrap` from that directory:

```sh
cd ~/.local/share/dotman/dotfiles
dotman bootstrap
```

### Build from source

Requires Rust and Cargo. See [Rust installation](https://rustup.rs/).

## Commands

- `make help`: print the available public commands.
- `make build`: build the Rust backend without changing machine state.
- `make bootstrap`: build `dotman`, check manifests, install missing supported dependencies, link dotfiles, and run doctor.
- `make link`: link managed files from `dotfiles.toml`.
- `make link DRY_RUN=1`: preview link actions.
- `make link CONFLICT=fail`: fail on target conflicts.
- `make link CONFLICT=backup`: back up target conflicts before linking.
- `make link CONFLICT=overwrite`: overwrite target conflicts before linking.
- `make doctor`: inspect installed commands, versions, and linked files.
- `make shell`: interactively set fish as the login shell.
- `make check`: validate manifests and host support.
- `make update-deps-list`: list pinned download_binary deps
- `make update-deps-check`: check for newer GitHub releases
- `make lint`: run formatting and static analysis checks.
- `make test`: run Rust tests.
- `make ci`: run local verification (`lint` -> `check` -> `test`).
- `make release-check`: build the current host release artifact and verify its checksum.

### Release artifacts

`make release-check` builds and verifies the current host artifact locally.
For a tagged release, run the **Release Artifacts** GitHub Actions workflow with
the release tag to build the supported macOS and Linux tarballs on native
runners and optionally attach them to the GitHub Release.

## Development Dependencies

- Rust toolchain with `cargo`, `rustfmt`, and `clippy`
- C compiler/linker for Rust crates with native dependencies
- GNU Make
- Git

These are bootstrap prerequisites: they must exist before `make bootstrap` can
build and run `dotman`. Dependencies in `deps.toml` are managed after the Rust
backend has already been built.

On Ubuntu/Debian, install the non-Rust build prerequisites first:

```sh
sudo apt-get install -y git make build-essential
```

On macOS, install Xcode Command Line Tools first:

```sh
xcode-select --install
```

Install Rust with rustup, then make sure `cargo`, `rustc`, `rustfmt`, and
`clippy` are available on `PATH`.

## Layout

- `config/`: source dotfiles tracked by this repo
- `dotfiles.toml`: managed file mappings
- `deps.toml`: dependency installer policy
- `docs/manifest-schema.md`: authoritative schema reference for `deps.toml` and `dotfiles.toml`
- `docs/platform-support.md`: platform support policy and Unix-specific code audit
- `docs/release-policy.md`: versioning, compatibility, and release process
- `CHANGELOG.md`: project changelog following Keep a Changelog
- `docs/recovery.md`: cleanup, backup, and uninstall procedures
- `src/`: Rust backend source
- `tests/`: CLI integration tests

## Current Scope

The Rust backend supports these dependency installers:

- `system`
- `brew`
- `cask`
- `apt`
- `repo_package`
- `official_script`
- `download_binary`

## First-Time Setup

Fast path:

```sh
make bootstrap
```

`make bootstrap` runs `doctor` in the same process environment after installing
dependencies. `dotman` searches `$HOME/.local/bin` and `$HOME/.cargo/bin`
directly, so newly installed user-local tools can be verified before opening a
new fish shell. It does not change your login shell; switch shells manually
after bootstrap if desired.

To set fish as the default login shell after bootstrap:

```sh
make shell
```

This command prints the `chsh` command it will run and requires interactive
confirmation. Non-interactive runs fail with the equivalent manual command.

Cautious path:

```sh
make build
make check
make link DRY_RUN=1
make link CONFLICT=backup
make doctor
```

## Safe Dry-Run Workflow

```sh
make build
make check
make link DRY_RUN=1
```

## Recovery / Rollback

`make link CONFLICT=backup` creates backup files for conflicting link targets.
If link results are not desired, restore from those backup files manually.

Automatic rollback is not currently supported. Dependency installation side
effects from package managers are not rolled back by this workflow.
