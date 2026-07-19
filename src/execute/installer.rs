use super::ExecuteEvent;
use super::command::{ensure_sudo_session, run_command_streaming_with_events};
use super::result::{cap_output_len, push_output_line};
use crate::model::{ActionStatus, OutputLine, OutputStream};
use crate::ops::{install, shell};
use anyhow::Result;
use std::path::Path;
use std::thread;
use std::time::Duration;

/// Default retry config (used when item doesn't override).
pub(super) const DEFAULT_INSTALL_RETRIES: u32 = 2;
const RETRY_INITIAL_DELAY_SECS: u64 = 5;

/// Install with retry and streaming output.
pub(super) fn run_install_streaming<F, C>(
    spec: &install::InstallSpec,
    max_retries: u32,
    item_name: &str,
    emit: &mut F,
    should_abort: &C,
    sudo_auth: &mut impl FnMut(&str) -> bool,
) -> Result<(ActionStatus, Option<String>, u32, Vec<OutputLine>)>
where
    F: FnMut(ExecuteEvent),
    C: Fn() -> bool,
{
    let entry = &spec.entry;
    let binary = &entry.name;
    let presence_command = spec.command.as_deref();
    if matches!(
        install::detect_presence(entry, presence_command),
        install::InstallPresence::Present
    ) {
        let msg = format!("already installed: {}", entry.name);
        emit(ExecuteEvent::ActionMessage {
            item: item_name.to_string(),
            message: msg.clone(),
        });
        return Ok((
            ActionStatus::NoChange,
            None,
            0,
            vec![OutputLine {
                stream: OutputStream::Action,
                line: msg,
            }],
        ));
    }

    // Fonts can use package-manager installs on platforms where a package
    // exists, with source_url as a fallback for platforms without one.
    if entry.kind == "font" && spec.command.is_none() {
        if entry.source_url.is_empty() {
            let msg = format!("font {} missing source_url", entry.name);
            emit(ExecuteEvent::ActionError {
                item: item_name.to_string(),
                message: msg.clone(),
            });
            return Ok((ActionStatus::WillFail, Some(msg), 0, vec![]));
        }

        let home = std::env::var("HOME").unwrap_or_default();
        if home.is_empty() {
            let msg = "HOME not set".to_string();
            emit(ExecuteEvent::ActionError {
                item: item_name.to_string(),
                message: msg.clone(),
            });
            return Ok((ActionStatus::WillFail, Some(msg), 0, vec![]));
        }
        let fonts_dir = format!("{home}/.local/share/fonts");

        // Download + unzip as a single streaming shell command so curl
        // progress appears in real time.
        let installed_check = if entry.font_family.is_empty() {
            String::new()
        } else {
            format!(
                "if command -v fc-list >/dev/null 2>&1 && fc-list | grep -qi {}; then echo {}; exit 0; fi && ",
                shell_quote(&entry.font_family),
                shell_quote(&format!("font already installed: {}", entry.font_family)),
            )
        };
        let verify_fontconfig = if entry.font_family.is_empty() {
            String::new()
        } else {
            format!(
                " && if command -v fc-list >/dev/null 2>&1; then fc-list | grep -qi {} || {{ echo {}; exit 1; }}; fi",
                shell_quote(&entry.font_family),
                shell_quote(&format!(
                    "installed font files, but fontconfig did not report: {}",
                    entry.font_family
                )),
            )
        };
        let font_cmd = format!(
            "{}tmpdir=$(mktemp -d) && trap 'rm -rf \"$tmpdir\"' EXIT HUP INT TERM && mkdir -p {} && curl -fsSL {} -o \"$tmpdir/font.zip\" && unzip -o -q \"$tmpdir/font.zip\" -d \"$tmpdir/font\" && find \"$tmpdir/font\" -type f \\( -name '*.ttf' -o -name '*.otf' \\) -exec cp {{}} {}/ \\; && if command -v fc-cache >/dev/null 2>&1; then fc-cache -f {}; fi{}",
            installed_check,
            shell_quote(&fonts_dir),
            shell_quote(&entry.source_url),
            shell_quote(&fonts_dir),
            shell_quote(&fonts_dir),
            verify_fontconfig,
        );

        let config_dir = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
        let (exit_code, mut output) = run_command_streaming_with_events(
            &font_cmd,
            &config_dir,
            item_name,
            emit,
            should_abort,
        )?;
        match exit_code {
            Some(0) => {
                let msg = format!("font {} installed to {fonts_dir}", entry.name);
                emit(ExecuteEvent::ActionMessage {
                    item: item_name.to_string(),
                    message: msg.clone(),
                });
                push_output_line(&mut output, OutputStream::Action, &msg);
                return Ok((ActionStatus::WillInstall, None, 1, output));
            }
            Some(code) => {
                let msg = format!("font install failed (exit {code})");
                emit(ExecuteEvent::ActionError {
                    item: item_name.to_string(),
                    message: msg.clone(),
                });
                push_output_line(&mut output, OutputStream::Action, &msg);
                return Ok((ActionStatus::WillFail, Some(msg), 1, output));
            }
            None => {
                return Ok((ActionStatus::Aborted, Some("aborted".into()), 1, output));
            }
        }
    }

    let cmd = match &spec.command {
        Some(command) => command.clone(),
        None => {
            let msg = spec
                .error
                .clone()
                .unwrap_or_else(|| format!("no install command for {}", spec.pkg_mgr));
            emit(ExecuteEvent::ActionError {
                item: item_name.to_string(),
                message: msg.clone(),
            });
            return Ok((ActionStatus::WillFail, Some(msg), 0, vec![]));
        }
    };

    let config_dir = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
    let mut last_err: Option<String> = None;
    let mut all_output: Vec<OutputLine> = Vec::new();
    let mut attempt = 0u32;
    let max = max_retries + 1;

    while attempt < max {
        if should_abort() {
            return Ok((
                ActionStatus::Aborted,
                Some("aborted".into()),
                attempt,
                all_output,
            ));
        }

        attempt += 1;
        let status_line = format!("install {binary}: attempt {attempt}/{max}");
        emit(ExecuteEvent::ActionMessage {
            item: item_name.to_string(),
            message: status_line.clone(),
        });
        push_output_line(&mut all_output, OutputStream::Action, &status_line);

        // Keep sudo fresh before install commands that need it. This must stay
        // non-interactive so abort can still work in the TUI.
        if shell::command_contains_sudo(&cmd) && !ensure_sudo_session(item_name, sudo_auth) {
            let msg = "sudo session expired — re-run to re-authenticate".to_string();
            emit(ExecuteEvent::ActionError {
                item: item_name.to_string(),
                message: msg.clone(),
            });
            push_output_line(&mut all_output, OutputStream::Action, &msg);
            return Ok((
                ActionStatus::WillFail,
                Some("sudo session expired".into()),
                attempt,
                all_output,
            ));
        }

        let (exit_code, output) =
            run_command_streaming_with_events(&cmd, &config_dir, item_name, emit, should_abort)?;
        all_output.extend(output);
        cap_output_len(&mut all_output);

        match exit_code {
            Some(0) => {
                return Ok((ActionStatus::WillInstall, None, attempt, all_output));
            }
            Some(code) => {
                last_err = Some(format!("install failed (exit {code})"));
            }
            None => {
                return Ok((
                    ActionStatus::Aborted,
                    Some("aborted".into()),
                    attempt,
                    all_output,
                ));
            }
        }

        if attempt < max {
            let delay = RETRY_INITIAL_DELAY_SECS * 2u64.pow(attempt - 1);
            let retry_msg = format!("retrying {binary} in {delay}s");
            emit(ExecuteEvent::ActionMessage {
                item: item_name.to_string(),
                message: retry_msg.clone(),
            });
            push_output_line(&mut all_output, OutputStream::Action, &retry_msg);

            // Sleep in 1-second increments, checking abort each time.
            for _ in 0..delay {
                if should_abort() {
                    return Ok((
                        ActionStatus::Aborted,
                        Some("aborted".into()),
                        attempt,
                        all_output,
                    ));
                }
                thread::sleep(Duration::from_secs(1));
            }
        }
    }

    if let Some(message) = &last_err {
        emit(ExecuteEvent::ActionError {
            item: item_name.to_string(),
            message: message.clone(),
        });
        push_output_line(&mut all_output, OutputStream::Action, message);
    }
    Ok((ActionStatus::WillFail, last_err, attempt, all_output))
}

/// Return a single-quoted shell string safe for embedding in `sh -c` commands.
pub(super) fn shell_quote(s: &str) -> String {
    let escaped = s.replace('\'', "'\\''");
    format!("'{escaped}'")
}
