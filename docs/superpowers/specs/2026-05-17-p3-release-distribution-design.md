# Release Distribution Design

## Goal

Enable installing `dotman` without cloning the repository, by providing
versioned release artifacts and a documented installation channel.

## Scope

- Add `make release` target that builds optimized release binaries and
  packages them as tarballs with checksums.
- Create a `scripts/install.sh` bootstrap script that downloads and installs
  the correct binary for the host platform.
- Document installation in `README.md`.
- No GitHub Actions release automation (manual release process for now).

## Non-Goals

- Do not set up GitHub Releases CI/CD pipeline.
- Do not publish to package registries (Homebrew, crates.io, apt).
- Do not add cross-compilation (release builds require the host platform's
  toolchain).
- Do not sign release artifacts.

## Design

### Release Artifacts

`make release` produces per-platform tarballs and checksums in `dist/`:
```
dist/
  dotman-aarch64-apple-darwin-0.1.0.tar.gz
  dotman-aarch64-apple-darwin-0.1.0.tar.gz.sha256
  dotman-x86_64-apple-darwin-0.1.0.tar.gz
  dotman-x86_64-apple-darwin-0.1.0.tar.gz.sha256
  dotman-x86_64-unknown-linux-gnu-0.1.0.tar.gz
  dotman-x86_64-unknown-linux-gnu-0.1.0.tar.gz.sha256
```
Version is read from `Cargo.toml`. Only the current host's target is built
(`make release` does not cross-compile).

### Install Script

`scripts/install.sh`:
- Detects OS and architecture.
- Maps to the correct release artifact name.
- Downloads from a configurable base URL (default: GitHub Releases).
- Extracts `dotman` to `$HOME/.local/bin`.
- Prints usage instructions on success.

### README

Add an **Install** section with:
- The one-liner curl command.
- Prerequisites (Rust toolchain not required for binary install).
- Link to `docs/release-policy.md` for artifact naming details.

## Error Handling

- `make release`: fails if Cargo.toml version can't be read, if build fails,
  or if `dist/` can't be created.
- `scripts/install.sh`: fails if OS/arch unsupported, if download fails, if
  checksum mismatch, or if `$HOME/.local/bin` can't be created.

## Verification Strategy

- `make release` produces expected files on the current host.
- `scripts/install.sh` runs successfully with a local file:// URL.
- `cargo test` — all existing tests pass.
- `cargo clippy` — zero warnings.

## Regression Coverage Expectations

- `make build`, `make test`, `make lint` continue to work.
- No Rust source changes — `dotman` behavior is unchanged.
- `make release` does not interfere with existing targets.
