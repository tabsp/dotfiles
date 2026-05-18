# Dotman Roadmap

## Direction

Dotman prioritizes safe, inspectable bootstrap behavior. A failed dependency
install, archive extraction, link operation, cleanup, or release installation
should not leave the machine in a worse state than before.

This roadmap is written for both human maintenance and future agentic sessions.
Use it as the first project context document before creating detailed specs or
implementation plans.

The next phase should move Dotman from "core flow works" to "release/install
trust boundaries are hard, machine state is explainable, and future roadmap
work is schedulable."

## Priority Rules

1. Safety before convenience.
2. Inspectability before automation.
3. Quality verification before feature expansion.
4. Maintainability before feature breadth.
5. Coverage after the core flow is reliable.
6. Read-only state modeling before destructive or corrective actions.
7. Productization only after release, security, and recovery boundaries are
   explicit.

## Status Values

- `proposed`: agreed direction, not yet specified in detail.
- `specified`: design/spec exists.
- `planned`: implementation plan exists.
- `in_progress`: implementation has started.
- `done`: shipped and verified.
- `deferred`: intentionally postponed.

## Agent Scheduling Rules

- `Next Queue` contains the only roadmap epics that agent sessions should
  select for normal roadmap work.
- `Completed Foundation` is historical context and is not selectable.
- `Deferred / Non-Goals` are intentionally not selectable.
- Do not start implementation directly from this roadmap. Create or update a
  spec and plan first unless the user explicitly asks for a small direct edit.
- If no eligible `Next Queue` item exists, perform a read-only Roadmap Planning
  Review instead of force-starting a completed item.
- Roadmap Planning Review may propose the next queue, risk register entries,
  deferred items, and planning lessons. It must not implement code or advance an
  epic.

## Next Queue

### P0 - Recovery Safety And Ownership Model

Status: done
Category: safety / recovery

Spec:
`docs/superpowers/specs/2026-05-18-p0-recovery-safety-and-ownership-model-design.md`

Plan:
`docs/superpowers/plans/2026-05-18-p0-recovery-safety-and-ownership-model.md`

Depends on: P0 - Multi-Agent Review Protocol

Current signal: recovery documentation includes broad manual removal guidance,
while cleanup only knows about stale backup and staging directories. Dotman does
not yet have a complete ownership model for installed binaries, linked dotfiles,
source checkout, backups, and staging state.

Outcome: recovery guidance follows an inspect-first model, dangerous broad
deletion examples are removed or replaced, and a read-only ownership model is
specified before any automatic uninstall, adopt, unmanage, or corrective cleanup
work.

### P1 - Managed State Inventory And Status

Status: done
Category: inspectability

Spec:
`docs/superpowers/specs/2026-05-18-p1-managed-state-inventory-and-status-design.md`

Plan:
`docs/superpowers/plans/2026-05-18-p1-managed-state-inventory-and-status.md`

Depends on: P0 - Recovery Safety And Ownership Model

Current signal: `doctor` reports health and `link --dry-run` previews link
actions, but there is no single read-only view of desired state versus current
machine state.

Outcome: Dotman can explain what it considers managed and how current machine
state differs from manifests, covering dependency commands, versions, managed
links, backup/staging directories, and release/source checkout state. This is a
read-only prerequisite for diff, reconcile, cleanup, adopt, and unmanage.

### P1 - Release Install Smoke Verification

Status: proposed
Category: quality / distribution

Depends on: P0 - Release Installer Trust Boundary Hardening

Current signal: CI runs local checks on Ubuntu, and release artifact workflow
builds supported artifacts on native runners. The missing check is an end-to-end
release install smoke path that validates tag, artifact names, checksums,
installer behavior, and source checkout consistency together.

Outcome: release readiness includes an explicit smoke verification path for the
installation chain without requiring broad macOS PR CI by default.

### P1 - Security And Project Governance Baseline

Status: proposed
Category: governance / security

Current signal: release policy and changelog exist, but productization-facing
project files and policies are incomplete. Security-sensitive behaviors such as
remote scripts, downloaded binaries, sudo package managers, symlink overwrite,
and cleanup need documented trust and support boundaries.

Outcome: the repository has a minimal security and governance baseline, including
security policy, license decision, release checklist, and documented trust
policies for remote script and downloaded binary installers. Contribution
templates can follow after the safety baseline is clear.

### P1 - Handoff And Plan Quality Gates

Status: proposed
Category: automation / quality

Depends on: P0 - Roadmap Refresh And Agent Queue Reset

