# Verified Extraction Pipeline Design

## Goal

Make the binary archive extraction pipeline explicit about each trust boundary:
manifest URL validation, final URL validation, checksum verification, archive
path safety, and link-entry behavior. Close path traversal and unsafe link gaps
in tar and zip extraction.

## References

Primary references:

- CWE-22: Improper Limitation of a Pathname to a Restricted Directory
  ("Path Traversal"), https://cwe.mitre.org/data/definitions/22.html
- `tar` crate `Archive::entries()` with manual entry validation as an
  alternative to `Archive::unpack()` which does not validate entry paths.
- `zip` crate `ZipArchive::by_index()` with `mangled_name()` — partial
  mitigation; needs additional path validation.
- Rust `std::path::Path::components()` for canonicalizing and validating paths.
- Existing code in `src/archive.rs` (`unpack`), `src/http.rs` (URL validation),
  `src/installers.rs` (`verify_sha256`, `install_download_binary`).

## Scope

- Add path traversal protection to tar extraction: validate each entry path
  resolves within the destination directory.
- Add path traversal protection to zip extraction: validate each entry path
  resolves within the destination directory, supplementing `mangled_name()`.
- Reject symlinks and hardlinks in extracted archives. If an upstream archive
  contains symlinks or hardlinks, fail with a clear error message.
- Document the extraction trust boundaries in a pipeline-level comment or
  module doc.
- Keep URL validation and checksum verification as-is (they already work).
- Add unit tests for malicious tar/zip archives with path traversal attempts
  and symlink entries.

## Non-Goals

- Do not change the `install_download_binary` caller flow.
- Do not add new archive formats.
- Do not change checksum or URL validation.
- Do not add payload allowlist/denylist for specific binary names (beyond
  the existing `binary_path` resolution).
- Do not change `copy_dir_recursive` (it operates on already-extracted,
  trusted temp content).

## Design

### Current behavior

`unpack` in `src/archive.rs`:

- **TarGz/TarXz**: calls `tar::Archive::unpack(dest)`. The tar crate's
  `unpack` method does NOT validate that entry paths stay within `dest`.
  A malicious archive with entries like `../../../.ssh/authorized_keys`
  would write outside the destination.
- **Zip**: uses `entry.mangled_name()` which strips leading `/` and `..`
  components, then joins with `dest`. This is a partial mitigation but
  `mangled_name()` may not handle all edge cases (e.g., Windows-style paths
  with backslashes, or deeply nested `..` patterns).
- **Raw**: writes directly to a fixed path; no traversal risk.
- No explicit symlink or hardlink policy. Both tar and zip silently extract
  symlinks and hardlinks if present.

### Target behavior

1. **Tar extraction**: Replace `archive.unpack(dest)` with manual
   `archive.entries()` iteration. For each entry:
   - Resolve the entry path relative to `dest`.
   - Validate it starts with `dest` (canonical or component-based check).
   - Reject symlink and hardlink entry types.
   - Extract regular files and directories normally.

2. **Zip extraction**: Keep `mangled_name()`, then add a post-join validation:
   - After joining with `dest`, verify the canonical path starts with `dest`.
   - Reject entries where `mangled_name()` produces an empty path (stripped
     to nothing).

3. **Link policy**: Both tar and zip extraction reject entries with
   `EntryType::Symlink`, `EntryType::Hardlink`, or equivalent. Error message
   includes the archive entry path.

4. **Error codes**:
   - `AGENT_EXTRACT_PATH_TRAVERSAL`: entry path escapes destination directory.
   - `AGENT_EXTRACT_SYMLINK_REJECTED`: symlink entry found in archive.
   - `AGENT_EXTRACT_HARDLINK_REJECTED`: hardlink entry found in archive.
   - `AGENT_EXTRACT_EMPTY_PATH`: entry resolves to empty or invalid path.

5. **Module documentation**: Add a doc comment at the top of `src/archive.rs`
   describing the extraction pipeline trust boundaries and safety guarantees.

## Error Handling

- Path traversal attempts fail with `AGENT_EXTRACT_PATH_TRAVERSAL` including
  the offending entry path.
- Symlink/hardlink entries fail with the corresponding error code and entry
  path.
- Empty or unresolvable entry paths fail with `AGENT_EXTRACT_EMPTY_PATH`.
- All errors are propagated through the existing `Result<_, String>` return
  type.

## Verification Strategy

- `cargo test archive` — new path safety and link rejection tests
- `cargo test` — full test suite, zero regressions
- `cargo clippy` — zero warnings
