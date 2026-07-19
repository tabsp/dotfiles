//! Deterministic, UI-only Ratatui frame export for the website demo.
//!
//! This module never executes deployment actions. It renders fixture states
//! through the same widgets used by the interactive TUI and serializes the
//! resulting terminal buffer for a browser renderer.

use super::*;
use crate::model::{OutputLine, RunAction};
use ratatui::backend::TestBackend;
use ratatui::buffer::Cell;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct DemoBundle {
    pub schema: u8,
    pub width: u16,
    pub height: u16,
    pub default_fg: String,
    pub default_bg: String,
    pub seed: DemoSeed,
    pub frames: Vec<DemoFrame>,
}

#[derive(Debug, Serialize)]
pub struct DemoSeed {
    pub items: Vec<DemoSeedItem>,
    pub review_entries: Vec<DemoSeedReviewEntry>,
}

#[derive(Debug, Serialize)]
pub struct DemoSeedItem {
    pub name: String,
    pub layer: String,
    pub action_count: usize,
    pub selected: bool,
}

#[derive(Debug, Serialize)]
pub struct DemoSeedReviewEntry {
    pub item_index: usize,
    pub order: usize,
    pub kind: String,
    pub kind_icon: String,
    pub severity: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Serialize)]
pub struct DemoFrame {
    pub id: String,
    pub screen: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    pub cells: Vec<DemoCell>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DemoCell {
    pub x: u16,
    pub y: u16,
    pub symbol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub bold: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub dim: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub italic: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub underlined: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub reversed: bool,
}

/// Render the canonical website demo frames from a real dotman config.
pub fn export(config_path: &Path, width: u16, height: u16) -> Result<DemoBundle> {
    if width < 72 || height < 22 {
        anyhow::bail!("web demo requires at least 72 columns and 22 rows");
    }

    let config = config::load(config_path)?;
    let mut plan = crate::plan::build(&config, PlanMode::Deploy)?;
    plan.sync_auto_steps();

    let mut frames = Vec::new();
    for selected in 0..4 {
        let mut app = base_app(&config, &plan);
        app.menu_state.select(Some(selected));
        app.runs = vec![finished_run(&plan)];
        frames.push(render_frame(
            format!("main-menu-{selected}"),
            "main-menu",
            &mut app,
            width,
            height,
        )?);
    }

    let mut plan_app = base_app(&config, &plan);
    plan_app.screen = Screen::PlanView;
    frames.push(render_frame(
        "plan".into(),
        "plan",
        &mut plan_app,
        width,
        height,
    )?);

    let review_entries = demo_review_entries(&plan);
    let mut review_app = base_app(&config, &plan);
    review_app.screen = Screen::ConfirmView;
    review_app.review_entries = review_entries.clone();
    frames.push(render_frame(
        "review".into(),
        "review",
        &mut review_app,
        width,
        height,
    )?);

    for spinner_frame in [0, 3, 6] {
        let mut running_app = running_app(&config, &plan, spinner_frame);
        frames.push(render_frame(
            format!("run-{spinner_frame}"),
            "run",
            &mut running_app,
            width,
            height,
        )?);
    }

    let mut result_app = base_app(&config, &plan);
    result_app.screen = Screen::RunView;
    result_app.progress = (selected_action_total(&plan), selected_action_total(&plan));
    result_app.run = Some(finished_run(&plan));
    result_app.current_log = demo_log(true);
    frames.push(render_frame(
        "result".into(),
        "result",
        &mut result_app,
        width,
        height,
    )?);

    let runs = demo_runs(&plan);
    for selected in 0..runs.len() {
        let mut history_app = base_app(&config, &plan);
        history_app.screen = Screen::HistoryView;
        history_app.runs = runs.clone();
        history_app.history_state.select(Some(selected));
        frames.push(render_frame(
            format!("history-{selected}"),
            "history",
            &mut history_app,
            width,
            height,
        )?);
    }

    for (run_index, run) in runs.iter().enumerate() {
        let action_count = run
            .items
            .iter()
            .map(|item| item.actions.len().max(1))
            .sum::<usize>();
        for selected in 0..action_count.min(24) {
            let mut replay_app = base_app(&config, &plan);
            replay_app.screen = Screen::RunReplay;
            replay_app.runs = runs.clone();
            replay_app.run = Some(run.clone());
            replay_app.replay_state.select(Some(selected));
            replay_app.replay_follow_selection = true;
            frames.push(render_frame(
                format!("replay-{run_index}-{selected}"),
                "replay",
                &mut replay_app,
                width,
                height,
            )?);
        }
    }

    compact_frames(&mut frames);

    Ok(DemoBundle {
        schema: 1,
        width,
        height,
        default_fg: color_to_css(CATPPUCCIN_MOCHA.fg).expect("theme foreground is RGB"),
        default_bg: color_to_css(CATPPUCCIN_MOCHA.bg).expect("theme background is RGB"),
        seed: demo_seed(&plan, &review_entries),
        frames,
    })
}

fn demo_seed(plan: &Plan, review_entries: &[review::ReviewEntry]) -> DemoSeed {
    let items = plan
        .items
        .iter()
        .map(|item| DemoSeedItem {
            name: item.name.clone(),
            layer: item.layer.clone(),
            action_count: item.actions.len(),
            selected: item.selected,
        })
        .collect();
    let review_entries = review_entries
        .iter()
        .map(|entry| DemoSeedReviewEntry {
            item_index: plan
                .items
                .iter()
                .position(|item| item.name == entry.item)
                .unwrap_or(0),
            order: entry.order,
            kind: entry.kind.into(),
            kind_icon: entry.kind_icon.into(),
            severity: format!("{:?}", entry.severity).to_ascii_lowercase(),
            status: entry.status.clone(),
            detail: entry.detail.clone(),
        })
        .collect();
    DemoSeed {
        items,
        review_entries,
    }
}

fn demo_runs(plan: &Plan) -> Vec<Run> {
    let mut success = finished_run(plan);
    success.id = "01WEBDEMO0000000000000000".into();
    success.started_at = "2026-07-16T12:00:00+08:00".into();

    let mut failed = success.clone();
    failed.id = "01WEBDEMOFAILED00000000000".into();
    failed.started_at = "2026-07-15T18:20:00+08:00".into();
    failed.status = RunStatus::Failed;
    if let Some(item) = failed
        .items
        .iter_mut()
        .find(|item| !item.actions.is_empty())
    {
        item.status = ActionStatus::WillFail;
        item.error = Some("simulated package check failure".into());
        item.actions[0].status = ActionStatus::WillFail;
        item.actions[0].error = Some("simulated package check failure".into());
    }

    let mut aborted = success.clone();
    aborted.id = "01WEBDEMOABORTED0000000000".into();
    aborted.started_at = "2026-07-14T09:30:00+08:00".into();
    aborted.status = RunStatus::Aborted;

    vec![success, failed, aborted]
}

fn base_app(config: &config::Config, plan: &Plan) -> App {
    let mut app = App::new(Mode::Deploy);
    app.config = Some(config.clone());
    app.plan = Some(plan.clone());
    app
}

fn running_app(config: &config::Config, plan: &Plan, spinner_frame: usize) -> App {
    let mut app = base_app(config, plan);
    app.screen = Screen::RunView;
    app.spinner_frame = spinner_frame;
    let total = selected_action_total(plan);
    let done = total.min(3);
    app.progress = (done, total);
    app.run_started = Some(Instant::now());
    app.run_item_statuses = vec![None; plan.items.len()];
    app.run_action_statuses = plan
        .items
        .iter()
        .map(|item| vec![None; item.actions.len()])
        .collect();

    let mut completed = 0;
    'items: for (item_index, item) in plan.items.iter().enumerate() {
        if !item.selected {
            app.run_item_statuses[item_index] = Some(ActionStatus::WillSkip);
            for status in &mut app.run_action_statuses[item_index] {
                *status = Some(ActionStatus::WillSkip);
            }
            continue;
        }
        for action_index in 0..item.actions.len().max(1) {
            if completed < done {
                if item.actions.is_empty() {
                    app.run_item_statuses[item_index] = Some(ActionStatus::Executed);
                } else {
                    app.run_action_statuses[item_index][action_index] =
                        Some(result_status(&item.actions[action_index], completed));
                }
                completed += 1;
                continue;
            }
            app.current_item = Some(item_index);
            if !item.actions.is_empty() {
                app.current_action = Some((item_index, action_index));
            }
            break 'items;
        }
    }
    app.current_log = demo_log(false);
    app
}