Current signal: historical handoff files sometimes contain stale phases or raw
formatting issues, and generic plan templates do not yet distinguish
machine-state epics from ordinary documentation or CLI work.

Outcome: future handoffs are reliable audit artifacts, and plans for
machine-state changes explicitly require dry-run/execute boundaries, failure-path
tests, recovery notes, manual smoke checks, and non-destructive scope limits.

### P2 - Narrow Diff And Reconcile Reporting

Status: proposed
Category: inspectability

Spec:
`docs/superpowers/specs/2026-05-18-p1-managed-state-inventory-and-status-design.md`

Plan:
`docs/superpowers/plans/2026-05-18-p1-managed-state-inventory-and-status.md`

Depends on: P1 - Managed State Inventory And Status

Current signal: Dotman can validate manifests and inspect health, but it cannot
yet produce a narrow drift report that explains what would change.

Outcome: Dotman can report focused differences for links, dependency presence,
version drift, and backup/staging state. Reconcile remains advisory or
snippet-generating until ownership, backup, and recovery semantics are mature.

### P2 - Manifest Compatibility Guardrails

Status: proposed
Category: maintainability

Current signal: manifest schema documentation, defaults, and compatibility
policy exist. The next need is not broad schema expansion, but small guardrails
around compatibility, deprecation, and migration expectations.

Outcome: manifest evolution remains boring: compatibility tests, deprecation
rules, and schema version decision points are explicit before adding higher-level
manifest features.

## Risk Register

| Risk | Evidence | Severity | Linked Epic | Status |
|------|----------|----------|-------------|--------|
| Release installer trust boundary is weaker than internal binary install path. | `scripts/install.sh` currently treats checksum download failure as non-fatal and verifies only when checksum file is present. | high | P0 - Release Installer Trust Boundary Hardening | open |
| README "latest" install wording can drift from hard-coded installer default version. | README describes latest install while installer defaults to `0.1.0`. | medium | P0 - Release Installer Trust Boundary Hardening | open |
| Dotfiles source archive lacks the same verification story as binary artifacts. | Installer downloads GitHub source archive separately from release artifact checksum flow. | high | P0 - Release Installer Trust Boundary Hardening | open |
| Recovery guidance can encourage broad deletion outside known managed state. | Full uninstall docs include broad `$HOME/.local/bin` removal guidance. | high | P0 - Recovery Safety And Ownership Model | open |
| Cleanup and uninstall cannot be made safely automatic without ownership inventory. | Cleanup currently targets stale backup/staging names, not a complete managed-state model. | high | P0 - Recovery Safety And Ownership Model | open |
| `official_script` is an intentional remote-code trust boundary that needs policy. | HTTPS script downloads are executed after download and local permission setup. | medium | P1 - Security And Project Governance Baseline | open |
| Historical handoffs may be unreliable audit inputs. | Some finished handoffs have stale phase text or formatting issues. | medium | P1 - Handoff And Plan Quality Gates | open |
| High-risk P0 work could proceed without independent safety/product/workflow review. | Multi-agent review worked in this planning cycle and has initial documentation, but is not yet fully specified or verified as a reusable protocol. | medium | P0 - Multi-Agent Review Protocol | open |
| Complex product features could expand safety surface before state is explainable. | Templates, secrets, plugins, adopt/unmanage, and cleanup all depend on mature ownership and trust boundaries. | medium | Deferred / Non-Goals | open |
| No automated release pipeline; artifact matrix workflow requires manual trigger. | Release artifact matrix workflow is manual-only; no CI trigger on tag push. | low | P2 - CI Automation | open |

## Deferred / Non-Goals

These directions are not rejected forever, but they are intentionally not part of
the next roadmap phase.

- Full template engine: defer until status/diff and manifest compatibility are
  mature.
- Secret or encryption management: defer; prefer external secret managers if
  this ever becomes necessary.
- Plugin ecosystem or remote registry: defer until core APIs, trust boundaries,
  and versioning are stable.
- Windows support: defer as a separate compatibility product line; current
  support remains macOS and Linux.
- Automatic package-manager rollback: defer; package manager side effects are
  not safely reversible by Dotman today.
- Automatic full uninstall: defer until ownership inventory, backup policy, and
  recovery semantics are mature.
- Adopt and unmanage write operations: defer; dry-run suggestions may be
  explored after inventory/status exists.
- New distribution channels such as Homebrew tap, crates.io, apt repository, or
  plugin-managed install channels: defer until the current release installer
  trust boundary is hardened.

