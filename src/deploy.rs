use crate::ColorChoice;
use crate::path::expand_home;
use clap::ValueEnum;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::io::{IsTerminal, Read, Write};
use std::os::unix::fs as unix_fs;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Directive {
    Defaults,
    Link,
    Create,
    Shell,
    Clean,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum Step {
    Defaults { defaults: DefaultsStep },
    Link { link: BTreeMap<String, LinkValue> },
    Create { create: CreateValue },
    Shell { shell: Vec<ShellValue> },
    Clean { clean: Vec<String> },
}

#[derive(Clone, Debug, Default, Deserialize)]
struct DefaultsStep {
    #[serde(default)]
    link: LinkOptions,
    #[serde(default)]
    shell: ShellOptions,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct LinkOptions {
    #[serde(default)]
    create: Option<bool>,
    #[serde(default)]
    relink: Option<bool>,
    #[serde(default)]
    backup: Option<bool>,
    #[serde(default)]
    relative: Option<bool>,
}

impl LinkOptions {
    fn has_values(&self) -> bool {
        self.create.is_some()
            || self.relink.is_some()
            || self.backup.is_some()
            || self.relative.is_some()
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum LinkValue {
    Path(String),
    Options(LinkItem),
}

#[derive(Clone, Debug, Deserialize)]
struct LinkItem {
    path: String,
    #[serde(flatten)]
    options: LinkOptions,
    #[serde(rename = "if")]
    condition: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum CreateValue {
    Paths(Vec<String>),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum ShellValue {
    Command(String),
    Options(ShellItem),
}

#[derive(Clone, Debug, Deserialize)]
struct ShellItem {
    command: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    optional: bool,
    #[serde(flatten)]
    options: ShellOptions,
    #[serde(rename = "if")]
    condition: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
struct ShellOptions {
    #[serde(default)]
    stdout: Option<bool>,
    #[serde(default)]
    stderr: Option<bool>,
}

impl ShellOptions {
    fn has_values(&self) -> bool {
        self.stdout.is_some() || self.stderr.is_some()
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct ShellDefaults {
    stdout: bool,
    stderr: bool,
}

impl ShellDefaults {
    fn apply(&mut self, options: &ShellOptions) {
        if let Some(value) = options.stdout {
            self.stdout = value;
        }
        if let Some(value) = options.stderr {
            self.stderr = value;
        }
    }

    fn merged(&self, options: &ShellOptions) -> ShellSettings {
        ShellSettings {
            stdout: options.stdout.unwrap_or(self.stdout),
            stderr: options.stderr.unwrap_or(self.stderr),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ShellSettings {
    stdout: bool,
    stderr: bool,
}

#[derive(Clone, Debug, Default)]
struct LinkDefaults {
    create: bool,
    relink: bool,
    backup: bool,
    relative: bool,
}

impl LinkDefaults {
    fn apply(&mut self, options: &LinkOptions) {
        if let Some(value) = options.create {
            self.create = value;
        }
        if let Some(value) = options.relink {
            self.relink = value;
        }
        if let Some(value) = options.backup {
            self.backup = value;
        }
        if let Some(value) = options.relative {
            self.relative = value;
        }
    }

    fn merged(&self, options: &LinkOptions) -> LinkSettings {
        LinkSettings {
            create: options.create.unwrap_or(self.create),
            relink: options.relink.unwrap_or(self.relink),
            backup: options.backup.unwrap_or(self.backup),
            relative: options.relative.unwrap_or(self.relative),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct LinkSettings {
    create: bool,
    relink: bool,
    backup: bool,
    relative: bool,
}

#[derive(Clone, Debug)]
struct LinkPlan {
    target: PathBuf,
    source: PathBuf,
    link_source: PathBuf,
    settings: LinkSettings,
    action: LinkAction,
}

#[derive(Clone, Debug)]
enum LinkAction {
    Link,
    Relink,
    Backup(PathBuf),
    Skip,
    Fail(String),
}

#[derive(Clone, Copy, Debug)]
pub struct OutputStyle {
    color: bool,
    summary: bool,
}

impl OutputStyle {
    pub fn new(color: ColorChoice, summary: bool) -> Self {
        let color = match color {
            ColorChoice::Always => true,
            ColorChoice::Never => false,
            ColorChoice::Auto => {
                std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal()
            }
        };
        Self { color, summary }
    }

    fn icon(icon: Icon) -> &'static str {
        match icon {
            Icon::Ok => "✓",
            Icon::Action => "→",
            Icon::Create => "+",
            Icon::Relink => "↻",
            Icon::Backup => "⤴",
            Icon::Skip => "-",
            Icon::Warn => "!",
            Icon::Fail => "✗",
            Icon::Running => "…",
        }
    }

    fn spinner_frames() -> &'static [&'static str] {
        &["◐", "◓", "◑", "◒"]
    }

    fn paint(self, text: impl Into<String>, color: Color) -> String {
        let text = text.into();
        if self.color {
            format!("{}{}{}", color.code(), text, "\x1b[0m")
        } else {
            text
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Icon {
    Ok,
    Action,
    Create,
    Relink,
    Backup,
    Skip,
    Warn,
    Fail,
    Running,
}

#[derive(Clone, Copy, Debug)]
enum Color {
    Blue,
    Cyan,
    Green,
    Yellow,
    Red,
    Magenta,
    Dim,
}

impl Color {
    fn code(self) -> &'static str {
        match self {
            Color::Blue => "\x1b[38;2;137;180;250m",
            Color::Cyan => "\x1b[38;2;148;226;213m",
            Color::Green => "\x1b[38;2;166;227;161m",
            Color::Yellow => "\x1b[38;2;249;226;175m",
            Color::Red => "\x1b[38;2;243;139;168m",
            Color::Magenta => "\x1b[38;2;245;194;231m",
            Color::Dim => "\x1b[2m",
        }
    }
}

struct Reporter {
    repo: PathBuf,
    dry_run: bool,
    style: OutputStyle,
    directives: Vec<(Directive, usize, usize)>,
}

impl Reporter {
    fn new(command_name: &str, repo: &Path, dry_run: bool, style: OutputStyle) -> Self {
        if !style.summary {
            let mode = if dry_run { " --dry-run" } else { "" };
            println!(
                "{} {}",
                style.paint(command_name, Color::Cyan),
                style.paint(mode, Color::Dim)
            );
        }
        Self {
            repo: repo.to_path_buf(),
            dry_run,
            style,
            directives: Vec::new(),
        }
    }

    fn section(&self, title: &str) {
        if !self.style.summary {
            println!("{}", self.style.paint(title, Color::Blue));
        }
    }

    fn row(&self, icon: Icon, icon_color: Color, left: impl AsRef<str>, detail: impl AsRef<str>) {
        if self.style.summary {
            return;
        }
        let left = left.as_ref();
        let detail = detail.as_ref();
        let icon = self.style.paint(OutputStyle::icon(icon), icon_color);
        if detail.is_empty() {
            println!("  {icon} {left}");
        } else {
            println!("  {icon} {left}  {}", self.style.paint(detail, Color::Dim));
        }
    }

    fn detail(&self, label: &str, value: &str) {
        if self.style.summary {
            return;
        }
        println!(
            "    {} {}",
            self.style.paint(format!("{label:<8}"), Color::Dim),
            self.style.paint(value, Color::Magenta)
        );
    }

    fn path(&self, path: &Path) -> String {
        display_path(path, &self.repo)
    }

    fn record_directive(&mut self, directive: Directive, ok: usize, changed: usize) {
        self.directives.push((directive, ok, changed));
    }

    fn finish(&self, _elapsed: std::time::Duration, failed: bool) {
        if self.style.summary {
            for (directive, ok, _changed) in &self.directives {
                let name = match directive {
                    Directive::Link => "link",
                    Directive::Create => "create",
                    Directive::Shell => "shell",
                    Directive::Clean => "clean",
                    Directive::Defaults => continue,
                };
                println!("{name}: {ok} ok");
            }
            return;
        }
        println!();
        let icon = if failed { "✗" } else { "✓" };
        let color = if failed { Color::Red } else { Color::Green };
        let title = if self.dry_run {
            "Dry run complete"
        } else if failed {
            "Failed"
        } else {
            "Done"
        };
        let total_links: usize = self
            .directives
            .iter()
            .filter(|d| d.0 == Directive::Link)
            .map(|d| d.1)
            .sum();
        let total_dirs: usize = self
            .directives
            .iter()
            .filter(|d| d.0 == Directive::Create)
            .map(|d| d.1)
            .sum();
        let total_shell: usize = self
            .directives
            .iter()
            .filter(|d| d.0 == Directive::Shell)
            .map(|d| d.1)
            .sum();
        let mut parts: Vec<String> = Vec::new();
        if total_links > 0 {
            parts.push(format!("{total_links} linked"));
        }
        if total_dirs > 0 {
            parts.push(format!("{total_dirs} dirs"));
        }
        if total_shell > 0 {
            parts.push(format!("{total_shell} shell"));
        }
        if parts.is_empty() {
            parts.push("nothing to do".to_string());
        }
        println!(
            "{} {}   {}",
            self.style.paint(icon, color),
            self.style.paint(title, color),
            self.style.paint(parts.join(" · "), Color::Dim)
        );
    }
}

pub fn run_deploy(
    command_name: &str,
    config_path: &Path,
    dry_run: bool,
    only: &[Directive],
    except: &[Directive],
    color: ColorChoice,
    summary: bool,
) -> Result<(), String> {
    if !only.is_empty() && !except.is_empty() {
        return Err("--only and --except cannot be used together".to_string());
    }

    let repo =
        std::env::current_dir().map_err(|err| format!("failed to read current dir: {err}"))?;
    let config_dir = config_dir(config_path)?;
    let steps = load_steps(config_path)?;
    let mut link_defaults = LinkDefaults::default();
    let mut shell_defaults = ShellDefaults::default();
    let start = Instant::now();
    let style = OutputStyle::new(color, summary);
    let mut reporter = Reporter::new(command_name, &repo, dry_run, style);

    for step in steps {
        let directive = step.directive();
        if !should_run(directive, only, except) {
            continue;
        }

        let result = match step {
            Step::Defaults { defaults } => {
                link_defaults.apply(&defaults.link);
                shell_defaults.apply(&defaults.shell);
                print_defaults_plan(&reporter, &defaults);
                Ok(())
            }
            Step::Link { link } => {
                run_link_step(&config_dir, &link, link_defaults.clone(), &mut reporter)
            }
            Step::Create { create } => run_create_step(create, &mut reporter),
            Step::Shell { shell } => {
                run_shell_step(&config_dir, &shell, shell_defaults, &mut reporter)
            }
            Step::Clean { clean } => run_clean_step(&clean, &mut reporter),
        };
        if let Err(err) = result {
            reporter.finish(start.elapsed(), true);
            return Err(err);
        }
    }

    reporter.finish(start.elapsed(), false);
    Ok(())
}

fn print_defaults_plan(reporter: &Reporter, defaults: &DefaultsStep) {
    if !defaults.link.has_values() && !defaults.shell.has_values() {
        return;
    }
    reporter.section("Defaults");
    if defaults.link.has_values() {
        reporter.row(
            Icon::Action,
            Color::Blue,
            "link",
            format_link_options(&defaults.link),
        );
    }
    if defaults.shell.has_values() {
        reporter.row(
            Icon::Action,
            Color::Blue,
            "shell",
            format_shell_options(&defaults.shell),
        );
    }
    println!();
}

impl Step {
    fn directive(&self) -> Directive {
        match self {
            Step::Defaults { .. } => Directive::Defaults,
            Step::Link { .. } => Directive::Link,
            Step::Create { .. } => Directive::Create,
            Step::Shell { .. } => Directive::Shell,
            Step::Clean { .. } => Directive::Clean,
        }
    }
}

fn should_run(directive: Directive, only: &[Directive], except: &[Directive]) -> bool {
    if directive == Directive::Defaults {
        return !except.contains(&Directive::Defaults);
    }
    if !only.is_empty() {
        only.contains(&directive)
    } else {
        !except.contains(&directive)
    }
}

fn load_steps(path: &Path) -> Result<Vec<Step>, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_yaml::from_str(&raw).map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

fn config_dir(path: &Path) -> Result<PathBuf, String> {
    let dir = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if dir.is_absolute() {
        Ok(dir.to_path_buf())
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(dir))
            .map_err(|err| format!("failed to read current dir: {err}"))
    }
}

fn run_link_step(
    config_dir: &Path,
    items: &BTreeMap<String, LinkValue>,
    defaults: LinkDefaults,
    reporter: &mut Reporter,
) -> Result<(), String> {
    reporter.section("Links");
    let total = items.len();
    let mut already_linked = 0usize;
    let mut links_changed = 0usize;
    for (target, value) in items.iter() {
        let (source, settings, condition) = match value {
            LinkValue::Path(path) => (
                path.as_str(),
                defaults.merged(&LinkOptions::default()),
                None,
            ),
            LinkValue::Options(item) => (
                item.path.as_str(),
                defaults.merged(&item.options),
                item.condition.as_deref(),
            ),
        };

        if let Some(condition) = condition
            && !condition_matches(config_dir, condition)?
        {
            reporter.row(
                Icon::Skip,
                Color::Dim,
                target.as_str(),
                "skipped, condition not met",
            );
            continue;
        }

        let plan = plan_link(config_dir, source, target, settings)?;

        if reporter.dry_run {
            if matches!(plan.action, LinkAction::Skip) {
                already_linked += 1;
            } else {
                print_link_plan_row(&plan, reporter);
            }
            if let LinkAction::Fail(reason) = &plan.action {
                return Err(format!("dry-run: would fail linking {}: {reason}", target));
            }
            continue;
        }

        if let LinkAction::Fail(reason) = &plan.action {
            print_link_plan_row(&plan, reporter);
            return Err(reason.clone());
        }

        if matches!(plan.action, LinkAction::Skip) {
            already_linked += 1;
            continue;
        }

        links_changed += 1;

        let link_label = format!(
            "{} → {}",
            reporter.path(&plan.target),
            reporter.path(&plan.source)
        );
        run_spinner_task(reporter, &link_label, move || apply_link_plan(plan))?;
    }
    if already_linked > 0 {
        reporter.row(
            Icon::Ok,
            Color::Green,
            format!("{already_linked} already linked"),
            "",
        );
    }
    reporter.record_directive(Directive::Link, total, links_changed);
    println!();
    Ok(())
}

fn plan_link(
    config_dir: &Path,
    source: &str,
    target: &str,
    settings: LinkSettings,
) -> Result<LinkPlan, String> {
    let source = Path::new(source);
    let source = if source.is_absolute() {
        source.to_path_buf()
    } else {
        config_dir.join(source)
    };
    let target = expand_home(target)?;

    if !source.exists() {
        return Ok(LinkPlan {
            target: target.clone(),
            source: source.clone(),
            link_source: source,
            settings,
            action: LinkAction::Fail("source does not exist".to_string()),
        });
    }

    let link_source = if settings.relative {
        relative_link_source(&source, &target)?
    } else {
        source.clone()
    };

    let action = if target.exists() || target.is_symlink() {
        if is_expected_symlink(&target, &source) {
            LinkAction::Skip
        } else if target.is_symlink() && settings.relink {
            LinkAction::Relink
        } else if settings.backup {
            LinkAction::Backup(unique_backup_path(&target))
        } else {
            LinkAction::Fail(describe_conflict(&target, &source))
        }
    } else {
        LinkAction::Link
    };

    Ok(LinkPlan {
        target,
        source,
        link_source,
        settings,
        action,
    })
}

fn print_link_plan_row(plan: &LinkPlan, reporter: &mut Reporter) {
    let target = reporter.path(&plan.target);
    let source = reporter.path(&plan.source);
    match &plan.action {
        LinkAction::Link => {
            reporter.row(Icon::Action, Color::Blue, target, source);
        }
        LinkAction::Relink => {
            reporter.row(
                Icon::Relink,
                Color::Yellow,
                target,
                format!("relink to {source}"),
            );
        }
        LinkAction::Backup(backup) => {
            reporter.row(
                Icon::Backup,
                Color::Yellow,
                target,
                format!("backup to {}; link to {source}", reporter.path(backup)),
            );
        }
        LinkAction::Skip => {
            reporter.row(Icon::Ok, Color::Green, target, "already linked");
        }
        LinkAction::Fail(reason) => {
            reporter.row(Icon::Fail, Color::Red, target, reason);
        }
    }
}

fn apply_link_plan(plan: LinkPlan) -> Result<(), String> {
    match &plan.action {
        LinkAction::Fail(reason) => Err(format!(
            "target conflict: {} ({reason})",
            plan.target.display()
        )),
        LinkAction::Skip => Ok(()),
        LinkAction::Link | LinkAction::Relink | LinkAction::Backup(_) => {
            if plan.settings.create {
                ensure_parent_dir(&plan.target)?;
            } else {
                ensure_existing_parent_dir(&plan.target)?;
            }

            match &plan.action {
                LinkAction::Relink => fs::remove_file(&plan.target).map_err(|err| {
                    format!("failed to remove link {}: {err}", plan.target.display())
                })?,
                LinkAction::Backup(backup) => fs::rename(&plan.target, backup)
                    .map_err(|err| format!("failed to backup {}: {err}", plan.target.display()))?,
                _ => {}
            }

            unix_fs::symlink(&plan.link_source, &plan.target)
                .map_err(|err| format!("failed to link {}: {err}", plan.target.display()))
        }
    }
}

fn run_create_step(create: CreateValue, reporter: &mut Reporter) -> Result<(), String> {
    reporter.section("Directories");
    let CreateValue::Paths(paths) = create;
    let total = paths.len();
    let mut existing_dirs = 0usize;
    let mut created = 0usize;
    for path in paths.into_iter() {
        let path = expand_home(&path)?;
        if reporter.dry_run {
            if path.exists() && path.is_dir() {
                existing_dirs += 1;
            } else {
                reporter.row(Icon::Create, Color::Cyan, reporter.path(&path), "create");
            }
            continue;
        }

        if path.exists() && path.is_dir() {
            existing_dirs += 1;
            continue;
        }
        created += 1;
        let dir_label = reporter.path(&path).to_string();
        run_spinner_task(reporter, &dir_label, move || {
            create_dir_all_following_symlinks(&path)
        })?;
    }
    if existing_dirs > 0 {
        reporter.row(
            Icon::Ok,
            Color::Green,
            format!("{existing_dirs} already exist"),
            "",
        );
    }
    reporter.record_directive(Directive::Create, total, created);
    println!();
    Ok(())
}

fn run_shell_step(
    config_dir: &Path,
    items: &[ShellValue],
    defaults: ShellDefaults,
    reporter: &mut Reporter,
) -> Result<(), String> {
    reporter.section("Shell");
    let total = items.len();
    let mut completed = 0usize;
    let mut skipped = 0usize;
    for item in items.iter() {
        let shell = match item {
            ShellValue::Command(command) => ShellItem {
                command: command.clone(),
                description: None,
                optional: false,
                options: ShellOptions::default(),
                condition: None,
            },
            ShellValue::Options(item) => item.clone(),
        };
        let settings = defaults.merged(&shell.options);

        let label = shell.description.as_deref().unwrap_or(&shell.command);
        if let Some(condition) = shell.condition.as_deref()
            && !condition_matches(config_dir, condition)?
        {
            if reporter.dry_run {
                reporter.row(Icon::Skip, Color::Dim, label, "skipped");
                reporter.detail("condition", condition);
            } else {
                skipped += 1;
            }
            continue;
        }

        if reporter.dry_run {
            reporter.row(Icon::Action, Color::Blue, label, "");
            reporter.detail("command", &shell.command);
            continue;
        }

        let can_animate = std::io::stdout().is_terminal();
        if !can_animate {
            println!(
                "  {} {}",
                reporter
                    .style
                    .paint(OutputStyle::icon(Icon::Running), Color::Cyan),
                reporter.style.paint(label, Color::Dim)
            );
        }
        let mut command = Command::new("sh");
        command.arg("-c").arg(&shell.command);
        command.current_dir(config_dir);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        let output = run_shell_command(command, reporter, label, can_animate)
            .map_err(|err| format!("failed to run shell command '{}': {err}", shell.command))?;
        let command_failed = !output.status.success();
        if (settings.stdout || command_failed) && !output.stdout.is_empty() {
            print!("{}", output.stdout);
            if !output.stdout.ends_with('\n') {
                println!();
            }
            std::io::stdout()
                .flush()
                .map_err(|err| format!("failed to flush stdout: {err}"))?;
        }
        if (settings.stderr || command_failed) && !output.stderr.is_empty() {
            eprint!("{}", output.stderr);
            if !output.stderr.ends_with('\n') {
                eprintln!();
            }
            std::io::stderr()
                .flush()
                .map_err(|err| format!("failed to flush stderr: {err}"))?;
        }
        if command_failed {
            if shell.optional {
                reporter.row(
                    Icon::Warn,
                    Color::Yellow,
                    label,
                    format!("optional command failed: {}", output.status),
                );
                continue;
            }
            println!(
                "  {} {}",
                reporter
                    .style
                    .paint(OutputStyle::icon(Icon::Fail), Color::Red),
                reporter.style.paint(label, Color::Red)
            );
            return Err(format!("shell command failed: {}", shell.command));
        }
        completed += 1;
    }
    reporter.record_directive(Directive::Shell, total, completed);
    let completed_label = match completed {
        0 => None,
        1 => Some("1 shell command completed".to_string()),
        completed => Some(format!("{completed} shell commands completed")),
    };
    let skipped_label = match skipped {
        0 => None,
        1 => Some("1 skipped".to_string()),
        skipped => Some(format!("{skipped} skipped")),
    };
    match (completed_label, skipped_label) {
        (Some(completed), Some(skipped)) => {
            reporter.row(
                Icon::Ok,
                Color::Green,
                format!("{completed}, {skipped}"),
                "",
            );
        }
        (Some(completed), None) => reporter.row(Icon::Ok, Color::Green, completed, ""),
        (None, Some(skipped)) => reporter.row(Icon::Skip, Color::Dim, skipped, ""),
        (None, None) => {}
    }
    println!();
    Ok(())
}

fn run_spinner_task<T, F>(reporter: &Reporter, label: &str, task: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    if !std::io::stdout().is_terminal() {
        println!(
            "  {} {}",
            reporter
                .style
                .paint(OutputStyle::icon(Icon::Running), Color::Cyan),
            reporter.style.paint(label, Color::Dim)
        );
        return task();
    }

    let task = std::thread::spawn(task);
    let frames = OutputStyle::spinner_frames();
    let mut frame_index = 0usize;
    while !task.is_finished() {
        let frame = frames[frame_index % frames.len()];
        print!(
            "\r\x1b[2K  {} {}",
            reporter.style.paint(frame, Color::Cyan),
            reporter.style.paint(label, Color::Dim)
        );
        std::io::stdout()
            .flush()
            .map_err(|err| format!("failed to flush stdout: {err}"))?;
        frame_index += 1;
        std::thread::sleep(Duration::from_millis(90));
    }
    print!("\r\x1b[2K");
    std::io::stdout()
        .flush()
        .map_err(|err| format!("failed to flush stdout: {err}"))?;

    task.join()
        .map_err(|_| "task panicked while running".to_string())?
}

struct ShellCommandOutput {
    status: std::process::ExitStatus,
    stdout: String,
    stderr: String,
}

fn run_shell_command(
    mut command: Command,
    reporter: &Reporter,
    label: &str,
    can_animate: bool,
) -> Result<ShellCommandOutput, std::io::Error> {
    let mut child = command.spawn()?;
    let stdout_reader = child
        .stdout
        .take()
        .map(|mut stdout| std::thread::spawn(move || read_command_output(&mut stdout)));
    let stderr_reader = child
        .stderr
        .take()
        .map(|mut stderr| std::thread::spawn(move || read_command_output(&mut stderr)));

    let frames = OutputStyle::spinner_frames();
    let mut frame_index = 0usize;
    let status = if can_animate {
        loop {
            if let Some(status) = child.try_wait()? {
                print!("\r\x1b[2K");
                std::io::stdout().flush()?;
                break status;
            }

            let frame = frames[frame_index % frames.len()];
            print!(
                "\r\x1b[2K  {} {}",
                reporter.style.paint(frame, Color::Cyan),
                reporter.style.paint(label, Color::Dim)
            );
            std::io::stdout().flush()?;
            frame_index += 1;
            std::thread::sleep(Duration::from_millis(90));
        }
    } else {
        child.wait()?
    };

    let stdout = join_command_output(stdout_reader)?;
    let stderr = join_command_output(stderr_reader)?;

    Ok(ShellCommandOutput {
        status,
        stdout,
        stderr,
    })
}

fn read_command_output(reader: &mut impl Read) -> std::io::Result<String> {
    let mut output = String::new();
    reader.read_to_string(&mut output)?;
    Ok(output)
}

fn join_command_output(
    reader: Option<std::thread::JoinHandle<std::io::Result<String>>>,
) -> std::io::Result<String> {
    match reader {
        Some(reader) => reader
            .join()
            .unwrap_or_else(|_| Err(std::io::Error::other("failed to join output reader"))),
        None => Ok(String::new()),
    }
}

fn run_clean_step(paths: &[String], reporter: &mut Reporter) -> Result<(), String> {
    reporter.section("Clean");
    for path in paths {
        let path = expand_home(path)?;
        if reporter.dry_run {
            reporter.row(
                Icon::Action,
                Color::Blue,
                reporter.path(&path),
                "would clean",
            );
        } else {
            reporter.row(
                Icon::Skip,
                Color::Dim,
                reporter.path(&path),
                "not implemented",
            );
        }
    }
    println!();
    Ok(())
}

fn format_link_options(options: &LinkOptions) -> String {
    let mut labels = Vec::new();
    if let Some(value) = options.create {
        labels.push(format!("create={value}"));
    }
    if let Some(value) = options.relink {
        labels.push(format!("relink={value}"));
    }
    if let Some(value) = options.backup {
        labels.push(format!("backup={value}"));
    }
    if let Some(value) = options.relative {
        labels.push(format!("relative={value}"));
    }
    labels.join(", ")
}

fn format_shell_options(options: &ShellOptions) -> String {
    let mut labels = Vec::new();
    if let Some(value) = options.stdout {
        labels.push(format!("stdout={value}"));
    }
    if let Some(value) = options.stderr {
        labels.push(format!("stderr={value}"));
    }
    labels.join(", ")
}

fn display_path(path: &Path, repo: &Path) -> String {
    if let Ok(home) = std::env::var("HOME") {
        let home = PathBuf::from(home);
        if let Ok(rest) = path.strip_prefix(&home) {
            if rest.as_os_str().is_empty() {
                return "~".to_string();
            }
            return format!("~/{}", rest.display());
        }
    }

    if let Ok(rest) = path.strip_prefix(repo)
        && !rest.as_os_str().is_empty()
    {
        return rest.display().to_string();
    }

    path.display().to_string()
}

fn condition_matches(config_dir: &Path, condition: &str) -> Result<bool, String> {
    let status = Command::new("sh")
        .arg("-c")
        .arg(condition)
        .current_dir(config_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|err| format!("failed to evaluate condition '{condition}': {err}"))?;
    Ok(status.success())
}

fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("target has no parent: {}", path.display()))?;
    if parent.exists() && !parent.is_dir() {
        return Err(format!(
            "target parent is not a directory: {}",
            parent.display()
        ));
    }
    fs::create_dir_all(parent)
        .map_err(|err| format!("failed to create {}: {err}", parent.display()))
}

fn create_dir_all_following_symlinks(path: &Path) -> Result<(), String> {
    if path.exists() {
        return if path.is_dir() {
            Ok(())
        } else {
            Err(format!("path is not a directory: {}", path.display()))
        };
    }

    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        if current.as_os_str().is_empty() || current.exists() {
            if current.exists() && !current.is_dir() {
                return Err(format!("path is not a directory: {}", current.display()));
            }
            continue;
        }
        match fs::create_dir(&current) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists && current.is_dir() => {}
            Err(err) => return Err(format!("failed to create {}: {err}", current.display())),
        }
    }
    Ok(())
}

fn ensure_existing_parent_dir(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("target has no parent: {}", path.display()))?;
    if parent.is_dir() {
        Ok(())
    } else {
        Err(format!(
            "target parent does not exist or is not a directory: {}",
            parent.display()
        ))
    }
}

fn is_expected_symlink(target: &Path, source: &Path) -> bool {
    fs::read_link(target)
        .map(|actual| paths_match_from_link(target, &actual, source))
        .unwrap_or(false)
}

fn paths_match_from_link(link: &Path, actual: &Path, expected: &Path) -> bool {
    let actual_abs = if actual.is_absolute() {
        actual.to_path_buf()
    } else {
        link.parent()
            .map(|parent| parent.join(actual))
            .unwrap_or_else(|| actual.to_path_buf())
    };
    match (fs::canonicalize(&actual_abs), fs::canonicalize(expected)) {
        (Ok(actual), Ok(expected)) => actual == expected,
        _ => actual_abs == expected,
    }
}

fn describe_conflict(target: &Path, source: &Path) -> String {
    if let Ok(actual) = fs::read_link(target) {
        return format!(
            "symlink points to {}, expected {}",
            actual.display(),
            source.display()
        );
    }
    if target.is_dir() {
        "target is an existing directory".to_string()
    } else if target.is_file() {
        "target is an existing file".to_string()
    } else {
        "target exists with unsupported file type".to_string()
    }
}

fn unique_backup_path(target: &Path) -> PathBuf {
    let ts = timestamp();
    let mut candidate = PathBuf::from(format!("{}.backup.{ts}", target.display()));
    let mut counter = 1;
    while candidate.exists() {
        candidate = PathBuf::from(format!("{}.backup.{ts}.{counter}", target.display()));
        counter += 1;
    }
    candidate
}

fn timestamp() -> String {
    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    let format = time::macros::format_description!("[year][month][day][hour][minute][second]");
    now.format(&format)
        .unwrap_or_else(|_| "19700101000000".to_string())
}

fn relative_link_source(source: &Path, target: &Path) -> Result<PathBuf, String> {
    let target_parent = target
        .parent()
        .ok_or_else(|| format!("target has no parent: {}", target.display()))?;
    diff_paths(&absolute_path(source)?, &absolute_path(target_parent)?)
        .ok_or_else(|| format!("failed to compute relative link for {}", target.display()))
}

fn absolute_path(path: &Path) -> Result<PathBuf, String> {
    if let Ok(canonical) = fs::canonicalize(path) {
        return Ok(canonical);
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .map_err(|err| format!("failed to read current dir: {err}"))?
    };

    let mut missing = Vec::new();
    let mut existing = absolute.as_path();
    while !existing.exists() {
        let file_name = existing
            .file_name()
            .ok_or_else(|| format!("failed to resolve path: {}", path.display()))?;
        missing.push(file_name.to_os_string());
        existing = existing
            .parent()
            .ok_or_else(|| format!("failed to resolve path: {}", path.display()))?;
    }

    let mut resolved = fs::canonicalize(existing)
        .map_err(|err| format!("failed to resolve {}: {err}", existing.display()))?;
    for component in missing.iter().rev() {
        resolved.push(component);
    }
    Ok(resolved)
}

fn diff_paths(path: &Path, base: &Path) -> Option<PathBuf> {
    let path_components = normal_components(path)?;
    let base_components = normal_components(base)?;
    let common = path_components
        .iter()
        .zip(base_components.iter())
        .take_while(|(left, right)| left == right)
        .count();

    let mut result = PathBuf::new();
    for _ in common..base_components.len() {
        result.push("..");
    }
    for component in &path_components[common..] {
        result.push(component);
    }
    if result.as_os_str().is_empty() {
        result.push(".");
    }
    Some(result)
}

fn normal_components(path: &Path) -> Option<Vec<String>> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::RootDir => parts.push(String::new()),
            Component::Normal(value) => parts.push(value.to_string_lossy().to_string()),
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop()?;
            }
            Component::Prefix(_) => return None,
        }
    }
    Some(parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dotbot_style_steps() {
        let raw = r#"
- defaults:
    link:
      create: true
      relink: true
      relative: true
- link:
    ~/.config/fish: config/fish
    ~/.tmux.conf:
      path: config/tmux.conf
      backup: true
- create:
    - ~/.config/fish/local.d
- shell:
    - command: fish -lc 'fisher update'
      description: Sync fish plugins
      stdout: true
"#;
        let steps: Vec<Step> = serde_yaml::from_str(raw).expect("parse");
        assert_eq!(steps.len(), 4);
        assert!(matches!(steps[0], Step::Defaults { .. }));
        assert!(matches!(steps[1], Step::Link { .. }));
        assert!(matches!(steps[2], Step::Create { .. }));
        assert!(matches!(steps[3], Step::Shell { .. }));
    }

    #[test]
    fn relative_link_source_points_from_target_parent_to_source() {
        let repo = tempfile::tempdir().expect("repo");
        let home = tempfile::tempdir().expect("home");
        let source = repo.path().join("config/fish");
        let target_parent = home.path().join(".config");
        let target = target_parent.join("fish");
        fs::create_dir_all(&source).expect("source");
        fs::create_dir_all(&target_parent).expect("target parent");

        let relative = relative_link_source(&source, &target).expect("relative");
        assert!(!relative.is_absolute(), "expected relative path");
    }
}
