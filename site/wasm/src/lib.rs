//! Side-effect-free interactive Ratatui runtime for the static dotman demo.

use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::BTreeSet;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const SCREEN_MAIN: u32 = 0;
const SCREEN_PLAN: u32 = 1;
const SCREEN_REVIEW: u32 = 2;
const SCREEN_RUN: u32 = 3;
const SCREEN_RESULT: u32 = 4;
const SCREEN_HISTORY: u32 = 5;
const SCREEN_REPLAY: u32 = 6;

const KEY_UP: u32 = 1;
const KEY_DOWN: u32 = 2;
const KEY_LEFT: u32 = 3;
const KEY_RIGHT: u32 = 4;
const KEY_ENTER: u32 = 5;
const KEY_SPACE: u32 = 6;
const KEY_BACK: u32 = 7;
const KEY_RUN: u32 = 8;
const KEY_ESCAPE: u32 = 9;
const KEY_PAGE_UP: u32 = 10;
const KEY_PAGE_DOWN: u32 = 11;
const KEY_HOME: u32 = 12;
const KEY_END: u32 = 13;
const KEY_ALL: u32 = 14;
const KEY_NONE: u32 = 15;
const KEY_SAVE: u32 = 16;
const KEY_TAB: u32 = 17;
const KEY_DEPLOY: u32 = 20;
const KEY_PLAN: u32 = 21;
const KEY_HISTORY: u32 = 22;
const KEY_DISCARD: u32 = 23;
const KEY_LAYER_1: u32 = 31;
const KEY_LAYER_6: u32 = 36;

const BG: Color = Color::Rgb(30, 30, 46);
const FG: Color = Color::Rgb(205, 214, 244);
const FG_DIM: Color = Color::Rgb(108, 112, 134);
const MUTED: Color = Color::Rgb(147, 153, 178);
const PRIMARY: Color = Color::Rgb(203, 166, 247);
const SUCCESS: Color = Color::Rgb(166, 227, 161);
const WARNING: Color = Color::Rgb(249, 226, 175);
const ACCENT: Color = Color::Rgb(137, 180, 250);
const ACTIVE_BG: Color = Color::Rgb(39, 36, 52);
const DIVIDER: Color = Color::Rgb(55, 57, 73);

const LAYERS: [&str; 6] = [
    "terminal",
    "shell",
    "multiplexer",
    "software",
    "enhancement",
    "misc",
];

#[derive(Clone, Deserialize)]
struct Seed {
    items: Vec<SeedItem>,
    review_entries: Vec<ReviewEntry>,
}

#[derive(Clone, Deserialize)]
struct SeedItem {
    name: String,
    layer: String,
    action_count: usize,
    selected: bool,
}

#[derive(Clone, Deserialize)]
struct ReviewEntry {
    item_index: usize,
    order: usize,
    kind: String,
    kind_icon: String,
    severity: String,
    status: String,
    detail: String,
}

#[derive(Clone)]
enum PlanRow {
    Header { layer: String, ordinal: usize },
    Item(usize),
    Inline(Vec<usize>),
    Divider,
}

#[derive(Serialize)]
struct RenderedFrame {
    screen: &'static str,
    cells: Vec<RenderedCell>,
}

#[derive(Serialize)]
struct RenderedCell {
    x: u16,
    y: u16,
    symbol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    fg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bg: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    bold: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    dim: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    italic: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    underlined: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    reversed: bool,
}

struct DemoState {
    screen: u32,
    index: u32,
    parent_index: u32,
    run_step: u32,
    run_scroll: usize,
    run_filter: u32,
    run_folded: bool,
    run_aborting: bool,
    run_aborted: bool,
    history_count: u32,
    replay_count: u32,
    replay_expanded: bool,
    seed: Seed,
    saved_selection: Vec<bool>,
    plan_row: usize,
    grid_col: usize,
    collapsed: BTreeSet<String>,
    dirty: bool,
    exit_pending: bool,
    status: String,
    review_scroll: usize,
    width: u16,
    height: u16,
}

impl Default for DemoState {
    fn default() -> Self {
        Self {
            screen: SCREEN_MAIN,
            index: 0,
            parent_index: 0,
            run_step: 0,
            run_scroll: 0,
            run_filter: 0,
            run_folded: false,
            run_aborting: false,
            run_aborted: false,
            history_count: 1,
            replay_count: 1,
            replay_expanded: false,
            seed: Seed {
                items: Vec::new(),
                review_entries: Vec::new(),
            },
            saved_selection: Vec::new(),
            plan_row: 0,
            grid_col: 0,
            collapsed: BTreeSet::new(),
            dirty: false,
            exit_pending: false,
            status: String::new(),
            review_scroll: 0,
            width: 160,
            height: 40,
        }
    }
}

impl DemoState {
    fn rows(&self) -> Vec<PlanRow> {
        build_rows(&self.seed.items, &self.collapsed, plan_columns(self.width))
    }

    fn selected_item_index(&self) -> Option<usize> {
        match self.rows().get(self.plan_row)? {
            PlanRow::Item(index) => Some(*index),
            PlanRow::Inline(indices) => indices.get(self.grid_col).copied(),
            _ => None,
        }
    }

    fn clamp_plan_focus(&mut self) {
        let rows = self.rows();
        if rows.is_empty() {
            self.plan_row = 0;
            return;
        }
        if self.plan_row >= rows.len() || matches!(rows[self.plan_row], PlanRow::Divider) {
            self.plan_row = rows
                .iter()
                .position(|row| !matches!(row, PlanRow::Divider))
                .unwrap_or(0);
        }
        if let Some(PlanRow::Inline(indices)) = rows.get(self.plan_row) {
            self.grid_col = self.grid_col.min(indices.len().saturating_sub(1));
        } else {
            self.grid_col = 0;
        }
    }

    fn move_plan(&mut self, down: bool, amount: usize) {
        let rows = self.rows();
        for _ in 0..amount {
            let candidate = if down {
                ((self.plan_row + 1)..rows.len())
                    .find(|index| !matches!(rows[*index], PlanRow::Divider))
            } else {
                (0..self.plan_row)
                    .rev()
                    .find(|index| !matches!(rows[*index], PlanRow::Divider))
            };
            if let Some(candidate) = candidate {
                self.plan_row = candidate;
                self.grid_col = 0;
            }
        }
        self.update_focus_status();
    }

