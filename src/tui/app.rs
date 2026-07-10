use super::*;

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    MainMenu,
    PlanView,
    ConfirmView,
    RunView,
    HistoryView,
    RunReplay,
}

pub struct App {
    pub screen: Screen,
    pub mode: Mode,
    pub config: Option<config::Config>,
    pub plan: Option<Plan>,
    pub run: Option<Run>,
    pub runs: Vec<Run>,
    pub(super) review_entries: Vec<review::ReviewEntry>,
    pub(super) review_scroll: usize,
    pub(super) review_danger_confirmed: bool,
    pub(super) review_thread: Option<std::thread::JoinHandle<Vec<review::ReviewEntry>>>,
    pub menu_state: ListState,
    pub plan_state: ListState,
    pub history_state: ListState,
    pub replay_state: ListState,
    pub replay_scroll: usize,
    pub replay_follow_selection: bool,
    pub replay_expanded: BTreeSet<String>,
    pub grid_col: usize,
    pub plan_columns: usize,
    pub plan_exit_pending: bool,
    pub collapsed_layers: BTreeSet<String>,
    pub status_message: String,
    pub status_kind: NoticeKind,
    pub status_is_focus_info: bool,
    pub should_quit: bool,
    pub vim_pending_g: bool,
    pub dirty: bool,
    // For RunView
    pub spinner_frame: usize,
    pub run_thread: Option<std::thread::JoinHandle<RunThreadResult>>,
    pub run_events: Option<mpsc::Receiver<crate::execute::ExecuteEvent>>,
    pub abort_flag: Option<Arc<AtomicBool>>,
    pub progress: (usize, usize), // (done, total)
    pub current_log: Vec<LogLine>,
    pub log_scroll: usize,
    pub log_viewport_height: usize,
    pub log_follow: bool,
    pub log_dropped_count: usize,
    pub log_group: Option<String>,
    pub active_log_group: Option<String>,
    pub log_filter: LogFilter,
    pub collapsed_log_groups: BTreeSet<String>,
    pub run_error: Option<String>,
    pub run_save_warning: Option<String>,
    pub current_item: Option<usize>,
    pub last_item_index: Option<usize>,
    pub current_action: Option<(usize, usize)>,
    pub run_item_statuses: Vec<Option<ActionStatus>>,
    pub run_action_statuses: Vec<Vec<Option<ActionStatus>>>,
    pub run_started: Option<Instant>,
    /// Set to true after `sudo -v` restores the terminal; signals the event
    /// loop to recreate the Terminal backend on the next tick.
    pub needs_terminal_reset: bool,
}

/// A single log line with optional color.
#[derive(Debug, Clone)]
pub struct LogLine {
    pub text: String,
    pub fg: Option<Color>,
    pub indent: usize,
    pub group: Option<String>,
    pub kind: LogKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogKind {
    Header,
    Stdout,
    Stderr,
    Action,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFilter {
    All,
    Current,
    Errors,
}

#[derive(Debug)]
pub struct RunThreadResult {
    pub run: Option<Run>,
    pub error: Option<String>,
    pub save_warning: Option<String>,
}

impl LogFilter {
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Current,
            Self::Current => Self::Errors,
            Self::Errors => Self::All,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Current => "current",
            Self::Errors => "errors",
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Self::All => Self::Errors,
            Self::Current => Self::All,
            Self::Errors => Self::Current,
        }
    }
}

