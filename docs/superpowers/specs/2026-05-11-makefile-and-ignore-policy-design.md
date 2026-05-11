# Makefile and Ignore Policy Design

## Goal

Improve local project ergonomics with discoverable Makefile targets and a sane
Git ignore policy.

## Non-Goals

- Do not refactor `dotman` runtime behavior.
- Do not change installer behavior.
- Do not expand the backend optimization roadmap here.

## Make Targets

These targets are project-operation entry points owned by the Makefile. They do
not expand the `dotman` backend CLI or runtime scope.

`make` and `make help` should print a concise command list. `make` should use
`help` as its default goal. The output should include the current operational
commands and the developer checks:

- `make build`
- `make bootstrap`
- `make link`
- `make link DRY_RUN=1`
- `make link CONFLICT=fail`
- `make link CONFLICT=backup`
- `make link CONFLICT=overwrite`
- `make doctor`
- `make check`
- `make lint`
- `make test`
- `make ci`

`make help` should list only public user/developer targets in the order shown
above. It should not list internal Makefile implementation targets such as
`build-dotman` or `cargo-preflight`. It should stay a concise command list;
workflow guidance belongs in the README, not in terminal help output.

`make lint` should run formatting and static analysis:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
```

`make lint` requires the Rust `rustfmt` and `clippy` components. If either
component is missing, the command should fail with Cargo's normal error output;
the README should list these as development requirements.

`make ci` should run the local verification suite in this order:

```sh
make lint
make check
make test
```

`make lint` is intentionally first so static failures stop the pipeline before
running the slower read-only and test steps.

`make test` should run:

```sh
cargo test
```

`make check` remains the read-only manifest and host-support check implemented
by `dotman`.

`make ci` should not run `make doctor`. `doctor` validates current-machine
state, so it remains a manual diagnostic command rather than part of stable
local CI. `make ci` should also not run `make link DRY_RUN=1` because dry-run
results depend on current home-directory state.

The Makefile should declare the new targets as phony and make `help` the
default goal:

```make
.PHONY: help bootstrap link doctor check lint test ci build build-dotman cargo-preflight
.DEFAULT_GOAL := help
```

## Git Ignore Policy

Machine-generated or machine-local state should not be tracked. Stable,
intentional configuration should be tracked.

`config/fish/fish_variables` is fish universal-variable state. It can contain
absolute paths, local tool ordering, and UI preferences. The current ignore
entry `.config/fish/fish_variables` does not match the repository path and
must be replaced with `config/fish/fish_variables`.

The tracked `config/fish/fish_variables` file should be removed from the Git
index while preserving the local file on disk.

Neovim `lazy-lock.json` should remain ignored. It is machine-generated plugin
resolution state for this repository and should not be committed in v1.

Editor and OS logs should stay ignored:

- `.DS_Store`
- `.nvimlog`
- `nvim.log`
- `__pycache__/`
- `target/`

## Verification

This change is verified locally with:

```sh
make help
make link DRY_RUN=1
make lint
make test
make ci
test -f config/fish/fish_variables
! git ls-files --error-unmatch config/fish/fish_variables
```

The README command list and development dependency section should be updated in
the same change so the documented user entry points match the Makefile.

The README should also explain when to use the commands, not only list what each
target does. It should include these usage workflows:

- first-time setup
- safe dry-run workflow
- recovery / rollback workflow

The workflow documentation should show concrete command order instead of making
users infer it from the command list. For example:

```sh
make build
make check
make link DRY_RUN=1
make link CONFLICT=backup
make doctor
```

The first-time setup section should document two paths:

- fast path: `make bootstrap`
- cautious path:

```sh
make build
make check
make link DRY_RUN=1
make link CONFLICT=backup
make doctor
```

The recovery / rollback section should be scoped to dotfile link recovery only:

- `make link CONFLICT=backup` creates backups for conflicting targets
- users can manually restore from those backup files if the link result is not
  desired
- there is no automatic rollback in v1
- dependency installation side effects from `brew`, `apt`, or other installers
  are not rolled back by this workflow