## Completed Foundation

### P0 - Release Installer Trust Boundary Hardening

Status: done
Category: safety / distribution

Depends on: P0 - Multi-Agent Review Protocol

Spec:
`docs/superpowers/specs/2026-05-17-p0-release-installer-trust-boundary-hardening-design.md`

Plan:
`docs/superpowers/plans/2026-05-17-p0-release-installer-trust-boundary-hardening.md`

Current signal: Rust `download_binary` installs have checksum and archive safety
guards, but the external `scripts/install.sh` entry point is less strict. The
installer defaults to a fixed version while README describes a latest install
path, treats missing checksum downloads as non-fatal, verifies checksums only
when available, and downloads the dotfiles source archive without the same trust
boundary as release artifacts.

Outcome: the release installer has an explicit trust contract. Checksums are
required and verified, checksum tooling failures are fatal, dotfiles source
retrieval is verified or otherwise made explicit, and version/tag/artifact/source
consistency is covered by release smoke verification.


Outcome: installer enforces mandatory checksum verification for both dotman
binary and dotfiles source archive. Checksum download failures are fatal.
Checksum tool (shasum/sha256sum) detection is early and mandatory. Two failure
path tests added (missing checksum, checksum mismatch).

Handoff: `docs/superpowers/agent/handoffs/2026-05-17-p0-release-installer-trust-boundary-hardening.md`


### P0 - Multi-Agent Review Protocol

Status: done
Category: governance / quality

Spec:
`docs/superpowers/specs/2026-05-17-p0-multi-agent-review-protocol-design.md`

Plan:
`docs/superpowers/plans/2026-05-17-p0-multi-agent-review-protocol.md`

Depends on: P0 - Roadmap Refresh And Agent Queue Reset

Current signal: this planning cycle benefited from independent safety/release,
product/community, and roadmap/agent workflow reviews. The pattern now has an
initial roadmap entry and template, but it is not yet fully specified and
verified as a reusable gate for future high-risk P0 work.

Outcome: multi-agent review protocol formalized as a documented,
reusable gate for high-risk P0 work. Fixed reviewer roles (Safety/Release,
Product/Community, Workflow/Harness) with read-only constraints and isolated
context. Coordinator synthesis rules cover consensus, disagreements, accepted
and rejected changes, and risk register updates. Templates at
`docs/superpowers/agent/templates/multi-agent-review.md` and
`roadmap-review.md` updated; AGENTS.md references protocol and template paths.

Handoff: `docs/superpowers/agent/handoffs/2026-05-17-p0-multi-agent-review-protocol.md`


### P0 - Roadmap Refresh And Agent Queue Reset

Status: done
Category: governance

Spec:
`docs/superpowers/specs/2026-05-17-p0-roadmap-refresh-and-agent-queue-reset-design.md`

Plan:
`docs/superpowers/plans/2026-05-17-p0-roadmap-refresh-and-agent-queue-reset.md`

Current signal: the previous `Active Queue` contained only `done` items while
the handoff notes still told new sessions to pick an active item. This blocks
future agent scheduling and makes completed work look selectable.

Outcome: agent harness parser fixed to recognize `## Next Queue` and
`## Completed Foundation` sections; roadmap structure verified (archive, risk
register, agent scheduling rules already in place); one missing risk register
entry added; `make agent-next` and `make agent-start` work correctly.

Handoff: `docs/superpowers/agent/handoffs/2026-05-17-p0-roadmap-refresh-and-agent-queue-reset.md`


### P0 - Roadmap Agent Harness

Status: done
Category: automation
Spec: `docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md`
Plan: `docs/superpowers/plans/2026-05-14-roadmap-agent-harness.md`
Handoff: `docs/superpowers/agent/handoffs/2026-05-17-p0-roadmap-agent-harness.md`

Outcome: agent sessions can initialize context, select one roadmap epic, lock
active work, create spec and plan templates, validate artifacts, run workflow
checks, advance phases deliberately, record verification, finish active locks,
and leave structured handoff notes.

Residual risk: queue exhaustion needs an explicit Roadmap Planning Review path.

### P0 - Atomic Directory Install

Status: done
Category: safety
Spec: `docs/superpowers/specs/2026-05-16-p0-atomic-directory-install-design.md`
Plan: `docs/superpowers/plans/2026-05-16-p0-atomic-directory-install.md`
Handoff: `docs/superpowers/agent/handoffs/2026-05-16-p0-atomic-directory-install.md`

