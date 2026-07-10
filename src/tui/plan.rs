use super::*;

#[derive(Debug, Clone)]
pub(super) enum PlanRow {
    Header {
        layer: String,
        ordinal: usize,
        enabled: usize,
        total: usize,
    },
    Item(usize),
    InlineItems(Vec<usize>),
    Divider,
}

pub(super) const GRID_COLUMNS: usize = 3;

// ---------------- PlanView ----------------

pub(super) fn handle_plan(app: &mut App, key: KeyCode) -> Result<()> {
    let rows = app
        .plan
        .as_ref()
        .map(|plan| build_plan_rows(plan, &app.collapsed_layers, app.plan_columns))
        .unwrap_or_default();
    match key {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.screen = Screen::MainMenu;
        }
        KeyCode::Char('s') => {
            if let Err(e) = app::save_current_selection(app) {
                app.status_message = e;
                app.status_is_focus_info = false;
            }
        }
        KeyCode::Char('j') | KeyCode::Down => {
            select_next_plan_row(app, &rows);
            update_plan_focus_info(app);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            select_prev_plan_row(app, &rows);
            update_plan_focus_info(app);
        }
        KeyCode::Char('h') | KeyCode::Left => {
            move_grid_col(app, &rows, -1);
            update_plan_focus_info(app);
        }
        KeyCode::Char('l') | KeyCode::Right => {
            move_grid_col(app, &rows, 1);
            update_plan_focus_info(app);
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            if let Some(plan) = &mut app.plan
                && let Some(row_idx) = app.plan_state.selected()
            {
                match rows.get(row_idx) {
                    Some(PlanRow::Header { layer, .. }) => {
                        toggle_layer(&mut app.collapsed_layers, layer);
                        keep_selection_in_range(app);
                    }
                    Some(PlanRow::Item(item_idx)) => {
                        if let Some(item) = plan.items.get_mut(*item_idx) {
                            item.selected = !item.selected;
                            app.dirty = true;
                        }
                    }
                    Some(PlanRow::InlineItems(item_indices)) => {
                        let col = app.grid_col.min(item_indices.len().saturating_sub(1));
                        if let Some(item_idx) = item_indices.get(col)
                            && let Some(item) = plan.items.get_mut(*item_idx)
                        {
                            item.selected = !item.selected;
                            app.dirty = true;
                        }
                    }
                    _ => {}
                }
            }
            update_plan_focus_info(app);
        }
        KeyCode::Char('1') => {
            toggle_layer_by_number(app, 1);
            update_plan_focus_info(app);
        }
        KeyCode::Char('2') => {
            toggle_layer_by_number(app, 2);
            update_plan_focus_info(app);
        }
        KeyCode::Char('3') => {
            toggle_layer_by_number(app, 3);
            update_plan_focus_info(app);
        }
        KeyCode::Char('4') => {
            toggle_layer_by_number(app, 4);
            update_plan_focus_info(app);
        }
        KeyCode::Char('5') => {
            toggle_layer_by_number(app, 5);
            update_plan_focus_info(app);
        }
        KeyCode::Char('6') => {
            toggle_layer_by_number(app, 6);
            update_plan_focus_info(app);
        }
        KeyCode::Char('a') => {
            if let Some(plan) = &mut app.plan {
                for item in plan.items.iter_mut() {
                    item.selected = true;
                }
                app.dirty = true;
            }
        }
        KeyCode::Char('n') => {
            if let Some(plan) = &mut app.plan {
                for item in plan.items.iter_mut() {
                    item.selected = false;
                }
                app.dirty = true;
            }
        }
        KeyCode::Char('r') => {
            if matches!(app.mode, Mode::Plan) {
                app.status_message = "plan mode is read-only; choose deploy to run".into();
                app.status_is_focus_info = false;
            } else if review::selected_item_count(app.plan.as_ref()) == 0 {
                app.status_message = "nothing selected".into();
                app.status_is_focus_info = false;
            } else {
                app.review_entries = if let Some(plan) = app.plan.as_ref() {
                    review::review_entries(plan, app.config.as_ref())
                } else {
                    Vec::new()
                };
                app.review_scroll = 0;
                app.screen = Screen::ConfirmView;
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn render_plan(f: &mut Frame, app: &mut App) {
    let icon_set = icons::current();
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(3),
        ])
        .split(area);

    app.plan_columns = plan_grid_columns(usize::from(chunks[1].width));
    app.grid_col = clamped_grid_col_for_selection(app);

    let plan = match &app.plan {
        Some(p) => p,
        None => {
            let msg = Paragraph::new("no plan loaded").alignment(Alignment::Center);
            f.render_widget(msg, chunks[0]);
            return;
        }
    };

    let selected = review::selected_item_count(Some(plan));
    let actions = review::selected_action_count(Some(plan));
    let state = if app.dirty { "unsaved" } else { "saved" };
    let status_prefix = format!(
        "{}  dotman - Plan (○ {state})  {selected} selected · {actions} actions ",
        icon_set.app
    );
    let divider_width = usize::from(chunks[0].width).saturating_sub(display_width(&status_prefix));
    let status = Paragraph::new(Line::from(vec![
        Span::styled(
            format!("{}  dotman - Plan (", icon_set.app),
            Style::default()
                .fg(CATPPUCCIN_MOCHA.fg_dim)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            if app.dirty {
                "○ unsaved"
            } else {
                "○ saved"
            },
            Style::default().fg(if app.dirty {
                CATPPUCCIN_MOCHA.warning
            } else {
                CATPPUCCIN_MOCHA.text_muted
            }),
        ),
        Span::styled(")  ", Style::default().fg(CATPPUCCIN_MOCHA.fg_dim)),
        Span::styled(
            format!("{selected} selected · {actions} actions"),
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        ),
        Span::styled(
            format!(" {}", "─".repeat(divider_width)),
            Style::default().fg(CATPPUCCIN_MOCHA.border_subtle),
        ),
    ]));
    f.render_widget(status, chunks[0]);

    let rows = build_plan_rows(plan, &app.collapsed_layers, app.plan_columns);
    let mut items: Vec<ListItem> = Vec::new();
    let row_width = usize::from(chunks[1].width);
    let cell_width = grid_cell_width(row_width, app.plan_columns);
    for (row_index, row) in rows.iter().enumerate() {
        let selected_row = app
            .plan_state
            .selected()
            .is_some_and(|selected| selected == row_index);
        match row {
            PlanRow::Header {
                layer,
                ordinal,
                enabled,
                total,
            } => {
                items.push(plan_header_line(
                    layer,
                    *ordinal,
                    *enabled,
                    *total,
                    app.collapsed_layers.contains(layer),
                    selected_row,
                    row_width,
                ));
            }
            PlanRow::Item(item_idx) => {
                let it = &plan.items[*item_idx];
                if selected_row {
                    items.push(selected_item_line(it, row_width));
                } else {
                    items.push(plan_item_line(it, row_width));
                }
            }
            PlanRow::InlineItems(item_indices) => {
                let mut spans = vec![Span::raw("  ")];
                for (i, item_idx) in item_indices.iter().enumerate() {
                    let it = &plan.items[*item_idx];
                    let selected_cell = selected_row && app.grid_col == i;
                    spans.extend(grid_cell_spans(it, cell_width, selected_cell));
                    if i + 1 < item_indices.len() {
                        spans.push(Span::raw("    "));
                    }
                }
                items.push(ListItem::new(Line::from(spans)));
            }
            PlanRow::Divider => {
                items.push(ListItem::new(Line::from(Span::styled(
                    format!("  {}", "─".repeat(row_width.saturating_sub(2))),
                    divider_style(),
                ))));
            }
        }
    }

    let list = List::new(items)
        .highlight_style(Style::default())
        .highlight_symbol("");
    f.render_stateful_widget(list, chunks[1], &mut app.plan_state);

    let status_line = if app.status_message.is_empty() {
        Line::from("")
    } else {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                &app.status_message,
                Style::default().fg(CATPPUCCIN_MOCHA.text_muted),
            ),
        ])
    };
    let help = Paragraph::new(vec![
        status_line,
        plan_help_line(usize::from(chunks[2].width), matches!(app.mode, Mode::Plan)),
    ]);
    f.render_widget(help, chunks[2]);
}