fn selected_action_total(plan: &Plan) -> usize {
    plan.items
        .iter()
        .filter(|item| item.selected)
        .map(|item| item.actions.len().max(1))
        .sum()
}

fn demo_review_entries(plan: &Plan) -> Vec<review::ReviewEntry> {
    let icon_set = icons::current();
    let mut entries = Vec::new();
    for item in plan.items.iter().filter(|item| item.selected) {
        for action in &item.actions {
            let (kind, kind_icon, severity, status, detail) = match action {
                Action::Install { spec } => (
                    "install",
                    icon_set.action_install,
                    review::ReviewSeverity::Run,
                    "missing".to_string(),
                    spec.command
                        .clone()
                        .unwrap_or_else(|| format!("install {}", spec.entry.name)),
                ),
                Action::Link { target, source, .. } => (
                    "link",
                    icon_set.action_link,
                    review::ReviewSeverity::Run,
                    "link".to_string(),
                    format!("{} -> {}", target.display(), source.display()),
                ),
                Action::Create { target } => (
                    "create",
                    icon_set.action_create,
                    review::ReviewSeverity::Run,
                    "create".to_string(),
                    target.display().to_string(),
                ),
                Action::Shell {
                    command,
                    description,
                    optional,
                    ..
                } => {
                    let needs_sudo = shell::command_contains_sudo(command);
                    let mut status = if *optional { "optional" } else { "run" }.to_string();
                    if needs_sudo {
                        status.push_str(" · sudo");
                    }
                    (
                        "shell",
                        icon_set.action_shell,
                        if needs_sudo {
                            review::ReviewSeverity::Warning
                        } else {
                            review::ReviewSeverity::Run
                        },
                        status,
                        description.clone().unwrap_or_else(|| command.clone()),
                    )
                }
                Action::Clean { target, force } => (
                    "clean",
                    icon_set.action_clean,
                    if *force {
                        review::ReviewSeverity::Warning
                    } else {
                        review::ReviewSeverity::Run
                    },
                    if *force { "backup + remove" } else { "remove" }.to_string(),
                    target.display().to_string(),
                ),
            };
            entries.push(review::ReviewEntry {
                order: entries.len(),
                item: item.name.clone(),
                kind,
                kind_icon,
                severity,
                status,
                detail,
            });
        }
    }
    entries
}

