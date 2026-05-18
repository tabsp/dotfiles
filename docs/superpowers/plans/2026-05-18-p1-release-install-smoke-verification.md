# Release Install Smoke Verification Implementation Plan

**Spec:** `docs/superpowers/specs/2026-05-18-p1-release-install-smoke-verification-design.md`

**Goal:** Add `make smoke-test` — end-to-end validation of the release install chain.

**Architecture:** Shell script (`scripts/smoke-test.sh`) + Makefile target. Uses
`make release-check` for artifact building, then validates naming, checksums,
installer run, and output verification in an isolated temp directory.

---

## Existing Code Map

- `Makefile`: `release` and `release-check` targets.
- `scripts/install.sh`: the release installer.
- `tests/install_script.rs`: 5 Rust-level unit tests for installer (isolation).
  Smoke test covers the full chain; unit tests remain the fast iterator loop.

## Task 1: Create smoke test script

**Files:**
- New: `scripts/smoke-test.sh`
- Modify: `Makefile`

- [ ] Create `scripts/smoke-test.sh`:
  - Extract version and target from Cargo.toml / rustc (passed via Makefile).
  - Validate artifact names match: `dotman-<target>-<version>.tar.gz`.
  - Validate checksum file has one line: `<64-hex>  <filename>`.
  - Run `shasum -c` on the checksum file.
  - Set up temp release layout: `releases/download/v<version>/`.
  - Package dotfiles source archive + checksum in the same layout dir
    (installer fetches source checksum from `$BASE_URL/v$VERSION/`).
  - Run `scripts/install.sh` with `HOME=<tmp>`, `BASE_URL=file://<tmp>/releases/download`,
    `DOTFILES_ARCHIVE_URL=file://<tmp>/dotfiles-<version>.tar.gz`.
  - Verify: `$HOME/.local/bin/dotman --help` exit 0.
  - Verify: `$HOME/.local/bin/dotman --version` contains the version string.
  - Verify: source checkout has `deps.toml`, `dotfiles.toml`, `scripts/install.sh`.
  - Clean up temp dir on exit (trap).
  - Use `shasum -a 256` on macOS, `sha256sum` on Linux (mirrors installer).
- [ ] Add `smoke-test` to `make help`.
- [ ] Add `smoke-test: release-check` target to Makefile that invokes the script
  with version and target passed as arguments.

## Task 2: Review and review doc

**Files:**
- New: `docs/superpowers/agent/reviews/2026-05-18-p1-release-install-smoke-verification-review.md`

- [ ] Fill review doc with Gate 1 synthesis.

## Verification Commands

- `make smoke-test` — passes on current host.
- `cargo test` — all 157 tests pass (Rust tests unchanged).
- `cargo clippy` — zero warnings.
- `make check` — manifest validation passes.
- `make agent-check` — harness validation.

## Expected Outcomes

- `make smoke-test` validates: build → name → checksum → install → verify.
- Works without network access.
- Isolated from real machine state (temp HOME).
- Listed in `make help`.

## Test Level

- Shell script (smoke test is itself the test).
- Existing Rust tests (`cargo test`) unchanged.

## Regression Coverage Expectations

- `make release-check` unchanged.
- `scripts/install.sh` unchanged.
- Existing `cargo test` (including `tests/install_script.rs`) unchanged.
- Smoke test runs in isolated temp directory; no real machine state touched.
