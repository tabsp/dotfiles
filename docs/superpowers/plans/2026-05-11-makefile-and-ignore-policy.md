# Makefile and Ignore Policy Implementation Plan

**Spec:** `docs/superpowers/specs/2026-05-11-makefile-and-ignore-policy-design.md`

**Goal:** Implement discoverable Makefile project targets, update ignore policy
for machine-local files, and refresh README usage guidance without changing
`dotman` runtime behavior.

## Task 1: Update Makefile Entry Points

**Files:**

- Modify: `Makefile`

- [ ] Add the new public targets to `.PHONY`: `help`, `lint`, `test`, and
  `ci`.
- [ ] Add `.DEFAULT_GOAL := help`.
- [ ] Implement `help` as a concise public command list.
- [ ] Include only public user/developer targets in help output:
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
- [ ] Do not list internal targets such as `build-dotman` or `cargo-preflight`.
- [ ] Add `lint`:

```make
lint: cargo-preflight
	cargo fmt --check
	cargo clippy --all-targets --all-features -- -D warnings
```

- [ ] Add `test`:

```make
test: cargo-preflight
	cargo test
```

- [ ] Add `ci` in stable order:

```make
ci: lint check test
```

- [ ] Ensure `ci` does not run `doctor`.
- [ ] Ensure `ci` does not run `link DRY_RUN=1`.

## Task 2: Update Git Ignore Policy

**Files:**

- Modify: `.gitignore`
- Remove from Git index only: `config/fish/fish_variables`

- [ ] Replace the stale ignore entry:

```gitignore
.config/fish/fish_variables
```

with:

```gitignore
config/fish/fish_variables
```

- [ ] Keep `lazy-lock.json` ignored.
- [ ] Preserve existing editor/build ignores:
  - `.DS_Store`
  - `.nvimlog`
  - `nvim.log`
  - `__pycache__/`
  - `target/`
- [ ] Remove `config/fish/fish_variables` from the Git index while preserving
  the local file:

```sh
git rm --cached config/fish/fish_variables
```

Expected:

- `config/fish/fish_variables` still exists on disk.
- `git ls-files --error-unmatch config/fish/fish_variables` exits non-zero.

## Task 3: Update README Usage Guidance

**Files:**

- Modify: `README.md`

- [ ] Add `make help`, `make lint`, `make test`, and `make ci` to the command
  list.
- [ ] Update development dependencies to include:
  - Rust toolchain with `cargo`
  - Rust `rustfmt` component
  - Rust `clippy` component
  - GNU Make
  - Git
- [ ] Keep README focused on user/developer workflows; do not document
  `fish_variables` ignore internals.
- [ ] Add `First-time setup` with two paths:

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

- [ ] Add `Safe dry-run workflow`:

```sh
make build
make check
make link DRY_RUN=1
```

- [ ] Add `Recovery / rollback` scoped to dotfile links:
  - `make link CONFLICT=backup` creates backups for conflicting targets.
  - Users can manually restore from backup files if needed.
  - v1 does not provide automatic rollback.
  - Dependency installation side effects from package managers are not rolled
    back.

## Task 4: Verify

Run:

```sh
make help
make link DRY_RUN=1
make lint
make test
make ci
test -f config/fish/fish_variables
! git ls-files --error-unmatch config/fish/fish_variables
```

Expected:

- `make help` exits 0 and lists only public targets.
- `make link DRY_RUN=1` exits according to current link state and prints dry-run
  output.
- `make lint` exits 0.
- `make test` exits 0.
- `make ci` exits 0.
- `config/fish/fish_variables` remains on disk.
- `config/fish/fish_variables` is no longer tracked by Git.

## Task 5: Commit

- [ ] Review `git status --short`.
- [ ] Ensure no unrelated local changes are staged, especially user-local
  changes outside this task.
- [ ] Commit the Makefile, README, `.gitignore`, and Git index update together.