impl App {
    pub fn new(mode: Mode) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        let mut plan_state = ListState::default();
        plan_state.select(Some(0));
        let mut history_state = ListState::default();
        history_state.select(None);
        Self {
            screen: Screen::MainMenu,
            mode,
            config: None,
            plan: None,
            run: None,
            runs: Vec::new(),
            review_entries: Vec::new(),
            review_scroll: 0,
            review_danger_confirmed: false,
            review_thread: None,
            menu_state: list_state,
            plan_state,
            history_state,
            replay_state: ListState::default(),
            replay_scroll: 0,
            replay_follow_selection: false,
            replay_expanded: BTreeSet::new(),
            grid_col: 0,
            plan_columns: plan::GRID_COLUMNS,
            plan_exit_pending: false,
            collapsed_layers: BTreeSet::new(),
            status_message: String::new(),
            status_kind: NoticeKind::Info,
            status_is_focus_info: false,
            should_quit: false,
            vim_pending_g: false,
            dirty: false,
            spinner_frame: 0,
            run_thread: None,
            run_events: None,
            abort_flag: None,
            progress: (0, 0),
            current_log: Vec::new(),
            log_scroll: 0,
            log_viewport_height: 8,
            log_follow: true,
            log_dropped_count: 0,
            log_group: None,
            active_log_group: None,
            log_filter: LogFilter::All,
            collapsed_log_groups: BTreeSet::new(),
            run_error: None,
            run_save_warning: None,
            current_item: None,
            last_item_index: None,
            current_action: None,
            run_item_statuses: Vec::new(),
            run_action_statuses: Vec::new(),
            run_started: None,
            needs_terminal_reset: false,
        }
    }

    pub fn load_config(&mut self) -> Result<(), String> {
        let path = if std::path::Path::new("dotman.yaml").exists() {
            std::path::PathBuf::from("dotman.yaml")
        } else if let Ok(Some(p)) = crate::profile::active_config_path() {
            p
        } else {
            return Err("no dotman.yaml found in current directory or active profile".into());
        };
        let cfg = config::load(&path).map_err(|e| e.to_string())?;
        self.config = Some(cfg);
        Ok(())
    }

    pub fn load_config_from(&mut self, config_path: &std::path::Path) -> Result<(), String> {
        let cfg = config::load(config_path).map_err(|e| e.to_string())?;
        self.config = Some(cfg);
        Ok(())
    }

    pub fn build_plan(&mut self) -> Result<(), String> {
        let cfg = self.config.as_ref().ok_or("config not loaded")?;
        let plan_mode = match self.mode {
            Mode::Menu => PlanMode::Deploy,
            Mode::Deploy | Mode::Plan => PlanMode::Deploy,
            _ => PlanMode::Deploy,
        };
        let mut plan = crate::plan::build(cfg, plan_mode).map_err(|e| e.to_string())?;
        if let Err(error) = apply_saved_selection(&mut plan) {
            self.status_message = format!("selection ignored: {error}");
            self.status_kind = NoticeKind::Warning;
        }
        plan.sync_auto_steps();
        self.plan = Some(plan);
        self.review_entries.clear();
        self.review_thread = None;
        self.review_scroll = 0;
        self.review_danger_confirmed = false;
        self.plan_exit_pending = false;
        plan::select_first_plan_row(
            &mut self.plan_state,
            self.plan.as_ref(),
            &self.collapsed_layers,
            self.plan_columns,
        );
        self.dirty = false;
        Ok(())
    }

    pub fn tick(&mut self) {
        self.spinner_frame = (self.spinner_frame + 1) % icons::SPINNER_BRAILLE.len();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeKind {
    Info,
    Success,
    Warning,
    Error,
}

pub(super) fn initialize_screen(app: &mut App) {
    match app.mode.clone() {
        Mode::Menu => {
            load_runs(app);
            history::clamp_menu_selection(app);
        }
        Mode::Deploy | Mode::Plan => {
            if let Err(e) = app.build_plan() {
                app.status_message = e;
                app.status_kind = NoticeKind::Error;
            }
            app.screen = Screen::PlanView;
        }
        Mode::History => {
            load_runs(app);
            history::clamp_history_selection(app);
            app.screen = Screen::HistoryView;
        }
        Mode::Run(id) => match store::load(&id) {
            Ok(run) => {
                app.run = Some(run);
                app.screen = Screen::RunReplay;
            }
            Err(e) => {
                load_runs(app);
                app.status_message = e.to_string();
                app.status_kind = NoticeKind::Error;
                app.screen = Screen::HistoryView;
            }
        },
    }
}

pub(super) fn load_runs(app: &mut App) {
    match store::list_detailed() {
        Ok(report) => {
            app.runs = report.runs;
            apply_history_warnings(app, &report.warnings);
        }
        Err(error) => {
            app.runs.clear();
            app.status_message = format!("failed to read history: {error}");
            app.status_kind = NoticeKind::Error;
            app.status_is_focus_info = false;
        }
    }
}

pub(super) fn apply_history_warnings(app: &mut App, warnings: &[String]) {
    let Some(warning) = warnings.first() else {
        app.status_message.clear();
        app.status_kind = NoticeKind::Info;
        app.status_is_focus_info = false;
        return;
    };
    let suffix = if warnings.len() > 1 {
        format!(" (+{} more)", warnings.len() - 1)
    } else {
        String::new()
    };
    app.status_message = format!("{warning}{suffix}");
    app.status_kind = NoticeKind::Warning;
    app.status_is_focus_info = false;
}

pub(super) fn apply_saved_selection(plan: &mut Plan) -> Result<(), String> {
    let selection = store::load_selection_scoped(&plan.config_path, &plan.config_hash)
        .map_err(|e| e.to_string())?;
    for item in &mut plan.items {
        if let Some(selected) = selection.items.get(&item.id) {
            item.selected = *selected;
        }
    }
    Ok(())
}

pub(super) fn save_current_selection(app: &mut App) -> Result<(), String> {
    let plan = app.plan.as_ref().ok_or("no plan loaded")?;
    let selection = Selection {
        items: plan
            .items
            .iter()
            .map(|item| (item.id.clone(), item.selected))
            .collect(),
    };
    let path = store::save_selection_scoped(&plan.config_path, &plan.config_hash, &selection)
        .map_err(|e| e.to_string())?;
    app.dirty = false;
    app.status_message = format!("saved selection to {}", path.display());
    app.status_kind = NoticeKind::Success;
    app.status_is_focus_info = false;
    Ok(())
}