    fn update_focus_status(&mut self) {
        self.status = self
            .selected_item_index()
            .and_then(|index| self.seed.items.get(index))
            .map(|item| {
                if item.action_count > 1 {
                    format!("{}: {} actions", item.name, item.action_count)
                } else {
                    format!("{}: 1 action", item.name)
                }
            })
            .unwrap_or_default();
    }

    fn toggle_current(&mut self) {
        let rows = self.rows();
        match rows.get(self.plan_row) {
            Some(PlanRow::Header { layer, .. }) => {
                if !self.collapsed.remove(layer) {
                    self.collapsed.insert(layer.clone());
                }
                self.clamp_plan_focus();
            }
            _ => {
                if let Some(index) = self.selected_item_index()
                    && let Some(item) = self.seed.items.get_mut(index)
                {
                    item.selected = !item.selected;
                    self.dirty = true;
                }
            }
        }
        self.update_focus_status();
    }

    fn selected_counts(&self) -> (usize, usize) {
        self.seed
            .items
            .iter()
            .filter(|item| item.selected)
            .fold((0, 0), |(items, actions), item| {
                (items + 1, actions + item.action_count)
            })
    }

    fn save_selection(&mut self) {
        self.saved_selection = self.seed.items.iter().map(|item| item.selected).collect();
        self.dirty = false;
    }

    fn restore_saved_selection(&mut self) {
        for (item, selected) in self.seed.items.iter_mut().zip(&self.saved_selection) {
            item.selected = *selected;
        }
        self.dirty = false;
    }

    fn input(&mut self, key: u32) {
        match self.screen {
            SCREEN_MAIN => self.input_main(key),
            SCREEN_PLAN => self.input_plan(key),
            SCREEN_REVIEW => self.input_review(key),
            SCREEN_RUN => match key {
                KEY_UP => self.run_scroll = self.run_scroll.saturating_add(1),
                KEY_DOWN => self.run_scroll = self.run_scroll.saturating_sub(1),
                KEY_TAB => self.run_filter = (self.run_filter + 1) % 3,
                KEY_ENTER | KEY_SPACE => self.run_folded = !self.run_folded,
                KEY_BACK | KEY_ESCAPE => {
                    self.run_aborting = true;
                }
                _ => {}
            },
            SCREEN_RESULT => match key {
                KEY_UP => self.run_scroll = self.run_scroll.saturating_add(1),
                KEY_DOWN => self.run_scroll = self.run_scroll.saturating_sub(1),
                KEY_TAB => self.run_filter = (self.run_filter + 1) % 3,
                KEY_ENTER | KEY_SPACE => self.run_folded = !self.run_folded,
                KEY_BACK | KEY_ESCAPE => {
                    self.screen = SCREEN_MAIN;
                    self.index = self.parent_index;
                    self.run_scroll = 0;
                }
                _ => {}
            },
            SCREEN_HISTORY => self.input_history(key),
            SCREEN_REPLAY => self.input_replay(key),
            _ => self.screen = SCREEN_MAIN,
        }
    }

    fn pointer(&mut self, column: u16, row: u16, clicks: u32) {
        match self.screen {
            SCREEN_MAIN => {
                let target = match row {
                    3 | 4 => Some(0),
                    6 | 7 => Some(1),
                    9 | 10 => Some(2),
                    12 | 13 => Some(3),
                    _ => None,
                };
                if let Some(target) = target {
                    self.index = target;
                    if clicks >= 2 {
                        self.input_main(KEY_ENTER);
                    }
                }
            }
            SCREEN_PLAN => self.pointer_plan(column, row, clicks),
            SCREEN_RUN | SCREEN_RESULT => {
                let log_top = self.height.saturating_sub(11);
                if row == log_top {
                    self.run_filter = (self.run_filter + 1) % 3;
                } else if row > log_top && row < self.height.saturating_sub(1) {
                    self.run_folded = !self.run_folded;
                }
            }
            SCREEN_HISTORY if row >= 2 => {
                let target = u32::from(row - 2);
                if target < self.history_count {
                    self.index = target;
                    if clicks >= 2 {
                        self.input_history(KEY_ENTER);
                    }
                }
            }
            SCREEN_REPLAY if row >= 3 => {
                let mut target = u32::from(row - 3);
                if self.replay_expanded && target > self.index {
                    target = target.saturating_sub(1);
                }
                if target < self.replay_count {
                    self.index = target;
                    self.replay_expanded = clicks >= 2;
                }
            }
            _ => {}
        }
    }

    fn pointer_plan(&mut self, column: u16, row: u16, clicks: u32) {
        let body_y = 2;
        let footer_y = self.height.saturating_sub(4);
        if row < body_y || row >= footer_y {
            return;
        }
        let rows = self.rows();
        let max_rows = footer_y.saturating_sub(body_y) as usize;
        let offset = list_offset(self.plan_row, rows.len(), max_rows);
        let row_index = offset + usize::from(row - body_y);
        let Some(target) = rows.get(row_index).cloned() else {
            return;
        };
        match target {
            PlanRow::Header { layer, .. } => {
                self.plan_row = row_index;
                self.grid_col = 0;
                if !self.collapsed.remove(&layer) {
                    self.collapsed.insert(layer);
                }
                self.clamp_plan_focus();
            }
            PlanRow::Item(_) => {
                self.plan_row = row_index;
                self.grid_col = 0;
                if clicks >= 2 || (5..=7).contains(&column) {
                    self.toggle_current();
                } else {
                    self.update_focus_status();
                }
            }
            PlanRow::Inline(indices) => {
                let width = self.width.saturating_sub(2) as usize;
                let cell_width = grid_cell_width(width, plan_columns(self.width));
                let stride = cell_width + 4;
                let clicked = usize::from(column.saturating_sub(3)) / stride;
                self.plan_row = row_index;
                self.grid_col = clicked.min(indices.len().saturating_sub(1));
                let start = 3 + self.grid_col * stride;
                if clicks >= 2 || (start + 2..=start + 4).contains(&usize::from(column)) {
                    self.toggle_current();
                } else {
                    self.update_focus_status();
                }
            }
            PlanRow::Divider => {}
        }
    }

