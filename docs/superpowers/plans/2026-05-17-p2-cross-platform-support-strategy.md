# Cross-Platform Support Strategy Implementation Plan

**Spec:** `docs/superpowers/specs/2026-05-17-p2-cross-platform-support-strategy-design.md`

**Goal:** Document platform support policy, audit Unix-specific code, create `docs/platform-support.md`.

**Architecture:** Documentation-only. No runtime code changes.

**Tech Stack:** Markdown.

---

## Existing Code Map

- `src/installers.rs`: 19 `#[cfg(unix)]` guards + 1 `#[cfg(not(unix))]` fallback.
- `src/link.rs`: inherently Unix-only (symlink module).
- `src/process.rs`: `#[cfg(test)]` guarded Unix import.
- `src/platform.rs`, `src/update.rs`, `src/config.rs`, `src/check.rs`: implicit Unix-only platform support.
- `docs/roadmap.md`: P2 Cross-Platform Support Strategy entry.
- `README.md`: documentation index.

## Task 1: Create platform support documentation

**Files:**
- New: `docs/platform-support.md`
- Modify: `README.md`

- [ ] Write `docs/platform-support.md` with policy, audit table, and conventions.
- [ ] Link from `README.md` documentation index.
- [ ] Ensure all `#[cfg(unix)]` guards are accurately mapped in audit table.

## Verification Commands

- `cargo test`
- `cargo clippy`
- `make lint`
- `make check`

## Test Level

- No new tests (documentation-only change).
- Unit tests: `cargo test` (existing).
- CLI integration tests: `cargo test` (existing).

## Expected Outcomes

- `docs/platform-support.md` exists and is linked from `README.md`.
- All existing tests pass.
- `cargo clippy` zero warnings.
- `make agent-check` passes.

## Regression Coverage Expectations

- All 121 existing tests must continue to pass.
- No `#[cfg(unix)]` guards modified.
- No runtime behavior changes.
- `cargo build` succeeds on macOS.
