# dotfiles

Personal dotfiles managed by `make` and the internal Rust backend `dotman`.

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
- `make check`: validate manifests and host support.
- `make lint`: run formatting and static analysis checks.
- `make test`: run Rust tests.
- `make ci`: run local verification (`lint` -> `check` -> `test`).

## Development Dependencies

- Rust toolchain with `cargo`, `rustfmt`, and `clippy`
- GNU Make
- Git

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

This project does not provide automatic rollback in v1. Dependency installation
side effects from package managers are not rolled back by this workflow.
