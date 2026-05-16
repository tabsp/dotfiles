# Agent Handoff

## Current Epic

P0 - Atomic Directory Install

## Phase

done

## Exception Reason

- None.

## Completed

Refactored `install_archive_dir` in `src/installers.rs` to use atomic staging-then-rename instead of remove-then-copy. Added `sibling_tempdir` helper. Added 5 unit tests covering first-time install, upgrade, staging failure recovery, leftover backup cleanup, and intermediate directory creation. Added `AGENT_ARCHIVE_DIR_*` error codes to all failure paths.

## Verification

- `cargo test atomic_install` passed: 5 tests passed
- `cargo test` passed: 107 tests passed, zero failures
- `cargo clippy` passed: zero warnings

## Modified Files

- `src/installers.rs`: refactored `install_archive_dir` for atomic rename, added `sibling_tempdir` helper, added 5 unit tests, added error code prefixes.
- `docs/roadmap.md`: linked spec and plan, updated status.
- `docs/superpowers/specs/2026-05-16-p0-atomic-directory-install-design.md`: new spec.
- `docs/superpowers/plans/2026-05-16-p0-atomic-directory-install.md`: new plan.

## Unresolved Risks

- None.

## Next Step

Proceed to P0 - Verified Extraction Pipeline.
