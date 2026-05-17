# Agent Instructions

## Project Context

This repository contains personal dotfiles and the Rust backend `dotman`.
Before planning or implementation, read:

1. `README.md`
2. `docs/roadmap.md`
3. Relevant specs under `docs/superpowers/specs/`
4. Relevant implementation plans under `docs/superpowers/plans/`

## Working Rules

- Follow the priority rules in `docs/roadmap.md`.
- Work on one roadmap epic at a time.
- Do not turn roadmap items directly into code. Create or update a spec or plan
  first unless the user explicitly asks for a small direct edit.
- Preserve existing user changes. Do not revert unrelated files.
- Prefer small, verifiable changes.


## Agent Harness

Before starting any roadmap work, use the deterministic harness commands to
select and lock the active epic:

```sh
make agent-init      # once per checkout: create state.toml
make agent-next      # print next eligible epic
make agent-start EPIC="P0 - ..."  # lock one epic
make agent-status    # confirm lock
```

Use `make agent-check` before claiming completion of any phase. Use
`make agent-advance` to move through the workflow (specified → planned →
in_progress → verifying). Use `make agent-set-roadmap-status STATUS=done` to update roadmap status (never use sed). Use `make agent-finish` only after recording
passing verification with `make agent-record-verification`.

The harness enforces:
- One active epic at a time
- Spec and plan artifacts before implementation
- P0 prerequisite ordering
- Structured handoff notes before finish

Runtime state lives in `docs/superpowers/agent/state.toml`. Handoff notes are tracked under `docs/superpowers/agent/handoffs/`.

## Commit After Each Epic

After `make agent-finish` completes successfully, commit all changes for that
epic before starting the next one. Use `make agent-set-roadmap-status` to update
the roadmap instead of manual sed edits. Run `git status` to verify the
working tree is clean before calling `make agent-next`.

## Commit Style

Use Conventional Commits for new commits:

- `feat:` for user-visible capabilities
- `fix:` for bug fixes
- `docs:` for documentation-only changes
- `test:` for test-only changes
- `refactor:` for behavior-preserving code changes
- `chore:` for maintenance

Use imperative, lowercase summaries after the prefix.

## Verification

Use the narrowest relevant command first, then run broader checks before claiming
completion.

Common commands:

```sh
cargo test <test_name> # Related test first
cargo test             # All Rust tests
make check             # Validate manifests and host support
make lint              # rustfmt and clippy checks
make ci                # Full local verification: lint -> check -> test
```

`make build` depends on `cargo-preflight`, which checks for the Rust toolchain
before building `dotman`.

Unit tests live alongside Rust modules in `src/*.rs`. CLI integration tests live
under `tests/`.

## Safety

This project manages machine state. Changes to dependency installation, archive
extraction, symlinks, shell configuration, or bootstrap behavior require tests
that cover failure paths.
