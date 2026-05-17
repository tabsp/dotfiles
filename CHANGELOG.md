# Changelog

All notable changes to `dotman` are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- Atomic directory install with staging-then-rename for crash-safe updates.
- Verified extraction pipeline with path traversal defense and symlink/hardlink rejection.
- `--dry-run` flag on bootstrap for preview without mutation.
- Doctor summary line with `--json` machine-readable output.
- Test Level and Regression Coverage Expectations quality gates in spec/plan templates.
- CI automation via GitHub Actions with `rust-toolchain.toml`.
- Manifest schema reference (`docs/manifest-schema.md`).
- `default` section in `deps.toml` for field inheritance across entries.
- `dotman update` subcommand for listing and checking download_binary deps.
- Platform support policy with Unix-specific code audit (`docs/platform-support.md`).
- Release policy and changelog (`docs/release-policy.md`).

### Changed
- Roadmap agent harness enforces spec → plan → implement workflow.
- `AGENTS.md` requires commit after each completed epic.

### Fixed
- Removed redundant `use std::os::unix::fs::PermissionsExt` inside `#[cfg(unix)]` test blocks.

## [0.1.0] — unreleased

Initial development version. Core bootstrap, link, doctor, shell, and dependency
installation workflows.
