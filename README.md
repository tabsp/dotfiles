# dotfiles

Personal dotfiles managed by `dotman`, a safety-first bootstrap manager for
macOS and Linux.

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
- `make lint`: run formatting and static analysis checks.
- `make test`: run Rust tests.
- `make ci`: run local verification (`lint` -> `check` -> `test`).

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
