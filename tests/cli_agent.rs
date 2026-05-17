use predicates::prelude::*;

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

    let state = std::fs::read_to_string(temp.path().join("docs/superpowers/agent/state.toml"))
        .expect("state");
    assert!(state.contains("phase = \"initialized\""));
    assert!(state.contains("schema_version = 1"));
    assert!(state.contains("locked = false"));
    assert!(state.contains("current_epic = \"\""));
    assert!(
        !temp
            .path()
            .join("docs/superpowers/agent/current-handoff.md")
            .exists()
    );
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
        .stdout(predicate::str::contains(
            "docs/superpowers/plans/2026-05-14-roadmap-agent-harness.md",
        ));

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
        &["agent", "start", "--epic", "P0 - Roadmap Agent Harness"],
    );

    let mut status = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    status
        .current_dir(temp.path())
        .args(["agent", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "current epic: P0 - Roadmap Agent Harness",
        ))
        .stdout(predicate::str::contains("phase: specified"))
        .stdout(predicate::str::contains("locked: true"))
        .stdout(predicate::str::contains("work kind: roadmap"))
        .stdout(predicate::str::contains("last handoff:"))
        .stdout(predicate::str::contains("last verification:"));

    let mut second = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    second
        .current_dir(temp.path())
        .args([
            "agent",
            "start",
            "--epic",
            "P1 - Doctor Summary And Machine Output",
        ])
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
        temp.path()
            .join("docs/superpowers/agent/current-handoff.md"),
    )
    .expect("handoff");
    assert!(handoff.contains("## Exception Reason"));
    assert!(handoff.contains("user requested direct docs correction"));
}

#[test]
fn handoff_create_validate_and_record_verification_work_together() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    run_dotman(temp.path(), &["agent", "init"]);
    run_dotman(
        temp.path(),
        &["agent", "start", "--epic", "P0 - Roadmap Agent Harness"],
    );

    run_dotman(temp.path(), &["agent", "handoff", "--mode", "create"]);
    let handoff_path = temp
        .path()
        .join("docs/superpowers/agent/current-handoff.md");
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
        &["agent", "start", "--epic", "P0 - Roadmap Agent Harness"],
    );
    let handoff_path = temp
        .path()
        .join("docs/superpowers/agent/current-handoff.md");
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
        &["agent", "start", "--epic", "P0 - Roadmap Agent Harness"],
    );
    let handoff_path = temp
        .path()
        .join("docs/superpowers/agent/current-handoff.md");
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
    let handoff_path = temp
        .path()
        .join("docs/superpowers/agent/current-handoff.md");
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
        &["agent", "start", "--epic", "P0 - Roadmap Agent Harness"],
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
            "--value=- Wrote implementation plan.",
        ],
    );

    let handoff = std::fs::read_to_string(
        temp.path()
            .join("docs/superpowers/agent/current-handoff.md"),
    )
    .expect("handoff");
    assert!(handoff.contains("## Completed\n\n- Wrote implementation plan."));
    assert!(handoff.contains("## Current Epic\n\nP0 - Roadmap Agent Harness"));
}

