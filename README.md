# dotfiles

Personal dotfiles managed by `make` and the internal Rust backend `dotman`.

## Commands

- `make build`: build the Rust backend without changing machine state.
- `make bootstrap`: build `dotman`, check manifests, install missing supported dependencies, link dotfiles, and run doctor.
- `make link`: link managed files from `dotfiles.toml`.
- `make link DRY_RUN=1`: preview link actions.
- `make link CONFLICT=fail`: fail on target conflicts.
- `make link CONFLICT=backup`: back up target conflicts before linking.
- `make link CONFLICT=overwrite`: overwrite target conflicts before linking.
- `make doctor`: inspect installed commands, versions, and linked files.
- `make check`: validate manifests and host support.

## Development Dependencies

- Rust toolchain with `cargo`
- GNU Make
- Git

## Layout

- `config/`: source dotfiles tracked by this repo
- `dotfiles.toml`: managed file mappings
- `deps.toml`: dependency installer policy
- `src/`: Rust backend source
- `tests/`: CLI integration tests

## Current Scope

The first runnable cut supports these dependency installers:

- `system`
- `brew`
- `cask`
- `apt`

These installer kinds remain deferred from execution in the current slice:

- `repo_package`
- `official_script`
- `download_binary`

## CI

CI is deferred from the first runnable slice. The first CI target should run `cargo test` and `make check` on macOS plus Ubuntu/Debian, covering both `arm64` and `x86_64` where runners are available.