    fn input_main(&mut self, key: u32) {
        match key {
            KEY_UP => self.index = self.index.saturating_sub(1),
            KEY_DOWN => self.index = (self.index + 1).min(3),
            KEY_PAGE_UP | KEY_HOME => self.index = 0,
            KEY_PAGE_DOWN | KEY_END => self.index = 3,
            KEY_ENTER => match self.index {
                0 | 1 => self.open_plan(self.index),
                2 => {
                    self.screen = SCREEN_HISTORY;
                    self.index = 0;
                }
                3 => self.index = 0,
                _ => {}
            },
            KEY_DEPLOY => self.open_plan(0),
            KEY_PLAN => self.open_plan(1),
            KEY_HISTORY => {
                self.screen = SCREEN_HISTORY;
                self.index = 0;
            }
            _ => {}
        }
    }

    fn open_plan(&mut self, mode: u32) {
        self.parent_index = mode;
        self.screen = SCREEN_PLAN;
        self.plan_row = 0;
        self.grid_col = 0;
        self.status.clear();
    }

    fn input_plan(&mut self, key: u32) {
        if self.exit_pending {
            match key {
                KEY_SAVE => {
                    self.save_selection();
                    self.exit_pending = false;
                    self.screen = SCREEN_MAIN;
                    self.index = self.parent_index;
                }
                KEY_DISCARD => {
                    self.exit_pending = false;
                    self.restore_saved_selection();
                    self.screen = SCREEN_MAIN;
                    self.index = self.parent_index;
                }
                KEY_ESCAPE => {
                    self.exit_pending = false;
                    self.status.clear();
                }
                _ => {}
            }
            return;
        }
        match key {
            KEY_UP => self.move_plan(false, 1),
            KEY_DOWN => self.move_plan(true, 1),
            KEY_PAGE_UP => self.move_plan(false, 8),
            KEY_PAGE_DOWN => self.move_plan(true, 8),
            KEY_HOME => {
                self.plan_row = 0;
                self.clamp_plan_focus();
                self.update_focus_status();
            }
            KEY_END => {
                let rows = self.rows();
                self.plan_row = rows
                    .iter()
                    .rposition(|row| !matches!(row, PlanRow::Divider))
                    .unwrap_or(0);
                self.clamp_plan_focus();
                self.update_focus_status();
            }
            KEY_LEFT => self.grid_col = self.grid_col.saturating_sub(1),
            KEY_RIGHT => {
                if let Some(PlanRow::Inline(indices)) = self.rows().get(self.plan_row) {
                    self.grid_col = (self.grid_col + 1).min(indices.len().saturating_sub(1));
                }
                self.update_focus_status();
            }
            KEY_ENTER | KEY_SPACE => self.toggle_current(),
            KEY_ALL | KEY_NONE => {
                let selected = key == KEY_ALL;
                if self.seed.items.iter().any(|item| item.selected != selected) {
                    for item in &mut self.seed.items {
                        item.selected = selected;
                    }
                    self.dirty = true;
                }
            }
            KEY_SAVE => {
                self.save_selection();
                self.status = "saved selection in this demo session".into();
            }
            KEY_LAYER_1..=KEY_LAYER_6 => {
                let layer = LAYERS[(key - KEY_LAYER_1) as usize];
                if !self.collapsed.remove(layer) {
                    self.collapsed.insert(layer.into());
                }
                self.clamp_plan_focus();
            }
            KEY_RUN => {
                if self.parent_index == 1 {
                    self.status = "plan mode is read-only; choose deploy to run".into();
                } else if self.selected_counts().0 == 0 {
                    self.status = "nothing selected".into();
                } else {
                    self.screen = SCREEN_REVIEW;
                    self.review_scroll = 0;
                    self.status.clear();
                }
            }
            KEY_BACK | KEY_ESCAPE => {
                if self.dirty {
                    self.exit_pending = true;
                    self.status =
                        "Unsaved selection changes  [S] Save  [D] Discard  [Esc] Cancel".into();
                } else {
                    self.screen = SCREEN_MAIN;
                    self.index = self.parent_index;
                }
            }
            _ => {}
        }
    }

    fn input_review(&mut self, key: u32) {
        match key {
            KEY_UP => self.review_scroll = self.review_scroll.saturating_sub(1),
            KEY_DOWN => self.review_scroll = self.review_scroll.saturating_add(1),
            KEY_PAGE_UP => self.review_scroll = self.review_scroll.saturating_sub(8),
            KEY_PAGE_DOWN => self.review_scroll = self.review_scroll.saturating_add(8),
            KEY_HOME => self.review_scroll = 0,
            KEY_END => self.review_scroll = usize::MAX,
            KEY_RUN | KEY_ENTER => {
                self.screen = SCREEN_RUN;
                self.run_step = 0;
                self.run_scroll = 0;
                self.run_filter = 0;
                self.run_folded = false;
                self.run_aborting = false;
                self.run_aborted = false;
            }
            KEY_BACK | KEY_ESCAPE => self.screen = SCREEN_PLAN,
            _ => {}
        }
    }

    fn input_history(&mut self, key: u32) {
        match key {
            KEY_UP => self.index = self.index.saturating_sub(1),
            KEY_DOWN => self.index = (self.index + 1).min(self.history_count.saturating_sub(1)),
            KEY_ENTER => {
                self.parent_index = self.index;
                self.screen = SCREEN_REPLAY;
                self.index = 0;
                self.replay_expanded = false;
            }
            KEY_BACK | KEY_ESCAPE => {
                self.screen = SCREEN_MAIN;
                self.index = 2;
            }
            _ => {}
        }
    }

    fn input_replay(&mut self, key: u32) {
        match key {
            KEY_UP => {
                self.index = self.index.saturating_sub(1);
                self.replay_expanded = false;
            }
            KEY_DOWN => {
                self.index = (self.index + 1).min(self.replay_count.saturating_sub(1));
                self.replay_expanded = false;
            }
            KEY_ENTER | KEY_SPACE => self.replay_expanded = !self.replay_expanded,
            KEY_BACK | KEY_ESCAPE => {
                self.screen = SCREEN_HISTORY;
                self.index = self.parent_index;
                self.replay_expanded = false;
            }
            _ => {}
        }
    }