#[test]
fn advance_requires_required_artifacts_and_rejects_phase_skips() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    run_dotman(temp.path(), &["agent", "init"]);
    run_dotman(
        temp.path(),
        &["agent", "start", "--epic", "P0 - Roadmap Agent Harness"],
    );

    // Artifact check runs first: advancing to planned without a plan file fails
    let mut planned = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    planned
        .current_dir(temp.path())
        .args(["agent", "advance", "--phase", "planned"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("AGENT_MISSING_PLAN"));

    // Create a plan file so artifact check passes, then verify phase skip is caught
    std::fs::write(
        temp.path()
            .join("docs/superpowers/plans/2026-05-14-roadmap-agent-harness.md"),
        "# Plan

## Verification Commands

- `cargo test`
",
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

    let mut skip = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    skip.current_dir(temp.path())
        .args(["agent", "advance", "--phase", "in_progress"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot skip phase"));
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
        &["agent", "start", "--epic", "P0 - Roadmap Agent Harness"],
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
        temp.path()
            .join("docs/superpowers/specs/atomic-directory-install-design.md"),
        "# Atomic Directory Install Design\n",
    )
    .expect("atomic spec");
    std::fs::write(
        temp.path()
            .join("docs/superpowers/plans/atomic-directory-install.md"),
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
        temp.path()
            .join("docs/superpowers/specs/atomic-directory-install-design.md"),
        "# Atomic Directory Install Design\n",
    )
    .expect("atomic spec");
    std::fs::write(
        temp.path()
            .join("docs/superpowers/plans/atomic-directory-install.md"),
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
        &["agent", "start", "--epic", "P0 - Roadmap Agent Harness"],
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
    // start sets phase to specified (spec exists), advance to planned needs plan file
    std::fs::write(
        temp.path()
            .join("docs/superpowers/plans/2026-05-14-roadmap-agent-harness.md"),
        "# Roadmap Agent Harness Plan

## Verification Commands

- `cargo test`
",
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
        .stderr(predicate::str::contains("AGENT_ROADMAP_SPEC_UNLINKED"));
}

#[test]
fn finish_requires_verifying_phase_passing_verification_and_complete_handoff() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    run_dotman(temp.path(), &["agent", "init"]);
    run_dotman(
        temp.path(),
        &["agent", "start", "--epic", "P0 - Roadmap Agent Harness"],
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
    run_dotman(temp.path(), &["agent", "advance", "--phase", "verifying"]);
    run_dotman(
        temp.path(),
        &[
            "agent",
            "handoff",
            "--mode",
            "set",
            "--section",
            "Phase",
            "--value=verifying",
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
            "Exception Reason",
            "--value=- None.",
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
            "--value=- Implemented harness runtime.",
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
            "--value=- src/agent.rs",
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
            "--value=- None recorded.",
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
            "--value=Mark roadmap done after final review.",
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
    assert!(
        !temp
            .path()
            .join("docs/superpowers/agent/current-handoff.md")
            .exists()
    );
    let handoffs = temp.path().join("docs/superpowers/agent/handoffs");
    assert!(
        std::fs::read_dir(handoffs)
            .expect("handoffs")
            .next()
            .is_some()
    );
}

#[test]
fn agent_check_reports_spec_unlinked_from_cli() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
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
phase = "in_progress"
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
    // Write a spec that is missing required sections
    std::fs::write(
        temp.path()
            .join("docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md"),
        "# Bare Spec\n\nNo required sections.\n",
    )
    .expect("bare spec");
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
        .stderr(predicate::str::contains("AGENT_MISSING_SPEC_SECTION"));
}

#[test]
fn agent_check_reports_broken_plan_structure_from_cli() {
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

Plan:
`docs/superpowers/plans/2026-05-14-roadmap-agent-harness.md`
"#,
    )
    .expect("roadmap");
    // Write a plan that is missing required sections
    std::fs::write(
        temp.path()
            .join("docs/superpowers/plans/2026-05-14-roadmap-agent-harness.md"),
        "# Bare Plan\n\nNo required sections.\n",
    )
    .expect("bare plan");
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

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .args([
            "agent",
            "handoff",
            "--mode",
            "set",
            "--section",
            "NonexistentSection",
            "--value=some value",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown handoff section"));
}

#[test]
fn default_spec_path_uses_date_and_avoids_unowned_collisions() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    // This test exercises default_spec_path collision avoidance through the
    // template command, which calls default_spec_path internally.
    run_dotman(temp.path(), &["agent", "init"]);
    // Create a spec that belongs to a different epic
    let collision = temp
        .path()
        .join("docs/superpowers/specs/2026-05-17-p0-new-epic-design.md");
    std::fs::write(&collision, "# Existing unrelated spec\n").expect("spec");
    // Write roadmap with a new epic
    std::fs::write(
        temp.path().join("docs/roadmap.md"),
        r#"# Dotman Roadmap

## Active Queue

### P0 - New Epic

Status: proposed
Category: automation
"#,
    )
    .expect("roadmap");
    std::fs::create_dir_all(temp.path().join("docs/superpowers/agent")).expect("agent");
    std::fs::write(
        temp.path().join("docs/superpowers/agent/state.toml"),
        r#"schema_version = 1
current_epic = "P0 - New Epic"
phase = "selected"
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

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .args(["agent", "template", "--kind", "spec"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("p0-new-epic-2-design.md")
                .or(predicate::str::contains("p0-new-epic-design.md")),
        );
}

#[test]
fn agent_commands_do_not_require_dotfile_manifests_in_fixture_repo() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    run_dotman(temp.path(), &["agent", "init"]);
    // These commands should work without any manifests or dotfiles present
    run_dotman(temp.path(), &["agent", "next"]);
    run_dotman(temp.path(), &["agent", "status"]);
    run_dotman(
        temp.path(),
        &["agent", "start", "--epic", "P0 - Roadmap Agent Harness"],
    );
    // Check may fail on artifact issues but must not mention bootstrap/manifests
    let mut check = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    let output = check
        .current_dir(temp.path())
        .args(["agent", "check"])
        .output()
        .expect("check");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("bootstrap") && !stderr.contains("manifest"),
        "agent check should not require dotfile manifests: {stderr}"
    );
}

#[test]
fn agent_set_roadmap_status_updates_only_active_epic() {
    let temp = tempfile::tempdir().expect("tempdir");
    dotfiles_agent_fixture(temp.path());
    // Write a roadmap with multiple epics, some sharing the same status
    std::fs::write(
        temp.path().join("docs/roadmap.md"),
        r#"# Dotman Roadmap

## Active Queue

### P0 - Roadmap Agent Harness

Status: specified
Category: automation

### P0 - Atomic Directory Install

Status: specified
Category: safety
"#,
    )
    .expect("roadmap");
    run_dotman(temp.path(), &["agent", "init"]);
    run_dotman(
        temp.path(),
        &["agent", "start", "--epic", "P0 - Roadmap Agent Harness"],
    );

    run_dotman(
        temp.path(),
        &["agent", "set-roadmap-status", "--status", "planned"],
    );

    let roadmap = std::fs::read_to_string(temp.path().join("docs/roadmap.md")).expect("roadmap");
    // Only the harness epic should change; Atomic Directory Install stays specified
    assert!(roadmap.contains("### P0 - Roadmap Agent Harness\n\nStatus: planned"));
    assert!(roadmap.contains("### P0 - Atomic Directory Install\n\nStatus: specified"));
}
