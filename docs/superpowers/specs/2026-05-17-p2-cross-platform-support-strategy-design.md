# Cross-Platform Support Strategy Design

## Goal

Define an explicit platform support policy for dotman and document all
platform-specific code paths so future contributors know what is deliberate
vs. accidental.

## Scope

- Document the platform support policy: macOS and Linux are supported;
  Windows is intentionally unsupported.
- Audit all `#[cfg(unix)]` guards and `std::os::unix` usage in `src/`.
- Identify any code that would fail to compile or run on non-unix targets.
- Add a `docs/platform-support.md` document that explains the policy and
  maps out each platform-specific code section.
- Ensure the current `#[cfg(not(unix))]` fallback in `src/installers.rs` is
  explicit and intentional.

## Non-Goals

- Do not add Windows support.
- Do not add CI matrix builds for multiple platforms.
- Do not modify runtime behavior.

## Design

### Platform Policy

- **Supported**: macOS (arm64 + x86_64), Linux (x86_64, glibc-based distros).
- **Intentionally unsupported**: Windows. The codebase uses Unix-specific
  filesystem operations (symlinks, permissions, exit status codes). Adding
  Windows support would be a separate, scoped epic.
- **Untested**: non-glibc Linux, BSD, WSL1. May work but no guarantees.

### Code Audit

| File | Unix-specific usage | Guarded? |
|------|-------------------|----------|
| `src/installers.rs` | `std::os::unix::fs::PermissionsExt`, `symlink` | Yes (`#[cfg(unix)]`) |
| `src/installers.rs` | Fallback `install_binary` for non-unix | Yes (`#[cfg(not(unix))]`) |
| `src/link.rs` | `std::os::unix::fs::symlink` | No — compiles only on unix |
| `src/process.rs` | `std::os::unix::process::ExitStatusExt` | Guarded by `#[cfg(test)]` |
| `src/platform.rs` | Platform detection for mac/linux only | Implicit (no Windows path) |
| `src/update.rs` | Iterates `["mac", "linux"]` | Implicit |
| `src/config.rs` | `linux` field, `matches_distro` | Implicit |
| `src/check.rs` | Validates linux-only installers | Implicit |

### Action Items

1. Add `docs/platform-support.md` with:
   - Supported platform policy statement
   - Table of platform-specific code with file + line references
   - Guidance for adding new platform-specific code
   - Note that `src/link.rs` is inherently Unix-only (no cfg guard needed
     since the module is meaningless on non-unix)
   - Note that Windows support is a separate future epic

2. Link from `README.md` and `docs/roadmap.md`.

### Verification

- `cargo build` succeeds on the current (macOS) host.
- `cargo test` passes.
- `cargo clippy` passes.
- Existing `#[cfg(unix)]` guards are not modified.

## Error Handling

- Missing or incomplete `docs/platform-support.md` is a documentation gap, not
  a runtime error — no error codes needed.
- If the audit table becomes stale, `agent-check` will not catch it (manual
  review required).

## Verification Strategy

- `cargo test` — all existing tests pass (no new tests needed; docs-only change).
- `cargo clippy` — zero warnings.
- `make check` — manifest validation passes.
- Manual review of `docs/platform-support.md` for accuracy.

## Regression Coverage Expectations

- All existing `#[cfg(unix)]` guards remain unchanged.
- `cargo build` continues to succeed on macOS.
- No runtime behavior changes — this is a documentation-only epic.
