# Platform Support

## Policy

**Supported platforms**: macOS (arm64 + x86_64) and Linux (x86_64, glibc-based
distributions).

**Intentionally unsupported**: Windows. Dotman relies on Unix-specific
filesystem operations (symlinks, file permissions, process exit status codes).
Windows support would require a separate, scoped compatibility epic — it is
not an accidental omission.

**Untested**: non-glibc Linux (musl), BSD, WSL1. These may work but are not
validated.

## Platform-Specific Code

All Unix-only operations are guarded with `#[cfg(unix)]`. Non-unix fallbacks
use `#[cfg(not(unix))]`.

| File | Line(s) | Usage | Guard |
|------|---------|-------|-------|
| `src/installers.rs` | 7 | `use std::os::unix::fs::PermissionsExt` | Unguarded (module-level import) |
| `src/installers.rs` | 191 | `set_permissions` via `PermissionsExt` | `#[cfg(unix)]` |
| `src/installers.rs` | 274 | Symlink install | `#[cfg(unix)]` |
| `src/installers.rs` | 302 | Symlink reinstall | `#[cfg(unix)]` |
| `src/installers.rs` | 436–444 | `std::os::unix::fs::symlink` call | `#[cfg(unix)]` |
| `src/installers.rs` | 446 | `install_binary` fallback | `#[cfg(not(unix))]` |
| `src/installers.rs` | 570 | Symlink for download_binary | `#[cfg(unix)]` |
| `src/installers.rs` | 768 | Test: symlink install | `#[cfg(unix)]` |
| `src/installers.rs` | 783 | Test: symlink reinstall | `#[cfg(unix)]` |
| `src/installers.rs` | 793–813 | Test: symlink setup + assertions | `#[cfg(unix)]` |
| `src/installers.rs` | 830 | Test: permissions check | `#[cfg(unix)]` |
| `src/installers.rs` | 867 | Test: permissions for download_binary | `#[cfg(unix)]` |
| `src/installers.rs` | 899–915 | Test: symlink + permissions | `#[cfg(unix)]` |
| `src/installers.rs` | 964 | Test: permissions edge case | `#[cfg(unix)]` |
| `src/installers.rs` | 999 | Test: permissions edge case | `#[cfg(unix)]` |
| `src/installers.rs` | 1172 | Test: atomic install permissions | `#[cfg(unix)]` |
| `src/installers.rs` | 1210 | Test: atomic install permissions | `#[cfg(unix)]` |
| `src/installers.rs` | 1228–1230 | Test: symlink + permissions | `#[cfg(unix)]` |
| `src/installers.rs` | 1253 | Test: permissions edge case | `#[cfg(unix)]` |
| `src/installers.rs` | 1345 | Test: permissions edge case | `#[cfg(unix)]` |
| `src/link.rs` | 5 | `use std::os::unix::fs as unix_fs` | Unguarded (module is inherently unix-only) |
| `src/link.rs` | 113, 118, 125 | `unix_fs::symlink` calls | Unguarded (module is inherently unix-only) |
| `src/process.rs` | 44 | `use std::os::unix::process::ExitStatusExt` | Guarded by `#[cfg(test)]` |
| `src/platform.rs` | 26, 42–43, 54, 73 | Platform detection: mac/linux only | Implicit (no Windows path) |
| `src/update.rs` | 7, 10, 47, 50 | Iterates `["mac", "linux"]` | Implicit |
| `src/config.rs` | 23, 46 | `linux` field, platform dispatch | Implicit |
| `src/check.rs` | 112, 128 | Validates linux-only installers | Implicit |

### `src/link.rs` is Inherently Unix-Only

The entire `link` module exists to manage symlinks. Symlinks are a Unix
concept. The module does not need internal `#[cfg(unix)]` guards because it
makes no sense to compile it on non-unix targets. If Windows support is ever
added, this module will need a complete rework or a Windows-specific
counterpart.

## Convention

When adding new platform-specific code:

- Use `#[cfg(unix)]` for Unix-only paths (symlinks, permissions, signals).
- Provide a `#[cfg(not(unix))]` fallback or `compile_error!` for non-unix
  targets if the operation is essential.
- Prefer `#[cfg(unix)]` over `#[cfg(target_os = "macos")]` or
  `#[cfg(target_os = "linux")]` unless the logic genuinely differs between
  macOS and Linux.
- Add new platform-specific sections to the audit table above.

## Future

Windows support is intentionally out of scope for the current project.
