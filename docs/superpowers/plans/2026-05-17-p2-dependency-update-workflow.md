# Dependency Update Workflow Implementation Plan

**Spec:** `docs/superpowers/specs/2026-05-17-p2-dependency-update-workflow-design.md`

**Goal:** Add `dotman update` subcommand to list and check download_binary deps.

**Architecture:** New `src/update.rs` module. Parse deps.toml, extract DownloadBinary entries. CLI integration in main.rs.

**Tech Stack:** Rust 2024, reqwest (already used for downloads), serde_json (for GitHub API).

---

## Existing Code Map

- `src/main.rs`: add `Update { check: bool }` to Command enum.
- `src/config.rs`: load_deps, InstallEntry, Dependency types.
- `src/http.rs`: download_https for network requests.
- `deps.toml`: source data.

## Task 1: Implement dotman update command

**Files:**
- New: `src/update.rs`
- Modify: `src/main.rs`
- Modify: `Makefile`
- Modify: `README.md`

- [ ] Create `src/update.rs` with `list_deps` and `check_deps` functions.
- [ ] Add `Command::Update { check: bool }` to CLI.
- [ ] Add `make update-deps-list` and `make update-deps-check` targets.
- [ ] Document in README.

## Verification Commands

- `cargo test`
- `cargo clippy`

## Test Level

- Unit tests: `cargo test update`
