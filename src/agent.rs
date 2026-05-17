use crate::output;
use clap::{Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

const AGENT_DIR: &str = "docs/superpowers/agent";
const STATE_PATH: &str = "docs/superpowers/agent/state.toml";

#[derive(Debug, Subcommand)]
pub(crate) enum AgentCommand {
    Template {
        #[arg(long)]
        kind: TemplateKind,
    },
    Init,
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
    Check,
    Advance {
        #[arg(long)]
        phase: Phase,
    },
    Finish,
    SetRoadmapStatus {
        #[arg(long)]
        status: String,
    },
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ValueEnum)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct VerificationEntry {
    command: String,
    result: VerificationResult,
    summary: String,
    recorded_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum TemplateKind {
    Spec,
    Plan,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
#[value(rename_all = "snake_case")]
pub(crate) enum VerificationResult {
    Passed,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum HandoffMode {
    Create,
    Validate,
    Set,
}

pub(crate) fn run_agent(command: AgentCommand) -> Result<(), String> {
    let repo =
        std::env::current_dir().map_err(|err| format!("failed to read current dir: {err}"))?;
    match command {
        AgentCommand::Init => init(&repo),
        AgentCommand::Template { kind } => create_template(&repo, kind),
        AgentCommand::Next => next(&repo),
        AgentCommand::Start {
            epic,
            work_kind,
            exception_reason,
        } => start(&repo, &epic, work_kind, &exception_reason),
        AgentCommand::Status => status(&repo),
        AgentCommand::Handoff {
            mode,
            section,
            value,
        } => handoff(&repo, mode, section, value),
        AgentCommand::RecordVerification {
            command,
            result,
            summary,
        } => record_verification(&repo, &command, result, &summary),
        AgentCommand::Check => check(&repo),
        AgentCommand::Advance { phase } => advance(&repo, phase),
        AgentCommand::Finish => finish(&repo),
        AgentCommand::SetRoadmapStatus { status } => set_roadmap_status(&repo, &status),
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
            return Err(
                "AGENT_UNSUPPORTED_STATE_SCHEMA: unsupported agent state schema".to_string(),
            );
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

const TEMPLATE_DIR: &str = "docs/superpowers/agent/templates";

fn create_template(repo: &Path, kind: TemplateKind) -> Result<(), String> {
    let state = read_state(repo)?;
    let (template_name, output_path) = match kind {
        TemplateKind::Spec => {
            let path = if state.spec.is_empty() {
                let items = read_roadmap(repo)?;
                let today = today_utc();
                default_spec_path(repo, &items, &state.current_epic, &today)
            } else {
                state.spec.clone()
            };
            ("spec.md", path)
        }
        TemplateKind::Plan => {
            let spec_stem = state
                .spec
                .strip_suffix("-design.md")
                .unwrap_or(&state.spec)
                .strip_prefix("docs/superpowers/specs/")
                .unwrap_or("");
            if spec_stem.is_empty() {
                return Err("cannot derive plan path: no spec linked".to_string());
            }
            let path = format!("docs/superpowers/plans/{}.md", spec_stem);
            ("plan.md", path)
        }
    };

    let output = repo.join(&output_path);
    if output.exists() {
        return Err(format!("refusing to overwrite {}", output_path));
    }

    let template = fs::read_to_string(repo.join(TEMPLATE_DIR).join(template_name))
        .map_err(|err| format!("failed to read template {template_name}: {err}"))?;

    let rendered = template
        .replace("{{EPIC}}", &state.current_epic)
        .replace("{{PHASE}}", &format_phase(&state.phase))
        .replace("{{SPEC_PATH}}", &state.spec)
        .replace("{{EXCEPTION_REASON}}", &state.exception_reason);

    fs::write(&output, rendered).map_err(|err| format!("failed to write {output_path}: {err}"))?;

    output::progress(format!("created {output_path}"));
    Ok(())
}

fn slugify_title(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn format_phase(phase: &Phase) -> String {
    match phase {
        Phase::Uninitialized => "uninitialized",
        Phase::Initialized => "initialized",
        Phase::Selected => "selected",
        Phase::Specified => "specified",
        Phase::Planned => "planned",
        Phase::InProgress => "in_progress",
        Phase::Verifying => "verifying",
        Phase::Done => "done",
    }
    .to_string()
}

fn read_roadmap(repo: &Path) -> Result<Vec<RoadmapItem>, String> {
    let input = fs::read_to_string(repo.join("docs/roadmap.md"))
        .map_err(|err| format!("failed to read roadmap: {err}"))?;
    parse_roadmap(&input)
}

fn find_item<'a>(items: &'a [RoadmapItem], epic: &str) -> Result<&'a RoadmapItem, String> {
    items
        .iter()
        .find(|i| i.title == epic)
        .ok_or_else(|| format!("roadmap item not found: {epic}"))
}

fn default_spec_path(repo: &Path, items: &[RoadmapItem], epic: &str, today: &str) -> String {
    let slug = slugify_title(epic);
    let base = format!("docs/superpowers/specs/{today}-{slug}");

    // First try without suffix
    let candidate = format!("{base}-design.md");
    if !repo.join(&candidate).exists() {
        return candidate;
    }
    // If it exists and is linked to the same epic, reuse it
    if let Some(item) = items.iter().find(|i| i.title == epic)
        && item.spec.as_deref() == Some(&candidate)
    {
        return candidate;
    }
    // Otherwise, try numeric suffixes
    for n in 2..100 {
        let candidate = format!("{base}-{n}-design.md");
        if !repo.join(&candidate).exists() {
            return candidate;
        }
        if let Some(item) = items.iter().find(|i| i.title == epic)
            && item.spec.as_deref() == Some(&candidate)
        {
            return candidate;
        }
    }
    // Fallback (shouldn't happen)
    format!("{base}-design.md")
}

fn default_plan_path(spec_path: &str) -> String {
    spec_path
        .replace("specs/", "plans/")
        .replace("-design.md", ".md")
}

fn is_exception_work_kind(wk: &WorkKind) -> bool {
    matches!(
        wk,
        WorkKind::SmallDirectEdit | WorkKind::EmergencyFix | WorkKind::HarnessDocs
    )
}

fn next(repo: &Path) -> Result<(), String> {
    let items = read_roadmap(repo)?;
    let mut candidates: Vec<&RoadmapItem> = items
        .iter()
        .filter(|i| i.status != RoadmapStatus::Deferred && i.status != RoadmapStatus::Done)
        .collect();

    // Filter out items with blocked dependencies
    candidates.retain(|item| {
        item.dependencies.iter().all(|dep_title| {
            items
                .iter()
                .any(|i| i.title == *dep_title && i.status == RoadmapStatus::Done)
        })
    });

    if candidates.is_empty() {
        return Err("no eligible roadmap epic".to_string());
    }

    // Sort by priority, then by original order in the roadmap
    candidates.sort_by_key(|i| i.priority);

    let chosen = candidates.first().unwrap();
    let mut stdout = io::stdout();
    writeln!(stdout, "{}", chosen.title).map_err(|e| e.to_string())?;
    Ok(())
}

fn start(
    repo: &Path,
    epic: &str,
    work_kind: WorkKind,
    exception_reason: &str,
) -> Result<(), String> {
    let mut state = read_state(repo)?;
    if state.locked {
        return Err("another epic is already locked".to_string());
    }
    if is_exception_work_kind(&work_kind) && exception_reason.is_empty() {
        return Err("exception reason is required".to_string());
    }

    let items = read_roadmap(repo)?;
    let item = find_item(&items, epic)?;

    let mut phase = Phase::Selected;
    let spec_path = item.spec.clone().unwrap_or_else(|| {
        // Derive candidate spec path from today's date
        let today = today_utc();
        default_spec_path(repo, &items, epic, &today)
    });
    let plan_path = item
        .plan
        .clone()
        .unwrap_or_else(|| default_plan_path(&spec_path));

    // For roadmap work, collapse phase if artifacts exist
    if work_kind == WorkKind::Roadmap {
        let spec_exists = repo.join(&spec_path).exists();
        let plan_exists = repo.join(&plan_path).exists();
        if plan_exists && spec_exists {
            phase = Phase::Planned;
        } else if spec_exists {
            phase = Phase::Specified;
        }
    }

    state.current_epic = epic.to_string();
    state.phase = phase;
    state.locked = true;
    state.work_kind = work_kind;
    state.exception_reason = exception_reason.to_string();
    state.spec = spec_path;
    state.plan = plan_path;
    state.current_handoff = "docs/superpowers/agent/current-handoff.md".to_string();

    write_state(repo, &state)?;
    output::progress(format!("locked epic: {epic}"));
    Ok(())
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    // Convert days since 0000-03-01 to a civil date
    // Algorithm from Howard Hinnant
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

fn status(repo: &Path) -> Result<(), String> {
    let state = read_state(repo)?;
    let spec_display = format_path_status(repo, &state.spec);
    let plan_display = format_path_status(repo, &state.plan);
    let last_handoff = if state.last_handoff.is_empty() {
        String::new()
    } else {
        state.last_handoff.clone()
    };
    let last_verification = state
        .verification
        .last()
        .map(|v| format!("{} ({})", v.command, v.result_str()))
        .unwrap_or_default();

    let exception_reason = if state.exception_reason.is_empty() {
        String::new()
    } else {
        state.exception_reason.clone()
    };

    let mut stdout = io::stdout();
    writeln!(stdout, "current epic: {}", state.current_epic).map_err(|e| e.to_string())?;
    writeln!(stdout, "phase: {}", state.phase.to_status_str()).map_err(|e| e.to_string())?;
    writeln!(stdout, "locked: {}", state.locked).map_err(|e| e.to_string())?;
    writeln!(stdout, "work kind: {}", state.work_kind.to_status_str())
        .map_err(|e| e.to_string())?;
    writeln!(stdout, "exception reason: {exception_reason}").map_err(|e| e.to_string())?;
    writeln!(stdout, "spec: {spec_display}").map_err(|e| e.to_string())?;
    writeln!(stdout, "plan: {plan_display}").map_err(|e| e.to_string())?;
    writeln!(stdout, "last handoff: {last_handoff}").map_err(|e| e.to_string())?;
    writeln!(stdout, "last verification: {last_verification}").map_err(|e| e.to_string())?;
    Ok(())
}

fn format_path_status(repo: &Path, path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }
    let exists = repo.join(path).exists();
    if exists {
        format!("{path} (exists)")
    } else {
        format!("{path} (missing)")
    }
}

impl VerificationEntry {
    fn result_str(&self) -> &str {
        match self.result {
            VerificationResult::Passed => "passed",
            VerificationResult::Failed => "failed",
        }
    }
}

#[allow(clippy::wrong_self_convention)]
impl WorkKind {
    fn to_status_str(&self) -> &str {
        match self {
            WorkKind::Roadmap => "roadmap",
            WorkKind::SmallDirectEdit => "small_direct_edit",
            WorkKind::EmergencyFix => "emergency_fix",
            WorkKind::HarnessDocs => "harness_docs",
        }
    }
}

#[allow(clippy::wrong_self_convention)]
impl Phase {
    fn to_status_str(&self) -> &str {
        match self {
            Phase::Uninitialized => "uninitialized",
            Phase::Initialized => "initialized",
            Phase::Selected => "selected",
            Phase::Specified => "specified",
            Phase::Planned => "planned",
            Phase::InProgress => "in_progress",
            Phase::Verifying => "verifying",
            Phase::Done => "done",
        }
    }
}

const HANDOFF_PATH: &str = "docs/superpowers/agent/current-handoff.md";
const HANDOFF_DIR: &str = "docs/superpowers/agent/handoffs";

const REQUIRED_HANDOFF_SECTIONS: &[&str] = &[
    "Current Epic",
    "Phase",
    "Exception Reason",
    "Completed",
    "Verification",
    "Modified Files",
    "Unresolved Risks",
    "Next Step",
];

const PLACEHOLDER_CONTENT: &[&str] = &[
    "Not recorded yet.",
    "Not run yet.",
    "None.",
    "Record the next concrete action.",
];

fn handoff(
    repo: &Path,
    mode: HandoffMode,
    section: Option<String>,
    value: Option<String>,
) -> Result<(), String> {
    let state = read_state(repo)?;
    match mode {
        HandoffMode::Create => create_handoff(repo, &state),
        HandoffMode::Validate => validate_handoff(repo, &state),
        HandoffMode::Set => {
            let section = section.ok_or("SECTION is required for handoff set")?;
            let value = value.ok_or("VALUE is required for handoff set")?;
            set_section_handoff(repo, &section, &value)
        }
    }
}

fn create_handoff(repo: &Path, state: &AgentState) -> Result<(), String> {
    let handoff_path = repo.join(HANDOFF_PATH);
    if handoff_path.exists() {
        let existing = fs::read_to_string(&handoff_path)
            .map_err(|e| format!("failed to read handoff: {e}"))?;
        let epic_body = section_body(&existing, "Current Epic").unwrap_or_default();
        let epic_body = epic_body.trim();
        if epic_body != state.current_epic {
            return Err("stale handoff: current Epic does not match active epic".to_string());
        }
    }
    let template = fs::read_to_string(repo.join("docs/superpowers/agent/templates/handoff.md"))
        .map_err(|e| format!("failed to read handoff template: {e}"))?;
    let rendered = render_handoff_template(&template, state);
    fs::write(&handoff_path, rendered).map_err(|e| format!("failed to write handoff: {e}"))?;
    output::progress("created current-handoff.md");
    Ok(())
}

fn validate_handoff(repo: &Path, state: &AgentState) -> Result<(), String> {
    let handoff_path = repo.join(HANDOFF_PATH);
    let content =
        fs::read_to_string(&handoff_path).map_err(|e| format!("failed to read handoff: {e}"))?;

    // Check epic match
    let epic_body = section_body(&content, "Current Epic").unwrap_or_default();
    if epic_body.trim() != state.current_epic {
        return Err("AGENT_HANDOFF_MISMATCH: handoff epic does not match active epic".to_string());
    }

    // Check phase match
    let phase_body = section_body(&content, "Phase").unwrap_or_default();
    let expected_phase = format_phase(&state.phase);
    if phase_body.trim() != expected_phase {
        return Err(format!(
            "AGENT_HANDOFF_MISMATCH: handoff phase {phase_body:?} does not match state phase {expected_phase:?}",
        ));
    }

    // Check all required sections have non-placeholder content
    for &section in REQUIRED_HANDOFF_SECTIONS {
        let body = section_body(&content, section).unwrap_or_default();
        let trimmed = body.trim();
        if trimmed.is_empty() || is_placeholder(trimmed) {
            return Err("AGENT_HANDOFF_INCOMPLETE: handoff section remains empty".to_string());
        }
    }

    Ok(())
}

fn is_placeholder(content: &str) -> bool {
    let trimmed = content.trim();
    PLACEHOLDER_CONTENT.contains(&trimmed)
}

fn render_handoff_template(template: &str, state: &AgentState) -> String {
    template
        .replace("{{EPIC}}", &state.current_epic)
        .replace("{{PHASE}}", &format_phase(&state.phase))
        .replace(
            "{{EXCEPTION_REASON}}",
            if state.exception_reason.is_empty() {
                "- None."
            } else {
                &state.exception_reason
            },
        )
}

fn section_body(markdown: &str, heading: &str) -> Option<String> {
    let marker = format!("## {heading}");
    let start = markdown.find(&marker)?;
    let after_heading = &markdown[start + marker.len()..];
    // Skip the newline after the heading
    let after_newline = after_heading.strip_prefix("\n").unwrap_or(after_heading);
    // Find the next heading or end
    let end = after_newline
        .find(
            "
## ",
        )
        .unwrap_or(after_newline.len());
    Some(after_newline[..end].trim().to_string())
}

fn set_section_handoff(repo: &Path, section: &str, value: &str) -> Result<(), String> {
    let handoff_path = repo.join(HANDOFF_PATH);
    let content =
        fs::read_to_string(&handoff_path).map_err(|e| format!("failed to read handoff: {e}"))?;
    let new_content = set_section_body(&content, section, value)?;
    fs::write(&handoff_path, new_content).map_err(|e| format!("failed to write handoff: {e}"))?;
    output::progress(format!("updated handoff section {section:?}"));
    Ok(())
}

fn set_section_body(markdown: &str, heading: &str, value: &str) -> Result<String, String> {
    let marker = format!("## {heading}");
    let start = markdown
        .find(&marker)
        .ok_or_else(|| format!("unknown handoff section: {heading}"))?;
    let after_heading = &markdown[start + marker.len()..];

    // Count leading newlines (heading line break + optional blank line)
    let leading = after_heading.chars().take_while(|&c| c == '\n').count();
    let body_start = start + marker.len() + leading;
    let rest = &markdown[body_start..];
    let body_end = rest.find("\n## ").unwrap_or(rest.len());

    let before = &markdown[..body_start];
    let after = &markdown[body_start + body_end..];

    Ok(format!("{before}{value}{after}"))
}

fn record_verification(
    repo: &Path,
    command: &str,
    result: VerificationResult,
    summary: &str,
) -> Result<(), String> {
    let normalized = normalize_command(command.trim());
    if normalized.is_empty() {
        return Err("verification COMMAND must not be empty".to_string());
    }
    if summary.trim().is_empty() {
        return Err("verification SUMMARY must not be empty".to_string());
    }

    let handoff_path = repo.join(HANDOFF_PATH);
    if !handoff_path.exists() {
        return Err(
            "current-handoff.md is missing; run agent handoff --mode create first".to_string(),
        );
    }

    let recorded_at = today_utc();
    let entry = VerificationEntry {
        command: normalized.clone(),
        result,
        summary: summary.trim().to_string(),
        recorded_at,
    };

    let mut state = read_state(repo)?;
    state.verification.push(entry.clone());
    write_state(repo, &state)?;

    append_verification_to_handoff(repo, &entry)?;

    output::progress("verification recorded");
    Ok(())
}

fn today_utc() -> String {
    use std::time::SystemTime;
    let dur = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let days = secs / 86400;
    let (y, m, d) = civil_from_days(days as i64);
    format!("{y:04}-{m:02}-{d:02}")
}

fn append_verification_to_handoff(repo: &Path, entry: &VerificationEntry) -> Result<(), String> {
    let handoff_path = repo.join(HANDOFF_PATH);
    let content =
        fs::read_to_string(&handoff_path).map_err(|e| format!("failed to read handoff: {e}"))?;
    let result_str = match entry.result {
        VerificationResult::Passed => "passed",
        VerificationResult::Failed => "failed",
    };
    let line = format!("- `{}` {}: {}\n", entry.command, result_str, entry.summary);
    let new_content = append_to_section(&content, "Verification", &line)?;
    fs::write(&handoff_path, new_content).map_err(|e| format!("failed to write handoff: {e}"))?;
    Ok(())
}

fn append_to_section(markdown: &str, heading: &str, line: &str) -> Result<String, String> {
    let marker = format!("## {heading}");
    let start = markdown
        .find(&marker)
        .ok_or_else(|| format!("handoff section not found: {heading}"))?;
    let after_heading = &markdown[start + marker.len()..];
    let after_newline = after_heading.strip_prefix('\n').unwrap_or(after_heading);

    // Find end of section
    let section_end = after_newline.find("\n## ").unwrap_or(after_newline.len());
    let section_body = &after_newline[..section_end];

    // Check if there's already content (not a placeholder)
    let trimmed = section_body.trim();
    if !trimmed.is_empty() && !is_placeholder(trimmed) {
        // Append to existing content
        let before_section = &markdown[..start + marker.len() + 1];
        let after_section = &markdown[start + marker.len() + 1 + section_end..];
        return Ok(format!(
            "{before_section}{section_body}\n{line}{after_section}"
        ));
    }

    // Replace placeholder content
    let before_section = &markdown[..start + marker.len() + 1];
    let after_section = &markdown[start + marker.len() + 1 + section_end..];
    Ok(format!("{before_section}{line}{after_section}"))
}

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
) -> CheckReport {
    let mut report = CheckReport::default();
    let Some(item) = items.iter().find(|i| i.title == state.current_epic) else {
        report
            .errors
            .push("AGENT_ROADMAP_SPEC_UNLINKED: active epic not found in roadmap".to_string());
        return report;
    };

    // Phase vs roadmap status mismatch
    let runtime_rank = phase_rank(state.phase);
    let roadmap_rank = roadmap_status_rank(item.status);

    match (state.phase, item.status) {
        // Fail: runtime ahead of roadmap or runtime done but roadmap deferred
        (
            Phase::InProgress,
            RoadmapStatus::Proposed | RoadmapStatus::Specified | RoadmapStatus::Planned,
        )
        | (
            Phase::Verifying,
            RoadmapStatus::Proposed | RoadmapStatus::Specified | RoadmapStatus::Planned,
        )
        | (Phase::Specified, RoadmapStatus::Deferred)
        | (Phase::Done, RoadmapStatus::Deferred) => {
            report.errors.push(format!(
                "AGENT_PHASE_AHEAD: runtime phase {:?} is ahead of roadmap status {:?}",
                state.phase, item.status
            ));
        }
        // Warn: roadmap ahead of runtime
        _ if roadmap_rank > runtime_rank && state.phase != Phase::Done => {
            report.warnings.push(format!(
                "roadmap ahead: runtime phase {:?}, roadmap status {:?}",
                state.phase, item.status
            ));
        }
        _ => {}
    }

    // Artifact checks
    let needs_spec = matches!(
        item.status,
        RoadmapStatus::Specified
            | RoadmapStatus::Planned
            | RoadmapStatus::InProgress
            | RoadmapStatus::Done
    );
    let needs_plan = matches!(
        item.status,
        RoadmapStatus::Planned | RoadmapStatus::InProgress | RoadmapStatus::Done
    );

    if needs_spec {
        if item.spec.is_none() {
            report
                .errors
                .push("AGENT_ROADMAP_SPEC_UNLINKED: roadmap item has no spec link".to_string());
        }
        if state.spec.is_empty() {
            report
                .errors
                .push("AGENT_MISSING_SPEC: state has no spec path recorded".to_string());
        } else if !artifacts.spec_exists {
            report
                .errors
                .push("AGENT_MISSING_SPEC: spec file does not exist".to_string());
        }
    }
    if needs_plan {
        if item.plan.is_none() {
            report
                .errors
                .push("AGENT_ROADMAP_PLAN_UNLINKED: roadmap item has no plan link".to_string());
        }
        if !state.plan.is_empty() && !artifacts.plan_exists {
            report
                .errors
                .push("AGENT_MISSING_PLAN: plan file does not exist".to_string());
        }
    }

    // Handoff check — current-handoff.md is required for active phases.
    // After finish the file is moved; the durable record is last_handoff.
    if matches!(state.phase, Phase::InProgress | Phase::Verifying) && !artifacts.handoff_exists {
        report
            .errors
            .push("AGENT_HANDOFF_MISSING: handoff file is missing".to_string());
    }
    if state.phase == Phase::Done && !artifacts.handoff_exists && state.last_handoff.is_empty() {
        report
            .errors
            .push("AGENT_HANDOFF_MISSING: no handoff record after finish".to_string());
    }

    // P0 prerequisite: non-harness implementation work requires harness done
    if state.work_kind == WorkKind::Roadmap
        && matches!(state.phase, Phase::InProgress | Phase::Verifying)
        && state.current_epic != "P0 - Roadmap Agent Harness"
    {
        let harness_done = items
            .iter()
            .any(|i| i.title == "P0 - Roadmap Agent Harness" && i.status == RoadmapStatus::Done);
        if !harness_done {
            report.errors.push(
                "AGENT_P0_PREREQUISITE: P0 - Roadmap Agent Harness blocks implementation work"
                    .to_string(),
            );
        }
    }

    report
}

fn compare_expected_verification(
    expected: &[String],
    recorded: &[VerificationEntry],
) -> CheckReport {
    let mut report = CheckReport::default();
    for cmd in expected {
        let found = recorded
            .iter()
            .any(|v| v.command == *cmd && matches!(v.result, VerificationResult::Passed));
        if !found {
            report.warnings.push(format!(
                "expected verification command not recorded as passed: {cmd}"
            ));
        }
    }
    report
}

fn validate_spec_structure(markdown: &str) -> CheckReport {
    let mut report = CheckReport::default();
    let required = [
        "## Goal",
        "## Scope",
        "## Non-Goals",
        "## Design",
        "## Error Handling",
        "## Verification Strategy",
        "## Regression Coverage Expectations",
    ];
    let lower = markdown.to_lowercase();
    for heading in &required {
        let lower_heading = heading.to_lowercase();
        if !lower.contains(&lower_heading) {
            report.errors.push(format!(
                "AGENT_MISSING_SPEC_SECTION: missing required section {heading}"
            ));
        }
    }
    report
}

fn validate_plan_structure(markdown: &str) -> CheckReport {
    let mut report = CheckReport::default();

    // Check required headings
    let has_heading = |h: &str| {
        markdown
            .to_lowercase()
            .contains(&format!("## {}", h.to_lowercase()))
    };
    if !has_heading("existing code map") {
        report
            .errors
            .push("AGENT_MISSING_PLAN_SECTION: missing ## Existing Code Map".to_string());
    }
    if !has_heading("verification commands") {
        report
            .errors
            .push("AGENT_MISSING_PLAN_SECTION: missing ## Verification Commands".to_string());
    }
    if !has_heading("expected outcomes") {
        report
            .errors
            .push("AGENT_MISSING_PLAN_SECTION: missing ## Expected Outcomes".to_string());
    }
    if !has_heading("test level") {
        report
            .errors
            .push("AGENT_MISSING_PLAN_SECTION: missing ## Test Level".to_string());
    }
    if !has_heading("regression coverage expectations") {
        report.errors.push(
            "AGENT_MISSING_PLAN_SECTION: missing ## Regression Coverage Expectations".to_string(),
        );
    }

    // Check for at least one task heading
    let has_task = markdown.lines().any(|l| {
        let trimmed = l.trim();
        trimmed.starts_with("## Task") || trimmed.starts_with("### Task")
    });
    if !has_task {
        report
            .errors
            .push("AGENT_MISSING_PLAN_SECTION: no task heading found".to_string());
    }

    // Check for at least one checkbox
    let has_checkbox = markdown
        .lines()
        .any(|l| l.trim().starts_with("- [ ]") || l.trim().starts_with("- [x]"));
    if !has_checkbox {
        report
            .errors
            .push("AGENT_MISSING_PLAN_SECTION: no checkbox items found".to_string());
    }

    // Check for **Files:** block
    let has_files_block = markdown.to_lowercase().contains("**files:**");
    if !has_files_block {
        report
            .errors
            .push("AGENT_MISSING_PLAN_SECTION: no **Files:** block found".to_string());
    }

    // Check for backticked path in Files block
    let has_backticked_path = markdown
        .lines()
        .any(|l| l.contains('`') && l.contains('/') && l.contains('.'));
    if !has_backticked_path {
        report
            .errors
            .push("AGENT_MISSING_PLAN_SECTION: no backticked file path found".to_string());
    }

    // Check for backticked command under Verification Commands
    let in_vc = markdown.find("## Verification Commands").is_some();
    if in_vc {
        let has_vc_cmd = markdown
            .lines()
            .any(|l| l.trim().starts_with("- `") && l.contains("cargo") || l.contains("make "));
        if !has_vc_cmd {
            report
                .errors
                .push("AGENT_MISSING_PLAN_SECTION: no backticked verification command".to_string());
        }
    }

    report
}

fn validate_handoff_structure(markdown: &str) -> CheckReport {
    let mut report = CheckReport::default();
    for &section in REQUIRED_HANDOFF_SECTIONS {
        let body = section_body(markdown, section).unwrap_or_default();
        let trimmed = body.trim();
        if trimmed.is_empty() || is_placeholder(trimmed) {
            report
                .errors
                .push("AGENT_HANDOFF_INCOMPLETE: handoff section remains empty".to_string());
            break;
        }
    }
    report
}

fn expected_verification_commands(plan: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let mut in_vc = false;
    for line in plan.lines() {
        let trimmed = line.trim();
        if trimmed == "## Verification Commands" {
            in_vc = true;
            continue;
        }
        if in_vc && trimmed.starts_with("## ") {
            break;
        }
        if in_vc
            && trimmed.starts_with("- `")
            && let Some(cmd) = backticked_path(trimmed)
        {
            commands.push(normalize_command(&cmd));
        }
    }
    commands
}

fn phase_rank(phase: Phase) -> u8 {
    match phase {
        Phase::Uninitialized => 0,
        Phase::Initialized => 1,
        Phase::Selected => 2,
        Phase::Specified => 3,
        Phase::Planned => 4,
        Phase::InProgress => 5,
        Phase::Verifying => 6,
        Phase::Done => 7,
    }
}

fn roadmap_status_rank(status: RoadmapStatus) -> u8 {
    match status {
        RoadmapStatus::Proposed => 0,
        RoadmapStatus::Deferred => 0,
        RoadmapStatus::Specified => 2,
        RoadmapStatus::Planned => 3,
        RoadmapStatus::InProgress => 4,
        RoadmapStatus::Done => 5,
    }
}

fn next_phase(current: Phase, target: Phase, work_kind: &WorkKind) -> Result<(), String> {
    let cur_rank = phase_rank(current);
    let target_rank = phase_rank(target);

    if target_rank <= cur_rank {
        return Err("cannot move backward in phase".to_string());
    }

    // Exception work kinds can skip from selected to in_progress
    if is_exception_work_kind(work_kind) {
        if current == Phase::Selected && target == Phase::InProgress {
            return Ok(());
        }
        // Otherwise, normal progression
        if target_rank != cur_rank + 1 {
            return Err("cannot skip phase".to_string());
        }
        return Ok(());
    }

    // Normal progression: one step at a time
    if target_rank != cur_rank + 1 {
        return Err("cannot skip phase".to_string());
    }

    Ok(())
}

fn artifact_status_from_repo(repo: &Path, state: &AgentState) -> ArtifactStatus {
    ArtifactStatus {
        spec_exists: !state.spec.is_empty() && repo.join(&state.spec).exists(),
        plan_exists: !state.plan.is_empty() && repo.join(&state.plan).exists(),
        handoff_exists: repo.join(HANDOFF_PATH).exists(),
    }
}

fn check(repo: &Path) -> Result<(), String> {
    let state = read_state(repo)?;
    let items = read_roadmap(repo)?;
    let artifacts = artifact_status_from_repo(repo, &state);
    let mut report = check_state_consistency(&state, &items, &artifacts);

    // Exception work kinds must have a non-empty exception_reason
    if is_exception_work_kind(&state.work_kind) && state.exception_reason.is_empty() {
        report.errors.push(
            "AGENT_P0_PREREQUISITE: exception work kind requires a non-empty exception reason"
                .to_string(),
        );
    }

    // Validate spec structure if spec exists
    if artifacts.spec_exists
        && let Ok(spec_content) = fs::read_to_string(repo.join(&state.spec))
    {
        let spec_report = validate_spec_structure(&spec_content);
        report.errors.extend(spec_report.errors);
        report.warnings.extend(spec_report.warnings);
    }

    // Validate plan structure and verification commands if plan exists
    if artifacts.plan_exists
        && let Ok(plan_content) = fs::read_to_string(repo.join(&state.plan))
    {
        let plan_report = validate_plan_structure(&plan_content);
        report.errors.extend(plan_report.errors);
        report.warnings.extend(plan_report.warnings);

        // Compare expected vs recorded verification
        let expected = expected_verification_commands(&plan_content);
        if !expected.is_empty() && !state.verification.is_empty() {
            let v_report = compare_expected_verification(&expected, &state.verification);
            report.warnings.extend(v_report.warnings);
        }
    }

    // Validate handoff if it exists
    if artifacts.handoff_exists
        && let Ok(handoff_content) = fs::read_to_string(repo.join(HANDOFF_PATH))
    {
        // Check epic/phase mismatch
        let epic_body = section_body(&handoff_content, "Current Epic").unwrap_or_default();
        if epic_body.trim() != state.current_epic {
            report.errors.push(
                "AGENT_HANDOFF_MISMATCH: handoff epic does not match active epic".to_string(),
            );
        }
        let phase_body = section_body(&handoff_content, "Phase").unwrap_or_default();
        if phase_body.trim() != format_phase(&state.phase) {
            report.errors.push(
                "AGENT_HANDOFF_MISMATCH: handoff phase does not match state phase".to_string(),
            );
        }

        let h_report = validate_handoff_structure(&handoff_content);
        report.errors.extend(h_report.errors);
        report.warnings.extend(h_report.warnings);
    }

    // Print warnings
    for warn in &report.warnings {
        output::warn(warn);
    }

    if report.errors.is_empty() {
        output::progress("agent check passed");
        Ok(())
    } else {
        for err in &report.errors {
            eprintln!("error: {err}");
        }
        Err("agent check failed".to_string())
    }
}

fn advance(repo: &Path, target: Phase) -> Result<(), String> {
    let mut state = read_state(repo)?;
    let items = read_roadmap(repo)?;

    // Validate required artifacts exist before checking phase transition.
    // Placed first so artifact requirements are explicit regardless of work kind.
    if state.work_kind == WorkKind::Roadmap {
        match target {
            Phase::Specified => {
                let item = find_item(&items, &state.current_epic)?;
                if item.spec.is_none() {
                    return Err("AGENT_MISSING_SPEC: roadmap item has no spec link".to_string());
                }
                if !repo.join(state.spec.as_str()).exists() {
                    return Err("AGENT_MISSING_SPEC: spec file does not exist".to_string());
                }
            }
            Phase::Planned | Phase::InProgress | Phase::Verifying => {
                let item = find_item(&items, &state.current_epic)?;
                if item.plan.is_none() {
                    return Err("AGENT_MISSING_PLAN: roadmap item has no plan link".to_string());
                }
                if !repo.join(state.plan.as_str()).exists() {
                    return Err("AGENT_MISSING_PLAN: plan file does not exist".to_string());
                }
            }
            _ => {}
        }
    }

    next_phase(state.phase, target, &state.work_kind)?;

    // P0 prerequisite: non-harness advance to implementation phases requires harness done
    if state.work_kind == WorkKind::Roadmap
        && matches!(target, Phase::InProgress | Phase::Verifying)
        && state.current_epic != "P0 - Roadmap Agent Harness"
    {
        let harness_done = items
            .iter()
            .any(|i| i.title == "P0 - Roadmap Agent Harness" && i.status == RoadmapStatus::Done);
        if !harness_done {
            return Err(
                "AGENT_P0_PREREQUISITE: P0 - Roadmap Agent Harness blocks implementation work"
                    .to_string(),
            );
        }
    }

    state.phase = target;
    write_state(repo, &state)?;
    output::progress(format!("advanced to {:?}", target));
    Ok(())
}

fn parse_roadmap(input: &str) -> Result<Vec<RoadmapItem>, String> {
    let mut items = Vec::new();
    let mut current: Option<RoadmapItem> = None;
    let mut in_queue = false;

    let mut lines_iter = input.lines().peekable();
    while let Some(line) = lines_iter.next() {
        let trimmed = line.trim();
        if trimmed == "## Active Queue" || trimmed == "## Next Queue" || trimmed == "## Completed Foundation" {
            in_queue = true;
            continue;
        }
        if !in_queue {
            continue;
        }
        if trimmed.starts_with("## ") && trimmed != "## Active Queue" && trimmed != "## Next Queue" && trimmed != "## Completed Foundation" {
            // Flush current item and resume scanning for next queue section
            if let Some(item) = current.take() {
                items.push(item);
            }
            in_queue = false;
            continue;
        }
        if let Some(stripped) = trimmed.strip_prefix("### ") {
            if let Some(item) = current.take() {
                items.push(item);
            }
            let title = stripped.to_string();
            let priority = parse_priority(&title)?;
            current = Some(RoadmapItem {
                title,
                priority,
                status: RoadmapStatus::Proposed,
                spec: None,
                plan: None,
                dependencies: Vec::new(),
            });
            continue;
        }
        if let Some(ref mut item) = current {
            if let Some(value) = trimmed.strip_prefix("Status: ") {
                item.status = parse_status(item.title.as_str(), value.trim())?;
            } else if let Some(rest) = trimmed.strip_prefix("Depends on: ") {
                for dep in rest.split(',') {
                    let dep = dep.trim().to_string();
                    if !dep.is_empty() {
                        item.dependencies.push(dep);
                    }
                }
            } else if trimmed == "Spec:"
                && let Some(next) = lines_iter.peek()
                && let Some(path) = backticked_path(next.trim())
            {
                item.spec = Some(path);
                lines_iter.next();
            } else if trimmed == "Plan:"
                && let Some(next) = lines_iter.peek()
                && let Some(path) = backticked_path(next.trim())
            {
                item.plan = Some(path);
                lines_iter.next();
            }
        }
    }
    if let Some(item) = current {
        items.push(item);
    }
    Ok(items)
}

fn parse_priority(title: &str) -> Result<Priority, String> {
    if title.starts_with("P0") || title.starts_with("p0") {
        Ok(Priority::P0)
    } else if title.starts_with("P1") || title.starts_with("p1") {
        Ok(Priority::P1)
    } else if title.starts_with("P2") || title.starts_with("p2") {
        Ok(Priority::P2)
    } else if title.starts_with("P3") || title.starts_with("p3") {
        Ok(Priority::P3)
    } else {
        Err(format!("cannot determine priority from title: {title}"))
    }
}

fn parse_status(title: &str, value: &str) -> Result<RoadmapStatus, String> {
    match value {
        "proposed" => Ok(RoadmapStatus::Proposed),
        "specified" => Ok(RoadmapStatus::Specified),
        "planned" => Ok(RoadmapStatus::Planned),
        "in_progress" => Ok(RoadmapStatus::InProgress),
        "done" => Ok(RoadmapStatus::Done),
        "deferred" => Ok(RoadmapStatus::Deferred),
        _ => Err(format!("invalid roadmap status for {title}: {value}")),
    }
}

fn backticked_path(line: &str) -> Option<String> {
    let start = line.find('`')?;
    let end = line[start + 1..].find('`')?;
    Some(line[start + 1..start + 1 + end].to_string())
}

fn normalize_command(command: &str) -> String {
    command.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn finish(repo: &Path) -> Result<(), String> {
    let mut state = read_state(repo)?;

    if state.phase != Phase::Verifying {
        return Err("AGENT_FINISH_WRONG_PHASE: phase must be verifying to finish".to_string());
    }

    let has_passed = state
        .verification
        .iter()
        .any(|v| matches!(v.result, VerificationResult::Passed));
    if !has_passed {
        return Err(
            "AGENT_FINISH_NO_VERIFICATION: at least one passing verification is required"
                .to_string(),
        );
    }

    // Validate handoff
    let handoff_path = repo.join(HANDOFF_PATH);
    if !handoff_path.exists() {
        return Err("AGENT_HANDOFF_MISSING: handoff file is missing".to_string());
    }
    let handoff_content =
        fs::read_to_string(&handoff_path).map_err(|e| format!("failed to read handoff: {e}"))?;

    // Check epic match
    let epic_body = section_body(&handoff_content, "Current Epic").unwrap_or_default();
    if epic_body.trim() != state.current_epic {
        return Err("AGENT_HANDOFF_MISMATCH: handoff epic does not match active epic".to_string());
    }

    // Check handoff completeness
    let h_report = validate_handoff_structure(&handoff_content);
    if !h_report.errors.is_empty() {
        return Err(h_report.errors.join("\n"));
    }

    // Move handoff to finished path
    let finished_path = finished_handoff_path(repo, &state);
    let handoffs_dir = repo.join(HANDOFF_DIR);
    fs::create_dir_all(&handoffs_dir).map_err(|e| format!("failed to create handoffs dir: {e}"))?;

    fs::rename(&handoff_path, repo.join(&finished_path))
        .map_err(|e| format!("failed to move handoff: {e}"))?;

    state.phase = Phase::Done;
    state.locked = false;
    state.last_handoff = finished_path;
    state.current_handoff.clear();
    write_state(repo, &state)?;

    output::progress("epic finished");
    Ok(())
}

fn finished_handoff_path(repo: &Path, state: &AgentState) -> String {
    let slug = slugify_title(&state.current_epic);
    let today = today_utc();
    let base = format!("docs/superpowers/agent/handoffs/{today}-{slug}");

    // First candidate
    let candidate = format!("{base}.md");
    if !repo.join(&candidate).exists() {
        return candidate;
    }

    // Try numeric suffixes
    for n in 2..100 {
        let candidate = format!("{base}-{n}.md");
        if !repo.join(&candidate).exists() {
            return candidate;
        }
    }

    // Fallback
    format!("{base}.md")
}

fn set_roadmap_status(repo: &Path, status: &str) -> Result<(), String> {
    let state = read_state(repo)?;
    let roadmap_path = repo.join("docs/roadmap.md");
    let input = fs::read_to_string(&roadmap_path)
        .map_err(|err| format!("failed to read roadmap: {err}"))?;

    let status = parse_status_for_update(status)?;
    let heading = format!("### {}", state.current_epic);
    let heading_start = input
        .find(&heading)
        .ok_or_else(|| format!("epic not found in roadmap: {}", state.current_epic))?;

    // Find the Status: line after this heading
    let after_heading = &input[heading_start..];
    let status_line_start = after_heading
        .find("Status: ")
        .ok_or_else(|| "Status: line not found after epic heading".to_string())?;

    let status_value_start = heading_start + status_line_start + "Status: ".len();
    let rest = &input[status_value_start..];
    let status_value_end = rest.find('\n').unwrap_or(rest.len());

    let mut output = String::with_capacity(input.len());
    output.push_str(&input[..status_value_start]);
    output.push_str(&status);
    output.push_str(&input[status_value_start + status_value_end..]);

    fs::write(&roadmap_path, output).map_err(|err| format!("failed to write roadmap: {err}"))?;
    output::progress(format!(
        "roadmap status updated: {} -> {status}",
        state.current_epic
    ));
    Ok(())
}

fn parse_status_for_update(value: &str) -> Result<String, String> {
    match value {
        "proposed" | "specified" | "planned" | "in_progress" | "done" | "deferred" => {
            Ok(value.to_string())
        }
        _ => Err(format!("invalid roadmap status: {value}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(
            report
                .errors
                .iter()
                .any(|err| err.contains("AGENT_PHASE_AHEAD"))
        );
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
        assert!(
            report
                .warnings
                .iter()
                .any(|warn| warn.contains("make check"))
        );
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
                spec: Some(
                    "docs/superpowers/specs/2026-05-14-roadmap-agent-harness-design.md".to_string(),
                ),
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
            err.contains("P0 - Roadmap Agent Harness") && err.contains("blocks implementation work")
        }));
    }

    #[test]
    fn structural_quality_checks_require_core_spec_and_plan_sections() {
        let spec_report = validate_spec_structure("# Spec\n\n## Goal\n\nBuild it.\n");
        assert!(
            spec_report
                .errors
                .iter()
                .any(|err| err.contains("AGENT_MISSING_SPEC_SECTION"))
        );

        let plan_report =
            validate_plan_structure("# Plan\n\n## Verification Commands\n\n- `cargo test`\n");
        assert!(
            plan_report
                .errors
                .iter()
                .any(|err| err.contains("AGENT_MISSING_PLAN_SECTION"))
        );
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

        assert!(
            plan_report
                .errors
                .iter()
                .any(|err| err.contains("AGENT_MISSING_PLAN_SECTION"))
        );
    }

    fn item_with(status: RoadmapStatus) -> RoadmapItem {
        RoadmapItem {
            title: "P0 - Test Epic".to_string(),
            priority: Priority::P0,
            status,
            spec: Some("docs/superpowers/specs/test-design.md".to_string()),
            plan: Some("docs/superpowers/plans/test.md".to_string()),
            dependencies: Vec::new(),
        }
    }

    fn state_with(phase: Phase, wk: WorkKind) -> AgentState {
        AgentState {
            schema_version: 1,
            current_epic: "P0 - Test Epic".to_string(),
            phase,
            locked: true,
            work_kind: wk,
            exception_reason: String::new(),
            spec: "docs/superpowers/specs/test-design.md".to_string(),
            plan: "docs/superpowers/plans/test.md".to_string(),
            current_handoff: "docs/superpowers/agent/current-handoff.md".to_string(),
            last_handoff: String::new(),
            verification: Vec::new(),
        }
    }

    #[test]
    fn severity_rules_match_spec_table() {
        let fail_cases = [
            (Phase::Specified, RoadmapStatus::Deferred),
            (Phase::InProgress, RoadmapStatus::Proposed),
            (Phase::InProgress, RoadmapStatus::Specified),
            (Phase::InProgress, RoadmapStatus::Planned),
            (Phase::Verifying, RoadmapStatus::Proposed),
            (Phase::Verifying, RoadmapStatus::Specified),
            (Phase::Verifying, RoadmapStatus::Planned),
            (Phase::Done, RoadmapStatus::Deferred),
        ];
        for (phase, rstatus) in &fail_cases {
            let report = check_state_consistency(
                &state_with(*phase, WorkKind::Roadmap),
                &[item_with(*rstatus)],
                &ArtifactStatus {
                    spec_exists: true,
                    plan_exists: true,
                    handoff_exists: true,
                },
            );
            assert!(
                report.errors.iter().any(|e| e.contains("AGENT_")),
                "expected failure for {:?} + {:?}",
                phase,
                rstatus
            );
        }
        // warn: roadmap ahead of runtime
        let warn_report = check_state_consistency(
            &state_with(Phase::Specified, WorkKind::Roadmap),
            &[item_with(RoadmapStatus::InProgress)],
            &ArtifactStatus {
                spec_exists: true,
                plan_exists: true,
                handoff_exists: true,
            },
        );
        assert!(
            warn_report
                .warnings
                .iter()
                .any(|w| w.contains("roadmap ahead"))
        );
        // pass: runtime done + roadmap done
        let pass_report = check_state_consistency(
            &state_with(Phase::Done, WorkKind::Roadmap),
            &[item_with(RoadmapStatus::Done)],
            &ArtifactStatus {
                spec_exists: true,
                plan_exists: true,
                handoff_exists: true,
            },
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
        assert!(
            report
                .errors
                .iter()
                .any(|err| err.contains("AGENT_HANDOFF_INCOMPLETE"))
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
    fn set_section_body_preserves_blank_line() {
        let input = "# Agent Handoff\n\n## Current Epic\n\nP0 - Test\n\n## Completed\n\n- Not recorded yet.\n\n## Next Step\n\nRecord the next concrete action.\n";
        let result =
            set_section_body(input, "Completed", "- Wrote implementation plan.").expect("set");
        eprintln!("=== RESULT ===\n{result}\n=== END ===");
        assert!(
            result.contains("## Completed\n\n- Wrote implementation plan."),
            "got: {result:?}"
        );
    }

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
        assert_eq!(
            normalize_command("  cargo   test   agent_check  "),
            "cargo test agent_check"
        );
    }

    #[test]
    fn finished_handoff_path_avoids_collisions() {
        let temp = tempfile::tempdir().expect("tempdir");
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

    #[test]
    fn parses_next_queue_section() {
        let roadmap = r#"
## Next Queue

### P0 - Roadmap Refresh And Agent Queue Reset

Status: proposed
Category: governance

### P2 - Manifest Schema Migration Tool

Status: proposed
Category: maintainability
"#;

        let items = parse_roadmap(roadmap).expect("items");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "P0 - Roadmap Refresh And Agent Queue Reset");
        assert_eq!(items[0].priority, Priority::P0);
        assert_eq!(items[0].status, RoadmapStatus::Proposed);
        assert_eq!(items[1].title, "P2 - Manifest Schema Migration Tool");
        assert_eq!(items[1].priority, Priority::P2);
    }
}
