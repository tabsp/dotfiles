use super::ExecuteEvent;
use super::result::push_output_line;
use crate::model::OutputLine;
use crate::ops::shell::{self, StreamLine};
use anyhow::Result;
use std::path::Path;
use std::sync::mpsc;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::Duration;

/// Run a shell command with real-time streaming output via events.
///
/// Spawns the command in a background thread, drains output through the emit
/// callback, and checks the abort signal periodically.
pub(super) fn run_command_streaming_with_events<F, C>(
    command: &str,
    config_dir: &Path,
    item_name: &str,
    emit: &mut F,
    should_abort: &C,
) -> Result<(Option<i32>, Vec<OutputLine>)>
where
    F: FnMut(ExecuteEvent),
    C: Fn() -> bool,
{
    // Check abort before spawning.
    if should_abort() {
        return Ok((None, vec![]));
    }

    let cmd_owned = command.to_string();
    let dir_owned = config_dir.to_path_buf();
    let (tx, rx) = mpsc::channel::<StreamLine>();
    let abort = Arc::new(AtomicBool::new(false));
    let cmd_abort = Arc::clone(&abort);

    let handle =
        thread::spawn(move || shell::run_command_streaming(&cmd_owned, &dir_owned, &tx, cmd_abort));

    let mut output_lines: Vec<OutputLine> = Vec::new();

    // Drain output while command runs.
    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(line) => {
                emit(ExecuteEvent::Output {
                    item: item_name.to_string(),
                    stream: line.stream,
                    line: line.line.clone(),
                });
                push_output_line(&mut output_lines, line.stream, &line.line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if should_abort() {
                    abort.store(true, Ordering::SeqCst);
                }
                if handle.is_finished() {
                    // Command process has exited but reader threads may still have
                    // buffered lines in the channel. Drain them before returning to
                    // avoid losing tail output.
                    while let Ok(line) = rx.try_recv() {
                        emit(ExecuteEvent::Output {
                            item: item_name.to_string(),
                            stream: line.stream,
                            line: line.line.clone(),
                        });
                        push_output_line(&mut output_lines, line.stream, &line.line);
                    }
                    break;
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let exit_code = handle
        .join()
        .map_err(|_| anyhow::anyhow!("command thread panicked"))??;

    Ok((exit_code, output_lines))
}

pub(super) fn ensure_sudo_session(item: &str, sudo_auth: &mut impl FnMut(&str) -> bool) -> bool {
    shell::refresh_sudo().unwrap_or(false) || sudo_auth(item)
}