Outcome: directory installs are staged and promoted atomically where possible.
The old install remains usable if staging or verification fails.

### P0 - Verified Extraction Pipeline

Status: done
Category: safety
Spec: `docs/superpowers/specs/2026-05-17-p0-verified-extraction-pipeline-design.md`
Plan: `docs/superpowers/plans/2026-05-17-p0-verified-extraction-pipeline.md`
Handoff: `docs/superpowers/agent/handoffs/2026-05-16-p0-verified-extraction-pipeline.md`

Outcome: binary downloads use a documented extraction pipeline that rejects
unsafe paths, verifies expected payloads, and has an explicit symlink and
hardlink rejection policy.

### P1 - Bootstrap Dry Run

Status: done
Category: inspectability

Spec:
`docs/superpowers/specs/2026-05-18-p1-managed-state-inventory-and-status-design.md`

Plan:
`docs/superpowers/plans/2026-05-18-p1-managed-state-inventory-and-status.md`
Spec: `docs/superpowers/specs/2026-05-17-p1-bootstrap-dry-run-design.md`
Plan: `docs/superpowers/plans/2026-05-17-p1-bootstrap-dry-run.md`
Handoff: `docs/superpowers/agent/handoffs/2026-05-16-p1-bootstrap-dry-run.md`

Outcome: first-time setup can be previewed before package installs or filesystem
changes.

### P1 - Doctor Summary And Machine Output

Status: done
Category: observability
Spec: `docs/superpowers/specs/2026-05-17-p1-doctor-summary-machine-output-design.md`
Plan: `docs/superpowers/plans/2026-05-17-p1-doctor-summary-machine-output.md`
Handoff: `docs/superpowers/agent/handoffs/2026-05-16-p1-doctor-summary-and-machine-output.md`

Outcome: doctor prints a clear summary and exposes optional structured output for
automation.

### P1 - Quality Gates And Coverage Policy

Status: done
Category: quality
Spec: `docs/superpowers/specs/2026-05-17-p1-quality-gates-coverage-policy-design.md`
Plan: `docs/superpowers/plans/2026-05-17-p1-quality-gates-coverage-policy.md`
Handoff: `docs/superpowers/agent/handoffs/2026-05-16-p1-quality-gates-and-coverage-policy.md`

Outcome: each roadmap epic defines its test level, required verification
command, and coverage guidelines.

### P2 - CI Automation

Status: done
Category: quality
Spec: `docs/superpowers/specs/2026-05-17-p2-ci-automation-design.md`
Plan: `docs/superpowers/plans/2026-05-17-p2-ci-automation.md`
Handoff: `docs/superpowers/agent/handoffs/2026-05-17-p2-ci-automation.md`

Outcome: repository automation runs the agreed local verification command on
push to main and on pull requests targeting main.

Residual risk: release/install smoke verification should validate the artifact
and installer path without necessarily expanding ordinary PR CI.

### P2 - Manifest Schema Evolution

Status: done
Category: maintainability
Spec: `docs/superpowers/specs/2026-05-17-p2-manifest-schema-evolution-design.md`
Plan: `docs/superpowers/plans/2026-05-17-p2-manifest-schema-evolution.md`
Handoff: `docs/superpowers/agent/handoffs/2026-05-17-p2-manifest-schema-evolution.md`

Outcome: manifest fields, compatibility rules, validation behavior, and
migration expectations are documented before higher-level schema features.

### P2 - Manifest Defaults

Status: done
Category: maintainability
Spec: `docs/superpowers/specs/2026-05-17-p2-manifest-defaults-design.md`
Plan: `docs/superpowers/plans/2026-05-17-p2-manifest-defaults.md`
Handoff: `docs/superpowers/agent/handoffs/2026-05-17-p2-manifest-defaults.md`

Outcome: common dependency metadata can be declared once and overridden per
platform or architecture.

### P2 - Dependency Update Workflow

Status: done
Category: maintainability
Spec: `docs/superpowers/specs/2026-05-17-p2-dependency-update-workflow-design.md`
Plan: `docs/superpowers/plans/2026-05-17-p2-dependency-update-workflow.md`
Handoff: `docs/superpowers/agent/handoffs/2026-05-17-p2-dependency-update-workflow.md`

Outcome: Dotman provides a workflow to list pinned binary dependencies and check
for newer GitHub releases.

### P2 - Cross-Platform Support Strategy

