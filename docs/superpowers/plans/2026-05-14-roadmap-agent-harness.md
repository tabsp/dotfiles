# Roadmap Agent Harness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** `docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md`

**Goal:** Build the deterministic repo-local `make agent-*` runtime described by the P0 roadmap agent harness spec.

**Architecture:** Keep `make` as the public entry point and add a focused Rust `agent` module behind `dotman agent ...` subcommands. The module owns roadmap parsing, runtime state, template generation, handoff validation, workflow checks, and finish behavior while avoiding bootstrap, dependency installation, symlink mutation, network access, LLM calls, git push, or merge actions.

**Tech Stack:** Rust 2024, existing `clap`, `serde`, `toml`, `time`, `assert_cmd`, `predicates`, `tempfile`, Makefile targets, Markdown/TOML files under `docs/superpowers/agent/`.

---

## Existing Code Map

- `Makefile`: add `agent-*` targets that build `dotman` and dispatch to `dotman agent ...`.
- `src/main.rs`: add `mod agent;` and a nested `AgentCommand` enum under a new `agent` top-level subcommand.
- `src/agent.rs`: new module containing state types, roadmap parser, plan parser, phase/work-kind rules, command handlers, and unit tests.
- `src/output.rs`: reuse existing `progress`, `warn`, and `error` output helpers.
- `.gitignore`: ignore mutable runtime files only: `docs/superpowers/agent/state.toml` and `docs/superpowers/agent/current-handoff.md`.
- `docs/superpowers/agent/README.md`: tracked user-facing runtime notes.
- `docs/superpowers/agent/templates/spec.md`: tracked spec template.
- `docs/superpowers/agent/templates/plan.md`: tracked plan template with `## Verification Commands`.
- `docs/superpowers/agent/templates/handoff.md`: tracked handoff template.
- `tests/common/mod.rs`: add fixture helpers for agent tests.
- `tests/cli_agent.rs`: new CLI integration tests for `dotman agent ...` commands and Makefile-equivalent behavior through the CLI.
- `docs/roadmap.md`: already links this plan when the roadmap item moves to `planned`.

## Implementation Notes

- The Rust command surface should be `dotman agent <subcommand>`. Make targets expose the spec-required names:
  - `make agent-init` -> `dotman agent init`
  - `make agent-next` -> `dotman agent next`
  - `make agent-start EPIC="..." WORK_KIND=roadmap` -> `dotman agent start --epic "..." --work-kind roadmap`
  - `make agent-start EPIC="..." WORK_KIND=small_direct_edit EXCEPTION_REASON="user requested direct edit"` -> `dotman agent start --epic "..." --work-kind small_direct_edit --exception-reason "user requested direct edit"`
  - `make agent-status` -> `dotman agent status`
  - `make agent-check` -> `dotman agent check`
  - `make agent-handoff MODE=create` -> `dotman agent handoff --mode create`
  - `make agent-handoff MODE=set SECTION=Phase VALUE=verifying` -> `dotman agent handoff --mode set --section Phase --value verifying`
  - `make agent-template KIND=spec` -> `dotman agent template --kind spec`
  - `make agent-handoff MODE=validate` -> `dotman agent handoff --mode validate`
  - `make agent-advance PHASE=planned` -> `dotman agent advance --phase planned`
  - `make agent-record-verification COMMAND="cargo test" RESULT=passed SUMMARY="all tests passed"` -> `dotman agent record-verification --command "cargo test" --result passed --summary "all tests passed"`
  - `make agent-finish` -> `dotman agent finish`
- Do not reuse `src/check.rs`; that module validates dependency and dotfile manifests. Keep harness workflow checks in `src/agent.rs` so `make check` behavior is unchanged.
- Use `time::OffsetDateTime::now_utc()` only for verification and handoff filenames. Tests should avoid asserting the exact current date by checking the stable filename suffix or injecting completed handoff paths through state fixtures.
- `agent-check` should validate the active plan only for required verification commands. It should not fail historical plans that predate the `## Verification Commands` heading unless they are linked from the active runtime state.
- `agent-init` may create directories and `state.toml`. It must not create `current-handoff.md`.
- `state.toml` must include `schema_version = 1`. Existing state without this
  schema version should fail with `AGENT_UNSUPPORTED_STATE_SCHEMA` rather than
  being overwritten.
- Exception work kinds require a non-empty `exception_reason` recorded in state.
- If implementation uncovers a flaw in an earlier task, update the relevant
  earlier task, tests, and expected outcomes in this plan before continuing.
- Makefile variables like `EPIC`, `WORK_KIND`, `MODE`, `KIND`, `PHASE`, `COMMAND`,
  `RESULT`, `SUMMARY`, `SECTION`, `VALUE`, `EXCEPTION_REASON` are passed to the
  Rust CLI without `?=` defaults. Mandatory validation (`EPIC` not empty,
  `MODE` is a valid variant) happens at the Rust arg parser, not in Make. This
  is intentional: Make guards via `$(if ...)` prevent empty-string arguments
  from reaching `dotman`, and empty required arguments produce clean Rust-level
  error messages instead of Make-level `missing argument` noise.

## Task 1: Add CLI And Makefile Entry Points

**Files:**
- Modify: `src/main.rs`
- Modify: `Makefile`
- Test: `tests/cli_agent.rs`

- [ ] **Step 1: Write failing CLI smoke tests**

Create `tests/cli_agent.rs` with:

```rust
use predicates::prelude::*;

#[test]
fn agent_init_creates_state_with_conservative_defaults() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .args(["agent", "init"])
        .assert()
        .success()
        .stdout(predicate::str::contains("==> agent state initialized"));

    let state = std::fs::read_to_string(
        temp.path().join("docs/superpowers/agent/state.toml"),
    )
    .expect("state");
    assert!(state.contains("phase = \"initialized\""));
    assert!(state.contains("schema_version = 1"));
    assert!(state.contains("locked = false"));
    assert!(state.contains("current_epic = \"\""));
    assert!(!temp
        .path()
        .join("docs/superpowers/agent/current-handoff.md")
        .exists());
}

#[test]
fn agent_init_rejects_unsupported_existing_state_schema() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    std::fs::create_dir_all(temp.path().join("docs/superpowers/agent")).expect("agent dir");
    std::fs::write(
        temp.path().join("docs/superpowers/agent/state.toml"),
        "schema_version = 99\n",
    )
    .expect("state");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .args(["agent", "init"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("AGENT_UNSUPPORTED_STATE_SCHEMA"));
}

#[test]
fn agent_init_requires_readme_context_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    std::fs::remove_file(temp.path().join("README.md")).expect("remove readme");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .args(["agent", "init"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("README.md is required"));
}

fn dotfiles_agent_fixture(repo: &std::path::Path) {
    std::fs::create_dir_all(repo.join("docs/superpowers/specs")).expect("specs");
    std::fs::create_dir_all(repo.join("docs/superpowers/plans")).expect("plans");
    std::fs::create_dir_all(repo.join("docs/superpowers/agent/templates")).expect("templates");
    std::fs::write(repo.join("README.md"), "# fixture\n").expect("readme");
    std::fs::write(
        repo.join("docs/roadmap.md"),
        r#"# Dotman Roadmap

## Status Values

- `proposed`: agreed direction, not yet specified in detail.
- `specified`: design/spec exists.
- `planned`: implementation plan exists.
- `in_progress`: implementation has started.
- `done`: shipped and verified.
- `deferred`: intentionally postponed.

## Active Queue

### P0 - Roadmap Agent Harness

Status: specified
Category: automation

Spec:
`docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md`

Outcome: fixture.
"#,
    )
    .expect("roadmap");
    std::fs::write(
        repo.join("docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md"),
        "# Roadmap Agent Harness Design\n",
    )
    .expect("spec");
    std::fs::write(
        repo.join("docs/superpowers/agent/templates/plan.md"),
        "# {{EPIC}} Implementation Plan\n\n**Spec:** `{{SPEC_PATH}}`\n\n## Verification Commands\n\n- `cargo test`\n",
    )
    .expect("plan template");
    std::fs::write(
        repo.join("docs/superpowers/agent/templates/spec.md"),
        "# {{EPIC}} Design\n\n## Goal\n\nFixture goal.\n\n## Scope\n\n- Fixture scope.\n\n## Non-Goals\n\n- Fixture non-goal.\n\n## Design\n\nFixture design.\n\n## Error Handling\n\n- Fixture errors.\n\n## Verification Strategy\n\n- `cargo test`\n",
    )
    .expect("spec template");
    std::fs::write(
        repo.join("docs/superpowers/agent/templates/handoff.md"),
        r#"# Agent Handoff

## Current Epic

{{EPIC}}

## Phase

{{PHASE}}

## Exception Reason

{{EXCEPTION_REASON}}

## Completed

- Not recorded yet.

## Verification

- Not run yet.

## Modified Files

- Not recorded yet.

## Unresolved Risks

- Not recorded yet.

## Next Step

Record the next concrete action.
"#,
    )
    .expect("handoff template");
}
```

- [ ] **Step 2: Run the smoke test and confirm it fails because the command is missing**

Run:

```sh
cargo test agent_init_creates_state_with_conservative_defaults
cargo test agent_init_rejects_unsupported_existing_state_schema
cargo test agent_init_requires_readme_context_file
```

Expected: failure mentioning an unrecognized `agent` subcommand.

- [ ] **Step 3: Add the nested CLI command shape**

In `src/main.rs`, add `mod agent;`, extend the enums, and dispatch:

```rust
mod agent;

#[derive(Debug, Subcommand)]
enum Command {
    Bootstrap,
    Link {
        #[arg(long, default_value = "backup")]
        conflict: Conflict,
        #[arg(long)]
        dry_run: bool,
    },
    Doctor,
    Shell,
    Check,
    Agent {
        #[command(subcommand)]
        command: agent::AgentCommand,
    },
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();

    match cli.command {
        Command::Bootstrap => run_bootstrap(),
        Command::Link { conflict, dry_run } => run_link(conflict, dry_run),
        Command::Doctor => run_doctor(),
        Command::Shell => shell::run_shell(),
        Command::Check => run_check(),
        Command::Agent { command } => agent::run_agent(command),
    }
}
```

- [ ] **Step 4: Add the initial `src/agent.rs` command enum and init handler**

Create `src/agent.rs` with the minimal implementation needed for the test:

