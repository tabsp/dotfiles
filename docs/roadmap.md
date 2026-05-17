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

### P0 - Roadmap Agent Harness

Status: done
Category: automation
Current code signal: roadmap, specs, plans, and `AGENTS.md` define manual agent
rules, but there is no deterministic runtime for selecting, validating,
executing, and handing off roadmap work.

Spec:
`docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md`

Plan:
`docs/superpowers/plans/2026-05-14-roadmap-agent-harness.md`

This is a prerequisite for future roadmap implementation. Before other roadmap
epics move into agentic implementation, agent sessions need a stable workflow
runtime that preserves priority order, requires spec and plan artifacts, records
handoff state, and exposes checks for roadmap/spec/plan consistency.

Outcome: agent sessions can initialize context, select one roadmap epic, lock
active work, create spec and plan templates, validate spec and plan artifacts,
run workflow checks, advance runtime phase deliberately, record verification,
finish active locks, and leave structured handoff notes for the next session.

### P0 - Atomic Directory Install

Status: done
Category: safety

Spec:
`docs/superpowers/specs/2026-05-16-p0-atomic-directory-install-design.md`

Plan:
`docs/superpowers/plans/2026-05-16-p0-atomic-directory-install.md`
Current code signal: directory installs exist in `download_binary` but are
non-atomic through a remove-then-copy pattern.

Directory-based binary installs currently remove the old install directory before
copying the new one. If copying fails midway, a previously working install can be
left broken.

Outcome: directory installs are staged and promoted atomically where possible.
The old install remains usable if staging or verification fails.

### P0 - Verified Extraction Pipeline

Status: done
Category: safety

Spec:
`docs/superpowers/specs/2026-05-17-p0-verified-extraction-pipeline-design.md`

Plan:
`docs/superpowers/plans/2026-05-17-p0-verified-extraction-pipeline.md`
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

Status: done
Category: inspectability

Spec:
`docs/superpowers/specs/2026-05-17-p1-bootstrap-dry-run-design.md`

Plan:
`docs/superpowers/plans/2026-05-17-p1-bootstrap-dry-run.md`
Current code signal: link dry-run exists; bootstrap dry-run does not.

`link` supports dry-run, but `bootstrap` still mixes dependency installation,
linking, doctor checks, and post-bootstrap hints without a preview mode.

Outcome: first-time setup can be previewed before package installs or filesystem
changes.

### P1 - Doctor Summary And Machine Output

Status: done
Category: observability

Spec:
`docs/superpowers/specs/2026-05-17-p1-doctor-summary-machine-output-design.md`

Plan:
`docs/superpowers/plans/2026-05-17-p1-doctor-summary-machine-output.md`
Current code signal: internal ok, warning, and hard-error buckets exist.

Doctor already separates ok, warning, and hard-error states internally, but the
CLI output is not summarized or script-friendly.

Outcome: doctor prints a clear summary and exposes optional structured output for
automation.

### P1 - Quality Gates And Coverage Policy

Status: done
Category: quality

Spec:
`docs/superpowers/specs/2026-05-17-p1-quality-gates-coverage-policy-design.md`

Plan:
`docs/superpowers/plans/2026-05-17-p1-quality-gates-coverage-policy.md`
Current code signal: Rust unit and CLI integration tests exist and pass locally.

The project has useful tests around manifest validation, link behavior, doctor,
shell changes, archive parsing, and installer helpers, but roadmap-level quality
expectations are not explicit.

Outcome: each roadmap epic defines its test level, required verification command,
and regression coverage expectations before implementation starts.

### P2 - CI Automation

Status: done
Category: quality

Spec:
`docs/superpowers/specs/2026-05-17-p2-ci-automation-design.md`

Plan:
`docs/superpowers/plans/2026-05-17-p2-ci-automation.md`
Current code signal: local `make ci` exists; no repository CI configuration is
present.

The project has a local verification suite, but future changes still depend on
manual discipline unless the same checks run automatically.

Outcome: supported repository automation runs the agreed local verification
suite on relevant changes and documents any platform-specific gaps.

### P2 - Manifest Schema Evolution

Status: done
Category: maintainability

Spec:
`docs/superpowers/specs/2026-05-17-p2-manifest-schema-evolution-design.md`

