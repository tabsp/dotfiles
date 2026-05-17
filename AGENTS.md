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

## Roadmap Planning Review

If `make agent-next` reports no eligible roadmap epic, or if every selectable
queue item is `done` or `deferred`, do not force-start a completed epic.

Perform a read-only Roadmap Planning Review instead. Read the current roadmap,
README, relevant specs and plans, handoffs, release/install/recovery docs, CI,
and source code signals needed to verify roadmap state.

A Roadmap Planning Review may propose:

- next roadmap queue items
- completed item archival
- risk register entries
- deferred or non-goal items
- roadmap structure changes
- planning lessons for future reviews

It must not:

- implement code
- modify files unless the user explicitly requests a roadmap update
- run `agent-start`, `agent-advance`, or `agent-finish`
- convert roadmap items directly into implementation work

After the review, ask for confirmation before editing roadmap files or starting
normal roadmap execution.

## Multi-Agent Review

For roadmap refreshes and safety-sensitive epics, use multi-agent review before
writing the implementation spec or plan. Safety-sensitive epics include release
installation, bootstrap, dependency installation, archive extraction, symlinks,
remote scripts, cleanup, uninstall, recovery, and agent workflow changes.

Multi-agent review is a bounded planning review, not implementation. Reviewer
agents must be read-only unless the user explicitly authorizes file edits. Use
separate reviewer roles for safety/release, product/community, and
workflow/harness concerns. The coordinating agent must synthesize consensus,
disagreements, accepted changes, rejected changes, and risk register updates.

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