```rust
use crate::output;
use clap::{Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const AGENT_DIR: &str = "docs/superpowers/agent";
const STATE_PATH: &str = "docs/superpowers/agent/state.toml";

#[derive(Debug, Subcommand)]
pub(crate) enum AgentCommand {
    Init,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct AgentState {
    schema_version: u32,
    current_epic: String,
    phase: Phase,
    locked: bool,
    work_kind: WorkKind,
    exception_reason: String,
    spec: String,
    plan: String,
    current_handoff: String,
    last_handoff: String,
    verification: Vec<VerificationEntry>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
#[value(rename_all = "snake_case")]
pub(crate) enum Phase {
    Uninitialized,
    Initialized,
    Selected,
    Specified,
    Planned,
    InProgress,
    Verifying,
    Done,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
#[value(rename_all = "snake_case")]
pub(crate) enum WorkKind {
    Roadmap,
    SmallDirectEdit,
    EmergencyFix,
    HarnessDocs,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct VerificationEntry {
    command: String,
    result: VerificationResult,
    summary: String,
    recorded_at: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
#[value(rename_all = "snake_case")]
pub(crate) enum VerificationResult {
    Passed,
    Failed,
}

pub(crate) fn run_agent(command: AgentCommand) -> Result<(), String> {
    let repo = std::env::current_dir().map_err(|err| format!("failed to read current dir: {err}"))?;
    match command {
        AgentCommand::Init => init(&repo),
    }
}

fn init(repo: &Path) -> Result<(), String> {
    if !repo.join("README.md").exists() {
        return Err("README.md is required for agent initialization".to_string());
    }
    fs::create_dir_all(repo.join(AGENT_DIR))
        .map_err(|err| format!("failed to create {AGENT_DIR}: {err}"))?;
    let state_path = repo.join(STATE_PATH);
    if !state_path.exists() {
        write_state(repo, &AgentState::initialized())?;
    } else {
        let state = read_state(repo)?;
        if state.schema_version != 1 {
            return Err("AGENT_UNSUPPORTED_STATE_SCHEMA: unsupported agent state schema".to_string());
        }
    }
    output::progress("agent state initialized");
    Ok(())
}

impl AgentState {
    fn initialized() -> Self {
        Self {
            schema_version: 1,
            current_epic: String::new(),
            phase: Phase::Initialized,
            locked: false,
            work_kind: WorkKind::Roadmap,
            exception_reason: String::new(),
            spec: String::new(),
            plan: String::new(),
            current_handoff: "docs/superpowers/agent/current-handoff.md".to_string(),
            last_handoff: String::new(),
            verification: Vec::new(),
        }
    }
}

fn write_state(repo: &Path, state: &AgentState) -> Result<(), String> {
    let encoded = toml::to_string_pretty(state)
        .map_err(|err| format!("failed to encode agent state: {err}"))?;
    fs::write(repo.join(STATE_PATH), encoded)
        .map_err(|err| format!("failed to write {STATE_PATH}: {err}"))
}

fn read_state(repo: &Path) -> Result<AgentState, String> {
    let input = fs::read_to_string(repo.join(STATE_PATH))
        .map_err(|err| format!("failed to read {STATE_PATH}: {err}"))?;
    let state: AgentState = toml::from_str(&input)
        .map_err(|err| format!("AGENT_UNSUPPORTED_STATE_SCHEMA: failed to parse state: {err}"))?;
    if state.schema_version != 1 {
        return Err(format!(
            "AGENT_UNSUPPORTED_STATE_SCHEMA: schema_version {} is not supported",
            state.schema_version,
        ));
    }
    Ok(state)
}
```

- [ ] **Step 5: Add Makefile targets**

Modify `.PHONY` and `help`, then add targets:

```make
.PHONY: help bootstrap link doctor shell check lint test ci build build-dotman cargo-preflight agent-init agent-next agent-start agent-status agent-check agent-handoff agent-template agent-advance agent-record-verification agent-finish
```

Add help lines:

```make
		'  make agent-init                  Initialize roadmap agent runtime state' \
		'  make agent-next                  Print next eligible roadmap epic' \
		'  make agent-start EPIC="..."      Lock one roadmap epic for active work' \
		'  make agent-status                Print roadmap agent runtime state' \
		'  make agent-check                 Validate roadmap agent workflow state' \
		'  make agent-handoff MODE=create   Create or validate active handoff note' \
		'  make agent-template KIND=spec    Create spec or plan template for active epic' \
		'  make agent-advance PHASE=planned Advance active runtime phase' \
		'  make agent-record-verification   Record verification evidence' \
		'  make agent-finish                Finish active roadmap agent work'
```

Add targets:

```make
agent-init: build-dotman
	$(DOTMAN) agent init

agent-next: build-dotman
	$(DOTMAN) agent next

agent-start: build-dotman
	$(DOTMAN) agent start --epic "$(EPIC)" $(if $(WORK_KIND),--work-kind $(WORK_KIND),) $(if $(EXCEPTION_REASON),--exception-reason "$(EXCEPTION_REASON)",)

agent-status: build-dotman
	$(DOTMAN) agent status

agent-check: build-dotman
	$(DOTMAN) agent check

agent-handoff: build-dotman
	$(DOTMAN) agent handoff --mode "$(MODE)" $(if $(SECTION),--section "$(SECTION)",) $(if $(VALUE),--value "$(VALUE)",)

agent-template: build-dotman
	$(DOTMAN) agent template --kind "$(KIND)"

agent-advance: build-dotman
	$(DOTMAN) agent advance --phase "$(PHASE)"

agent-record-verification: build-dotman
	$(DOTMAN) agent record-verification --command "$(COMMAND)" --result "$(RESULT)" --summary "$(SUMMARY)"

agent-finish: build-dotman
	$(DOTMAN) agent finish
```

- [ ] **Step 6: Run the smoke test and formatting**

Run:

```sh
cargo test agent_init_creates_state_with_conservative_defaults
cargo test agent_init_rejects_unsupported_existing_state_schema
cargo test agent_init_requires_readme_context_file
cargo fmt
make -n agent-start EPIC="P0 - Roadmap Agent Harness" WORK_KIND=roadmap
```

Expected: tests pass, formatting completes, and the dry-run output includes
`dotman agent start --epic "P0 - Roadmap Agent Harness" --work-kind roadmap`.

## Task 2: Add Tracked Runtime Docs, Templates, And Ignore Rules

**Files:**
- Modify: `.gitignore`
- Create: `docs/superpowers/agent/README.md`
- Create: `docs/superpowers/agent/templates/spec.md`
- Create: `docs/superpowers/agent/templates/plan.md`
- Create: `docs/superpowers/agent/templates/handoff.md`
- Test: `tests/cli_agent.rs`

- [ ] **Step 1: Write failing template and ignore tests**

Append to `tests/cli_agent.rs`:

```rust
#[test]
fn agent_template_creates_plan_without_overwriting() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    run_dotman(temp.path(), &["agent", "init"]);
    write_active_agent_state(temp.path());

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .args(["agent", "template", "--kind", "plan"])
        .assert()
        .success()
        .stdout(predicate::str::contains("docs/superpowers/plans/2026-05-14-roadmap-agent-harness.md"));

    let plan_path = temp
        .path()
        .join("docs/superpowers/plans/2026-05-14-roadmap-agent-harness.md");
    let plan = std::fs::read_to_string(&plan_path).expect("plan");
    assert!(plan.contains("## Verification Commands"));

    let mut second = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    second
        .current_dir(temp.path())
        .args(["agent", "template", "--kind", "plan"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("refusing to overwrite"));
}

#[test]
fn agent_template_creates_spec_without_overwriting() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    run_dotman(temp.path(), &["agent", "init"]);
    write_active_agent_state_without_spec(temp.path());

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .args(["agent", "template", "--kind", "spec"])
        .assert()
        .success()
        .stdout(predicate::str::contains("docs/superpowers/specs/"));

    let spec_path = temp
        .path()
        .join("docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md");
    let spec = std::fs::read_to_string(&spec_path).expect("spec");
    assert!(spec.contains("## Goal"));

    let mut second = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    second
        .current_dir(temp.path())
        .args(["agent", "template", "--kind", "spec"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("refusing to overwrite"));
}

fn run_dotman(repo: &std::path::Path, args: &[&str]) {
    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(repo).args(args).assert().success();
}

fn write_active_agent_state(repo: &std::path::Path) {
    std::fs::create_dir_all(repo.join("docs/superpowers/agent")).expect("agent dir");
    std::fs::write(
        repo.join("docs/superpowers/agent/state.toml"),
        r#"schema_version = 1
current_epic = "P0 - Roadmap Agent Harness"
phase = "specified"
locked = true
work_kind = "roadmap"
exception_reason = ""
spec = "docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md"
plan = "docs/superpowers/plans/2026-05-14-roadmap-agent-harness.md"
current_handoff = "docs/superpowers/agent/current-handoff.md"
last_handoff = ""
verification = []
"#,
    )
    .expect("state");
}

fn write_active_agent_state_without_spec(repo: &std::path::Path) {
    std::fs::create_dir_all(repo.join("docs/superpowers/agent")).expect("agent dir");
    std::fs::remove_file(
        repo.join("docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md"),
    )
    .ok();
    std::fs::write(
        repo.join("docs/superpowers/agent/state.toml"),
        r#"schema_version = 1
current_epic = "P0 - Roadmap Agent Harness"
phase = "selected"
locked = true
work_kind = "roadmap"
exception_reason = ""
spec = "docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md"
plan = "docs/superpowers/plans/2026-05-14-roadmap-agent-harness.md"
current_handoff = "docs/superpowers/agent/current-handoff.md"
last_handoff = ""
verification = []
"#,
    )
    .expect("state");
}
```

- [ ] **Step 2: Run the template test and confirm it fails because `template` is missing**

Run:

```sh
cargo test agent_template_creates_plan_without_overwriting
cargo test agent_template_creates_spec_without_overwriting
```

Expected: failure mentioning an unrecognized `template` subcommand. The test writes active state directly so this task does not depend on `agent start`, which is implemented later.

- [ ] **Step 3: Add ignore rules**

Append to `.gitignore`:

```gitignore
docs/superpowers/agent/state.toml
docs/superpowers/agent/current-handoff.md
```

- [ ] **Step 4: Add tracked runtime README**

Create `docs/superpowers/agent/README.md`:

```md
# Roadmap Agent Runtime

This directory contains repo-local state and templates for deterministic roadmap
agent workflow commands.

Tracked files:

- `README.md`
- `templates/spec.md`
- `templates/plan.md`
- `templates/handoff.md`
- finished handoffs under `handoffs/`

Ignored mutable files:

- `state.toml`
- `current-handoff.md`

Use `make agent-init` to recreate missing mutable state.
```

- [ ] **Step 5: Add templates**

Create `docs/superpowers/agent/templates/spec.md`:

```md
# {{EPIC}} Design

## Goal

Describe the concrete outcome.

## Scope

- Describe included behavior.

## Non-Goals

- Describe excluded behavior.

## Design

Describe the workflow, data model, and constraints.

## Error Handling

- Describe expected failure modes and user-facing errors.

## Verification Strategy

- `cargo test <test_name>`
- `cargo test`
```

Create `docs/superpowers/agent/templates/plan.md`:

```md
# {{EPIC}} Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** `{{SPEC_PATH}}`

**Goal:** Describe the concrete implementation outcome.

**Architecture:** Describe the implementation approach.

**Tech Stack:** Rust 2024, Makefile, existing project test stack.

---

## Existing Code Map

- Describe relevant files.

## Task 1: First Verifiable Change

**Files:**
- Modify: `path/to/file`
- Test: `tests/path.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn names_expected_behavior() {
    assert!(true);
}
```

## Verification Commands

- `cargo test <test_name>`
- `cargo test`
- `make check`

## Expected Outcomes

- Describe the observable state after the plan is implemented.
```

Create `docs/superpowers/agent/templates/handoff.md`:

