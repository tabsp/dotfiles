# Release Readiness Design

## Goal

Define the release contract for `dotman`: versioning, artifact naming,
changelog expectations, and backward compatibility policy.

## Scope

- Define semantic versioning policy and what constitutes a breaking change.
- Define release artifact naming convention.
- Define changelog format and update cadence.
- Define backward compatibility guarantees for `deps.toml`, `dotfiles.toml`,
  CLI flags, and error codes.
- Document in `docs/release-policy.md`.

## Non-Goals

- Do not set up release automation or CI/CD distribution.
- Do not create actual release artifacts or tags.
- Do not add a LICENSE file (out of scope unless requested).

## Design

### Versioning

Follow [Semantic Versioning 2.0.0](https://semver.org/):
- **MAJOR**: breaking changes to `deps.toml`/`dotfiles.toml` schema, CLI
  flag removal/rename, error code changes that agent harness depends on.
- **MINOR**: new installers, new CLI flags, new manifest fields (backward
  compatible), new error codes.
- **PATCH**: bug fixes, doc updates, internal refactors.

Current version: `0.1.0`. Pre-1.0, MINOR bumps may include breaking changes
with clear changelog notes.

### Artifact Naming

Release binaries follow:
```
dotman-{target}-{version}.tar.gz
dotman-{target}-{version}.tar.gz.sha256
```
Where `{target}` is a Rust target triple, e.g.:
- `aarch64-apple-darwin`
- `x86_64-apple-darwin`
- `x86_64-unknown-linux-gnu`

### Changelog

Maintain `CHANGELOG.md` following [Keep a Changelog](https://keepachangelog.com/):
- Sections: Added, Changed, Deprecated, Removed, Fixed, Security.
- Updated with each release.
- Each entry links to the relevant roadmap epic or issue.

### Backward Compatibility

- `deps.toml` schema: additive changes only in MINOR; field removal or
  rename is MAJOR.
- `dotfiles.toml` schema: same policy.
- CLI: flags may be added in MINOR; removal/rename is MAJOR. Deprecated
  flags print a warning for at least one MINOR release before removal.
- Error codes: agent harness codes (`AGENT_*`) are stable. New codes added
  in MINOR. Renaming or renumbering existing codes is MAJOR.

### Deliverable

- `docs/release-policy.md` covering all four areas.
- `CHANGELOG.md` with initial entries summarizing completed epics.
- Link from `README.md`.

## Error Handling

- No runtime error codes needed (documentation-only).
- Policy violations are caught by human review, not automated checks.

## Verification Strategy

- `cargo test` — all existing tests pass (no code changes).
- `cargo clippy` — zero warnings.
- Manual review of `docs/release-policy.md` and `CHANGELOG.md` for accuracy.

## Regression Coverage Expectations

- All existing tests continue to pass.
- No runtime behavior changes.
- `Cargo.toml` version is not changed from `0.1.0`.