Status: done
Category: portability
Spec: `docs/superpowers/specs/2026-05-17-p2-cross-platform-support-strategy-design.md`
Plan: `docs/superpowers/plans/2026-05-17-p2-cross-platform-support-strategy.md`
Handoff: `docs/superpowers/agent/handoffs/2026-05-17-p2-cross-platform-support-strategy.md`

Outcome: platform support policy is documented, Unix-only behavior is clearly
guarded, and future Windows support is treated as a separate compatibility epic.

### P2 - Release Readiness

Status: done
Category: distribution
Spec: `docs/superpowers/specs/2026-05-17-p2-release-readiness-design.md`
Plan: `docs/superpowers/plans/2026-05-17-p2-release-readiness.md`
Handoff: `docs/superpowers/agent/handoffs/2026-05-17-p2-release-readiness.md`

Outcome: release versioning, artifact naming, changelog expectations, and
backward compatibility policy are documented and verified.

### P2 - Guided Add Workflow

Status: done
Category: usability
Spec: `docs/superpowers/specs/2026-05-17-guided-add-workflow-design.md`
Plan: `docs/superpowers/plans/2026-05-17-guided-add-workflow.md`
Handoff: `docs/superpowers/agent/handoffs/2026-05-17-p2-guided-add-workflow.md`

Outcome: `dotman add dep` and `dotman add config` can interactively build valid
manifest entries, support dry-run, deduplicate entries, and validate atomically.

### P3 - Release Distribution

Status: done
Category: distribution
Spec: `docs/superpowers/specs/2026-05-17-p3-release-distribution-design.md`
Plan: `docs/superpowers/plans/2026-05-17-p3-release-distribution.md`
Handoff: `docs/superpowers/agent/handoffs/2026-05-17-p3-release-distribution.md`

Outcome: versioned `dotman` release artifacts and supported installation
channels are defined.

Residual risk: the external installer needs a hardened trust boundary.

### P3 - Release Artifact Matrix

Status: done
Category: distribution
Spec: `docs/superpowers/specs/2026-05-17-p3-release-artifact-matrix-design.md`
Plan: `docs/superpowers/plans/2026-05-17-p3-release-artifact-matrix.md`
Handoff: `docs/superpowers/agent/handoffs/2026-05-17-p3-release-artifact-matrix.md`

Outcome: a manual GitHub Actions workflow builds and verifies supported release
artifacts for an input tag, uploads them as workflow artifacts, and can publish
them to the matching GitHub Release.

### P3 - Recovery And Cleanup

Status: done
Category: safety
Spec: `docs/superpowers/specs/2026-05-17-p3-recovery-and-cleanup-design.md`
Plan: `docs/superpowers/plans/2026-05-17-p3-recovery-and-cleanup.md`
Handoff: `docs/superpowers/agent/handoffs/2026-05-17-p3-recovery-and-cleanup.md`

Outcome: recovery boundaries are documented and Dotman provides explicit cleanup
or uninstall workflows for state it can safely identify as managed.

Residual risk: recovery and cleanup need an ownership inventory before expanding
automatic removal behavior.

### P3 - Managed Config Coverage

Status: done
Category: coverage
Spec: `docs/superpowers/specs/2026-05-17-p3-managed-config-coverage-design.md`
Plan: `docs/superpowers/plans/2026-05-17-p3-managed-config-coverage.md`
Handoff: `docs/superpowers/agent/handoffs/2026-05-17-p3-managed-config-coverage.md`

Outcome: dependency installation and managed configuration coverage are less
likely to drift for tools already tracked by Dotman.

## Roadmap Planning Review

Use Roadmap Planning Review when the next queue is exhausted, all selectable
items are `done` or `deferred`, the project is entering a new phase, or the user
explicitly asks for roadmap planning.

Roadmap Planning Review is a read-only planning mode unless the user explicitly
asks to update roadmap files. It may:

- read README, roadmap, specs, plans, handoffs, release/install/recovery docs,
  CI configuration, and relevant source code signals;
- compare current project state with existing roadmap claims;
- use community experience or web research when requested;
- run independent review agents for safety, product, workflow, or maintenance
  perspectives;
- propose next queue items, risk register entries, deferred items, roadmap
  adjustments, and planning lessons.

It must not:

- implement code;
- modify files without explicit user approval;
- run `agent-start`, `agent-advance`, or `agent-finish`;
- start a completed roadmap item;
- turn roadmap items directly into implementation work.

After a Roadmap Planning Review, ask for confirmation before editing roadmap
files or starting a normal roadmap epic.