    fn tick(&mut self) {
        if self.screen == SCREEN_RUN {
            if self.run_aborting {
                self.run_aborting = false;
                self.run_aborted = true;
                self.screen = SCREEN_RESULT;
                return;
            }
            self.run_step += 1;
            if self.run_step >= 6 {
                self.screen = SCREEN_RESULT;
            }
        }
    }

    fn render(&mut self) -> RenderedFrame {
        let mut buffer = Buffer::empty(Rect::new(0, 0, self.width, self.height));
        buffer.set_style(buffer.area, Style::default().bg(BG).fg(FG));
        let screen = match self.screen {
            SCREEN_REVIEW => {
                render_review(&mut buffer, self);
                "review"
            }
            SCREEN_RUN => {
                render_run(&mut buffer, self, false);
                "run"
            }
            SCREEN_RESULT => {
                render_run(&mut buffer, self, true);
                "result"
            }
            _ => {
                render_plan(&mut buffer, self);
                "plan"
            }
        };
        RenderedFrame {
            screen,
            cells: serialize_buffer(&buffer),
        }
    }
}

fn build_rows(items: &[SeedItem], collapsed: &BTreeSet<String>, columns: usize) -> Vec<PlanRow> {
    let mut rows = Vec::new();
    for (ordinal, layer) in LAYERS.iter().enumerate() {
        let indices = items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| (item.layer == *layer).then_some(index))
            .collect::<Vec<_>>();
        if indices.is_empty() {
            continue;
        }
        rows.push(PlanRow::Header {
            layer: (*layer).into(),
            ordinal: ordinal + 1,
        });
        if !collapsed.contains(*layer) {
            if ordinal < 3 || columns == 1 {
                rows.extend(indices.into_iter().map(PlanRow::Item));
            } else {
                rows.extend(
                    indices
                        .chunks(columns)
                        .map(|chunk| PlanRow::Inline(chunk.to_vec())),
                );
            }
        }
        if ordinal + 1 < LAYERS.len() {
            rows.push(PlanRow::Divider);
        }
    }
    rows
}

fn render_plan(buffer: &mut Buffer, state: &mut DemoState) {
    let width = state.width.saturating_sub(2) as usize;
    let body_y = 2;
    let footer_y = state.height.saturating_sub(4);
    let (selected, actions) = state.selected_counts();
    let saved = if state.dirty { "unsaved" } else { "saved" };
    let prefix = format!("  dotman - Plan (○ {saved})  {selected} selected · {actions} actions ");
    buffer.set_line(
        1,
        1,
        &Line::from(vec![
            Span::styled(
                "  dotman - Plan (",
                Style::default().fg(FG_DIM).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("○ {saved}"), Style::default().fg(MUTED)),
            Span::styled(")  ", Style::default().fg(FG_DIM)),
            Span::styled(
                format!("{selected} selected · {actions} actions"),
                Style::default().fg(FG_DIM),
            ),
            Span::styled(
                format!(" {}", "─".repeat(width.saturating_sub(text_width(&prefix)))),
                Style::default().fg(DIVIDER),
            ),
        ]),
        width as u16,
    );

    let rows = state.rows();
    state.clamp_plan_focus();
    let columns = plan_columns(state.width);
    let cell_width = grid_cell_width(width, columns);
    let max_rows = footer_y.saturating_sub(body_y) as usize;
    let offset = list_offset(state.plan_row, rows.len(), max_rows);
    for (visible, row) in rows.iter().skip(offset).take(max_rows).enumerate() {
        let y = body_y + visible as u16;
        let row_index = offset + visible;
        match row {
            PlanRow::Header { layer, ordinal } => {
                let indices = state
                    .seed
                    .items
                    .iter()
                    .enumerate()
                    .filter_map(|(index, item)| (item.layer == *layer).then_some(index))
                    .collect::<Vec<_>>();
                let enabled = indices
                    .iter()
                    .filter(|index| state.seed.items[**index].selected)
                    .count();
                let icon = if state.collapsed.contains(layer) {
                    ""
                } else {
                    ""
                };
                let left = format!("{icon} {ordinal:02}  {}", capitalize(layer));
                let right = format!("{enabled} / {}", indices.len());
                render_row(
                    buffer,
                    y,
                    width,
                    &left,
                    &right,
                    row_index == state.plan_row,
                    MUTED,
                );
            }
            PlanRow::Item(index) => render_plan_item(
                buffer,
                y,
                0,
                width,
                &state.seed.items[*index],
                row_index == state.plan_row,
            ),
            PlanRow::Inline(indices) => {
                for (column, index) in indices.iter().enumerate() {
                    let x = 2 + column * (cell_width + 4);
                    render_plan_item(
                        buffer,
                        y,
                        x,
                        cell_width,
                        &state.seed.items[*index],
                        row_index == state.plan_row && column == state.grid_col,
                    );
                }
            }
            PlanRow::Divider => put(
                buffer,
                1,
                y,
                &format!("  {}", "─".repeat(width.saturating_sub(2))),
                Style::default().fg(DIVIDER),
            ),
        }
    }

    if !state.status.is_empty() {
        put(
            buffer,
            1,
            footer_y,
            &format!("  {}", state.status),
            Style::default().fg(MUTED),
        );
    }
    let help = if state.parent_index == 1 {
        [
            ("↑↓", " Navigate  "),
            ("Space", " Toggle  "),
            ("s", " Save  "),
            ("q", " Back  "),
            ("read-only", ""),
        ]
        .as_slice()
    } else {
        [
            ("↑↓", " Navigate  "),
            ("Space", " Toggle  "),
            ("s", " Save  "),
            ("r", " Review  "),
            ("q", " Back"),
        ]
        .as_slice()
    };
    render_help(buffer, footer_y + 1, help);
}

fn render_help(buffer: &mut Buffer, y: u16, parts: &[(&str, &str)]) {
    let spans = parts
        .iter()
        .flat_map(|(key, label)| {
            [
                Span::styled(
                    format!("[{key}]"),
                    Style::default().fg(MUTED).add_modifier(Modifier::BOLD),
                ),
                Span::styled((*label).to_string(), Style::default().fg(FG_DIM)),
            ]
        })
        .collect::<Vec<_>>();
    buffer.set_line(
        1,
        y,
        &Line::from(spans),
        buffer.area.width.saturating_sub(2),
    );
}