pub(super) fn build_plan_rows(
    plan: &Plan,
    collapsed_layers: &BTreeSet<String>,
    grid_columns: usize,
) -> Vec<PlanRow> {
    let layers = [
        "terminal",
        "shell",
        "multiplexer",
        "software",
        "enhancement",
        "misc",
    ];
    let mut rows = Vec::new();
    for (i, layer) in layers.iter().enumerate() {
        let layer_items: Vec<usize> = plan
            .items
            .iter()
            .enumerate()
            .filter_map(|(idx, it)| (it.layer == *layer).then_some(idx))
            .collect();
        if layer_items.is_empty() {
            continue;
        }
        let enabled = layer_items
            .iter()
            .filter(|idx| plan.items[**idx].selected)
            .count();
        rows.push(PlanRow::Header {
            layer: (*layer).to_string(),
            ordinal: i + 1,
            enabled,
            total: layer_items.len(),
        });
        if !collapsed_layers.contains(*layer) {
            if i < 3 || grid_columns == 1 {
                rows.extend(layer_items.into_iter().map(PlanRow::Item));
            } else {
                for chunk in layer_items.chunks(grid_columns) {
                    rows.push(PlanRow::InlineItems(chunk.to_vec()));
                }
            }
        }
        if i + 1 < layers.len() {
            rows.push(PlanRow::Divider);
        }
    }
    rows
}

