# Verified Extraction Pipeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** `docs/superpowers/specs/2026-05-17-p0-verified-extraction-pipeline-design.md`

**Goal:** Add path traversal protection and explicit link-entry policy to tar and zip archive extraction. Document the extraction trust boundaries in the module.

**Architecture:** Replace `tar::Archive::unpack()` with manual `entries()` iteration that validates each entry path. Add path validation to zip extraction after `mangled_name()`. Reject symlinks and hardlinks in both formats. Add module-level documentation.

**Tech Stack:** Rust 2024, `tar`, `zip`, `flate2`, `xz2` crates, `std::path::Path`.

---

## Existing Code Map

- `src/archive.rs:22-67` (`unpack`): target function for safety refactor.
- `src/archive.rs:70-107` (tests): existing tests for archive kind parsing and raw unpack.
- `src/http.rs`: URL validation (unchanged).
- `src/installers.rs:424-434` (`verify_sha256`): checksum verification (unchanged).
- `src/installers.rs:110-158` (`install_download_binary`): caller of `unpack` (unchanged).

## Task 1: Add safe tar extraction with path validation

**Files:**
- Modify: `src/archive.rs`

- [ ] Replace `archive.unpack(dest)` with manual `archive.entries()` iteration.
- [ ] For each entry, resolve the path relative to `dest` and validate it stays within `dest`.
- [ ] Reject `EntryType::Symlink` and `EntryType::Hardlink` entries.
- [ ] Handle directories and regular files as before.
- [ ] Apply to both TarGz and TarXz code paths.

## Task 2: Add safe zip extraction with path validation

**Files:**
- Modify: `src/archive.rs`

- [ ] After `entry.mangled_name()`, validate the joined path stays within `dest`.
- [ ] Reject symlink entries in zip (check `entry.is_symlink()` or equivalent).
- [ ] Reject entries with empty `mangled_name()`.

## Task 3: Add module documentation and error codes

**Files:**
- Modify: `src/archive.rs`

- [ ] Add module-level doc comment describing trust boundaries.
- [ ] Add `AGENT_EXTRACT_*` error code prefixes to all new error paths.

## Task 4: Add unit tests

**Files:**
- Modify: `src/archive.rs`

- [ ] Test tar with path traversal entry (e.g., `../escape`).
- [ ] Test tar with symlink entry (rejected).
- [ ] Test zip with path traversal entry (rejected).
- [ ] Test zip with empty path entry (rejected).
- [ ] Test normal tar.gz archive still extracts correctly.
- [ ] Test normal zip archive still extracts correctly.

## Verification Commands

- `cargo test archive`
- `cargo test`
- `cargo clippy`

## Expected Outcomes

- Tar extraction validates every entry path against the destination root.
- Zip extraction validates paths after `mangled_name()`.
- Symlinks and hardlinks in archives are rejected with clear error codes.
- Module doc describes the pipeline's trust boundaries.
- All existing archive tests pass, new safety tests pass.