fn render_plan_item(
    buffer: &mut Buffer,
    y: u16,
    relative_x: usize,
    width: usize,
    item: &SeedItem,
    focused: bool,
) {
    let x = 1 + relative_x as u16;
    let label = if item.action_count > 1 {
        format!("{} (+{})", item.name, item.action_count - 1)
    } else {
        item.name.clone()
    };
    let checkbox = if item.selected { "󰄲" } else { "󰄱" };
    let marker = if relative_x == 0 {
        if focused { "  ▎ " } else { "    " }
    } else if focused {
        "▎ "
    } else {
        "  "
    };
    let content = format!("{marker}{checkbox}  {label}");
    let style = Style::default()
        .fg(FG)
        .bg(if focused { ACTIVE_BG } else { BG });
    if focused {
        buffer.set_style(Rect::new(x, y, width as u16, 1), style);
    }
    put(buffer, x, y, &fit(&content, width), style);
    let checkbox_x = x + if relative_x == 0 { 4 } else { 2 };
    put(
        buffer,
        checkbox_x,
        y,
        checkbox,
        style.fg(if item.selected { SUCCESS } else { FG_DIM }),
    );
    if focused {
        put(
            buffer,
            x + if relative_x == 0 { 2 } else { 0 },
            y,
            "▎",
            style.fg(PRIMARY),
        );
    }
}

fn render_row(
    buffer: &mut Buffer,
    y: u16,
    width: usize,
    left: &str,
    right: &str,
    focused: bool,
    color: Color,
) {
    let inner = width.saturating_sub(2);
    let gap = inner
        .saturating_sub(text_width(left) + text_width(right))
        .max(1);
    let line = format!("  {left}{}{right}", " ".repeat(gap));
    let style = Style::default()
        .fg(color)
        .bg(if focused { ACTIVE_BG } else { BG });
    if focused {
        buffer.set_style(Rect::new(1, y, width as u16, 1), style);
    }
    put(buffer, 1, y, &fit(&line, width), style);
    if focused {
        put(buffer, 1, y, "▎", style.fg(PRIMARY));
    }
}

fn render_review(buffer: &mut Buffer, state: &mut DemoState) {
    let width = state.width.saturating_sub(2) as usize;
    let (selected, actions) = state.selected_counts();
    let entries = state
        .seed
        .review_entries
        .iter()
        .filter(|entry| {
            state
                .seed
                .items
                .get(entry.item_index)
                .is_some_and(|item| item.selected)
        })
        .cloned()
        .collect::<Vec<_>>();
    let risk = entries
        .iter()
        .filter(|entry| matches!(entry.severity.as_str(), "warning" | "danger"))
        .count();
    let prefix = "  dotman - Review  Deploy  ";
    put(
        buffer,
        1,
        1,
        &format!(
            "{prefix}{}",
            "─".repeat(width.saturating_sub(text_width(prefix)))
        ),
        Style::default().fg(FG_DIM).add_modifier(Modifier::BOLD),
    );
    put(
        buffer,
        1,
        2,
        &format!("Selected: {selected} steps, {actions} actions"),
        Style::default().fg(FG),
    );
    put(
        buffer,
        1,
        3,
        &format!(
            "Skipped: {} steps",
            state.seed.items.len().saturating_sub(selected)
        ),
        Style::default().fg(FG),
    );
    put(
        buffer,
        1,
        4,
        &format!(
            "Review: {} actions, {} active, {risk} attention",
            entries.len(),
            entries.len()
        ),
        Style::default().fg(FG),
    );

    let body_y = 5;
    let body_height = state.height.saturating_sub(7) as usize;
    let lines = review_lines(&entries, &state.seed.items, width);
    let max_scroll = lines.len().saturating_sub(body_height);
    state.review_scroll = state.review_scroll.min(max_scroll);
    for (offset, line) in lines
        .iter()
        .skip(state.review_scroll)
        .take(body_height)
        .enumerate()
    {
        buffer.set_line(1, body_y + offset as u16, line, width as u16);
    }
    put(
        buffer,
        1,
        state.height.saturating_sub(2),
        "[↑↓] Scroll  [r] Run  [q] Back",
        Style::default().fg(FG_DIM),
    );
}

fn selected_review_entries(state: &DemoState) -> Vec<&ReviewEntry> {
    state
        .seed
        .review_entries
        .iter()
        .filter(|entry| {
            state
                .seed
                .items
                .get(entry.item_index)
                .is_some_and(|item| item.selected)
        })
        .collect()
}

