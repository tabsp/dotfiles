use crate::path::{paths_match, which};
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn run_shell() -> Result<(), String> {
    let fish = which("fish").ok_or_else(|| "fish is not installed or not on PATH".to_string())?;
    let current = std::env::var_os("SHELL").map(PathBuf::from);

    if current
        .as_deref()
        .is_some_and(|shell| shell_is_fish(shell, &fish))
    {
        println!("ok: login shell already uses fish ({})", fish.display());
        return Ok(());
    }

    let command = format!("chsh -s {}", fish.display());
    eprintln!("about to change login shell with: {command}");
    if !io::stdin().is_terminal() {
        return Err(format!(
            "refusing to change login shell without interactive confirmation; run manually: {command}"
        ));
    }

    eprint!("Change login shell to fish? [y/N] ");
    io::stderr()
        .flush()
        .map_err(|err| format!("failed to flush prompt: {err}"))?;

    let mut answer = String::new();
    io::stdin()
        .read_line(&mut answer)
        .map_err(|err| format!("failed to read confirmation: {err}"))?;
    if !confirmed(&answer) {
        return Err("login shell change cancelled".to_string());
    }

    let status = Command::new("chsh")
        .arg("-s")
        .arg(&fish)
        .status()
        .map_err(|err| format!("failed to run chsh: {err}"))?;
    if status.success() {
        println!("ok: login shell changed to fish ({})", fish.display());
        Ok(())
    } else {
        Err(format!("chsh exited {status}"))
    }
}

fn shell_is_fish(current: &Path, fish: &Path) -> bool {
    paths_match(current, fish) || current.file_name().is_some_and(|name| name == "fish")
}

fn confirmed(answer: &str) -> bool {
    matches!(answer.trim(), "y" | "Y" | "yes" | "YES" | "Yes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirmed_accepts_yes_forms() {
        assert!(confirmed("y\n"));
        assert!(confirmed("yes"));
        assert!(confirmed("YES"));
    }

    #[test]
    fn confirmed_rejects_default_and_other_text() {
        assert!(!confirmed(""));
        assert!(!confirmed("n"));
        assert!(!confirmed("sure"));
    }
}