fn finished_run(plan: &Plan) -> Run {
    let mut ordinal = 0;
    let items = plan
        .items
        .iter()
        .map(|item| {
            let selected = item.selected;
            let actions = item
                .actions
                .iter()
                .map(|action| {
                    let status = if selected {
                        let status = result_status(action, ordinal);
                        ordinal += 1;
                        status
                    } else {
                        ActionStatus::WillSkip
                    };
                    RunAction {
                        kind: action_kind(action).into(),
                        name: action.describe(),
                        status,
                        error: None,
                        output: Vec::new(),
                    }
                })
                .collect::<Vec<_>>();
            let status = if !selected {
                ActionStatus::WillSkip
            } else if actions
                .iter()
                .all(|action| action.status == ActionStatus::NoChange)
            {
                ActionStatus::NoChange
            } else {
                actions
                    .iter()
                    .map(|action| action.status)
                    .max()
                    .unwrap_or(ActionStatus::Executed)
            };
            RunItem {
                id: item.id.clone(),
                name: item.name.clone(),
                status,
                started_at: selected.then(|| "2026-07-16T12:00:00+08:00".into()),
                finished_at: selected.then(|| "2026-07-16T12:00:01+08:00".into()),
                duration_ms: selected.then_some(320),
                attempts: u32::from(selected),
                error: None,
                output: Vec::<OutputLine>::new(),
                actions,
            }
        })
        .collect();
    Run {
        id: "01WEBDEMO0000000000000000".into(),
        plan_id: Some(plan.id.clone()),
        mode: crate::model::Mode::Deploy,
        started_at: "2026-07-16T12:00:00+08:00".into(),
        finished_at: Some("2026-07-16T12:00:05+08:00".into()),
        status: RunStatus::Success,
        config_hash: plan.config_hash.clone(),
        config_path: Some(plan.config_path.clone()),
        host: Some(plan.host.clone()),
        items,
    }
}

fn result_status(action: &Action, ordinal: usize) -> ActionStatus {
    if ordinal % 4 == 3 {
        return ActionStatus::NoChange;
    }
    match action {
        Action::Install { .. } => ActionStatus::WillInstall,
        Action::Link { .. } => ActionStatus::WillLink,
        Action::Create { .. } => ActionStatus::WillCreate,
        Action::Shell { .. } => ActionStatus::Executed,
        Action::Clean { .. } => ActionStatus::WillClean,
    }
}