pub(super) fn plan_header_line(
    layer: &str,
    ordinal: usize,
    enabled: usize,
    total: usize,
    collapsed: bool,
    focused: bool,
    width: usize,
) -> ListItem<'static> {
    let icon_set = icons::current();
    let icon = if collapsed {
        icon_set.collapsed
    } else {
        icon_set.expanded
    };
    let left = format!("{} {:02}  {}", icon, ordinal, capitalize(layer));
    let right = format!("{enabled} / {total}");
    let content_width = width.saturating_sub(2);
    let right_width = display_width(&right);
    let left_width = content_width.saturating_sub(right_width + 1);
    let gap = content_width
        .saturating_sub(left_width + right_width)
        .max(1);

    if focused {
        let bg = focus_bg();
        ListItem::new(Line::from(vec![
            Span::styled(
                "▎",
                Style::default().fg(CATPPUCCIN_MOCHA.focus_marker).bg(bg),
            ),
            Span::styled(" ", Style::default().bg(bg)),
            Span::styled(
                fit_to_width(&left, left_width),
                Style::default().bg(bg).fg(CATPPUCCIN_MOCHA.text_muted),
            ),
            Span::styled(" ".repeat(gap), Style::default().bg(bg)),
            Span::styled(
                right,
                Style::default().bg(bg).fg(CATPPUCCIN_MOCHA.text_muted),
            ),
        ]))
    } else {
        ListItem::new(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                fit_to_width(&left, left_width),
                Style::default().fg(CATPPUCCIN_MOCHA.text_muted),
            ),
            Span::raw(" ".repeat(gap)),
            Span::styled(right, Style::default().fg(CATPPUCCIN_MOCHA.text_muted)),
        ]))
    }
}

pub(super) fn plan_item_line(item: &PlanItem, width: usize) -> ListItem<'static> {
    let label = item_label(item);
    let prefix_width = 6;
    let available = width.saturating_sub(prefix_width);

    ListItem::new(Line::from(vec![
        Span::raw("  "),
        Span::raw("  "),
        checkbox_span(item.selected, false),
        Span::raw("  "),
        Span::styled(
            fit_to_width(&label, available),
            Style::default().fg(CATPPUCCIN_MOCHA.fg),
        ),
    ]))
}

pub(super) fn select_first_plan_row(
    list_state: &mut ListState,
    plan: Option<&Plan>,
    collapsed_layers: &BTreeSet<String>,
    grid_columns: usize,
) {
    let Some(plan) = plan else {
        select_plan_row(list_state, 0, true);
        return;
    };
    let rows = build_plan_rows(plan, collapsed_layers, grid_columns);
    let first = rows.iter().position(is_selectable_plan_row).unwrap_or(0);
    select_plan_row(list_state, first, true);
}

pub(super) fn select_next_plan_row(app: &mut App, rows: &[PlanRow]) {
    if rows.is_empty() {
        select_plan_row(&mut app.plan_state, 0, true);
        return;
    }
    let current = app.plan_state.selected().unwrap_or(0);
    let start = app.plan_state.selected().unwrap_or(0).saturating_add(1);
    let next = (start..rows.len())
        .find(|idx| is_selectable_plan_row(&rows[*idx]))
        .or_else(|| rows.iter().position(is_selectable_plan_row))
        .unwrap_or(0);
    clamp_grid_col(app, rows.get(next));
    select_plan_row(&mut app.plan_state, next, next < current);
}

