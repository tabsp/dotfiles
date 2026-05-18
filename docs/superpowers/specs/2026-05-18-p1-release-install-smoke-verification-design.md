# Release Install Smoke Verification Design

## Goal

Add a `make smoke-test` target that validates the end-to-end release install
chain: artifact naming, checksum integrity, installer behavior, and source
checkout consistency — without requiring network access or a live GitHub Release.

## Motivation

The roadmap says: "release readiness includes an explicit smoke verification
path for the installation chain without requiring broad macOS PR CI by default."

Today `tests/install_script.rs` has 5 unit-level tests for the installer script
(checksum failures, successful install). `make release-check` builds and
checksums the current host artifact. But there is no single command that
validates the full chain: build → name → checksum → install → verify.

## Scope

- Add `make smoke-test` target that:
  1. Runs `make release-check` to build and checksum the release artifact.
  2. Validates artifact and checksum file naming conventions.
  3. Validates checksum file format and that the checksum matches.
  4. Serves the release artifacts via a local file:// URL (same pattern as
     existing `install_script.rs` tests).
  5. Runs `scripts/install.sh` against the local artifacts.
  6. Verifies the installed `dotman` binary runs (`dotman --help` exit 0).
  7. Verifies the installed dotfiles source checkout has expected files
     (`deps.toml`, `dotfiles.toml`, `scripts/install.sh`).
- Runs on the current host platform only (not cross-platform).
- Works without network access (uses local files).

## Non-Goals

- Do not run the full `make bootstrap` after install (too heavy for smoke).
- Do not add cross-platform smoke testing (CI matrix handles that).
- Do not add the smoke test to CI by default (can be added later).
- Do not change the installer script or release workflow behavior.

## Design

### `make smoke-test` Target

```
make smoke-test
```

Steps:

1. **Build release artifacts:**
   ```sh
   make release-check
   ```
   Produces `dist/dotman-<target>-<version>.tar.gz` and its `.sha256`.

2. **Validate artifact naming:**
   - Archive name: `dotman-<target>-<version>.tar.gz`
   - Checksum name: `dotman-<target>-<version>.tar.gz.sha256`
   - Target matches `rustc -vV | grep host`
   - Version matches `Cargo.toml`

3. **Validate checksum:**
   - Checksum file contains exactly one line: `<digest>  <filename>`
   - Digest is 64 hex characters (SHA-256)
   - Running `shasum -c` succeeds

4. **Prepare local release layout:**
   - Create temp dir simulating GitHub Release structure:
     `releases/download/v<version>/dotman-<target>-<version>.tar.gz`
     `releases/download/v<version>/dotman-<target>-<version>.tar.gz.sha256`
   - Package dotfiles source archive and checksum.

5. **Run installer:**
   - Execute `scripts/install.sh` with `BASE_URL=file://<tmp>/releases/download`
     and `DOTFILES_ARCHIVE_URL=file://<tmp>/dotfiles-<version>.tar.gz`.
   - Set `HOME` to a temp directory to avoid touching real state.
   - Expect exit 0.

6. **Verify installed artifacts:**
   - `$HOME/.local/bin/dotman` exists and is executable.
   - `$HOME/.local/bin/dotman --help` exits 0.
   - `$HOME/.local/share/dotman/dotfiles/deps.toml` exists.
   - `$HOME/.local/share/dotman/dotfiles/dotfiles.toml` exists.
   - `$HOME/.local/share/dotman/dotfiles/scripts/install.sh` exists.

7. **Cleanup:** Temp dir removed automatically (use `mktemp -d` with trap).

### Implementation

A single script: `scripts/smoke-test.sh`, invoked by `make smoke-test`. Uses
only POSIX shell, `tar`, `shasum`/`sha256sum`, and `make release-check`.

### Error Handling

- `make release-check` fails → smoke test fails immediately.
- Naming validation fails → reports expected vs actual, exits 1.
- Checksum mismatch → reports mismatch, exits 1.
- Installer fails → captures exit code and stderr, reports failure.
- Binary doesn't run → reports missing or non-executable, exits 1.
- Source checkout missing files → reports which files, exits 1.

### Verification Strategy

- `make smoke-test` on the current host passes.
- Existing `cargo test` (including `install_script.rs` tests) unchanged.
- `make check` unchanged.
- Manual verification: `make smoke-test` in a clean checkout.

### Regression Coverage Expectations

- Installer script behavior unchanged.
- Release artifact naming conventions unchanged.
- Existing CI workflow unchanged.