fn action_kind(action: &Action) -> &'static str {
    match action {
        Action::Install { .. } => "install",
        Action::Link { .. } => "link",
        Action::Create { .. } => "create",
        Action::Shell { .. } => "shell",
        Action::Clean { .. } => "clean",
    }
}

fn demo_log(finished: bool) -> Vec<LogLine> {
    let mut lines = vec![
        LogLine {
            text: "deploy started".into(),
            fg: Some(CATPPUCCIN_MOCHA.text_muted),
            indent: 0,
            group: None,
            kind: LogKind::System,
        },
        LogLine {
            text: "checking selected actions".into(),
            fg: Some(CATPPUCCIN_MOCHA.fg),
            indent: 1,
            group: None,
            kind: LogKind::Action,
        },
        LogLine {
            text: "configuration linked".into(),
            fg: Some(CATPPUCCIN_MOCHA.success),
            indent: 1,
            group: None,
            kind: LogKind::Stdout,
        },
    ];
    if finished {
        lines.push(LogLine {
            text: "run completed successfully".into(),
            fg: Some(CATPPUCCIN_MOCHA.success),
            indent: 0,
            group: None,
            kind: LogKind::System,
        });
    }
    lines
}

fn render_frame(
    id: String,
    screen: &str,
    app: &mut App,
    width: u16,
    height: u16,
) -> Result<DemoFrame> {
    let backend = TestBackend::new(width, height);
    let mut terminal = ratatui::Terminal::new(backend)?;
    terminal.draw(|frame| super::render(app, frame))?;
    let buffer = terminal.backend().buffer();
    let cells = buffer
        .content()
        .iter()
        .enumerate()
        .filter_map(|(index, cell)| serialize_cell(index, width, cell))
        .collect();
    Ok(DemoFrame {
        id,
        screen: screen.into(),
        base: None,
        cells,
    })
}

fn compact_frames(frames: &mut [DemoFrame]) {
    let full_cells = frames
        .iter()
        .map(|frame| (frame.id.clone(), frame.cells.clone()))
        .collect::<BTreeMap<_, _>>();

    for frame in frames {
        let Some(base_id) = compact_base_id(&frame.id) else {
            continue;
        };
        let Some(base_cells) = full_cells.get(&base_id) else {
            continue;
        };
        let base = base_cells
            .iter()
            .map(|cell| ((cell.x, cell.y), cell))
            .collect::<BTreeMap<_, _>>();
        let current = frame
            .cells
            .iter()
            .map(|cell| ((cell.x, cell.y), cell))
            .collect::<BTreeMap<_, _>>();
        let mut positions = base
            .keys()
            .chain(current.keys())
            .copied()
            .collect::<Vec<_>>();
        positions.sort_unstable();
        positions.dedup();

        frame.cells = positions
            .into_iter()
            .filter_map(
                |position| match (base.get(&position), current.get(&position)) {
                    (Some(before), Some(after)) if cells_equal(before, after) => None,
                    (_, Some(after)) => Some((*after).clone()),
                    (Some(_), None) => Some(blank_cell(position.0, position.1)),
                    (None, None) => None,
                },
            )
            .collect();
        frame.base = Some(base_id);
    }
}

fn compact_base_id(id: &str) -> Option<String> {
    if id.starts_with("main-menu-") && id != "main-menu-0" {
        return Some("main-menu-0".into());
    }
    if id.starts_with("run-") && id != "run-0" {
        return Some("run-0".into());
    }
    if id.starts_with("history-") && id != "history-0" {
        return Some("history-0".into());
    }
    if let Some(rest) = id.strip_prefix("replay-") {
        let mut parts = rest.split('-');
        let run = parts.next()?;
        let selected = parts.next()?;
        if selected != "0" {
            return Some(format!("replay-{run}-0"));
        }
    }
    None
}

fn cells_equal(left: &DemoCell, right: &DemoCell) -> bool {
    left.x == right.x
        && left.y == right.y
        && left.symbol == right.symbol
        && left.fg == right.fg
        && left.bg == right.bg
        && left.bold == right.bold
        && left.dim == right.dim
        && left.italic == right.italic
        && left.underlined == right.underlined
        && left.reversed == right.reversed
}

fn blank_cell(x: u16, y: u16) -> DemoCell {
    DemoCell {
        x,
        y,
        symbol: " ".into(),
        fg: None,
        bg: None,
        bold: false,
        dim: false,
        italic: false,
        underlined: false,
        reversed: false,
    }
}

