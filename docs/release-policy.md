# Release Policy

## Versioning

`dotman` follows [Semantic Versioning 2.0.0](https://semver.org/).

| Bump | Criteria |
|------|----------|
| **MAJOR** | Breaking changes to `deps.toml` / `dotfiles.toml` schema; CLI flag removal or rename; `AGENT_*` error code removal or rename |
| **MINOR** | New installer types; new CLI flags; new manifest fields (backward compatible); new `AGENT_*` error codes |
| **PATCH** | Bug fixes; documentation updates; internal refactors |

Pre-1.0 (current version `0.1.0`): MINOR bumps may include breaking changes.
All breaking changes are called out in the changelog.

## Artifact Naming

Release tarballs use the naming convention:

```
dotman-{target}-{version}.tar.gz
dotman-{target}-{version}.tar.gz.sha256
```

Where `{target}` is a Rust target triple:
- `aarch64-apple-darwin` (macOS Apple Silicon)
- `x86_64-apple-darwin` (macOS Intel)
- `x86_64-unknown-linux-gnu` (Linux x86_64)

The SHA256 checksum file contains the hex digest and filename, one per line.

## Changelog

`CHANGELOG.md` follows [Keep a Changelog](https://keepachangelog.com/):
- Sections: **Added**, **Changed**, **Deprecated**, **Removed**, **Fixed**, **Security**.
- Updated with each release.
- Each entry references the relevant roadmap epic or commit where practical.

## Backward Compatibility

### Manifests (`deps.toml`, `dotfiles.toml`)

- Additive field additions are MINOR, not MAJOR.
- Field removal, renaming, or semantic change is MAJOR.
- New top-level sections are MINOR.
- Removing or renaming existing top-level sections is MAJOR.

### CLI

- New flags and subcommands: MINOR.
- Flag/subcommand removal or rename: MAJOR.
- Deprecated flags print a warning for at least one MINOR release before removal.

### Error Codes (`AGENT_*`)

- `AGENT_*` codes used by the agent harness are stable within a MAJOR version.
- New codes: MINOR.
- Renaming or renumbering existing codes: MAJOR.

### Internal APIs

- No internal Rust API stability is guaranteed.
- `src/` module structure may change in any release.

## Release Cadence

- Releases are cut when a set of completed epics warrants one.
- No fixed schedule.
- Each release is tagged with `v{version}` (e.g., `v0.1.0`).
