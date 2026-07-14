# Repository Guidelines

## Repository Scope

This repository contains two related parts:

- `dotman`: a Rust CLI/TUI for installing and deploying dotfiles.
- Personal configuration under `config/`, deployed through `dotman.yaml`.

Treat the repository files as the source of truth. Do not edit deployed files under
`~/.config` or other home-directory targets directly.

## Safety and User State

- Inspect `git status --short` before editing.
- Preserve unrelated and pre-existing user changes.
- Never revert, overwrite, or reformat unrelated changes without confirmation.
- Do not run deployment, bootstrap, installation, self-update, or login-shell
  changing commands unless the user explicitly requests them.
- In particular, do not run `make deploy`, `dotman deploy`, `fisher update`,
  `mise install`, or Mason installers as an incidental validation step.
- Do not commit, push, tag, or publish a release unless explicitly requested.
- Never add credentials, tokens, SSH keys, host-specific paths, or private
  machine configuration to the repository.
- Machine-local Fish configuration belongs in `~/.config/fish-local/*.fish`.

## Sources of Truth

- `Makefile`: supported development, formatting, and validation commands.
- `dotman.yaml`: packages, links, directories, and deployment actions.
- `src/ops/db.toml`: package installation metadata.
- `config/`: shared application configuration.
- `README.md` and `README.zh-CN.md`: user-facing behavior and workflows.
- `docs/new-machine.md`: new-machine bootstrap procedure.

Prefer updating the source of truth instead of duplicating information elsewhere.

## Development Workflow

Use Makefile targets for recurring operations.

```sh
make build
make format
make format-check
make lint
make test
make nvim-check
make config-check
make ci
```

Before running repository-wide `make format`, inspect the worktree because it can
rewrite all supported tracked files. Use scoped formatters when unrelated user
changes must remain untouched.

After formatting, review `git diff` again before staging or committing.

## Validation by Change Area

Run the narrowest relevant validation while developing:

- Rust code: `make rust-lint && make test`
- Neovim configuration: `make nvim-check`
- Fish configuration: `make fish-check`
- Shell scripts: `make shell-lint`
- Docker E2E files: `make docker-lint`
- GitHub workflows: `make action-lint`
- Deployment, tmux, or cross-configuration changes: `make config-check`
- Repository-wide handoff: `make ci && make config-check`

If a required local tool is unavailable, report the missing validation instead
of silently skipping it.

## Rust Architecture

- Keep reusable application logic in `src/lib.rs` and feature-oriented modules.
- Keep `src/main.rs` focused on CLI startup and top-level command dispatch.
- Place installation, linking, cleaning, creation, and shell operations under
  `src/ops/`.
- Preserve the Plan -> Review -> Run model and headless/TUI behavioral parity.
- Add regression tests near the affected code. Both in-module unit tests and
  integration tests under `tests/` are valid.
- Keep filesystem and process tests isolated with temporary directories.
- Preserve useful error context and non-zero exit behavior for failed or
  aborted headless operations.

## Configuration Changes

- Keep shared configuration portable across supported macOS and Linux systems.
- Keep deployment shell commands idempotent and guarded with appropriate `if`
  conditions.
- Do not assume Homebrew uses the same prefix on macOS and Linux.
- Treat `Cargo.lock` and `config/nvim/lazy-lock.json` as tracked lock files.
  Update their contents only when dependency or plugin resolution changes;
  formatting-only changes are acceptable when produced by repository formatters.
- Validate Neovim behavior headlessly instead of relying only on interactive
  startup.
- Keep machine-specific overrides outside the repository.

## Documentation

- Keep equivalent user-facing changes synchronized between `README.md` and
  `README.zh-CN.md`.
- Update `docs/new-machine.md` when bootstrap, deployment, private configuration,
  or first-run requirements change.
- Document behavior and supported workflows, not transient implementation details.

## Git Conventions

Use Conventional Commits:

```text
<type>(<optional-scope>): <imperative description>
```

Common types are `feat`, `fix`, `docs`, `refactor`, `test`, `ci`, and `chore`.
Use a scope such as `nvim`, `fish`, `tmux`, or `tui` when the change is confined
to one area.