pub(super) fn select_prev_plan_row(app: &mut App, rows: &[PlanRow]) {
    if rows.is_empty() {
        select_plan_row(&mut app.plan_state, 0, true);
        return;
    }
    let current = app
        .plan_state
        .selected()
        .unwrap_or(rows.len())
        .min(rows.len());
    let start = app
        .plan_state
        .selected()
        .unwrap_or(rows.len())
        .min(rows.len());
    let prev = (0..start)
        .rev()
        .find(|idx| is_selectable_plan_row(&rows[*idx]))
        .or_else(|| rows.iter().rposition(is_selectable_plan_row))
        .unwrap_or(0);
    clamp_grid_col(app, rows.get(prev));
    select_plan_row(&mut app.plan_state, prev, prev > current);
}

pub(super) fn select_plan_row(list_state: &mut ListState, idx: usize, reset_offset: bool) {
    list_state.select(Some(idx));
    if reset_offset {
        *list_state.offset_mut() = 0;
    }
}

pub(super) fn is_selectable_plan_row(row: &PlanRow) -> bool {
    matches!(
        row,
        PlanRow::Header { .. } | PlanRow::Item(_) | PlanRow::InlineItems(_)
    )
}

pub(super) fn move_grid_col(app: &mut App, rows: &[PlanRow], delta: isize) {
    let Some(row_idx) = app.plan_state.selected() else {
        return;
    };
    let Some(PlanRow::InlineItems(item_indices)) = rows.get(row_idx) else {
        return;
    };
    let max_col = item_indices.len().saturating_sub(1);
    let next = if delta.is_negative() {
        app.grid_col.saturating_sub(delta.unsigned_abs())
    } else {
        app.grid_col.saturating_add(delta as usize)
    };
    app.grid_col = next.min(max_col);
}

pub(super) fn clamp_grid_col(app: &mut App, row: Option<&PlanRow>) {
    if let Some(PlanRow::InlineItems(item_indices)) = row {
        app.grid_col = app.grid_col.min(item_indices.len().saturating_sub(1));
    } else {
        app.grid_col = 0;
    }
}

pub(super) fn clamped_grid_col_for_selection(app: &App) -> usize {
    let Some(plan) = &app.plan else {
        return 0;
    };
    let rows = build_plan_rows(plan, &app.collapsed_layers, app.plan_columns);
    let Some(row_idx) = app.plan_state.selected() else {
        return 0;
    };
    match rows.get(row_idx) {
        Some(PlanRow::InlineItems(item_indices)) => {
            app.grid_col.min(item_indices.len().saturating_sub(1))
        }
        _ => 0,
    }
}

pub(super) fn toggle_layer(collapsed_layers: &mut BTreeSet<String>, layer: &str) {
    if !collapsed_layers.remove(layer) {
        collapsed_layers.insert(layer.to_string());
    }
}

pub(super) fn toggle_layer_by_number(app: &mut App, number: usize) {
    if let Some(layer) = layer_by_number(number) {
        toggle_layer(&mut app.collapsed_layers, layer);
        keep_selection_in_range(app);
    }
}

pub(super) fn layer_by_number(number: usize) -> Option<&'static str> {
    match number {
        1 => Some("terminal"),
        2 => Some("shell"),
        3 => Some("multiplexer"),
        4 => Some("software"),
        5 => Some("enhancement"),
        6 => Some("misc"),
        _ => None,
    }
}

pub(super) fn keep_selection_in_range(app: &mut App) {
    let rows = app
        .plan
        .as_ref()
        .map(|plan| build_plan_rows(plan, &app.collapsed_layers, app.plan_columns))
        .unwrap_or_default();
    let selected = app.plan_state.selected().unwrap_or(0);
    if selected >= rows.len() || !rows.get(selected).is_some_and(is_selectable_plan_row) {
        let first = rows.iter().position(is_selectable_plan_row).unwrap_or(0);
        clamp_grid_col(app, rows.get(first));
        select_plan_row(&mut app.plan_state, first, true);
    } else {
        clamp_grid_col(app, rows.get(selected));
    }
}

