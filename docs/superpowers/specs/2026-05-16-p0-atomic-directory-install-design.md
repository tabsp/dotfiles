# Atomic Directory Install Design

## Goal

Make directory-based binary installs atomic. Currently `install_archive_dir` uses
a remove-then-copy pattern: delete the old directory first, then copy the new
one. If the copy fails midway, a previously working install is left broken.

## References

Primary references:

- Rust `std::fs::rename` behavior on Unix: atomic when source and destination
  are on the same filesystem.
- `tempfile` crate for temporary directory creation in the same parent
  directory, ensuring same-filesystem renames.
- Existing code in `src/installers.rs`: `install_archive_dir` (line 310) and
  `copy_dir_recursive` (line 385).

## Scope

- Replace the non-atomic remove-then-copy pattern in `install_archive_dir` with
  a staging-and-rename approach.
- Stage new directory content in a sibling temp directory to enable atomic
  `rename`.
- Preserve the old directory as a `.old` backup during the transition; remove
  it only after the new install is in place.
- Handle the binary symlink atomically: recreate it after the directory rename
  completes.
- Keep `copy_dir_recursive` unchanged (it handles the actual file-level copy).
- Add unit tests covering: first-time install, upgrade, mid-copy failure
  recovery, and `install_dir_to` parent creation.

## Non-Goals

- Do not change the single-binary (non-directory) install path.
- Do not change `cleanup_temp` or the archive extraction pipeline.
- Do not make `copy_dir_recursive` atomic (it operates on temp-to-temp copies).
- Do not change the `manifest.json` schema or config format.

## Design

### Current behavior

1. Create parent directory for `install_dir_to`.
2. If `install_dir_to` exists, `remove_dir_all` it.
3. `copy_dir_recursive(source_dir, install_dir_to)`.
4. Remove old `install_to` (symlink).
5. Create new symlink `install_to -> install_dir_to/binary_path`.

Problem: step 2 destroys the old install before step 3 can succeed.

### Target behavior

The install follows a staged-rename pattern:

1. Create parent directory for `install_dir_to` if needed.
2. Determine a sibling staging path: `install_dir_to.staging-<random>`.
3. `copy_dir_recursive(source_dir, staging_path)`.
4. If `install_dir_to` exists, `rename(install_dir_to -> install_dir_to.old)`.
5. `rename(staging_path -> install_dir_to)`.
6. Remove old symlink at `install_to` if it exists.
7. Create new symlink `install_to -> install_dir_to/binary_path`.
8. If `install_dir_to.old` exists, `remove_dir_all(install_dir_to.old)`.

### Failure semantics

- If step 3 fails: staging path is cleaned up; old install remains usable.
- If step 4 fails (e.g., permission denied on rename): staging path cleaned up;
  old install remains usable.
- If step 5 fails: attempt to restore `install_dir_to.old` back to
  `install_dir_to`; if restoration fails, leave a clear error message with both
  paths.
- If step 6-7 fails: new directory is already in place, but symlink is
  missing/stale. This is a degraded but non-destructive state. Raise an error
  so the agent or user can retry.
- If step 8 fails: non-fatal; log a warning that `.old` could not be cleaned.

### Temp directory naming

Use `format!("{}.staging-{}", install_dir_to_filename, random_suffix)` so the
staging directory is a sibling of `install_dir_to` in the same parent
directory. This guarantees `rename` is atomic on Unix.

The `random_suffix` can use a simple counter or a short random string to avoid
collisions with leftover staging directories from previous failed runs.

## API Changes

`install_archive_dir` signature stays the same. Internal implementation changes
as described above.

## Error Handling

- `AGENT_ARCHIVE_DIR_STAGE_FAILED`: staging copy failed, old install preserved.
- `AGENT_ARCHIVE_DIR_RENAME_FAILED`: rename of old directory failed, staging
  cleaned up.
- `AGENT_ARCHIVE_DIR_PROMOTE_FAILED`: promote (rename staging → final) failed,
  attempt restoration from `.old`.
- `AGENT_ARCHIVE_DIR_SYMLINK_FAILED`: directory installed but symlink update
  failed.
- `AGENT_ARCHIVE_DIR_CLEANUP_FAILED`: non-fatal warning when `.old` removal
  fails.

## Testing

- Test first-time install (no existing `install_dir_to`).
- Test upgrade (existing `install_dir_to` present).
- Test recovery when the staging copy fails (simulated via read-only parent).
- Test that `.old` backup is cleaned up on success.
- Test that symlink points to the correct path after upgrade.

## Open Questions

None.

## Verification Strategy

- `cargo test atomic_install` — new atomic install unit tests
- `cargo test` — full test suite
- `cargo clippy -- -D warnings` — no new warnings
