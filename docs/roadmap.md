# Dotman Roadmap

## Direction

Dotman prioritizes safe, inspectable bootstrap behavior. A failed dependency
install, archive extraction, or link operation should not leave the machine in a
worse state than before.

This roadmap is written for both human maintenance and future agentic sessions.
Use it as the first project context document before creating detailed specs or
implementation plans.

## Priority Rules

1. Safety before convenience.
2. Inspectability before automation.
3. Quality verification before feature expansion.
4. Maintainability before feature breadth.
5. Coverage after the core flow is reliable.

## Status Values

- `proposed`: agreed direction, not yet specified in detail.
- `specified`: design/spec exists.
- `planned`: implementation plan exists.
- `in_progress`: implementation has started.
- `done`: shipped and verified.
- `deferred`: intentionally postponed.

## Active Queue

### P0 - Atomic Directory Install

Status: proposed
Category: safety
Current code signal: directory installs exist in `download_binary` but are
non-atomic through a remove-then-copy pattern.

Directory-based binary installs currently remove the old install directory before
copying the new one. If copying fails midway, a previously working install can be
left broken.

Outcome: directory installs are staged and promoted atomically where possible.
The old install remains usable if staging or verification fails.

### P0 - Verified Extraction Pipeline

Status: proposed
Category: safety
Current code signal: checksum verification exists; extraction policy is not yet
explicit.

Downloaded archives are part of the trusted install path for pinned binary
dependencies. The extraction pipeline should make each trust boundary explicit:
manifest URL validation, final URL validation, checksum verification, archive
path safety, and link-entry behavior.

Outcome: binary downloads use one documented extraction pipeline that rejects
unsafe paths, verifies expected payloads, and has an explicit symlink and
hardlink policy.

### P1 - Bootstrap Dry Run

Status: proposed
Category: inspectability
Current code signal: link dry-run exists; bootstrap dry-run does not.

`link` supports dry-run, but `bootstrap` still mixes dependency installation,
linking, doctor checks, and post-bootstrap hints without a preview mode.

Outcome: first-time setup can be previewed before package installs or filesystem
changes.

### P1 - Doctor Summary And Machine Output

Status: proposed
Category: observability
Current code signal: internal ok, warning, and hard-error buckets exist.

Doctor already separates ok, warning, and hard-error states internally, but the
CLI output is not summarized or script-friendly.

Outcome: doctor prints a clear summary and exposes optional structured output for
automation.

### P1 - Quality Gates And Coverage Policy

Status: proposed
Category: quality
Current code signal: Rust unit and CLI integration tests exist and pass locally.

The project has useful tests around manifest validation, link behavior, doctor,
shell changes, archive parsing, and installer helpers, but roadmap-level quality
expectations are not explicit.

Outcome: each roadmap epic defines its test level, required verification command,
and regression coverage expectations before implementation starts.

### P2 - CI Automation

Status: proposed
Category: quality
Current code signal: local `make ci` exists; no repository CI configuration is
present.

The project has a local verification suite, but future changes still depend on
manual discipline unless the same checks run automatically.

Outcome: supported repository automation runs the agreed local verification
suite on relevant changes and documents any platform-specific gaps.

### P2 - Manifest Schema Evolution

Status: proposed
Category: maintainability
Current code signal: `deps.toml` and `dotfiles.toml` schemas are implicit in Rust
deserialization and validation code.

The manifest formats are central project contracts, but schema changes currently
depend on reading implementation details.

Outcome: manifest fields, compatibility rules, validation behavior, and migration
expectations are documented before adding higher-level schema features.

### P2 - Manifest Defaults

Status: proposed
Category: maintainability
Depends on: P2 - Manifest Schema Evolution
Current code signal: manifest entries are explicit per platform and architecture.

`deps.toml` repeats many macOS and Linux architecture entries where only a small
number of fields differ.

Outcome: common dependency metadata can be declared once and overridden per
platform or architecture.

### P2 - Dependency Update Workflow

Status: proposed
Category: maintainability
Current code signal: pinned Linux binary metadata is stored directly in
`deps.toml`.

Pinned Linux binary versions, URLs, and SHA256 values are maintained manually.

Outcome: Dotman provides either a command or a documented workflow to check and
update pinned release metadata.

### P2 - Cross-Platform Support Strategy

Status: proposed
Category: portability
Current code signal: runtime host support is limited to macOS and Linux, with
some Unix-specific filesystem operations.

Dotman should be explicit about which platforms are supported, which are
intentionally unsupported, and how platform-specific code paths are isolated.

Outcome: platform support policy is documented, Unix-only behavior is clearly
guarded, and any future Windows support is treated as a separate compatibility
epic rather than an accidental extension.

### P3 - Managed Config Coverage

Status: proposed
Category: coverage
Current code signal: some dependencies have managed configs; others only have
install entries.

Some installed tools do not yet have managed config entries in `dotfiles.toml`,
which means dependency installation and configuration coverage can drift apart.

Outcome: add managed configs for tools that already have dependency entries after
the core manager flow is safer.

## Handoff Notes

When starting a new session, read this file first, then pick one Active Queue
item and create a detailed spec or implementation plan for that item only.

Do not treat this roadmap as an implementation plan. Concrete file edits, tests,
commands, and acceptance criteria belong in per-epic docs under
`docs/superpowers/specs/` or `docs/superpowers/plans/`.