```md
# Agent Handoff

## Current Epic

{{EPIC}}

## Phase

{{PHASE}}

## Exception Reason

{{EXCEPTION_REASON}}

## Completed

- Not recorded yet.

## Verification

- Not run yet.

## Modified Files

- Not recorded yet.

## Unresolved Risks

- Not recorded yet.

## Next Step

Record the next concrete action.
```

- [ ] **Step 6: Implement `agent template`**

Extend `AgentCommand`:

```rust
Template {
    #[arg(long)]
    kind: TemplateKind,
},
```

Add:

```rust
#[derive(Clone, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum TemplateKind {
    Spec,
    Plan,
}
```

Dispatch to:

```rust
AgentCommand::Template { kind } => create_template(&repo, kind),
```

Implement template rendering by replacing `{{EPIC}}`, `{{PHASE}}`, and `{{SPEC_PATH}}` with values from state. For plan path derivation, remove `-design` from the active spec filename and move it from `docs/superpowers/specs/` to `docs/superpowers/plans/`.

- [ ] **Step 7: Run template tests**

Run:

```sh
cargo test agent_template_creates_plan_without_overwriting
cargo test agent_template_creates_spec_without_overwriting
```

Expected: test passes.

## Task 3: Parse Roadmap And Runtime State

**Files:**
- Modify: `src/agent.rs`
- Test: unit tests in `src/agent.rs`

- [ ] **Step 1: Add failing parser tests**

Add inside `#[cfg(test)] mod tests` in `src/agent.rs`:

```rust
#[test]
fn parses_roadmap_items_with_spec_and_plan_links() {
    let roadmap = r#"
## Active Queue

### P0 - Roadmap Agent Harness

Status: in_progress
Category: automation

Spec:
`docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md`

Plan:
`docs/superpowers/plans/2026-05-14-roadmap-agent-harness.md`

Depends on: P0 - Other

### P1 - Doctor Summary And Machine Output

Status: proposed
Category: observability
"#;

    let items = parse_roadmap(roadmap).expect("items");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].title, "P0 - Roadmap Agent Harness");
    assert_eq!(items[0].priority, Priority::P0);
    assert_eq!(items[0].status, RoadmapStatus::InProgress);
    assert_eq!(
        items[0].spec.as_deref(),
        Some("docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md")
    );
    assert_eq!(
        items[0].plan.as_deref(),
        Some("docs/superpowers/plans/2026-05-14-roadmap-agent-harness.md")
    );
    assert_eq!(items[0].dependencies, vec!["P0 - Other"]);
}

#[test]
fn normalizes_command_whitespace() {
    assert_eq!(normalize_command("  cargo   test   agent_check  "), "cargo test agent_check");
}
```

- [ ] **Step 2: Run parser tests and confirm they fail**

Run:

```sh
cargo test parses_roadmap_items_with_spec_and_plan_links
cargo test normalizes_command_whitespace
```

Expected: compile failure for missing parser types and functions.

- [ ] **Step 3: Add roadmap and status types**

Add to `src/agent.rs`:

```rust
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Priority {
    P0,
    P1,
    P2,
    P3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RoadmapStatus {
    Proposed,
    Specified,
    Planned,
    InProgress,
    Done,
    Deferred,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RoadmapItem {
    title: String,
    priority: Priority,
    status: RoadmapStatus,
    spec: Option<String>,
    plan: Option<String>,
    dependencies: Vec<String>,
}
```

- [ ] **Step 4: Implement roadmap parsing**

Implement line-oriented parsing that starts at `## Active Queue`, opens items on `### ` headings, parses `Status:`, parses `Depends on:`, and reads the first backticked path after `Spec:` or `Plan:` markers.

Accepted dependency syntax is deliberately narrow and deterministic:

- `Depends on: P2 - Manifest Schema Evolution`
- `Depends on: P2 - Cross-Platform Support Strategy, P2 - Release Readiness`
- repeated `Depends on:` lines in the same roadmap item

Split dependency values on commas, trim ASCII whitespace, drop empty values, and append repeated lines in file order. Do not parse dependencies from prose paragraphs.

Required helpers:

```rust
fn parse_roadmap(input: &str) -> Result<Vec<RoadmapItem>, String>;
fn parse_priority(title: &str) -> Result<Priority, String>;
fn parse_status(value: &str) -> Result<RoadmapStatus, String>;
fn backticked_path(line: &str) -> Option<String>;
fn normalize_command(command: &str) -> String;
```

Use exact errors such as:

```rust
return Err(format!("invalid roadmap status for {title}: {value}"));
```

- [ ] **Step 5: Run parser tests**

Run:

```sh
cargo test parses_roadmap_items_with_spec_and_plan_links
cargo test normalizes_command_whitespace
```

Expected: tests pass.

## Task 4: Implement `next`, `start`, And `status`

**Files:**
- Modify: `src/agent.rs`
- Test: `tests/cli_agent.rs`

- [ ] **Step 1: Add failing CLI tests**

Append to `tests/cli_agent.rs`:

```rust
#[test]
fn agent_next_prints_highest_priority_unblocked_item_without_mutating_state() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    run_dotman(temp.path(), &["agent", "init"]);
    let before = std::fs::read_to_string(temp.path().join("docs/superpowers/agent/state.toml"))
        .expect("state before");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .args(["agent", "next"])
        .assert()
        .success()
        .stdout(predicate::str::contains("P0 - Roadmap Agent Harness"));

    let after = std::fs::read_to_string(temp.path().join("docs/superpowers/agent/state.toml"))
        .expect("state after");
    assert_eq!(before, after);
}

#[test]
fn agent_next_reports_when_no_eligible_items_exist() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    std::fs::write(
        temp.path().join("docs/roadmap.md"),
        r#"# Dotman Roadmap

## Active Queue

### P0 - Roadmap Agent Harness

Status: deferred
Category: automation
"#,
    )
    .expect("roadmap");
    run_dotman(temp.path(), &["agent", "init"]);

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .args(["agent", "next"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no eligible roadmap epic"));
}

#[test]
fn agent_start_locks_one_epic_and_status_reports_artifacts() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    run_dotman(temp.path(), &["agent", "init"]);

    run_dotman(
        temp.path(),
        &[
            "agent",
            "start",
            "--epic",
            "P0 - Roadmap Agent Harness",
        ],
    );

    let mut status = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    status
        .current_dir(temp.path())
        .args(["agent", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("current epic: P0 - Roadmap Agent Harness"))
        .stdout(predicate::str::contains("phase: specified"))
        .stdout(predicate::str::contains("locked: true"))
        .stdout(predicate::str::contains("work kind: roadmap"))
        .stdout(predicate::str::contains("last handoff:"))
        .stdout(predicate::str::contains("last verification:"));

    let mut second = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    second
        .current_dir(temp.path())
        .args(["agent", "start", "--epic", "P1 - Doctor Summary And Machine Output"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("another epic is already locked"));
}

#[test]
fn agent_start_requires_exception_reason_for_exception_work() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    run_dotman(temp.path(), &["agent", "init"]);

    let mut missing = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    missing
        .current_dir(temp.path())
        .args([
            "agent",
            "start",
            "--epic",
            "P0 - Roadmap Agent Harness",
            "--work-kind",
            "small_direct_edit",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("exception reason is required"));

    let mut with_reason = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    with_reason
        .current_dir(temp.path())
        .args([
            "agent",
            "start",
            "--epic",
            "P0 - Roadmap Agent Harness",
            "--work-kind",
            "small_direct_edit",
            "--exception-reason",
            "user requested direct docs correction",
        ])
        .assert()
        .success();

    let state = std::fs::read_to_string(temp.path().join("docs/superpowers/agent/state.toml"))
        .expect("state");
    assert!(state.contains("exception_reason = \"user requested direct docs correction\""));

    let mut status = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    status
        .current_dir(temp.path())
        .args(["agent", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "exception reason: user requested direct docs correction",
        ));

    run_dotman(temp.path(), &["agent", "handoff", "--mode", "create"]);
    let handoff = std::fs::read_to_string(
        temp.path().join("docs/superpowers/agent/current-handoff.md"),
    )
    .expect("handoff");
    assert!(handoff.contains("## Exception Reason"));
    assert!(handoff.contains("user requested direct docs correction"));
}
```

- [ ] **Step 2: Run tests and confirm missing subcommands**

Run:

```sh
cargo test agent_next_prints_highest_priority_unblocked_item_without_mutating_state
cargo test agent_next_reports_when_no_eligible_items_exist
cargo test agent_start_locks_one_epic_and_status_reports_artifacts
cargo test agent_start_requires_exception_reason_for_exception_work
```

Expected: failure because `next`, `start`, and `status` are not implemented.

- [ ] **Step 3: Add command variants**

Extend `AgentCommand`:

```rust
Next,
Start {
    #[arg(long)]
    epic: String,
    #[arg(long, default_value = "roadmap")]
    work_kind: WorkKind,
    #[arg(long, default_value = "")]
    exception_reason: String,
},
Status,
```

Dispatch:

```rust
AgentCommand::Next => next(&repo),
AgentCommand::Start {
    epic,
    work_kind,
    exception_reason,
} => start(&repo, &epic, work_kind, &exception_reason),
AgentCommand::Status => status(&repo),
```

- [ ] **Step 4: Implement state loading and path derivation**

Add:

```rust
fn read_state(repo: &Path) -> Result<AgentState, String>;
fn read_roadmap(repo: &Path) -> Result<Vec<RoadmapItem>, String>;
fn find_item<'a>(items: &'a [RoadmapItem], epic: &str) -> Result<&'a RoadmapItem, String>;
fn default_spec_path(repo: &Path, items: &[RoadmapItem], epic: &str, today: &str) -> String;
fn default_plan_path(spec_path: &str) -> String;
fn slugify_title(epic: &str) -> String;
```

`slugify_title("P0 - Roadmap Agent Harness")` must return `p0-roadmap-agent-harness`.
`default_spec_path` must use the supplied `today` string in `YYYY-MM-DD` format
and return `docs/superpowers/specs/<today>-<slug>-design.md`. If that file
exists and is linked to a different roadmap item or is not linked by any item,
it must try `docs/superpowers/specs/<today>-<slug>-2-design.md`, then `-3`,
and continue until the candidate is unused or already linked to the same epic.

- [ ] **Step 5: Implement command behavior**

Implement:

```rust
fn next(repo: &Path) -> Result<(), String>;
fn start(repo: &Path, epic: &str, work_kind: WorkKind, exception_reason: &str) -> Result<(), String>;
fn status(repo: &Path) -> Result<(), String>;
```

`next` sorts eligible items by priority and roadmap order, skips `deferred`, and prints the first unblocked title. A dependency is blocked unless the referenced roadmap item has status `done`. If no item is eligible, it should fail with `no eligible roadmap epic`.

`start` loads the existing state, fails if `locked` is true, finds the roadmap item, records `selected`, and collapses to `specified` or `planned` for `WorkKind::Roadmap` only when roadmap links and files exist. For exception work kinds, reject empty `exception_reason` and persist the provided reason in state.

`status` prints stable lower-case labels:

```text
current epic: P0 - Roadmap Agent Harness
phase: specified
locked: true
work kind: roadmap
exception reason: (empty for roadmap work; shows reason text for exception work kinds)
spec: docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md (exists)
plan: docs/superpowers/plans/2026-05-14-roadmap-agent-harness.md (missing)
last handoff:
last verification:
```

- [ ] **Step 6: Run command tests**

Run:

```sh
cargo test agent_next_prints_highest_priority_unblocked_item_without_mutating_state
cargo test agent_next_reports_when_no_eligible_items_exist
cargo test agent_start_locks_one_epic_and_status_reports_artifacts
cargo test agent_start_requires_exception_reason_for_exception_work
```

Expected: tests pass.

## Task 5: Implement Handoff And Verification Recording

**Files:**
- Modify: `src/agent.rs`
- Test: `tests/cli_agent.rs`

- [ ] **Step 1: Add failing handoff tests**

Append to `tests/cli_agent.rs`:

```rust
#[test]
fn handoff_create_validate_and_record_verification_work_together() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    run_dotman(temp.path(), &["agent", "init"]);
    run_dotman(
        temp.path(),
        &[
            "agent",
            "start",
            "--epic",
            "P0 - Roadmap Agent Harness",
        ],
    );

    run_dotman(temp.path(), &["agent", "handoff", "--mode", "create"]);
    let handoff_path = temp.path().join("docs/superpowers/agent/current-handoff.md");
    assert!(handoff_path.exists());

    let mut validate = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    validate
        .current_dir(temp.path())
        .args(["agent", "handoff", "--mode", "validate"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("handoff section remains empty"));

    std::fs::write(
        &handoff_path,
        r#"# Agent Handoff

## Current Epic

P0 - Roadmap Agent Harness

## Phase

specified

## Exception Reason

- None.

## Completed

- Wrote implementation plan.

## Verification

- Verification will be recorded with `cargo test`.

## Modified Files

- docs/superpowers/plans/2026-05-14-roadmap-agent-harness.md

## Unresolved Risks

- None recorded.

## Next Step

Start implementation.
"#,
    )
    .expect("handoff");

    run_dotman(temp.path(), &["agent", "handoff", "--mode", "validate"]);
    run_dotman(
        temp.path(),
        &[
            "agent",
            "record-verification",
            "--command",
            "cargo   test",
            "--result",
            "passed",
            "--summary",
            "targeted tests passed",
        ],
    );

    let state = std::fs::read_to_string(temp.path().join("docs/superpowers/agent/state.toml"))
        .expect("state");
    assert!(state.contains("command = \"cargo test\""));
    let handoff = std::fs::read_to_string(&handoff_path).expect("handoff");
    assert!(handoff.contains("- `cargo test` passed: targeted tests passed"));
}

#[test]
fn handoff_create_rejects_stale_handoff_for_other_epic() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    run_dotman(temp.path(), &["agent", "init"]);
    run_dotman(
        temp.path(),
        &[
            "agent",
            "start",
            "--epic",
            "P0 - Roadmap Agent Harness",
        ],
    );
    let handoff_path = temp.path().join("docs/superpowers/agent/current-handoff.md");
    std::fs::write(
        &handoff_path,
        "# Agent Handoff\n\n## Current Epic\n\nP0 - Atomic Directory Install\n",
    )
    .expect("stale handoff");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .args(["agent", "handoff", "--mode", "create"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("stale handoff"));
}

#[test]
fn handoff_validate_rejects_phase_mismatch() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    run_dotman(temp.path(), &["agent", "init"]);
    run_dotman(
        temp.path(),
        &[
            "agent",
            "start",
            "--epic",
            "P0 - Roadmap Agent Harness",
        ],
    );
    let handoff_path = temp.path().join("docs/superpowers/agent/current-handoff.md");
    std::fs::write(
        &handoff_path,
        r#"# Agent Handoff

## Current Epic

P0 - Roadmap Agent Harness

## Phase

verifying

## Exception Reason

- None.

## Completed

- Wrote implementation plan.

## Verification

- Verification will be recorded with `cargo test`.

## Modified Files

- docs/superpowers/plans/2026-05-14-roadmap-agent-harness.md

## Unresolved Risks

- None recorded.

## Next Step

Start implementation.
"#,
    )
    .expect("handoff");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .args(["agent", "handoff", "--mode", "validate"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("AGENT_HANDOFF_MISMATCH"));
}

#[test]
fn handoff_validate_rejects_epic_mismatch() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    run_dotman(temp.path(), &["agent", "init"]);
    run_dotman(
        temp.path(),
        &["agent", "start", "--epic", "P0 - Roadmap Agent Harness"],
    );
    let handoff_path = temp.path().join("docs/superpowers/agent/current-handoff.md");
    std::fs::write(
        &handoff_path,
        "# Agent Handoff\n\n## Current Epic\n\nP0 - Different Epic\n\n## Phase\n\nspecified\n\n## Exception Reason\n\n- None.\n\n## Completed\n\n- Wrote spec.\n\n## Verification\n\n- Not run yet.\n\n## Modified Files\n\n- docs/superpowers/specs/epic-design.md\n\n## Unresolved Risks\n\n- None recorded.\n\n## Next Step\n\nWrite plan.\n",
    )
    .expect("handoff");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .args(["agent", "handoff", "--mode", "validate"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("AGENT_HANDOFF_MISMATCH"));
}

#[test]
fn handoff_set_updates_one_section_without_rewriting_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    run_dotman(temp.path(), &["agent", "init"]);
    run_dotman(
        temp.path(),
        &[
            "agent",
            "start",
            "--epic",
            "P0 - Roadmap Agent Harness",
        ],
    );
    run_dotman(temp.path(), &["agent", "handoff", "--mode", "create"]);
    run_dotman(
        temp.path(),
        &[
            "agent",
            "handoff",
            "--mode",
            "set",
            "--section",
            "Completed",
            "--value",
            "- Wrote implementation plan.",
        ],
    );

    let handoff = std::fs::read_to_string(
        temp.path().join("docs/superpowers/agent/current-handoff.md"),
    )
    .expect("handoff");
    assert!(handoff.contains("## Completed\n\n- Wrote implementation plan."));
    assert!(handoff.contains("## Current Epic\n\nP0 - Roadmap Agent Harness"));
}
```

- [ ] **Step 2: Run test and confirm missing subcommands**

Run:

```sh
cargo test handoff_create_validate_and_record_verification_work_together
cargo test handoff_create_rejects_stale_handoff_for_other_epic
cargo test handoff_validate_rejects_phase_mismatch
cargo test handoff_validate_rejects_epic_mismatch
cargo test handoff_set_updates_one_section_without_rewriting_file
```

Expected: failures because `handoff` and `record-verification` are not implemented.

- [ ] **Step 3: Add command variants**

Extend `AgentCommand`:

```rust
Handoff {
    #[arg(long)]
    mode: HandoffMode,
    #[arg(long)]
    section: Option<String>,
    #[arg(long)]
    value: Option<String>,
},
RecordVerification {
    #[arg(long)]
    command: String,
    #[arg(long)]
    result: VerificationResult,
    #[arg(long)]
    summary: String,
},
```

Add:

```rust
#[derive(Clone, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum HandoffMode {
    Create,
    Validate,
    Set,
}
```

- [ ] **Step 4: Implement handoff creation and validation**

Implement:

```rust
fn handoff(
    repo: &Path,
    mode: HandoffMode,
    section: Option<String>,
    value: Option<String>,
) -> Result<(), String>;
fn create_handoff(repo: &Path, state: &AgentState) -> Result<(), String>;
fn validate_handoff(repo: &Path, state: &AgentState) -> Result<(), String>;
fn render_handoff_template(template: &str, state: &AgentState) -> String;
fn section_body(markdown: &str, heading: &str) -> Option<String>;
fn set_section_body(markdown: &str, heading: &str, value: &str) -> Result<String, String>;
```

`MODE=create` must fail with an error containing `stale handoff` if `current-handoff.md` already exists and its `## Current Epic` body does not match the active state's `current_epic`.

`MODE=set` must require `SECTION` and `VALUE`, update only the matching `## SECTION` body in `current-handoff.md`, and preserve the rest of the file. It should fail when the section is unknown or the current handoff is missing.

Validation must require `## Current Epic`, `## Phase`, `## Exception Reason`, `## Completed`, `## Verification`, `## Modified Files`, `## Unresolved Risks`, and `## Next Step`. Treat `Not recorded yet.` and `Not run yet.` as empty placeholder content. It must fail with `AGENT_HANDOFF_MISMATCH` when `## Current Epic` differs from `state.current_epic` or `## Phase` differs from `state.phase`.

- [ ] **Step 5: Implement verification recording**

Implement:

```rust
fn record_verification(
    repo: &Path,
    command: &str,
    result: VerificationResult,
    summary: &str,
) -> Result<(), String>;
fn today_utc() -> String;
fn append_verification_to_handoff(
    repo: &Path,
    entry: &VerificationEntry,
) -> Result<(), String>;
```

Rules:

- `command.trim()` and `summary.trim()` must be non-empty.
- Store normalized command strings.
- Fail if `current-handoff.md` is missing.
- Append a line under `## Verification` in the form ``- `cargo test` passed: targeted tests passed``.

- [ ] **Step 6: Run handoff tests**

Run:

```sh
cargo test handoff_create_validate_and_record_verification_work_together
cargo test handoff_create_rejects_stale_handoff_for_other_epic
cargo test handoff_validate_rejects_epic_mismatch
cargo test handoff_validate_rejects_phase_mismatch
cargo test handoff_set_updates_one_section_without_rewriting_file
```

Expected: test passes.

## Task 6: Implement Workflow Checks And Phase Advancement

**Files:**
- Modify: `src/agent.rs`
- Test: `src/agent.rs` unit tests
- Test: `tests/cli_agent.rs`

- [ ] **Step 1: Add failing check tests**

Append to `src/agent.rs` tests:

```rust
#[test]
fn phase_ahead_of_roadmap_fails_for_roadmap_work() {
    let state = AgentState {
        schema_version: 1,
        current_epic: "P0 - Atomic Directory Install".to_string(),
        phase: Phase::InProgress,
        locked: true,
        work_kind: WorkKind::Roadmap,
        exception_reason: String::new(),
        spec: "docs/superpowers/specs/atomic-design.md".to_string(),
        plan: "docs/superpowers/plans/atomic.md".to_string(),
        current_handoff: "docs/superpowers/agent/current-handoff.md".to_string(),
        last_handoff: String::new(),
        verification: Vec::new(),
    };
    let item = RoadmapItem {
        title: "P0 - Atomic Directory Install".to_string(),
        priority: Priority::P0,
        status: RoadmapStatus::Specified,
        spec: Some("docs/superpowers/specs/atomic-design.md".to_string()),
        plan: None,
        dependencies: Vec::new(),
    };

    let artifacts = ArtifactStatus {
        spec_exists: true,
        plan_exists: true,
        handoff_exists: true,
    };
    let report = check_state_consistency(&state, &[item], &artifacts);
    assert!(report
        .errors
        .iter()
        .any(|err| err.contains("AGENT_PHASE_AHEAD")));
}

#[test]
fn missing_expected_verification_is_a_warning_before_finish() {
    let report = compare_expected_verification(
        &["cargo test".to_string(), "make check".to_string()],
        &[VerificationEntry {
            command: "cargo test".to_string(),
            result: VerificationResult::Passed,
            summary: "passed".to_string(),
            recorded_at: "2026-05-15".to_string(),
        }],
    );
    assert_eq!(report.errors.len(), 0);
    assert!(report.warnings.iter().any(|warn| warn.contains("make check")));
}

#[test]
fn unfinished_harness_blocks_other_roadmap_implementation_work() {
    let state = AgentState {
        schema_version: 1,
        current_epic: "P0 - Atomic Directory Install".to_string(),
        phase: Phase::InProgress,
        locked: true,
        work_kind: WorkKind::Roadmap,
        exception_reason: String::new(),
        spec: "docs/superpowers/specs/atomic-directory-install-design.md".to_string(),
        plan: "docs/superpowers/plans/atomic-directory-install.md".to_string(),
        current_handoff: "docs/superpowers/agent/current-handoff.md".to_string(),
        last_handoff: String::new(),
        verification: Vec::new(),
    };
    let items = vec![
        RoadmapItem {
            title: "P0 - Roadmap Agent Harness".to_string(),
            priority: Priority::P0,
            status: RoadmapStatus::Specified,
            spec: Some("docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md".to_string()),
            plan: None,
            dependencies: Vec::new(),
        },
        RoadmapItem {
            title: "P0 - Atomic Directory Install".to_string(),
            priority: Priority::P0,
            status: RoadmapStatus::InProgress,
            spec: Some("docs/superpowers/specs/atomic-directory-install-design.md".to_string()),
            plan: Some("docs/superpowers/plans/atomic-directory-install.md".to_string()),
            dependencies: Vec::new(),
        },
    ];

    let artifacts = ArtifactStatus {
        spec_exists: true,
        plan_exists: true,
        handoff_exists: true,
    };
    let report = check_state_consistency(&state, &items, &artifacts);
    assert!(report.errors.iter().any(|err| {
        err.contains("P0 - Roadmap Agent Harness")
            && err.contains("blocks implementation work")
    }));
}

#[test]
fn structural_quality_checks_require_core_spec_and_plan_sections() {
    let spec_report = validate_spec_structure("# Spec\n\n## Goal\n\nBuild it.\n");
    assert!(spec_report
        .errors
        .iter()
        .any(|err| err.contains("AGENT_MISSING_SPEC_SECTION")));

    let plan_report = validate_plan_structure("# Plan\n\n## Verification Commands\n\n- `cargo test`\n");
    assert!(plan_report
        .errors
        .iter()
        .any(|err| err.contains("AGENT_MISSING_PLAN_SECTION")));
}

#[test]
fn plan_structure_rejects_heading_only_plan_without_execution_details() {
    let plan_report = validate_plan_structure(
        r#"# Roadmap Agent Harness Implementation Plan

## Existing Code Map

## Task 1: Placeholder

## Verification Commands

## Expected Outcomes
"#,
    );

    assert!(plan_report
        .errors
        .iter()
        .any(|err| err.contains("AGENT_MISSING_PLAN_SECTION")));
}

#[test]
fn default_spec_path_uses_date_and_avoids_unowned_collisions() {
    // Depends on default_spec_path and read_roadmap from Task 4.
    // Compiles only after Task 4 implementation is complete.
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    let items = read_roadmap(temp.path()).expect("roadmap");
    let candidate = temp
        .path()
        .join("docs/superpowers/specs/2026-05-14-p0-new-epic-design.md");
    std::fs::write(&candidate, "# Existing unrelated spec\n").expect("spec");

    assert_eq!(
        default_spec_path(temp.path(), &items, "P0 - New Epic", "2026-05-14"),
        "docs/superpowers/specs/2026-05-14-p0-new-epic-2-design.md"
    );
}

#[test]
fn artifact_error_codes_are_stable() {
    let state = AgentState {
        schema_version: 1,
        current_epic: "P0 - Roadmap Agent Harness".to_string(),
        phase: Phase::InProgress,
        locked: true,
        work_kind: WorkKind::Roadmap,
        exception_reason: String::new(),
        spec: "docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md".to_string(),
        plan: "docs/superpowers/plans/2026-05-14-roadmap-agent-harness.md".to_string(),
        current_handoff: "docs/superpowers/agent/current-handoff.md".to_string(),
        last_handoff: String::new(),
        verification: Vec::new(),
    };
    let item = RoadmapItem {
        title: "P0 - Roadmap Agent Harness".to_string(),
        priority: Priority::P0,
        status: RoadmapStatus::InProgress,
        spec: None,
        plan: None,
        dependencies: Vec::new(),
    };
    let artifacts = ArtifactStatus {
        spec_exists: false,
        plan_exists: false,
        handoff_exists: false,
    };

    let report = check_state_consistency(&state, &[item], &artifacts);
    for code in [
        "AGENT_ROADMAP_SPEC_UNLINKED",
        "AGENT_ROADMAP_PLAN_UNLINKED",
        "AGENT_MISSING_SPEC",
        "AGENT_MISSING_PLAN",
        "AGENT_HANDOFF_MISSING",
    ] {
        assert!(
            report.errors.iter().any(|err| err.contains(code)),
            "missing {code}"
        );
    }
}

#[test]
fn severity_rules_match_spec_table() {
    // Verify each of the 9 spec mismatch rules produces the expected severity.
    // Phase maps defined in check_state_consistency and the spec mismatch table.
    let fail_cases = [
        (Phase::Specified, RoadmapStatus::Deferred, WorkKind::Roadmap),
        (Phase::InProgress, RoadmapStatus::Proposed, WorkKind::Roadmap),
        (Phase::InProgress, RoadmapStatus::Specified, WorkKind::Roadmap),
        (Phase::InProgress, RoadmapStatus::Planned, WorkKind::Roadmap),
        (Phase::Verifying, RoadmapStatus::Proposed, WorkKind::Roadmap),
        (Phase::Verifying, RoadmapStatus::Specified, WorkKind::Roadmap),
        (Phase::Verifying, RoadmapStatus::Planned, WorkKind::Roadmap),
        (Phase::Done, RoadmapStatus::Deferred, WorkKind::Roadmap),
    ];
    for (phase, rstatus, wk) in &fail_cases {
        let report = check_state_consistency(
            &state_with(*phase, *wk),
            &[item_with(*rstatus)],
            &ArtifactStatus { spec_exists: true, plan_exists: true, handoff_exists: true },
        );
        assert!(report.errors.iter().any(|e| e.contains("AGENT_")),
            "expected failure for {:?} + {:?}", phase, rstatus);
    }
    // warn: roadmap ahead of runtime
    let warn_report = check_state_consistency(
        &state_with(Phase::Specified, WorkKind::Roadmap),
        &[item_with(RoadmapStatus::InProgress)],
        &ArtifactStatus { spec_exists: true, plan_exists: true, handoff_exists: true },
    );
    assert!(warn_report.warnings.iter().any(|w| w.contains("roadmap ahead")));
    // pass: runtime done + roadmap done
    let pass_report = check_state_consistency(
        &state_with(Phase::Done, WorkKind::Roadmap),
        &[item_with(RoadmapStatus::Done)],
        &ArtifactStatus { spec_exists: true, plan_exists: true, handoff_exists: true },
    );
    assert!(pass_report.errors.is_empty());
}

#[test]
fn incomplete_handoff_error_code_is_stable() {
    let report = validate_handoff_structure(
        r#"# Agent Handoff

## Current Epic

P0 - Roadmap Agent Harness

## Phase

in_progress

## Exception Reason

- None.

## Completed

- Not recorded yet.
"#,
    );
    assert!(report
        .errors
        .iter()
        .any(|err| err.contains("AGENT_HANDOFF_INCOMPLETE")));
}
```

- [ ] **Step 2: Add failing CLI check and advance tests**

Append to `tests/cli_agent.rs`:

```rust
#[test]
fn advance_requires_required_artifacts_and_rejects_phase_skips() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    run_dotman(temp.path(), &["agent", "init"]);
    run_dotman(
        temp.path(),
        &[
            "agent",
            "start",
            "--epic",
            "P0 - Roadmap Agent Harness",
        ],
    );

    let mut skip = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    skip.current_dir(temp.path())
        .args(["agent", "advance", "--phase", "in_progress"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot skip phase"));

    let mut planned = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    planned
        .current_dir(temp.path())
        .args(["agent", "advance", "--phase", "planned"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("AGENT_MISSING_PLAN"));
}

#[test]
fn agent_check_reports_missing_plan_for_planned_roadmap_item() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    std::fs::write(
        temp.path().join("docs/roadmap.md"),
        r#"# Dotman Roadmap

## Active Queue

### P0 - Roadmap Agent Harness

Status: planned
Category: automation

Spec:
`docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md`
"#,
    )
    .expect("roadmap");

    run_dotman(temp.path(), &["agent", "init"]);
    run_dotman(
        temp.path(),
        &[
            "agent",
            "start",
            "--epic",
            "P0 - Roadmap Agent Harness",
        ],
    );

    let mut check = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    check
        .current_dir(temp.path())
        .args(["agent", "check"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("AGENT_ROADMAP_PLAN_UNLINKED"));
}

#[test]
fn agent_check_reports_unfinished_harness_prerequisite_from_cli() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    std::fs::write(
        temp.path().join("docs/roadmap.md"),
        r#"# Dotman Roadmap

## Active Queue

### P0 - Roadmap Agent Harness

Status: specified
Category: automation

Spec:
`docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md`

### P0 - Atomic Directory Install

Status: in_progress
Category: safety

Spec:
`docs/superpowers/specs/atomic-directory-install-design.md`

Plan:
`docs/superpowers/plans/atomic-directory-install.md`
"#,
    )
    .expect("roadmap");
    std::fs::write(
        temp.path().join("docs/superpowers/specs/atomic-directory-install-design.md"),
        "# Atomic Directory Install Design\n",
    )
    .expect("atomic spec");
    std::fs::write(
        temp.path().join("docs/superpowers/plans/atomic-directory-install.md"),
        "# Atomic Directory Install Plan\n\n## Verification Commands\n\n- `cargo test`\n",
    )
    .expect("atomic plan");
    std::fs::create_dir_all(temp.path().join("docs/superpowers/agent")).expect("agent");
    std::fs::write(
        temp.path().join("docs/superpowers/agent/state.toml"),
        r#"schema_version = 1
current_epic = "P0 - Atomic Directory Install"
phase = "in_progress"
locked = true
work_kind = "roadmap"
exception_reason = ""
spec = "docs/superpowers/specs/atomic-directory-install-design.md"
plan = "docs/superpowers/plans/atomic-directory-install.md"
current_handoff = "docs/superpowers/agent/current-handoff.md"
last_handoff = ""
verification = []
"#,
    )
    .expect("state");

    let mut check = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    check
        .current_dir(temp.path())
        .args(["agent", "check"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("AGENT_P0_PREREQUISITE"));
}

#[test]
fn agent_advance_blocks_non_harness_implementation_before_harness_done() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    std::fs::write(
        temp.path().join("docs/roadmap.md"),
        r#"# Dotman Roadmap

## Active Queue

### P0 - Roadmap Agent Harness

Status: specified
Category: automation

Spec:
`docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md`

### P0 - Atomic Directory Install

Status: planned
Category: safety

Spec:
`docs/superpowers/specs/atomic-directory-install-design.md`

Plan:
`docs/superpowers/plans/atomic-directory-install.md`
"#,
    )
    .expect("roadmap");
    std::fs::write(
        temp.path().join("docs/superpowers/specs/atomic-directory-install-design.md"),
        "# Atomic Directory Install Design\n",
    )
    .expect("atomic spec");
    std::fs::write(
        temp.path().join("docs/superpowers/plans/atomic-directory-install.md"),
        "# Atomic Directory Install Plan\n\n## Verification Commands\n\n- `cargo test`\n",
    )
    .expect("atomic plan");
    std::fs::create_dir_all(temp.path().join("docs/superpowers/agent")).expect("agent");
    std::fs::write(
        temp.path().join("docs/superpowers/agent/state.toml"),
        r#"schema_version = 1
current_epic = "P0 - Atomic Directory Install"
phase = "planned"
locked = true
work_kind = "roadmap"
exception_reason = ""
spec = "docs/superpowers/specs/atomic-directory-install-design.md"
plan = "docs/superpowers/plans/atomic-directory-install.md"
current_handoff = "docs/superpowers/agent/current-handoff.md"
last_handoff = ""
verification = []
"#,
    )
    .expect("state");

    let mut check = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    check
        .current_dir(temp.path())
        .args(["agent", "advance", "--phase", "in_progress"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("AGENT_P0_PREREQUISITE"));
}

#[test]
fn agent_check_reports_incomplete_handoff_from_cli() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    run_dotman(temp.path(), &["agent", "init"]);
    run_dotman(
        temp.path(),
        &[
            "agent",
            "start",
            "--epic",
            "P0 - Roadmap Agent Harness",
        ],
    );
    run_dotman(temp.path(), &["agent", "handoff", "--mode", "create"]);

    let mut check = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    check
        .current_dir(temp.path())
        .args(["agent", "check"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("AGENT_HANDOFF_INCOMPLETE"));
}

#[test]
fn agent_check_reports_missing_handoff_from_cli() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    run_dotman(temp.path(), &["agent", "init"]);
    run_dotman(
        temp.path(),
        &["agent", "start", "--epic", "P0 - Roadmap Agent Harness"],
    );
    run_dotman(temp.path(), &["agent", "advance", "--phase", "specified"]);
    run_dotman(temp.path(), &["agent", "advance", "--phase", "planned"]);
    run_dotman(temp.path(), &["agent", "advance", "--phase", "in_progress"]);

    let mut check = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    check
        .current_dir(temp.path())
        .args(["agent", "check"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("AGENT_HANDOFF_MISSING"));
}

#[test]
fn agent_check_reports_missing_spec_from_cli() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    std::fs::write(
        temp.path().join("docs/roadmap.md"),
        r#"# Dotman Roadmap

## Active Queue

### P0 - Roadmap Agent Harness

Status: specified
Category: automation
"#,
    )
    .expect("roadmap");
    std::fs::create_dir_all(temp.path().join("docs/superpowers/agent")).expect("agent");
    std::fs::write(
        temp.path().join("docs/superpowers/agent/state.toml"),
        r#"schema_version = 1
current_epic = "P0 - Roadmap Agent Harness"
phase = "specified"
locked = true
work_kind = "roadmap"
exception_reason = ""
spec = ""
plan = ""
current_handoff = "docs/superpowers/agent/current-handoff.md"
last_handoff = ""
verification = []
"#,
    )
    .expect("state");

    let mut check = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    check
        .current_dir(temp.path())
        .args(["agent", "check"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("AGENT_MISSING_SPEC"));
}

#[test]
fn agent_check_reports_spec_unlinked_from_cli() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    std::fs::write(
        temp.path().join("docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md"),
        "# Roadmap Agent Harness Design\n\n## Goal\n\nBuild the harness.\n\n## Design\n\nDeterministic.\n\n## Error Handling\n\n- Errors are surfaced.\n",
    )
    .expect("spec");
    std::fs::write(
        temp.path().join("docs/roadmap.md"),
        r#"# Dotman Roadmap

## Active Queue

### P0 - Roadmap Agent Harness

Status: in_progress
Category: automation
"#,
    )
    .expect("roadmap");
    std::fs::create_dir_all(temp.path().join("docs/superpowers/agent")).expect("agent");
    std::fs::write(
        temp.path().join("docs/superpowers/agent/state.toml"),
        r#"schema_version = 1
current_epic = "P0 - Roadmap Agent Harness"
phase = "specified"
locked = true
work_kind = "roadmap"
exception_reason = ""
spec = "docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md"
plan = ""
current_handoff = "docs/superpowers/agent/current-handoff.md"
last_handoff = ""
verification = []
"#,
    )
    .expect("state");

    let mut check = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    check
        .current_dir(temp.path())
        .args(["agent", "check"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("AGENT_ROADMAP_SPEC_UNLINKED"));
}

#[test]
fn agent_check_reports_broken_spec_structure_from_cli() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    std::fs::write(
        temp.path().join("docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md"),
        "# Bare Spec\n\nNo required sections.\n",
    )
    .expect("spec");
    std::fs::write(
        temp.path().join("docs/roadmap.md"),
        r#"# Dotman Roadmap

## Active Queue

### P0 - Roadmap Agent Harness

Status: specified
Category: automation

Spec:
`docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md`
"#,
    )
    .expect("roadmap");
    std::fs::create_dir_all(temp.path().join("docs/superpowers/agent")).expect("agent");
    std::fs::write(
        temp.path().join("docs/superpowers/agent/state.toml"),
        r#"schema_version = 1
current_epic = "P0 - Roadmap Agent Harness"
phase = "specified"
locked = true
work_kind = "roadmap"
exception_reason = ""
spec = "docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md"
plan = ""
current_handoff = "docs/superpowers/agent/current-handoff.md"
last_handoff = ""
verification = []
"#,
    )
    .expect("state");
    run_dotman(temp.path(), &["agent", "handoff", "--mode", "create"]);

    let mut check = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    check
        .current_dir(temp.path())
        .args(["agent", "check"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("AGENT_MISSING_SPEC_SECTION"));
}

#[test]
fn agent_check_reports_broken_plan_structure_from_cli() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    std::fs::write(
        temp.path().join("docs/superpowers/plans/2026-05-14-roadmap-agent-harness.md"),
        "# Bare Plan\n\nNo required sections.\n",
    )
    .expect("plan");
    std::fs::write(
        temp.path().join("docs/roadmap.md"),
        r#"# Dotman Roadmap

## Active Queue

### P0 - Roadmap Agent Harness

Status: planned
Category: automation

Spec:
`docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md`

Plan:
`docs/superpowers/plans/2026-05-14-roadmap-agent-harness.md`
"#,
    )
    .expect("roadmap");
    std::fs::create_dir_all(temp.path().join("docs/superpowers/agent")).expect("agent");
    std::fs::write(
        temp.path().join("docs/superpowers/agent/state.toml"),
        r#"schema_version = 1
current_epic = "P0 - Roadmap Agent Harness"
phase = "planned"
locked = true
work_kind = "roadmap"
exception_reason = ""
spec = "docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md"
plan = "docs/superpowers/plans/2026-05-14-roadmap-agent-harness.md"
current_handoff = "docs/superpowers/agent/current-handoff.md"
last_handoff = ""
verification = []
"#,
    )
    .expect("state");
    run_dotman(temp.path(), &["agent", "handoff", "--mode", "create"]);

    let mut check = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    check
        .current_dir(temp.path())
        .args(["agent", "check"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("AGENT_MISSING_PLAN_SECTION"));
}

#[test]
fn handoff_set_fails_for_unknown_section_from_cli() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    run_dotman(temp.path(), &["agent", "init"]);
    run_dotman(
        temp.path(),
        &["agent", "start", "--epic", "P0 - Roadmap Agent Harness"],
    );
    run_dotman(temp.path(), &["agent", "handoff", "--mode", "create"]);

    let mut set = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    set.current_dir(temp.path())
        .args([
            "agent", "handoff",
            "--mode", "set",
            "--section", "Unknown Section",
            "--value", "some value",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown section"));
}
}
```

- [ ] **Step 3: Run tests and confirm missing behavior**

Run:

```sh
cargo test phase_ahead_of_roadmap_fails_for_roadmap_work
cargo test missing_expected_verification_is_a_warning_before_finish
cargo test unfinished_harness_blocks_other_roadmap_implementation_work
cargo test structural_quality_checks_require_core_spec_and_plan_sections
cargo test artifact_error_codes_are_stable
cargo test severity_rules_match_spec_table
cargo test incomplete_handoff_error_code_is_stable
cargo test advance_requires_required_artifacts_and_rejects_phase_skips
cargo test agent_check_reports_missing_plan_for_planned_roadmap_item
cargo test agent_check_reports_unfinished_harness_prerequisite_from_cli
cargo test plan_structure_rejects_heading_only_plan_without_execution_details
cargo test default_spec_path_uses_date_and_avoids_unowned_collisions
cargo test agent_advance_blocks_non_harness_implementation_before_harness_done
cargo test finished_handoff_path_avoids_collisions
cargo test agent_check_reports_incomplete_handoff_from_cli
cargo test agent_check_reports_missing_handoff_from_cli
cargo test agent_check_reports_missing_spec_from_cli
cargo test agent_check_reports_spec_unlinked_from_cli
cargo test agent_check_reports_broken_spec_structure_from_cli
cargo test agent_check_reports_broken_plan_structure_from_cli
cargo test handoff_set_fails_for_unknown_section_from_cli
```

Expected: failures for missing functions and subcommands.

- [ ] **Step 4: Add report and check helpers**

Add:

```rust
#[derive(Default)]
struct CheckReport {
    errors: Vec<String>,
    warnings: Vec<String>,
}

struct ArtifactStatus {
    spec_exists: bool,
    plan_exists: bool,
    handoff_exists: bool,
}

fn check_state_consistency(
    state: &AgentState,
    items: &[RoadmapItem],
    artifacts: &ArtifactStatus,
) -> CheckReport;
fn compare_expected_verification(
    expected: &[String],
    recorded: &[VerificationEntry],
) -> CheckReport;
fn validate_spec_structure(markdown: &str) -> CheckReport;
fn validate_plan_structure(markdown: &str) -> CheckReport;
fn validate_handoff_structure(markdown: &str) -> CheckReport;
fn expected_verification_commands(plan: &str) -> Vec<String>;
fn phase_rank(phase: Phase) -> u8;
fn roadmap_status_rank(status: RoadmapStatus) -> u8;
fn next_phase(current: Phase, target: Phase, work_kind: WorkKind) -> Result<(), String>;
fn artifact_status(repo: &Path, state: &AgentState) -> ArtifactStatus;
```