Plan:
`docs/superpowers/plans/2026-05-17-p2-manifest-schema-evolution.md`
Current code signal: `deps.toml` and `dotfiles.toml` schemas are implicit in Rust
deserialization and validation code.

The manifest formats are central project contracts, but schema changes currently
depend on reading implementation details.

Outcome: manifest fields, compatibility rules, validation behavior, and migration
expectations are documented before adding higher-level schema features.

### P2 - Manifest Defaults

Status: done
Category: maintainability

Spec:
`docs/superpowers/specs/2026-05-17-p2-manifest-defaults-design.md`

Plan:
`docs/superpowers/plans/2026-05-17-p2-manifest-defaults.md`
Current code signal: manifest entries are explicit per platform and architecture.
Depends on: P2 - Manifest Schema Evolution

`deps.toml` repeats many macOS and Linux architecture entries where only a small
number of fields differ.

Outcome: common dependency metadata can be declared once and overridden per
platform or architecture.

### P2 - Dependency Update Workflow

Status: done
Category: maintainability

Spec:
`docs/superpowers/specs/2026-05-17-p2-dependency-update-workflow-design.md`

Plan:
`docs/superpowers/plans/2026-05-17-p2-dependency-update-workflow.md`
Current code signal: pinned Linux binary metadata is stored directly in
`deps.toml`.

Pinned Linux binary versions, URLs, and SHA256 values are maintained manually.

Outcome: Dotman provides either a command or a documented workflow to check and
update pinned release metadata.

### P2 - Cross-Platform Support Strategy

Spec:
`docs/superpowers/specs/2026-05-17-p2-cross-platform-support-strategy-design.md`
Plan:
`docs/superpowers/plans/2026-05-17-p2-cross-platform-support-strategy.md`


Status: done
Category: portability
Current code signal: runtime host support is limited to macOS and Linux, with
some Unix-specific filesystem operations.

Dotman should be explicit about which platforms are supported, which are
intentionally unsupported, and how platform-specific code paths are isolated.

Outcome: platform support policy is documented, Unix-only behavior is clearly
guarded, and any future Windows support is treated as a separate compatibility
epic rather than an accidental extension.

### P2 - Release Readiness

Spec:
`docs/superpowers/specs/2026-05-17-p2-release-readiness-design.md`
Plan:
`docs/superpowers/plans/2026-05-17-p2-release-readiness.md`

Status: done
Category: distribution
Current code signal: `dotman` is versioned as a Rust package, but release
process, artifact naming, changelog policy, and upgrade compatibility are not
defined.
Depends on: P1 - Quality Gates And Coverage Policy

Dotman needs a defined release contract before adding convenience distribution
channels.

Outcome: release versioning, artifact naming, changelog expectations, and
backward compatibility policy are documented and verified.

### P3 - Release Distribution

Spec:
`docs/superpowers/specs/2026-05-17-p3-release-distribution-design.md`
Plan:
`docs/superpowers/plans/2026-05-17-p3-release-distribution.md`

Status: done
Category: distribution
Current code signal: `dotman` is built locally through Cargo and wrapped by
Makefile workflows.
Depends on: P2 - Cross-Platform Support Strategy, P2 - Release Readiness

Bootstrapping on a new machine currently requires cloning the repository and
building locally. A lower-friction install path would shorten the path from an
empty machine to a configured environment.

Outcome: publish versioned `dotman` release artifacts and define supported
installation channels.

### P3 - Recovery And Cleanup

Spec:
`docs/superpowers/specs/2026-05-17-p3-recovery-and-cleanup-design.md`
Plan:
`docs/superpowers/plans/2026-05-17-p3-recovery-and-cleanup.md`

Status: done
Category: safety
Current code signal: link conflicts can be backed up, temporary installer
directories are cleaned up, and README documents that automatic rollback is not
provided in v1.
Depends on: P0 - Atomic Directory Install, P0 - Verified Extraction Pipeline

Dotman prioritizes preventing broken machine state, but users also need a clear
path to inspect and clean up managed changes after a failed or unwanted
bootstrap.

Outcome: recovery boundaries are documented and Dotman provides explicit cleanup
or uninstall workflows for state it can safely identify as managed.

### P3 - Managed Config Coverage

Spec:
`docs/superpowers/specs/2026-05-17-p3-managed-config-coverage-design.md`
Plan:
`docs/superpowers/plans/2026-05-17-p3-managed-config-coverage.md`

Status: done
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
