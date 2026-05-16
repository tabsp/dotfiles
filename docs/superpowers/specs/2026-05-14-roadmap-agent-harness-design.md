# Roadmap Agent Harness Design

## Goal

Build a P0 roadmap agent harness so future agentic sessions can move through
the roadmap without depending on chat history, ad hoc memory, or unchecked
automation.

The harness is a deterministic local runtime for humans and agents. It provides
state, workflow gates, templates, and validation commands. It must not call an
LLM, require network access, or make unsafe machine-state changes on its own.

This epic is a prerequisite for future roadmap implementation work.

## References

Primary references:

- Anthropic, "Harness design for long-running application development"
  (https://www.anthropic.com/engineering/harness-design-long-running-apps,
  accessed 2026-05-14):
  initializer, progress files, sprint contracts, generator/evaluator loops, and
  file-based agent coordination.
- Anthropic, "Effective harnesses for long-running agents"
  (https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents,
  accessed 2026-05-14): feature lists, single-feature focus, progress tracking,
  and clean handoff state.
- OpenAI, "Harness engineering: leveraging Codex in an agent-first world"
  (https://openai.com/index/harness-engineering/, accessed 2026-05-14):
  repo-native skills, scripts, tests, review loops, and turning repeated agent
  failures into harness improvements.
- High-signal GitHub projects, accessed 2026-05-14:
  OpenHands (https://github.com/All-Hands-AI/OpenHands), SWE-agent
  (https://github.com/SWE-agent/SWE-agent), mini-swe-agent
  (https://github.com/SWE-agent/mini-swe-agent), SWE-bench
  (https://github.com/SWE-bench/SWE-bench), and OpenAI Evals
  (https://github.com/openai/evals). These inform runtime boundaries, task
  traces, lightweight CLI design, and evaluator-style checks.

Secondary references may inform terminology, but the design should stay rooted
in the primary references above and this repository's roadmap rules.

## Scope

- Add a P0 roadmap epic for the harness.
- Define a repo-local agent runtime with deterministic `make agent-*` entry
  points.
- Track the active epic, workflow phase, artifacts, and handoff state in files.
- Enforce one active roadmap epic at a time.
- Require spec and plan artifacts before implementation, except for explicit
  exception work kinds defined by the P0 prerequisite rule.
- Check consistency between roadmap status, runtime state, specs, plans, and
  handoff notes.
- Preserve this repository's safety-first bootstrap constraints.

## Non-Goals

- Do not build a general-purpose agent platform.
- Do not call external AI services from harness commands.
- Do not require network access for harness commands.
- Do not auto-run `make bootstrap` or dependency installers.
- Do not auto-merge, auto-push, or auto-create pull requests.
- Do not mark roadmap items `done` without recorded verification.
- Do not support multiple simultaneous active epics in v1.

## Design

The harness is implemented as a repo-local command runtime exposed through
`make agent-*` targets. It stores workflow state in ignored TOML, durable memory
in tracked handoff notes, and template inputs in tracked Markdown files. The
runtime validates roadmap, spec, plan, handoff, and verification consistency, but
it does not make semantic quality decisions or mutate machine state outside the
repository workflow files described here.

## Error Handling

Harness commands should fail closed when required context, artifacts, state
schema, or workflow gates are invalid. Failures should include a stable
`AGENT_*` error code where this spec defines one, followed by a human-readable
message that explains the corrective action. Commands must not silently overwrite
active state, stale handoffs, existing templates, roadmap status, dependency
state, shell state, symlinks, or git history.

## Design Principles

### Deterministic Runtime

The harness commands are local, deterministic rails. They inspect and update
repository files. They may create templates, print status, validate consistency,
and fail with actionable errors.

They must not invoke an LLM. The agent remains outside the runtime and uses the
runtime as a control and feedback layer.

### One Epic At A Time

The runtime allows only one active roadmap epic. This preserves the roadmap
priority rules and reduces partial work across unrelated safety-sensitive
areas.

### File-Based Memory

The repository, not the conversation, is the source of truth for agent state.
The runtime records current work in machine-readable state and human-readable
handoff notes.

### Feed-Forward And Feedback

Feed-forward artifacts tell the agent what to do:

- `AGENTS.md`
- `docs/roadmap.md`
- specs
- plans
- templates

Feedback artifacts tell the agent whether the work is acceptable:

- `make agent-check`
- narrow Rust tests
- `cargo test`
- `make check`
- `make lint`
- `make ci`

### Safety Before Automation

For this dotfiles manager, agent convenience must not outrank machine safety.
The harness may guide risky work, but it must not silently perform system
package installation, shell mutation, symlink changes, archive extraction, or
bootstrap execution.

## Runtime Commands

Expose deterministic commands through `make`:

```sh
make agent-init
make agent-next
make agent-start EPIC="P0 - Roadmap Agent Harness" WORK_KIND=roadmap
make agent-start EPIC="P0 - Roadmap Agent Harness" WORK_KIND=small_direct_edit EXCEPTION_REASON="user requested direct docs correction"
make agent-status
make agent-check
make agent-handoff MODE=create
make agent-handoff MODE=set SECTION=Phase VALUE=verifying
make agent-template KIND=spec
make agent-advance PHASE=planned
make agent-record-verification COMMAND="cargo test" RESULT=passed SUMMARY="all tests passed"
make agent-finish
```

### `make agent-init`

Initialize missing runtime state from repository context:

- README.md
- roadmap
- specs
- plans
- git status
- existing runtime state

It should create missing `state.toml` with conservative defaults. It should
never select an epic automatically.

`README.md` is an input-presence check only in v1. `agent-init` should fail if
it is missing, because missing project context means the repository is not in the
expected shape. It should not parse or derive runtime fields from README content.

Existing `state.toml` must be parsed before reuse. If the file has no
`schema_version`, an unsupported `schema_version`, or cannot be parsed,
`agent-init` should fail with a stable error code and instruct the user to move
or remove the stale state file. It should not overwrite an existing state file
silently.

### `make agent-next`

Print the next eligible roadmap epic according to priority, status, and declared
dependencies.

The command should prefer the highest-priority unblocked item and explain why
blocked items are skipped. It should not mutate state.

### `make agent-start`

Lock one roadmap epic for the current work session.

The command should fail if another epic is already locked. It should record the
epic title, priority, current roadmap status, expected spec path, expected plan
path, and phase.

It should accept optional structured metadata for deterministic checks:

```sh
make agent-start EPIC="P0 - Roadmap Agent Harness" WORK_KIND=roadmap
make agent-start EPIC="P0 - Roadmap Agent Harness" WORK_KIND=small_direct_edit EXCEPTION_REASON="user requested small direct edit"
```

Allowed `WORK_KIND` values:

- `roadmap`: normal roadmap epic work
- `small_direct_edit`: explicit user-requested small edit
- `emergency_fix`: explicit user-requested urgent fix
- `harness_docs`: documentation correction needed to finish the harness

When `WORK_KIND` is omitted, default to `roadmap`.

For `WORK_KIND=small_direct_edit`, `WORK_KIND=emergency_fix`, and
`WORK_KIND=harness_docs`, `EXCEPTION_REASON` is required and must be non-empty.
The runtime cannot prove user intent, so the exception boundary is auditability:
the caller must record a short reason that explicitly says why this work is an
exception to the spec-and-plan gate. `agent-start` should reject exception work
kinds without this reason, and `agent-check` should fail exception locks whose
state is missing it.

### `make agent-status`

Print concise human-readable state:

- current epic
- phase
- lock status
- spec path and existence
- plan path and existence
- last handoff path
- last verification summary

The human-readable output format is stable enough for smoke tests but not a
machine interface. The labels should be lower-case ASCII labels followed by a
colon, including at least:

```text
current epic:
phase:
locked:
work kind:
spec:
plan:
last handoff:
last verification:
```

Automation should use `state.toml` rather than parsing `agent-status`.

### `make agent-check`

Validate the workflow state without changing it.

Checks should include:

- active lock points to a roadmap item
- only one active epic is locked
- roadmap status is valid
- status and artifacts are consistent
- `specified` items have a spec
- `planned` and `in_progress` items have a spec and plan
- plans include explicit verification commands
- handoff notes include current epic, phase, exception reason, completed work,
  modified files, verification, unresolved risks, and next step
- P0 harness prerequisite is respected for runtime-declared implementation work

`agent-check` should emit stable error codes before human-readable messages.
Tests may assert these codes. Required codes include:

- `AGENT_MISSING_SPEC`
- `AGENT_MISSING_PLAN`
- `AGENT_ROADMAP_SPEC_UNLINKED`
- `AGENT_ROADMAP_PLAN_UNLINKED`
- `AGENT_PHASE_AHEAD`
- `AGENT_HANDOFF_MISSING`
- `AGENT_HANDOFF_INCOMPLETE`
- `AGENT_HANDOFF_MISMATCH`
- `AGENT_P0_PREREQUISITE`
- `AGENT_UNSUPPORTED_STATE_SCHEMA`
- `AGENT_MISSING_SPEC_SECTION`
- `AGENT_MISSING_PLAN_SECTION`
- `AGENT_FINISH_WRONG_PHASE`
- `AGENT_FINISH_NO_VERIFICATION`

### `make agent-handoff`

Create or validate a structured handoff note for the active epic.

The command should support deterministic modes:

```sh
make agent-handoff MODE=create
make agent-handoff MODE=validate
make agent-handoff MODE=set SECTION=Completed VALUE="- Wrote implementation plan."
```

`MODE=create` creates `current-handoff.md` from the template when it does not
exist. If it exists but does not match the active epic, `MODE=create` should
fail and instruct the user to finish or remove the stale handoff. `MODE=validate`
fails if required sections remain empty, the current epic does not match the
active lock, or the handoff phase does not match `state.phase`. The substantive
content should be written by the human or agent.

`MODE=set` updates one handoff section at a time and preserves the rest of the
file. It exists to avoid brittle ad hoc Markdown rewrites in agentic sessions.
It should require `SECTION` and `VALUE`, fail for unknown sections, and fail if
`current-handoff.md` is missing.

### `make agent-template`

Create a spec or plan file from the matching template.

Examples:

```sh
make agent-template KIND=spec
make agent-template KIND=plan
```

The command should use the active epic and deterministic path rules. It should
fail rather than overwrite an existing artifact.

### `make agent-advance`

Advance the active epic phase after the required artifact exists.

Examples:

```sh
make agent-advance PHASE=specified
make agent-advance PHASE=planned
make agent-advance PHASE=in_progress
make agent-advance PHASE=verifying
```

The command should update runtime state only. It should not edit
`docs/roadmap.md` automatically. If the roadmap status should change, the human
or agent must make that explicit document edit and `make agent-check` should
validate the result.

Before mutating state, `agent-advance` must run the same P0 prerequisite gate
used by `agent-check` for any transition into `in_progress` or `verifying`.
When `P0 - Roadmap Agent Harness` is not yet `done`, a different roadmap epic
must not be advanced into implementation work unless the active runtime state
uses an explicit exception work kind with a non-empty `exception_reason`.

### `make agent-record-verification`

Record structured verification evidence for the active epic.

Example:

```sh
make agent-record-verification COMMAND="cargo test" RESULT=passed SUMMARY="all tests passed"
```

The command should require:

- non-empty `COMMAND`
- `RESULT=passed` or `RESULT=failed`
- non-empty `SUMMARY`

It should append a structured entry to runtime state and to the
`## Verification` section in `current-handoff.md`. It should not run
verification commands itself. Humans and agents run the relevant commands first,
then record the result.

If `current-handoff.md` is missing, the command should fail and instruct the
user to run `make agent-handoff MODE=create` first.

For roadmap work, `agent-check` should warn when recorded verification commands
do not include the commands expected by the active implementation plan. It
should fail only when no verification evidence is recorded before finish.

Command comparison should use exact normalized command strings. Normalization is
limited to trimming leading and trailing whitespace and collapsing internal runs
of whitespace to one ASCII space. No shell parsing or semantic equivalence is
required.

### `make agent-finish`

Finish the active epic after verification has been recorded.

The command should:

- require current phase `verifying`
- require at least one recorded verification entry with `RESULT=passed`
- require a complete handoff note
- move `current-handoff.md` to a unique finished handoff path under `handoffs/`
- unlock the active epic
- record phase `done`

Failure conditions emit stable error codes:

- `AGENT_FINISH_WRONG_PHASE` when the active phase is not `verifying`
- `AGENT_FINISH_NO_VERIFICATION` when no verification entry with
  `RESULT=passed` is recorded
- `AGENT_HANDOFF_INCOMPLETE` (from handoff validation) when the handoff is
  missing required sections or contains only placeholder content

It should not change roadmap status automatically. The roadmap item may be
manually marked `done` after verification is recorded, then checked with
`make agent-check`.

After `agent-finish`, runtime phase `done` with roadmap status `proposed`,
`specified`, `planned`, or `in_progress` is a warning state, not an immediate
failure. It means local runtime completion has been recorded but the durable
roadmap queue still needs a manual status update. Runtime phase `done` with
roadmap status `done` is the fully consistent final state.

## Runtime Files

Add an agent runtime area:

```text
docs/superpowers/agent/
  README.md
  state.toml
  current-handoff.md
  handoffs/
    YYYY-MM-DD-<topic>.md
    YYYY-MM-DD-<topic>-2.md
  templates/
    spec.md
    plan.md
    handoff.md
```

`README.md` and templates should be tracked. `handoffs/` should be tracked when
handoff files are created because handoffs are durable project memory.

`state.toml` and `current-handoff.md` are mutable runtime files and should be
ignored by git. The implementation must update `.gitignore` for these paths.
`make agent-init` should recreate `state.toml` when missing.
`current-handoff.md` is created only by `make agent-handoff MODE=create` and is
moved away by `make agent-finish`. Finished handoffs should be tracked under
`handoffs/` so durable memory is committed without committing an active lock.
`agent-finish` must never overwrite an existing finished handoff. The first
candidate path is `handoffs/YYYY-MM-DD-<slug>.md`; if that exists, append
`-2`, then `-3`, and so on before `.md` until an unused path is found.

### `state.toml`

Machine-readable state:

```toml
schema_version = 1
current_epic = "P0 - Roadmap Agent Harness"
phase = "specified"
locked = true
work_kind = "roadmap"
exception_reason = ""
spec = "docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md"
plan = ""
current_handoff = "docs/superpowers/agent/current-handoff.md"
last_handoff = "docs/superpowers/agent/handoffs/2026-05-14-roadmap-agent-harness.md"

[[verification]]
command = "cargo test"
result = "passed"
summary = "all tests passed"
recorded_at = "2026-05-14"
```

`schema_version = 1` is required. v1 should reject unknown schema versions
rather than attempting a lossy migration. Additive optional fields may be
accepted only when they do not change the meaning of existing fields.

After `agent-finish`, state may retain `current_epic` for the completed item with
`phase = "done"` and `locked = false` so `agent-check` can compare the completed
runtime state with the durable roadmap status. The next successful `agent-start`
may overwrite that completed identity after verifying `locked = false`.

### Handoff Files

`current-handoff.md` is the working handoff for the active lock. On finish, it
should be moved into `handoffs/YYYY-MM-DD-<topic>.md` so handoff history is
durable and no stale active handoff remains.

Human-readable handoff format:

```md
# Agent Handoff

## Current Epic

P0 - Roadmap Agent Harness

## Phase

specified

## Exception Reason

- None.

## Completed

- Design direction selected: full deterministic agent runtime.

## Verification

- Not run yet; design phase only.

## Modified Files

- None yet.

## Unresolved Risks

- None recorded.

## Next Step

Write the implementation plan after this spec is reviewed.
```

## Workflow State Model

Runtime phases:

```text
uninitialized
initialized
selected
specified
planned
in_progress
verifying
done
```

Phase transitions:

```text
uninitialized -> initialized
initialized   -> selected      via agent-start
selected      -> specified     when spec exists and roadmap links it
specified     -> planned       when plan exists and roadmap links it
planned       -> in_progress   when implementation starts
in_progress   -> verifying     when implementation is ready for verification
verifying     -> done          via agent-finish after verification is recorded
```

Exception work-kind phase transitions:

```text
selected    -> in_progress     when exception work starts
in_progress -> verifying       when exception work is ready for verification
verifying   -> done            via agent-finish after verification is recorded
```

`agent-next` is always read-only and prints exactly one result: the
highest-priority unblocked epic. A `--all` or `--list` mode is deferred
to a future version; v1 focuses on single-epic workflow discipline.
`agent-start` records `selected`. For `WORK_KIND=roadmap`, it may
immediately record `specified` or `planned` if the required artifacts
already exist.

`agent-advance` should reject backward moves and phase skips, except that
exception work kinds may advance directly from `selected` to `in_progress`.
When the human updates `docs/roadmap.md` status ahead of the runtime (e.g.
manual status change to `in_progress` before running `agent-advance`),
the runtime should follow that lead. The normal order is: human edits roadmap,
then runtime advances to match. `agent-check` validates the resulting
consistency.
`agent-start` is the only command allowed to collapse initial roadmap-work
phases based on already existing artifacts. Missing roadmap links for an
existing spec or plan block roadmap-work advancement to `specified` or
`planned`; the artifact must be linked from the roadmap first.

Roadmap status mapping:

```text
proposed    -> no linked spec is recorded yet
specified   -> design/spec exists
planned     -> implementation plan exists
in_progress -> implementation has started
done        -> verification has passed and completion is recorded
deferred    -> skipped by agent-next unless explicitly requested
```

Invalid transitions should be reported by `make agent-check`. The runtime
should not silently rewrite roadmap status.

Roadmap/runtime mismatch severity:

- any active runtime phase with roadmap `deferred`: fail
- runtime `selected` with roadmap `proposed`: pass
- runtime `verifying` with roadmap `in_progress`: pass
- for `WORK_KIND=roadmap`, runtime phase ahead of roadmap status before
  `done`: fail
- for exception work kinds, runtime `in_progress` or `verifying` with roadmap
  `proposed`, `specified`, `planned`, or `in_progress`: pass
- roadmap status ahead of runtime phase: warn
- runtime `done` with roadmap `proposed`, `specified`, `planned`, or
  `in_progress`: warn
- runtime `done` with roadmap `done`: pass
- runtime `done` with roadmap `deferred`: fail

Artifact and handoff severity:

- roadmap `specified` or later without a linked spec path: fail
- roadmap `planned`, `in_progress`, or `done` without a linked plan path: fail
- linked spec path missing on disk for roadmap `specified` or later: fail
- linked plan path missing on disk for roadmap `planned`, `in_progress`, or
  `done`: fail
- spec file exists for the active epic but is not linked from roadmap when
  runtime phase is `specified` or later: fail
- plan file exists for the active epic but is not linked from roadmap when
  runtime phase is `planned` or later: fail
- missing current handoff for `selected`, `specified`, or `planned`: warn
- missing current handoff for `in_progress` or `verifying`: fail
- handoff required section is absent or contains only a placeholder: fail when
  handoff validation is required
- handoff current epic differs from `state.current_epic`: fail
- handoff phase differs from `state.phase`: fail

Placeholder handoff content is any required section whose trimmed body is empty,
`Not recorded yet.`, `Not run yet.`, or only a bullet containing one of those
phrases.

Self-review and user review are process gates for this repository's
brainstorming workflow, but they do not create additional roadmap status values.
When a spec exists and is linked from the roadmap, `specified` is the durable
roadmap status.

## P0 Prerequisite Rule

`P0 - Roadmap Agent Harness` must be completed before agentic implementation of
other roadmap epics begins.

Allowed exceptions:

- `WORK_KIND=small_direct_edit` for a user-requested small edit
- `WORK_KIND=emergency_fix` for a user-requested urgent fix
- `WORK_KIND=harness_docs` for documentation correction needed to finish the
  harness itself

Exception work kinds are not machine proof of user intent. They are allowed only
when `exception_reason` records a human-readable justification supplied at
`agent-start`. The reason is part of the audit trail and must be shown by
`agent-status`, recorded in state, and preserved in handoff notes.

Exception work still needs an active lock that points to a roadmap item. If the
work does not naturally belong to another roadmap epic, it should lock
`P0 - Roadmap Agent Harness` while that epic is active. This keeps all work
inside the one-epic runtime model and gives `agent-check` a deterministic item
to validate.

When the harness is not yet `done`, `make agent-check` should report other
runtime-declared implementation work as blocked unless it falls under an
explicit exception. The check must use explicit runtime metadata such as
`work_kind`; it must not infer user intent from file names or git diff
heuristics.

For `WORK_KIND=roadmap`, implementation work starts at runtime phase
`in_progress`. Other roadmap epics may still move through `selected`,
`specified`, and `planned` while the harness is unfinished, because spec and
plan preparation are allowed. Advancing any non-harness roadmap epic to
`in_progress` before the harness is `done` must fail unless the work kind is an
explicit exception.

Work-kind gates:

- `roadmap`
  - allowed phases: `selected..done`
  - required artifacts: spec for `specified+`; plan for `planned+`
- `small_direct_edit`
  - allowed phases: `selected`, `in_progress`, `verifying`, `done`
  - required artifacts: roadmap lock; handoff; verification before finish
- `emergency_fix`
  - allowed phases: `selected`, `in_progress`, `verifying`, `done`
  - required artifacts: roadmap lock; handoff; verification before finish
- `harness_docs`
  - allowed phases: `selected`, `in_progress`, `verifying`, `done`
  - required artifacts: roadmap lock; handoff; verification before finish

`small_direct_edit`, `emergency_fix`, and `harness_docs` bypass the
spec-before-implementation and plan-before-implementation gates, but they do
not bypass the one-epic lock, handoff, or verification gates.

## Quality Boundary

The harness is a workflow and evidence gate, not a semantic quality oracle. It
can check that specs, plans, handoffs, and verification records exist and meet
minimum structural requirements. It cannot prove that a spec is wise, a plan is
complete, or code is correct.

To reduce false confidence, v1 should enforce these structural quality floors:

- specs contain the required headings listed in this document
- plans contain `## Existing Code Map`, `## Expected Outcomes`, task headings,
  checkboxes, `**Files:**` blocks with backticked paths, and a
  parseable `## Verification Commands` section
- handoffs contain substantive notes for exception reason, completed work,
  modified files, verification, unresolved risks, and next step
- verification records include command, result, summary, and date
- final completion records at least one passing verification command

Human or agent review remains required for semantic quality, safety reasoning,
and whether the produced code actually satisfies the original request.

## Artifact Rules

### Specs

Specs live under:

```text
docs/superpowers/specs/YYYY-MM-DD-<topic>-design.md
```

A spec must include these parseable top-level headings:

- `## Goal`
- `## Scope`
- `## Non-Goals`
- `## Design`
- `## Error Handling`
- `## Verification Strategy`

Open questions may be included under `## Open Questions` only when they block
implementation planning.

Spec paths should be explicit in the roadmap when an item is `specified` or
later.

### Plans

Plans live under:

```text
docs/superpowers/plans/YYYY-MM-DD-<topic>.md
```

A plan should include:

- spec path
- goal
- architecture summary
- existing code or doc map
- task checklist
- concrete files
- expected verification commands
- expected outcomes

Expected verification commands must appear under a parseable heading:

```md
## Verification Commands

- `cargo test`
- `make check`
```

`agent-check` should compare recorded verification entries against these
backticked command strings using the normalized exact-match rule defined for
`agent-record-verification`.

Plan paths should be explicit in the roadmap when an item is `planned` or later.

The default plan path is derived from the spec path by:

1. removing the `-design` suffix from the file name
2. moving it from `docs/superpowers/specs/` to `docs/superpowers/plans/`

Example:

```text
docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md
docs/superpowers/plans/2026-05-14-roadmap-agent-harness.md
```

If a roadmap item does not yet have a spec, `make agent-start` should derive a
candidate topic slug from the epic title by lowercasing ASCII letters, replacing
non-alphanumeric runs with `-`, and trimming leading or trailing `-`.

The default spec path for an unlinked item is:

```text
docs/superpowers/specs/<current-local-date>-<slug>-design.md
```

The date is the local calendar date used by the running process, formatted as
`YYYY-MM-DD`. If the first candidate already exists and is linked to the same
roadmap item, reuse it. If it exists for another roadmap item or is unlinked,
append a numeric suffix before `-design.md`: `YYYY-MM-DD-<slug>-2-design.md`,
then `-3`, and so on until an unused or same-item-linked path is found.

### Handoff

Handoff notes should include:

- current epic
- phase
- exception reason
- completed work
- modified files
- verification commands and results
- unresolved risks
- next step

## Verification Strategy

The harness implementation should be tested without invoking AI or network
services.

Expected verification layers:

1. Unit tests for roadmap parsing, state parsing, status mapping, and dependency
   checks.
2. CLI or Makefile integration tests for `agent-*` commands using temporary
   fixture repositories.
3. Repository-level checks that validate the real roadmap, specs, plans, and
   agent state.
4. Existing project verification where relevant:
   - `cargo test <test_name>`
   - `cargo test`
   - `make check`
   - `make lint`
   - `make ci`

The implementation plan should choose the narrowest relevant command first and
broaden verification before claiming completion.

The implementation should include tests for edge cases where no roadmap item is
eligible because all items are deferred or blocked.

## Acceptance Criteria

- Roadmap contains a P0 harness epic and marks it as prerequisite work.
- Harness design exists and is reviewed.
- Harness implementation plan exists before code changes.
- `make agent-init` can initialize runtime state.
- `make agent-next` can identify the next eligible roadmap epic without
  mutating state.
- `make agent-start` can lock one epic and reject concurrent locks.
- exception `WORK_KIND` values require a non-empty exception reason.
- `make agent-status` prints current state.
- `make agent-check` catches missing specs, missing plans, invalid status
  transitions, incomplete handoff notes, and runtime-declared implementation
  work blocked by the P0 harness prerequisite.
- `agent-check` uses stable error codes for workflow failures.
- `make agent-handoff` creates or validates structured handoff notes.
- handoff validation rejects stale handoffs and phase mismatches.
- `make agent-template` creates spec or plan artifacts from templates without
  overwriting existing files.
- `make agent-advance` moves runtime phase only after required artifacts exist.
- `make agent-record-verification` records structured verification evidence in
  runtime state and handoff notes without running the verification commands.
- `make agent-finish` requires passing recorded verification and leaves no
  active lock.
- Mutable runtime files are ignored by git, `state.toml` is recreated by
  `make agent-init`, `current-handoff.md` is created by
  `make agent-handoff MODE=create`, and finished handoffs are tracked under
  `docs/superpowers/agent/handoffs/`.
- No harness command calls an LLM or requires network access.
- No harness command performs dependency installation, bootstrap, symlink
  mutation, shell mutation, git push, or merge actions.

## Risks

- The runtime could become too heavy for a personal dotfiles repository.
  Mitigation: keep v1 single-repo and single-epic only.
- The status model could duplicate roadmap state. Mitigation: runtime state
  records active work; roadmap remains the durable queue.
- Over-automation could hide risky actions. Mitigation: make harness commands
  inspect and validate, not perform machine-state changes.
- Agent sessions may still skip commands. Mitigation: make `AGENTS.md` and
  roadmap point to this P0 harness as a prerequisite once implemented.
- Concurrent sessions in the same checkout may race while reading and writing
  `state.toml` or `current-handoff.md`. Mitigation: v1 documents single-session
  use and avoids silent overwrites; cross-process locking is deferred unless
  repeated collisions occur.
- Exception work kinds can be abused by callers. Mitigation: require explicit
  `exception_reason`, preserve it in state and handoff notes, and make review
  responsible for validating that the exception was legitimate.
- Passing harness checks can create false confidence in low-quality artifacts.
  Mitigation: document the quality boundary and require review plus explicit
  verification evidence before marking work done.