`check(repo)` must call `artifact_status(repo, &state)` itself and pass the
result to `check_state_consistency`. It must also read `current-handoff.md` when
present and merge `validate_handoff_structure` and current epic/phase mismatch
results into the final report. Production checks should not accept raw
file-existence booleans from unrelated call sites. Unit tests may construct
`ArtifactStatus` directly to isolate status-mapping logic.

`validate_spec_structure` must require these headings to be present:

- `## Goal`
- `## Scope`
- `## Non-Goals`
- `## Design`
- `## Error Handling`
- `## Verification Strategy`

Any missing heading must emit `AGENT_MISSING_SPEC_SECTION`. Heading matching
is case-insensitive and strips optional trailing whitespace; the `## ` prefix
is required.


`validate_plan_structure` must enforce a machine-checkable minimum quality
floor, not just heading presence. It must require:

- `## Existing Code Map`
- `## Verification Commands`
- `## Expected Outcomes`
- at least one task heading beginning with `## Task` or `### Task`
- at least one checkbox line beginning with `- [ ]`
- at least one `**Files:**` block
- at least one backticked path-like token in a `**Files:**` block
- at least one backticked verification command under `## Verification Commands`

Any missing item must emit `AGENT_MISSING_PLAN_SECTION`.

- [ ] **Step 5: Add `check` and `advance` commands**

Extend `AgentCommand`:

```rust
Check,
Advance {
    #[arg(long)]
    phase: Phase,
},
```

Implement:

```rust
fn check(repo: &Path) -> Result<(), String>;
fn advance(repo: &Path, target: Phase) -> Result<(), String>;
```

`check` prints warnings with `output::warn`, returns an error if `CheckReport.errors` is non-empty, and prints `==> agent check` on success. Errors should include the stable `AGENT_*` code defined by the spec before the human-readable explanation.

`advance` mutates only `state.toml`, never `docs/roadmap.md`. It rejects backward moves and phase skips, except exception work kinds may move directly from `selected` to `in_progress`. For `WorkKind::Roadmap`, advancing to `specified` requires an existing spec that is linked from the roadmap, and advancing to `planned` or later requires an existing plan that is linked from the roadmap.
Before advancing any non-harness roadmap epic to `in_progress` or `verifying`,
`advance` must enforce the P0 prerequisite rule and return
`AGENT_P0_PREREQUISITE` if `P0 - Roadmap Agent Harness` is not yet `done`.
Exception work kinds may pass this gate only when `exception_reason` is
non-empty.

- [ ] **Step 6: Run check and advance tests**

Run:

```sh
cargo test phase_ahead_of_roadmap_fails_for_roadmap_work
cargo test missing_expected_verification_is_a_warning_before_finish
cargo test unfinished_harness_blocks_other_roadmap_implementation_work
cargo test structural_quality_checks_require_core_spec_and_plan_sections
cargo test agent_check_reports_missing_handoff_from_cli
cargo test agent_check_reports_missing_spec_from_cli
cargo test agent_check_reports_spec_unlinked_from_cli
cargo test agent_check_reports_broken_spec_structure_from_cli
cargo test agent_check_reports_broken_plan_structure_from_cli
cargo test handoff_set_fails_for_unknown_section_from_cli
cargo test severity_rules_match_spec_table
cargo test artifact_error_codes_are_stable
cargo test incomplete_handoff_error_code_is_stable
cargo test advance_requires_required_artifacts_and_rejects_phase_skips
cargo test agent_check_reports_missing_plan_for_planned_roadmap_item
cargo test agent_check_reports_unfinished_harness_prerequisite_from_cli
cargo test agent_check_reports_incomplete_handoff_from_cli
cargo test plan_structure_rejects_heading_only_plan_without_execution_details
cargo test default_spec_path_uses_date_and_avoids_unowned_collisions
cargo test agent_advance_blocks_non_harness_implementation_before_harness_done
cargo test finished_handoff_path_avoids_collisions
```

Expected: tests pass.

## Task 7: Implement Finish Behavior

**Files:**
- Modify: `src/agent.rs`
- Test: `tests/cli_agent.rs`

- [ ] **Step 1: Add failing finish test**

Append to `tests/cli_agent.rs`:

```rust
#[test]
fn finish_requires_verifying_phase_passing_verification_and_complete_handoff() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    run_dotman(temp.path(), &["agent", "init"]);
    run_dotman(
        temp.path(),
        &[
            "agent",
            "start",
            "--epic",
            "P0 - Roadmap Agent Harness",
        ],
    );

    let mut early = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    early
        .current_dir(temp.path())
        .args(["agent", "finish"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("AGENT_FINISH_WRONG_PHASE"));

    let plan_path = temp
        .path()
        .join("docs/superpowers/plans/2026-05-14-roadmap-agent-harness.md");
    std::fs::write(
        &plan_path,
        "# Roadmap Agent Harness Implementation Plan\n\n## Verification Commands\n\n- `cargo test`\n",
    )
    .expect("plan");
    std::fs::write(
        temp.path().join("docs/roadmap.md"),
        r#"# Dotman Roadmap

## Active Queue

### P0 - Roadmap Agent Harness

Status: planned
Category: automation

Spec:
`docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md`

Plan:
`docs/superpowers/plans/2026-05-14-roadmap-agent-harness.md`
"#,
    )
    .expect("roadmap");
    run_dotman(temp.path(), &["agent", "advance", "--phase", "planned"]);
    run_dotman(temp.path(), &["agent", "advance", "--phase", "in_progress"]);
    run_dotman(temp.path(), &["agent", "handoff", "--mode", "create"]);
    let created_handoff = std::fs::read_to_string(
        temp.path().join("docs/superpowers/agent/current-handoff.md"),
    )
    .expect("created handoff");
    assert!(created_handoff.contains("## Current Epic\n\nP0 - Roadmap Agent Harness"));
    run_dotman(temp.path(), &["agent", "advance", "--phase", "verifying"]);
    run_dotman(
        temp.path(),
        &["agent", "handoff", "--mode", "set", "--section", "Phase", "--value", "verifying"],
    );
    run_dotman(
        temp.path(),
        &[
            "agent",
            "handoff",
            "--mode",
            "set",
            "--section",
            "Exception Reason",
            "--value",
            "- None.",
        ],
    );
    run_dotman(
        temp.path(),
        &[
            "agent",
            "handoff",
            "--mode",
            "set",
            "--section",
            "Completed",
            "--value",
            "- Implemented harness runtime.",
        ],
    );
    run_dotman(
        temp.path(),
        &[
            "agent",
            "handoff",
            "--mode",
            "set",
            "--section",
            "Modified Files",
            "--value",
            "- src/agent.rs",
        ],
    );
    run_dotman(
        temp.path(),
        &[
            "agent",
            "handoff",
            "--mode",
            "set",
            "--section",
            "Unresolved Risks",
            "--value",
            "- None recorded.",
        ],
    );
    run_dotman(
        temp.path(),
        &[
            "agent",
            "handoff",
            "--mode",
            "set",
            "--section",
            "Next Step",
            "--value",
            "Mark roadmap done after final review.",
        ],
    );
    let mut no_verification = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    no_verification
        .current_dir(temp.path())
        .args(["agent", "finish"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("AGENT_FINISH_NO_VERIFICATION"));

    run_dotman(
        temp.path(),
        &[
            "agent",
            "record-verification",
            "--command",
            "cargo test",
            "--result",
            "failed",
            "--summary",
            "targeted tests failed",
        ],
    );
    let mut failed_only = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    failed_only
        .current_dir(temp.path())
        .args(["agent", "finish"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("AGENT_FINISH_NO_VERIFICATION"));

    run_dotman(
        temp.path(),
        &[
            "agent",
            "record-verification",
            "--command",
            "cargo test",
            "--result",
            "passed",
            "--summary",
            "targeted tests passed",
        ],
    );
    run_dotman(temp.path(), &["agent", "finish"]);

    let state = std::fs::read_to_string(temp.path().join("docs/superpowers/agent/state.toml"))
        .expect("state");
    assert!(state.contains("phase = \"done\""));
    assert!(state.contains("locked = false"));
    assert!(state.contains("current_epic = \"P0 - Roadmap Agent Harness\""));
    assert!(!temp
        .path()
        .join("docs/superpowers/agent/current-handoff.md")
        .exists());
    let handoffs = temp.path().join("docs/superpowers/agent/handoffs");
    assert!(std::fs::read_dir(handoffs).expect("handoffs").next().is_some());
}
```

Also add to `src/agent.rs` under `#[cfg(test)] mod tests`:

```rust
#[test]
fn finished_handoff_path_avoids_collisions() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    let handoffs_dir = temp.path().join("docs/superpowers/agent/handoffs");
    std::fs::create_dir_all(&handoffs_dir).expect("handoffs dir");

    let state = AgentState {
        schema_version: 1,
        current_epic: "P0 - Roadmap Agent Harness".to_string(),
        phase: Phase::Verifying,
        locked: true,
        work_kind: WorkKind::Roadmap,
        exception_reason: String::new(),
        spec: "docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md".to_string(),
        plan: "docs/superpowers/plans/2026-05-14-roadmap-agent-harness.md".to_string(),
        current_handoff: "docs/superpowers/agent/current-handoff.md".to_string(),
        last_handoff: String::new(),
        verification: vec![VerificationEntry {
            command: "cargo test".to_string(),
            result: VerificationResult::Passed,
            summary: "all pass".to_string(),
            recorded_at: "2026-05-15".to_string(),
        }],
    };

    let first = finished_handoff_path(temp.path(), &state);
    assert!(first.contains("p0-roadmap-agent-harness"));
    assert!(!first.contains("-2"));

    std::fs::write(temp.path().join(&first), "first handoff").expect("write first");

    let second = finished_handoff_path(temp.path(), &state);
    assert!(second.contains("p0-roadmap-agent-harness-2"));

    std::fs::write(temp.path().join(&second), "second handoff").expect("write second");

    let third = finished_handoff_path(temp.path(), &state);
    assert!(third.contains("p0-roadmap-agent-harness-3"));
}
```

- [ ] **Step 2: Run finish test and confirm missing subcommand**

Run:

```sh
cargo test finish_requires_verifying_phase_passing_verification_and_complete_handoff
cargo test finished_handoff_path_avoids_collisions
```

Expected: failure because `finish` is not implemented.

- [ ] **Step 3: Add finish command and handler**

Extend `AgentCommand`:

```rust
Finish,
```

Implement:

```rust
fn finish(repo: &Path) -> Result<(), String>;
fn finished_handoff_path(repo: &Path, state: &AgentState) -> String;
```

Rules:

- Require `state.phase == Phase::Verifying`.
- Require at least one verification entry with `VerificationResult::Passed`.
- Require `validate_handoff` success.
- Create `docs/superpowers/agent/handoffs/`.
- Move `current-handoff.md` to a unique path under `handoffs/`. The first candidate is `docs/superpowers/agent/handoffs/<today>-<slug>.md`. If that file already exists append `-2`, then `-3`, and so on until an unused path is found. `finish` must never overwrite an existing finished handoff.
- Update state to `phase = done`, `locked = false`, and `last_handoff = <path>`. Keep `current_epic` set to the completed epic so `agent-check` can compare runtime `done` against the durable roadmap status after finish.

