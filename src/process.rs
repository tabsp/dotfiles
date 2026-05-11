use std::process::{Command, Output};

const MAX_CAPTURED_BYTES: usize = 8 * 1024;

pub fn run_capture(command: &str, args: &[&str]) -> Result<Output, String> {
    Command::new(command)
        .args(args)
        .output()
        .map_err(|err| format!("failed to run {command}: {err}"))
}

pub fn failure_context(output: &Output) -> Option<String> {
    let stderr = truncate_bytes(&output.stderr);
    if !stderr.is_empty() {
        return Some(stderr);
    }
    let stdout = truncate_bytes(&output.stdout);
    if !stdout.is_empty() {
        return Some(stdout);
    }
    None
}

fn truncate_bytes(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    let truncated = bytes.len() > MAX_CAPTURED_BYTES;
    let slice = if truncated {
        &bytes[..MAX_CAPTURED_BYTES]
    } else {
        bytes
    };
    let mut text = String::from_utf8_lossy(slice).to_string();
    if truncated {
        text.push_str("\n...output truncated...");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;

    #[test]
    fn failure_context_prefers_stderr() {
        let output = Output {
            status: std::process::ExitStatus::from_raw(1 << 8),
            stdout: b"stdout".to_vec(),
            stderr: b"stderr".to_vec(),
        };
        let context = failure_context(&output).expect("context");
        assert_eq!(context, "stderr");
    }

    #[test]
    fn failure_context_truncates_large_output() {
        let output = Output {
            status: std::process::ExitStatus::from_raw(1 << 8),
            stdout: vec![b'a'; 9000],
            stderr: Vec::new(),
        };
        let context = failure_context(&output).expect("context");
        assert!(context.contains("...output truncated..."));
    }
}