fn render_run(buffer: &mut Buffer, state: &DemoState, finished: bool) {
    let width = state.width.saturating_sub(2) as usize;
    let entries = selected_review_entries(state);
    let total = entries.len();
    let completed = if finished && !state.run_aborted {
        total
    } else {
        total.saturating_mul(state.run_step as usize) / 6
    };
    let status = if state.run_aborted {
        "Aborted"
    } else if state.run_aborting {
        "Stopping"
    } else if finished {
        "Success"
    } else {
        "Running"
    };
    let filled = completed
        .saturating_mul(10)
        .checked_div(total)
        .unwrap_or(10);
    let progress = format!("{}{}", "█".repeat(filled), "░".repeat(10 - filled));
    let right = format!("{status}  {completed}/{total}  {progress}");
    let prefix = "  Run  ";
    let divider = width.saturating_sub(text_width(prefix) + text_width(&right));
    buffer.set_line(
        1,
        1,
        &Line::from(vec![
            Span::styled(
                prefix,
                Style::default().fg(FG_DIM).add_modifier(Modifier::BOLD),
            ),
            Span::styled("─".repeat(divider), Style::default().fg(DIVIDER)),
            Span::styled(
                right,
                Style::default()
                    .fg(if state.run_aborted || state.run_aborting {
                        WARNING
                    } else if finished {
                        SUCCESS
                    } else {
                        ACCENT
                    })
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        width as u16,
    );

    let (ran, changed, unchanged) = result_counts(&entries, completed);
    let current = if state.run_aborted {
        format!("  current  run aborted after {completed}/{total} actions")
    } else if finished {
        format!("  current  {ran} ran, {changed} changed, {unchanged} no change, 0 failed")
    } else if let Some(entry) = entries.get(completed) {
        format!(
            "  current  {} / {}",
            state.seed.items[entry.item_index].name, entry.detail
        )
    } else {
        "  current  finalizing run".into()
    };
    put(
        buffer,
        1,
        2,
        &fit(&current, width),
        Style::default().fg(MUTED),
    );

    let body_start = 3;
    let body_height = state.height.saturating_sub(15) as usize;
    let follow_offset = if finished {
        total.saturating_sub(body_height)
    } else {
        completed
            .saturating_sub(body_height / 2)
            .min(total.saturating_sub(body_height))
    };
    let offset = follow_offset.saturating_sub(state.run_scroll);
    for (visible, (index, entry)) in entries
        .iter()
        .enumerate()
        .skip(offset)
        .take(body_height)
        .enumerate()
    {
        let (marker, color, outcome) = if (!state.run_aborted && finished) || index < completed {
            let outcome = action_outcome(entry, index);
            (
                "",
                if outcome == "no change" {
                    MUTED
                } else {
                    SUCCESS
                },
                outcome,
            )
        } else if state.run_aborted && index == completed {
            ("", WARNING, "aborted")
        } else if !finished && index == completed {
            let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴"][state.run_step as usize % 6];
            (spinner, ACCENT, "running")
        } else {
            ("", FG_DIM, "pending")
        };
        let item = &state.seed.items[entry.item_index].name;
        let label = format!("{marker} {} {item} / {}", entry.kind_icon, entry.detail);
        render_run_row(
            buffer,
            body_start + visible as u16,
            width,
            &label,
            outcome,
            color,
        );
    }

    let log_top = state.height.saturating_sub(11);
    render_log_panel(buffer, log_top, width, state, finished);
    let finished_help = [
        ("↑↓", " Scroll  "),
        ("Tab", " Filter  "),
        ("Enter", " Fold  "),
        ("q", " Back"),
    ];
    let running_help = [
        ("↑↓", " Scroll  "),
        ("Tab", " Filter  "),
        ("Enter", " Fold  "),
        if state.run_aborting {
            ("q", " Stopping")
        } else {
            ("q", " Abort")
        },
    ];
    let help = if finished {
        &finished_help
    } else {
        &running_help
    };
    render_help(buffer, state.height.saturating_sub(1), help);
}

fn result_counts(entries: &[&ReviewEntry], completed: usize) -> (usize, usize, usize) {
    entries.iter().take(completed).enumerate().fold(
        (0, 0, 0),
        |(ran, changed, unchanged), (index, entry)| match action_outcome(entry, index) {
            "ran" => (ran + 1, changed, unchanged),
            "no change" => (ran, changed, unchanged + 1),
            _ => (ran, changed + 1, unchanged),
        },
    )
}

fn action_outcome(entry: &ReviewEntry, index: usize) -> &'static str {
    if entry.kind == "shell" {
        "ran"
    } else if index % 4 == 3 {
        "no change"
    } else {
        "changed"
    }
}

fn render_run_row(
    buffer: &mut Buffer,
    y: u16,
    width: usize,
    label: &str,
    outcome: &str,
    color: Color,
) {
    let gap = width
        .saturating_sub(text_width(label) + text_width(outcome))
        .max(1);
    buffer.set_line(
        1,
        y,
        &Line::from(vec![
            Span::styled(label.to_string(), Style::default().fg(color)),
            Span::raw(" ".repeat(gap)),
            Span::styled(outcome.to_string(), Style::default().fg(color)),
        ]),
        width as u16,
    );
}

fn render_log_panel(
    buffer: &mut Buffer,
    top: u16,
    width: usize,
    state: &DemoState,
    finished: bool,
) {
    let filter = match state.run_filter {
        1 => "current",
        2 => "errors",
        _ => "all",
    };
    let title = format!("┌ log: follow · {filter} ");
    put(
        buffer,
        1,
        top,
        &format!(
            "{title}{}┐",
            "─".repeat(width.saturating_sub(text_width(&title) + 1))
        ),
        Style::default().fg(DIVIDER),
    );
    let activity = [
        "deploy started",
        "  checking selected actions",
        "  resolving package state",
        "  linking portable configuration",
        "  creating required directories",
        "  running guarded shell steps",
    ];
    let activity_count = if finished && !state.run_aborted {
        activity.len()
    } else {
        (state.run_step as usize + 1).min(activity.len())
    };
    let mut messages = activity[..activity_count].to_vec();
    if state.run_aborted {
        messages.push("run aborted");
    } else if finished {
        messages.push("run completed successfully");
    }
    let filtered = if state.run_filter == 2 {
        vec!["no errors in this simulated run"]
    } else if state.run_filter == 1 {
        messages
            .iter()
            .rev()
            .take(2)
            .rev()
            .copied()
            .collect::<Vec<_>>()
    } else if state.run_folded && messages.len() > 1 {
        let mut folded = vec![messages[0], "  ▸ activity (details folded)"];
        if state.run_aborted {
            folded.push("run aborted");
        } else if finished {
            folded.push("run completed successfully");
        }
        folded
    } else {
        messages
    };
    for row in 0..8 {
        let message = filtered.get(row).copied().unwrap_or("");
        put(
            buffer,
            1,
            top + row as u16 + 1,
            &format!("│{}│", fit(message, width.saturating_sub(2))),
            Style::default().fg(if message.contains("completed") {
                SUCCESS
            } else if message.contains("aborted") {
                WARNING
            } else {
                FG
            }),
        );
    }
    put(
        buffer,
        1,
        top + 9,
        &format!("└{}┘", "─".repeat(width.saturating_sub(2))),
        Style::default().fg(DIVIDER),
    );
}

fn review_lines(entries: &[ReviewEntry], items: &[SeedItem], width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for (severity, label, color, icon) in [
        ("warning", "Attention", WARNING, ""),
        ("run", "Will Run", ACCENT, ""),
        ("success", "Already OK", SUCCESS, ""),
        ("skip", "Skipped", FG_DIM, ""),
    ] {
        let mut group = entries
            .iter()
            .filter(|entry| {
                entry.severity == severity || (severity == "warning" && entry.severity == "danger")
            })
            .collect::<Vec<_>>();
        group.sort_by_key(|entry| (kind_rank(&entry.kind), entry.order));
        if group.is_empty() {
            continue;
        }
        lines.push(Line::from(Span::styled(
            fit(&format!("{icon} {label} ({})", group.len()), width),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )));
        for entry in group {
            let item = items
                .get(entry.item_index)
                .map(|item| item.name.as_str())
                .unwrap_or("item");
            let status_icon = match entry.severity.as_str() {
                "warning" => "",
                "danger" => "",
                "success" => "",
                "skip" => "",
                _ => "",
            };
            let text = format!(
                "{status_icon} {} {:<7} {item}  {}  {}",
                entry.kind_icon, entry.kind, entry.status, entry.detail
            );
            lines.push(Line::from(Span::styled(
                fit(&text, width),
                Style::default().fg(FG),
            )));
        }
    }
    if lines.is_empty() {
        lines.push(Line::from("No selected actions."));
    }
    lines
}

fn kind_rank(kind: &str) -> usize {
    match kind {
        "install" => 0,
        "link" => 1,
        "create" => 2,
        "shell" => 3,
        "clean" => 4,
        _ => 5,
    }
}

fn plan_columns(width: u16) -> usize {
    let inner = width.saturating_sub(2);
    if inner < 90 {
        1
    } else if inner < 120 {
        2
    } else {
        3
    }
}

fn grid_cell_width(width: usize, columns: usize) -> usize {
    width
        .saturating_sub(4 + columns.saturating_sub(1) * 4)
        .checked_div(columns)
        .unwrap_or(18)
        .max(18)
}

fn list_offset(selected: usize, len: usize, viewport: usize) -> usize {
    if len <= viewport || selected < viewport {
        0
    } else {
        (selected + 1)
            .saturating_sub(viewport)
            .min(len.saturating_sub(viewport))
    }
}

fn put(buffer: &mut Buffer, x: u16, y: u16, value: &str, style: Style) {
    if y < buffer.area.height {
        buffer.set_stringn(
            x,
            y,
            value,
            buffer.area.width.saturating_sub(x) as usize,
            style,
        );
    }
}

fn fit(value: &str, width: usize) -> String {
    let len = text_width(value);
    if len <= width {
        return format!("{value}{}", " ".repeat(width - len));
    }
    if width <= 3 {
        return ".".repeat(width);
    }
    let mut result = String::new();
    let mut used = 0;
    for character in value.chars() {
        let char_width = character.width().unwrap_or(0);
        if used + char_width > width - 3 {
            break;
        }
        result.push(character);
        used += char_width;
    }
    result.push_str("...");
    result
}

fn text_width(value: &str) -> usize {
    UnicodeWidthStr::width(value)
}

fn capitalize(value: &str) -> String {
    let mut chars = value.chars();
    chars
        .next()
        .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
        .unwrap_or_default()
}

fn serialize_buffer(buffer: &Buffer) -> Vec<RenderedCell> {
    buffer
        .content()
        .iter()
        .enumerate()
        .filter_map(|(index, cell)| serialize_cell(index, buffer.area.width, cell))
        .collect()
}

fn serialize_cell(index: usize, width: u16, cell: &Cell) -> Option<RenderedCell> {
    let has_symbol = cell.symbol() != " ";
    let has_style = !matches!(cell.fg, Color::Reset | FG)
        || !matches!(cell.bg, Color::Reset | BG)
        || !cell.modifier.is_empty();
    if !has_symbol && !has_style {
        return None;
    }
    Some(RenderedCell {
        x: index as u16 % width,
        y: index as u16 / width,
        symbol: cell.symbol().into(),
        fg: if cell.fg == FG {
            None
        } else {
            color_css(cell.fg)
        },
        bg: if cell.bg == BG {
            None
        } else {
            color_css(cell.bg)
        },
        bold: cell.modifier.contains(Modifier::BOLD),
        dim: cell.modifier.contains(Modifier::DIM),
        italic: cell.modifier.contains(Modifier::ITALIC),
        underlined: cell.modifier.contains(Modifier::UNDERLINED),
        reversed: cell.modifier.contains(Modifier::REVERSED),
    })
}

fn color_css(color: Color) -> Option<String> {
    match color {
        Color::Reset => None,
        Color::Rgb(red, green, blue) => Some(format!("#{red:02x}{green:02x}{blue:02x}")),
        _ => None,
    }
}

thread_local! {
    static STATE: RefCell<DemoState> = RefCell::new(DemoState::default());
    static OUTPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

#[unsafe(no_mangle)]
pub extern "C" fn demo_alloc(length: usize) -> *mut u8 {
    let mut bytes = Vec::<u8>::with_capacity(length);
    let pointer = bytes.as_mut_ptr();
    std::mem::forget(bytes);
    pointer
}

#[unsafe(no_mangle)]
/// Initializes the demo from the JSON seed copied into WASM memory.
///
/// # Safety
///
/// `pointer` must come from [`demo_alloc`] and refer to exactly `length`
/// initialized bytes. This function takes ownership of that allocation.
pub unsafe extern "C" fn demo_init_seed(
    pointer: *mut u8,
    length: usize,
    width: u16,
    height: u16,
) -> u32 {
    let bytes = unsafe { Vec::from_raw_parts(pointer, length, length) };
    let Ok(seed) = serde_json::from_slice::<Seed>(&bytes) else {
        return 0;
    };
    let saved_selection = seed.items.iter().map(|item| item.selected).collect();
    STATE.with(|state| {
        let next = DemoState {
            seed,
            saved_selection,
            width,
            height,
            ..DemoState::default()
        };
        *state.borrow_mut() = next;
    });
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn demo_reset() {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let mut seed = state.seed.clone();
        let width = state.width;
        let height = state.height;
        let history_count = state.history_count;
        let replay_count = state.replay_count;
        let saved_selection = state.saved_selection.clone();
        for (item, selected) in seed.items.iter_mut().zip(&saved_selection) {
            item.selected = *selected;
        }
        *state = DemoState {
            seed,
            saved_selection,
            width,
            height,
            history_count,
            replay_count,
            replay_expanded: false,
            ..DemoState::default()
        };
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn demo_configure(_: u32, _: u32, history_count: u32, replay_count: u32) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.history_count = history_count.max(1);
        state.replay_count = replay_count.max(1);
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn demo_input(key: u32) {
    STATE.with(|state| state.borrow_mut().input(key));
}

#[unsafe(no_mangle)]
pub extern "C" fn demo_pointer(column: u16, row: u16, clicks: u32) {
    STATE.with(|state| state.borrow_mut().pointer(column, row, clicks));
}

#[unsafe(no_mangle)]
pub extern "C" fn demo_tick() {
    STATE.with(|state| state.borrow_mut().tick());
}

#[unsafe(no_mangle)]
pub extern "C" fn demo_render() {
    let bytes =
        STATE.with(|state| serde_json::to_vec(&state.borrow_mut().render()).unwrap_or_default());
    OUTPUT.with(|output| *output.borrow_mut() = bytes);
}

#[unsafe(no_mangle)]
pub extern "C" fn demo_output_ptr() -> *const u8 {
    OUTPUT.with(|output| output.borrow().as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn demo_output_len() -> usize {
    OUTPUT.with(|output| output.borrow().len())
}

#[unsafe(no_mangle)]
pub extern "C" fn demo_screen() -> u32 {
    STATE.with(|state| state.borrow().screen)
}

#[unsafe(no_mangle)]
pub extern "C" fn demo_index() -> u32 {
    STATE.with(|state| state.borrow().index)
}

#[unsafe(no_mangle)]
pub extern "C" fn demo_parent_index() -> u32 {
    STATE.with(|state| state.borrow().parent_index)
}

#[unsafe(no_mangle)]
pub extern "C" fn demo_run_step() -> u32 {
    STATE.with(|state| state.borrow().run_step)
}

#[unsafe(no_mangle)]
pub extern "C" fn demo_exit_pending() -> u32 {
    STATE.with(|state| u32::from(state.borrow().exit_pending))
}

#[unsafe(no_mangle)]
pub extern "C" fn demo_replay_expanded() -> u32 {
    STATE.with(|state| u32::from(state.borrow().replay_expanded))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> DemoState {
        DemoState {
            seed: Seed {
                items: vec![
                    SeedItem {
                        name: "ghostty".into(),
                        layer: "terminal".into(),
                        action_count: 2,
                        selected: true,
                    },
                    SeedItem {
                        name: "fish".into(),
                        layer: "shell".into(),
                        action_count: 1,
                        selected: true,
                    },
                ],
                review_entries: vec![ReviewEntry {
                    item_index: 0,
                    order: 0,
                    kind: "install".into(),
                    kind_icon: "".into(),
                    severity: "run".into(),
                    status: "missing".into(),
                    detail: "install ghostty".into(),
                }],
            },
            saved_selection: vec![true, true],
            ..DemoState::default()
        }
    }

    #[test]
    fn plan_selection_drives_review_counts() {
        let mut state = state();
        state.open_plan(0);
        state.move_plan(true, 1);
        state.toggle_current();
        assert_eq!(state.selected_counts(), (1, 1));
        assert!(state.dirty);
        state.input(KEY_RUN);
        assert_eq!(state.screen, SCREEN_REVIEW);
        let frame = state.render();
        assert_eq!(frame.screen, "review");
    }

    #[test]
    fn discard_restores_the_last_saved_selection() {
        let mut state = state();
        state.open_plan(0);
        state.move_plan(true, 1);
        state.toggle_current();
        assert!(!state.seed.items[0].selected);

        state.input(KEY_BACK);
        assert!(state.exit_pending);
        state.input(KEY_DISCARD);

        assert_eq!(state.screen, SCREEN_MAIN);
        assert!(state.seed.items[0].selected);
        assert!(!state.dirty);
    }

    #[test]
    fn discard_restores_a_selection_saved_during_the_session() {
        let mut state = state();
        state.open_plan(0);
        state.move_plan(true, 1);
        state.toggle_current();
        state.input(KEY_SAVE);
        assert!(!state.seed.items[0].selected);

        state.toggle_current();
        state.input(KEY_BACK);
        state.input(KEY_DISCARD);

        assert!(!state.seed.items[0].selected);
        assert!(!state.dirty);
    }

    #[test]
    fn core_flow_reaches_result() {
        let mut state = state();
        state.input(KEY_ENTER);
        state.input(KEY_RUN);
        state.input(KEY_RUN);
        for _ in 0..6 {
            state.tick();
        }
        assert_eq!(state.screen, SCREEN_RESULT);
        assert_eq!(state.render().screen, "result");
    }

    #[test]
    fn abort_waits_for_a_tick_and_reaches_an_aborted_result() {
        let mut state = state();
        state.screen = SCREEN_RUN;
        state.run_step = 2;

        state.input(KEY_BACK);
        assert_eq!(state.screen, SCREEN_RUN);
        assert!(state.run_aborting);
        assert!(!state.run_aborted);

        state.tick();
        assert_eq!(state.screen, SCREEN_RESULT);
        assert!(!state.run_aborting);
        assert!(state.run_aborted);
        assert_eq!(state.run_step, 2);
    }

    #[test]
    fn replay_selection_can_expand_and_move() {
        let mut state = state();
        state.screen = SCREEN_REPLAY;
        state.replay_count = 3;
        state.input(KEY_SPACE);
        assert!(state.replay_expanded);
        state.input(KEY_DOWN);
        assert_eq!(state.index, 1);
        assert!(!state.replay_expanded);
    }

    #[test]
    fn run_log_filter_and_fold_are_interactive() {
        let mut state = state();
        state.screen = SCREEN_RUN;
        state.input(KEY_TAB);
        state.input(KEY_ENTER);
        assert_eq!(state.run_filter, 1);
        assert!(state.run_folded);
        assert_eq!(state.render().screen, "run");
    }

    #[test]
    fn pointer_selects_and_activates_tui_rows() {
        let mut state = state();
        state.pointer(5, 9, 1);
        assert_eq!(state.index, 2);
        state.pointer(5, 9, 2);
        assert_eq!(state.screen, SCREEN_HISTORY);

        state.screen = SCREEN_PLAN;
        state.plan_row = 0;
        state.pointer(5, 3, 1);
        assert!(!state.seed.items[0].selected);
        assert!(state.dirty);

        state.screen = SCREEN_REPLAY;
        state.replay_count = 3;
        state.pointer(5, 5, 2);
        assert_eq!(state.index, 2);
        assert!(state.replay_expanded);

        state.screen = SCREEN_RUN;
        state.pointer(20, state.height - 11, 1);
        state.pointer(20, state.height - 10, 1);
        assert_eq!(state.run_filter, 1);
        assert!(state.run_folded);
    }
}