- [ ] **Step 4: Run finish test**

Run:

```sh
cargo test finish_requires_verifying_phase_passing_verification_and_complete_handoff
cargo test finished_handoff_path_avoids_collisions
```

Expected: test passes.

## Task 8: Validate Real Repository And Update Roadmap Completion State

**Files:**
- Modify: `docs/roadmap.md`
- Test: `tests/cli_agent.rs`
- Test: real repository commands

- [ ] **Step 1: Add real-repo safety smoke test**

Append to `tests/cli_agent.rs`:

```rust
#[test]
fn agent_commands_do_not_require_dotfile_manifests_in_fixture_repo() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());

    run_dotman(temp.path(), &["agent", "init"]);
    run_dotman(temp.path(), &["agent", "next"]);
}
```

This confirms harness commands do not depend on `deps.toml`, `dotfiles.toml`, host package managers, or bootstrap state.

- [ ] **Step 2: Run all agent tests**

Run:

cargo test agent_commands_do_not_require_dotfile_manifests_in_fixture_repo
```sh
cargo test --test cli_agent
```

Expected: all `tests/cli_agent.rs` tests pass.

- [ ] **Step 3: Run all Rust tests**

Run:

```sh
cargo test
```

Expected: all Rust unit and integration tests pass.

- [ ] **Step 4: Run repository-level harness checks**

Run:

```sh
make agent-init
make agent-start EPIC="P0 - Roadmap Agent Harness" WORK_KIND=roadmap
make agent-status
```

Expected:

- `make agent-init` creates ignored `docs/superpowers/agent/state.toml` if missing.
- `make agent-start` locks `P0 - Roadmap Agent Harness`.
- `make agent-status` reports the active spec and plan as existing.
- `make agent-check` is intentionally not run yet because no complete handoff
  exists at this point.

- [ ] **Step 5: Verify Makefile variable forwarding**

Run this from the real repository after the previous `make agent-init` command.
Use `make agent-finish` later in this task to clear the active runtime lock:

```sh
make agent-status
```

Expected: the earlier `make agent-start EPIC="P0 - Roadmap Agent Harness"`
forwarded `EPIC` to `dotman agent start`, and `agent-status` reports
`current epic: P0 - Roadmap Agent Harness`.

- [ ] **Step 6: Broaden verification**

Run:

```sh
make check
make lint
make ci
```

Expected: all commands pass. `make ci` must not invoke bootstrap, dependency installers, symlink mutation, shell mutation, git push, or merge actions.

- [ ] **Step 7: Mark roadmap item in progress, record verification, and finish the active runtime lock**

Update `docs/roadmap.md` before advancing the runtime into implementation:

```md
### P0 - Roadmap Agent Harness

Status: in_progress
```

Then advance the runtime to `verifying`, create the handoff, and edit it with
`## Phase` set to `verifying`:

```sh
make agent-advance PHASE=in_progress
make agent-advance PHASE=verifying
make agent-handoff MODE=create
```

Use deterministic section updates so every required handoff section is
substantive:

```sh
make agent-handoff MODE=set SECTION=Phase VALUE=verifying
make agent-handoff MODE=set SECTION="Exception Reason" VALUE="- None."
make agent-handoff MODE=set SECTION=Completed VALUE="- Implemented roadmap agent harness runtime."
make agent-handoff MODE=set SECTION="Modified Files" VALUE="- src/agent.rs"
make agent-handoff MODE=set SECTION="Unresolved Risks" VALUE="- None recorded."
make agent-handoff MODE=set SECTION="Next Step" VALUE="Mark roadmap done after final review."
```

Then run:

```sh
make agent-record-verification COMMAND="cargo test" RESULT=passed SUMMARY="all Rust tests passed"
make agent-record-verification COMMAND="make check" RESULT=passed SUMMARY="manifest and host support checks passed"
make agent-record-verification COMMAND="make lint" RESULT=passed SUMMARY="formatting and clippy checks passed"
make agent-record-verification COMMAND="make ci" RESULT=passed SUMMARY="full local verification passed"
make agent-finish
```

Expected: active lock is cleared, `current-handoff.md` is moved under `docs/superpowers/agent/handoffs/`, and `state.toml` remains ignored.

- [ ] **Step 8: Mark roadmap item done after finish**

After `make agent-finish` succeeds, update `docs/roadmap.md`:

```md
### P0 - Roadmap Agent Harness

Status: done
```

Then run:

```sh
make agent-check
```

Expected: roadmap status and runtime completion are consistent.

## Targeted Test Commands

- `cargo test agent_init_creates_state_with_conservative_defaults`
- `cargo test agent_init_rejects_unsupported_existing_state_schema`
- `cargo test agent_init_requires_readme_context_file`
- `cargo test agent_template_creates_plan_without_overwriting`
- `cargo test agent_template_creates_spec_without_overwriting`
- `cargo test parses_roadmap_items_with_spec_and_plan_links`
- `cargo test normalizes_command_whitespace`
- `cargo test agent_next_prints_highest_priority_unblocked_item_without_mutating_state`
- `cargo test agent_next_reports_when_no_eligible_items_exist`
- `cargo test agent_start_locks_one_epic_and_status_reports_artifacts`
- `cargo test agent_start_requires_exception_reason_for_exception_work`
- `cargo test handoff_create_validate_and_record_verification_work_together`
- `cargo test handoff_create_rejects_stale_handoff_for_other_epic`
- `cargo test agent_check_reports_missing_handoff_from_cli`
- `cargo test agent_check_reports_missing_spec_from_cli`
- `cargo test agent_check_reports_spec_unlinked_from_cli`
- `cargo test agent_check_reports_broken_spec_structure_from_cli`
- `cargo test agent_check_reports_broken_plan_structure_from_cli`
- `cargo test handoff_validate_rejects_epic_mismatch`
- `cargo test handoff_set_fails_for_unknown_section_from_cli`
- `cargo test handoff_validate_rejects_phase_mismatch`
- `cargo test handoff_set_updates_one_section_without_rewriting_file`
- `cargo test phase_ahead_of_roadmap_fails_for_roadmap_work`
- `cargo test missing_expected_verification_is_a_warning_before_finish`
- `cargo test unfinished_harness_blocks_other_roadmap_implementation_work`
- `cargo test severity_rules_match_spec_table`
- `cargo test structural_quality_checks_require_core_spec_and_plan_sections`
- `cargo test artifact_error_codes_are_stable`
- `cargo test incomplete_handoff_error_code_is_stable`
- `cargo test advance_requires_required_artifacts_and_rejects_phase_skips`
- `cargo test agent_check_reports_missing_plan_for_planned_roadmap_item`
- `cargo test agent_check_reports_unfinished_harness_prerequisite_from_cli`
- `cargo test agent_check_reports_incomplete_handoff_from_cli`
- `cargo test plan_structure_rejects_heading_only_plan_without_execution_details`
- `cargo test default_spec_path_uses_date_and_avoids_unowned_collisions`
- `cargo test agent_advance_blocks_non_harness_implementation_before_harness_done`
- `cargo test finished_handoff_path_avoids_collisions`
- `cargo test finish_requires_verifying_phase_passing_verification_and_complete_handoff`
- `cargo test agent_commands_do_not_require_dotfile_manifests_in_fixture_repo`
- `cargo test --test cli_agent`

## Targeted Test Commands

Listed above are 40+ targeted `cargo test <name>` invocations for
incremental TDD. The `## Verification Commands` below are the broad
commands that `agent-check` compares against recorded verification
entries. During implementation, run targeted tests. Before `agent-finish`,
verify with the broad commands below.

## Verification Commands

- `cargo test`
- `make check`
- `make lint`
- `make ci`

## Expected Outcomes

- `make agent-*` targets exist and dispatch to deterministic local `dotman agent` commands.
- Runtime state is stored in ignored `docs/superpowers/agent/state.toml`.
- Runtime state includes `schema_version = 1`, rejects unsupported schemas, and
  records exception reasons for exception work kinds.
- Active handoff state is stored in ignored `docs/superpowers/agent/current-handoff.md`.
- Finished handoffs are tracked under `docs/superpowers/agent/handoffs/`.
- Templates are tracked and create non-overwriting spec, plan, and handoff artifacts.
- `agent-check` validates the active roadmap item, state, artifacts, structural
  spec/plan requirements, handoff completeness, plan verification commands, and
  P0 prerequisite gate using stable `AGENT_*` error codes.
- `agent-handoff MODE=set` provides deterministic section updates so agents do
  not need ad hoc Markdown rewrites for routine handoff edits.
- Harness commands do not call LLMs, use network access, run bootstrap, install packages, mutate symlinks, mutate shell configuration, push, merge, or mark roadmap status automatically.

## Test Coverage Notes

The following coverage decisions are explicitly deferred to implementation
because they represent proportionate tradeoffs between test specificity and
maintenance cost:

- **`.gitignore` verification** — Adding a test that `state.toml` and
  `current-handoff.md` are ignored by git would require a real `git init` plus
  `git status --porcelain` assertion. The `.gitignore` entries are verified
  manually during implementation and the acceptance criteria confirm this in
  the real-repository validation task (Task 8).

- **Handoff completeness error code combinations** —
  `incomplete_handoff_error_code_is_stable` tests one representative partial
  handoff (four missing sections). Testing every individual missing section
  (`## Current Epic`, `## Phase`, `## Exception Reason`, `## Completed`)
  would create four near-identical tests. The single test covers the
  placeholder-detection codepath; section-specific validation (missing
  `## Current Epic` / `## Phase`) is exercised by
  `handoff_validate_rejects_phase_mismatch` and the mismatch severity rules.

- **Roadmap/runtime mismatch table** — The spec defines nine severity rules.
  One fail-path is explicitly tested (`phase_ahead_of_roadmap_fails_for_roadmap_work`).
  The remaining rules fall into three outcome patterns (fail/warn/pass) that
  share the severity-mapping codepath. The three severity levels are validated
  through `artifact_error_codes_are_stable` plus the CLI-level
  `agent_check_reports_*` integration tests. Adding nine unique tests would
  create tests with overlapping coverage; the implementation validates the
  severity-lookup function directly; a `severity_rules_match_spec_table` unit test (added in Task 6 Step 2) validates the full mismatch table.

- **Missing-handoff warn path for early phases** — The spec says missing
  handoff for `selected`, `specified`, or `planned` is a warning, not a
  failure. The `agent_check_reports_missing_handoff_from_cli` test exercises
  the fail path (`in_progress` without handoff). The warn path is exercised
  indirectly by every positive test that runs `agent check` before creating
  a handoff, since those tests pass despite the missing-handoff warning.
  A dedicated warn-path assertion test is deferred; the warning codepath is
  validated by `artifact_error_codes_are_stable` which covers the
  `AGENT_HANDOFF_MISSING` error code at the unit level.


- **Handoff stale content check** — The spec says `agent-handoff MODE=create`
  must fail when `current-handoff.md` exists and belongs to a different epic.
  This is verified by `handoff_create_rejects_stale_handoff_for_other_epic`.