fn serialize_cell(index: usize, width: u16, cell: &Cell) -> Option<DemoCell> {
    let has_symbol = cell.symbol() != " ";
    let has_style = cell.fg != Color::Reset || cell.bg != Color::Reset || !cell.modifier.is_empty();
    if !has_symbol && !has_style {
        return None;
    }
    Some(DemoCell {
        x: index as u16 % width,
        y: index as u16 / width,
        symbol: cell.symbol().into(),
        fg: color_to_css(cell.fg),
        bg: color_to_css(cell.bg),
        bold: cell.modifier.contains(Modifier::BOLD),
        dim: cell.modifier.contains(Modifier::DIM),
        italic: cell.modifier.contains(Modifier::ITALIC),
        underlined: cell.modifier.contains(Modifier::UNDERLINED),
        reversed: cell.modifier.contains(Modifier::REVERSED),
    })
}

fn color_to_css(color: Color) -> Option<String> {
    let rgb = match color {
        Color::Reset => return None,
        Color::Black => (0, 0, 0),
        Color::Red => (205, 49, 49),
        Color::Green => (13, 188, 121),
        Color::Yellow => (229, 229, 16),
        Color::Blue => (36, 114, 200),
        Color::Magenta => (188, 63, 188),
        Color::Cyan => (17, 168, 205),
        Color::Gray => (229, 229, 229),
        Color::DarkGray => (102, 102, 102),
        Color::LightRed => (241, 76, 76),
        Color::LightGreen => (35, 209, 139),
        Color::LightYellow => (245, 245, 67),
        Color::LightBlue => (59, 142, 234),
        Color::LightMagenta => (214, 112, 214),
        Color::LightCyan => (41, 184, 219),
        Color::White => (255, 255, 255),
        Color::Rgb(red, green, blue) => (red, green, blue),
        Color::Indexed(index) => xterm_color(index),
    };
    Some(format!("#{:02x}{:02x}{:02x}", rgb.0, rgb.1, rgb.2))
}

fn xterm_color(index: u8) -> (u8, u8, u8) {
    const ANSI: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (128, 0, 0),
        (0, 128, 0),
        (128, 128, 0),
        (0, 0, 128),
        (128, 0, 128),
        (0, 128, 128),
        (192, 192, 192),
        (128, 128, 128),
        (255, 0, 0),
        (0, 255, 0),
        (255, 255, 0),
        (0, 0, 255),
        (255, 0, 255),
        (0, 255, 255),
        (255, 255, 255),
    ];
    match index {
        0..=15 => ANSI[index as usize],
        16..=231 => {
            let value = index - 16;
            let red = value / 36;
            let green = (value % 36) / 6;
            let blue = value % 6;
            let component = |part: u8| if part == 0 { 0 } else { 55 + part * 40 };
            (component(red), component(green), component(blue))
        }
        232..=255 => {
            let value = 8 + (index - 232) * 10;
            (value, value, value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_color_preserves_theme_rgb() {
        assert_eq!(
            color_to_css(CATPPUCCIN_MOCHA.primary).as_deref(),
            Some("#cba6f7")
        );
        assert_eq!(color_to_css(Color::Reset), None);
    }

    #[test]
    fn xterm_cube_has_expected_endpoints() {
        assert_eq!(xterm_color(16), (0, 0, 0));
        assert_eq!(xterm_color(231), (255, 255, 255));
        assert_eq!(xterm_color(255), (238, 238, 238));
    }

    #[test]
    fn export_contains_the_complete_ui_only_flow() {
        let directory = tempfile::tempdir().unwrap();
        let config_path = directory.path().join("dotman.yaml");
        std::fs::write(
            &config_path,
            "auto_install_pkg_manager: false\ninstall: []\ncreate:\n  - ~/.config/dotman-demo\n",
        )
        .unwrap();

        let bundle = export(&config_path, 80, 24).unwrap();
        let ids = bundle
            .frames
            .iter()
            .map(|frame| frame.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(bundle.width, 80);
        assert_eq!(bundle.height, 24);
        for required in [
            "main-menu-0",
            "main-menu-1",
            "main-menu-2",
            "main-menu-3",
            "plan",
            "review",
            "run-0",
            "run-3",
            "run-6",
            "result",
            "history-0",
            "replay-0-0",
        ] {
            assert!(ids.contains(&required), "missing demo frame {required}");
        }
        assert!(bundle.frames.iter().all(|frame| {
            frame
                .cells
                .iter()
                .all(|cell| cell.x < bundle.width && cell.y < bundle.height)
        }));
    }
}
