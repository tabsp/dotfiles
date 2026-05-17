# Release Distribution Implementation Plan

**Spec:** `docs/superpowers/specs/2026-05-17-p3-release-distribution-design.md`

**Goal:** Add `make release` for binary artifacts and `scripts/install.sh` for bootstrapping.

**Architecture:** Makefile target + shell script. No Rust changes.

**Tech Stack:** Make, shell (POSIX sh), tar, shasum.

---

## Existing Code Map

- `Makefile`: existing `build`, `build-dotman`, `cargo-preflight` targets.
- `Cargo.toml`: version source for artifact naming.
- `docs/release-policy.md`: artifact naming convention.
- `README.md`: documentation index, existing install instructions via clone + build.

## Task 1: Add make release target

**Files:**
- Modify: `Makefile`

- [ ] Add `make release` target that builds release binary and packages tarball.
- [ ] Add `make release-check` target that builds and verifies checksum.
- [ ] Add `dist/` to `.gitignore`.

## Task 2: Create install script

**Files:**
- New: `scripts/install.sh`

- [ ] Detect OS and architecture.
- [ ] Map to correct release artifact.
- [ ] Download, verify checksum, extract to `$HOME/.local/bin`.
- [ ] Print success message with next steps.

## Task 3: Update documentation

**Files:**
- Modify: `README.md`

- [ ] Add Install section with curl one-liner.
- [ ] Note binary install does not require Rust toolchain.

## Verification Commands

- `make release` (on macOS, produces aarch64 tarball)
- `make release-check`
- `sh scripts/install.sh` (with local file URL)
- `cargo test`
- `cargo clippy`
- `make lint`
- `make check`

## Test Level

- No new Rust tests (Makefile + shell script only).
- Manual verification: `make release` and `scripts/install.sh` produce expected output.

## Expected Outcomes

- `make release` produces `dist/dotman-{target}-{version}.tar.gz` and checksum.
- `scripts/install.sh` downloads, verifies, and installs dotman.
- `README.md` has Install section with curl command.
- All existing tests pass.

## Regression Coverage Expectations

- `make build`, `make test`, `make lint` continue to work.
- No Rust source changes.