pub(super) fn update_plan_focus_info(app: &mut App) {
    if let Some(info) = focused_plan_item_info(app) {
        app.status_message = info;
        app.status_is_focus_info = true;
    } else {
        clear_focus_info(app);
    }
}

pub(super) fn focused_plan_item_info(app: &App) -> Option<String> {
    let plan = app.plan.as_ref()?;
    let row_idx = app.plan_state.selected()?;
    let rows = build_plan_rows(plan, &app.collapsed_layers, app.plan_columns);
    match rows.get(row_idx)? {
        PlanRow::Item(item_idx) => plan.items.get(*item_idx).map(plan_item_info),
        PlanRow::InlineItems(item_indices) => {
            let col = app.grid_col.min(item_indices.len().saturating_sub(1));
            item_indices
                .get(col)
                .and_then(|item_idx| plan.items.get(*item_idx))
                .map(plan_item_info)
        }
        PlanRow::Header { .. } | PlanRow::Divider => None,
    }
}

pub(super) fn clear_focus_info(app: &mut App) {
    if app.status_is_focus_info {
        app.status_message.clear();
        app.status_is_focus_info = false;
    }
}

pub(super) fn plan_item_info(item: &PlanItem) -> String {
    let actions = item
        .actions
        .iter()
        .map(Action::describe)
        .collect::<Vec<_>>()
        .join(" · ");
    if actions.is_empty() {
        item.name.clone()
    } else {
        format!("{}: {actions}", item.name)
    }
}

pub(super) fn selected_item_line(item: &PlanItem, width: usize) -> ListItem<'static> {
    let bg = focus_bg();
    let label = item_label(item);
    let fixed_width = 7;
    let label_width = width.saturating_sub(fixed_width);
    ListItem::new(Line::from(vec![
        Span::styled("  ", Style::default().bg(bg)),
        Span::styled(
            "▎",
            Style::default().fg(CATPPUCCIN_MOCHA.focus_marker).bg(bg),
        ),
        Span::styled(" ", Style::default().bg(bg)),
        checkbox_span(item.selected, true),
        Span::styled("  ", Style::default().bg(bg)),
        Span::styled(
            fit_to_width(&label, label_width),
            Style::default().bg(bg).fg(CATPPUCCIN_MOCHA.fg),
        ),
    ]))
}

pub(super) fn plan_grid_columns(width: usize) -> usize {
    if width < 90 {
        1
    } else if width < 120 {
        2
    } else {
        GRID_COLUMNS
    }
}

pub(super) fn grid_cell_width(row_width: usize, columns: usize) -> usize {
    let indent = 4;
    let gaps = columns.saturating_sub(1) * 4;
    row_width
        .saturating_sub(indent + gaps)
        .checked_div(columns)
        .unwrap_or(18)
        .max(18)
}

pub(super) fn grid_cell_spans(item: &PlanItem, width: usize, focused: bool) -> Vec<Span<'static>> {
    let bg = focused.then(focus_bg);
    let label = item_label(item);
    let prefix_width = 2;
    let fixed_width = prefix_width + 3;
    let label_width = width.saturating_sub(fixed_width);
    let prefix = if focused { "▎ " } else { "  " };
    let mut prefix_style = Style::default();
    if focused {
        prefix_style = prefix_style
            .fg(CATPPUCCIN_MOCHA.focus_marker)
            .bg(focus_bg());
    }
    vec![
        Span::styled(prefix, prefix_style),
        checkbox_span(item.selected, focused),
        Span::styled("  ", span_bg_style(bg)),
        Span::styled(
            fit_to_width(&label, label_width),
            span_bg_style(bg)
                .fg(CATPPUCCIN_MOCHA.fg)
                .add_modifier(Modifier::empty()),
        ),
    ]
}

pub(super) fn checkbox_span(selected: bool, highlighted: bool) -> Span<'static> {
    let icon_set = icons::current();
    Span::styled(
        if selected {
            icon_set.selected
        } else {
            icon_set.unselected
        },
        span_bg_style(highlighted.then(focus_bg)).fg(if selected {
            CATPPUCCIN_MOCHA.success
        } else {
            CATPPUCCIN_MOCHA.fg_dim
        }),
    )
}

pub(super) fn item_label(item: &PlanItem) -> String {
    if item.actions.len() > 1 {
        format!("{} (+{})", item.name, item.actions.len() - 1)
    } else {
        item.name.clone()
    }
}
