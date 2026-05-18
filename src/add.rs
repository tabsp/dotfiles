use crate::AddCommand;
use crate::config::{self, DepsManifest, DotfilesManifest, FileKind, Installer};
use crate::platform;
use std::collections::BTreeMap;
use std::io::{self, BufRead, Write};
use std::path::Path;

pub fn run_add(command: AddCommand) -> Result<(), String> {
    match command {
        AddCommand::Dep { dry_run } => add_dep(dry_run),
        AddCommand::Config { dry_run } => add_config(dry_run),
    }
}

// ── helpers ──────────────────────────────────────────────────────────────

fn prompt(msg: &str) -> io::Result<String> {
    let mut stdout = io::stdout();
    write!(stdout, "{msg} ")?;
    stdout.flush()?;
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

fn prompt_default(msg: &str, default: &str) -> io::Result<String> {
    let mut stdout = io::stdout();
    write!(stdout, "{msg} [{default}] ")?;
    stdout.flush()?;
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    let trimmed = line.trim().to_string();
    if trimmed.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(trimmed)
    }
}

fn prompt_yn(msg: &str, default_yes: bool) -> Result<bool, String> {
    let default_str = if default_yes { "Y/n" } else { "y/N" };
    let answer = prompt_default(msg, default_str).map_err(|e| format!("input error: {e}"))?;
    let lower = answer.to_lowercase();
    if default_yes {
        Ok(!lower.starts_with('n'))
    } else {
        Ok(lower.starts_with('y'))
    }
}

fn confirm(msg: &str) -> Result<bool, String> {
    prompt_yn(msg, true)
}

fn current_host_keys() -> (&'static str, &'static str) {
    let host = platform::detect_host().unwrap_or(platform::Host {
        platform: if cfg!(target_os = "macos") {
            platform::Platform::Mac
        } else {
            platform::Platform::Linux
        },
        arch: if cfg!(target_arch = "aarch64") {
            platform::Arch::Arm64
        } else {
            platform::Arch::X86_64
        },
        distro: None,
    });
    (host.platform.key(), host.arch.key())
}

// ── add dep ──────────────────────────────────────────────────────────────

fn add_dep(dry_run: bool) -> Result<(), String> {
    let repo = std::env::current_dir().map_err(|e| format!("cwd: {e}"))?;
    let deps_path = repo.join("deps.toml");

    let existing = config::load_deps(&deps_path).unwrap_or(DepsManifest {
        schema_version: None,
        deps: BTreeMap::new(),
    });

    // 1. Name
    let name = prompt_required("Dependency name (identifier):", |s| {
        if s.is_empty() {
            return Err("name must not be empty".to_string());
        }
        if s.contains(|c: char| c.is_whitespace() || c == '[' || c == ']' || c == '.') {
            return Err("name must not contain whitespace, [, ], or .".to_string());
        }
        if existing.deps.contains_key(s) {
            return Err(format!("dependency '{s}' already exists in deps.toml"));
        }
        Ok(())
    })?;

    // 2. Command
    let command = prompt_default("CLI command (checked by `which`):", &name)
        .map_err(|e| format!("input error: {e}"))?;
    for (existing_name, dep) in &existing.deps {
        if dep.command == command && existing_name != &name {
            return Err(format!(
                "command '{command}' is already used by dependency '{existing_name}'"
            ));
        }
    }

    // 3. Installer type
    let installer = prompt_installer()?;

    // 4. Version
    let version = prompt_default("Version (or 'latest'):", "latest")
        .map_err(|e| format!("input error: {e}"))?;

    // 5. Source URL (optional)
    let source = prompt_default("Project URL (optional, press Enter to skip):", "")
        .map_err(|e| format!("input error: {e}"))?;
    let source = if source.is_empty() {
        None
    } else {
        if !source.starts_with("https://") {
            return Err("source URL must start with https://".to_string());
        }
        Some(source)
    };

    // 6. Installer-specific params
    let params = prompt_installer_params(installer)?;

    // Build TOML
    let (platform, arch) = current_host_keys();
    let toml_snippet = build_dep_toml(
        &name,
        &command,
        installer,
        &version,
        source.as_deref(),
        &params,
        platform,
        arch,
    );

    // 7. Summary + confirm
    println!();
    if dry_run {
        println!("==> add dep (dry-run)");
        println!("{toml_snippet}");
        println!("==> dry-run complete (no changes made)");
        return Ok(());
    }

    println!("Will add the following to deps.toml:\n");
    println!("{toml_snippet}");
    if !confirm("Proceed?")? {
        println!("aborted.");
        return Ok(());
    }

    append_and_validate_deps(&deps_path, &toml_snippet, &repo)?;
    println!("hint: run `dotman check` to verify the updated manifest.");
    Ok(())
}

fn prompt_required<F>(msg: &str, validate: F) -> Result<String, String>
where
    F: Fn(&str) -> Result<(), String>,
{
    for attempt in 1..=3 {
        let input = prompt(msg).map_err(|e| format!("input error: {e}"))?;
        if let Err(err) = validate(&input) {
            if attempt == 3 {
                return Err(format!("{err} (max attempts reached)"));
            }
            eprintln!("error: {err}");
            continue;
        }
        return Ok(input);
    }
    Err("max attempts reached".to_string())
}

fn prompt_installer() -> Result<Installer, String> {
    println!("Available installers:");
    println!("  1. system");
    println!("  2. brew");
    println!("  3. cask");
    println!("  4. apt");
    println!("  5. repo_package");
    println!("  6. ppa");
    println!("  7. official_script");
    println!("  8. download_binary");

    for attempt in 1..=3 {
        let input = prompt("Installer type (1-8):").map_err(|e| format!("input error: {e}"))?;
        match input.trim() {
            "1" | "system" => return Ok(Installer::System),
            "2" | "brew" => return Ok(Installer::Brew),
            "3" | "cask" => return Ok(Installer::Cask),
            "4" | "apt" => return Ok(Installer::Apt),
            "5" | "repo_package" => return Ok(Installer::RepoPackage),
            "6" | "ppa" => return Ok(Installer::Ppa),
            "7" | "official_script" => return Ok(Installer::OfficialScript),
            "8" | "download_binary" => return Ok(Installer::DownloadBinary),
            _ => {
                if attempt == 3 {
                    return Err("invalid installer selection (max attempts reached)".to_string());
                }
                eprintln!("error: invalid selection, enter 1-8 or installer name");
            }
        }
    }
    Err("max attempts reached".to_string())
}

fn prompt_installer_params(installer: Installer) -> Result<BTreeMap<String, toml::Value>, String> {
    use toml::Value;
    let mut params = BTreeMap::new();

    match installer {
        Installer::System | Installer::Apt => {}
        Installer::Brew | Installer::Cask => {
            let pkg = prompt_required("Package name:", non_empty)?;
            params.insert("package".to_string(), Value::String(pkg));
        }
        Installer::RepoPackage => {
            params.insert(
                "package".to_string(),
                Value::String(prompt_required("Package name:", non_empty)?),
            );
            params.insert(
                "repo_url".to_string(),
                Value::String(prompt_required("Repository URL (https://...):", https_url)?),
            );
            params.insert(
                "repo_key_url".to_string(),
                Value::String(prompt_required(
                    "Repository key URL (https://...):",
                    https_url,
                )?),
            );
            params.insert(
                "repo_channel".to_string(),
                Value::String(prompt_required(
                    "Repository channel (e.g. stable):",
                    non_empty,
                )?),
            );
            let comps = prompt_required(
                "Repository components (comma-separated, e.g. main):",
                non_empty,
            )?;
            let arr: Vec<Value> = comps
                .split(',')
                .map(|s| Value::String(s.trim().to_string()))
                .collect();
            params.insert("repo_components".to_string(), Value::Array(arr));
        }
        Installer::Ppa => {
            params.insert(
                "ppa".to_string(),
                Value::String(prompt_required(
                    "PPA name (e.g. ppa:fish-shell/release-4):",
                    non_empty,
                )?),
            );
            params.insert(
                "package".to_string(),
                Value::String(prompt_required("Package name:", non_empty)?),
            );
        }
        Installer::OfficialScript => {
            params.insert(
                "script_url".to_string(),
                Value::String(prompt_required("Script URL (https://...):", https_url)?),
            );
            params.insert(
                "install_to".to_string(),
                Value::String(prompt_required(
                    "Install path (under ~/.local):",
                    install_path,
                )?),
            );
            let extra = prompt_default("Extra script args (space-separated, optional):", "")
                .map_err(|e| format!("input error: {e}"))?;
            if !extra.is_empty() {
                let arr: Vec<Value> = extra
                    .split_whitespace()
                    .map(|s| Value::String(s.to_string()))
                    .collect();
                params.insert("args".to_string(), Value::Array(arr));
            }
        }
        Installer::DownloadBinary => {
            params.insert(
                "url".to_string(),
                Value::String(prompt_required("Download URL (https://...):", https_url)?),
            );
            params.insert(
                "sha256".to_string(),
                Value::String(prompt_required("SHA256 hex digest:", non_empty)?),
            );
            params.insert(
                "archive_kind".to_string(),
                Value::String(prompt_required(
                    "Archive kind (raw, tar.gz, tar.xz, zip):",
                    |s| match s {
                        "raw" | "tar.gz" | "tar.xz" | "zip" => Ok(()),
                        _ => Err("must be one of: raw, tar.gz, tar.xz, zip".to_string()),
                    },
                )?),
            );
            params.insert(
                "binary_path".to_string(),
                Value::String(prompt_required("Binary path within archive:", non_empty)?),
            );
            params.insert(
                "install_to".to_string(),
                Value::String(prompt_required(
                    "Install path (under ~/.local):",
                    install_path,
                )?),
            );
            if prompt_yn("Include install_dir_from/install_dir_to?", false)? {
                params.insert(
                    "install_dir_from".to_string(),
                    Value::String(prompt_required("install_dir_from:", non_empty)?),
                );
                params.insert(
                    "install_dir_to".to_string(),
                    Value::String(prompt_required(
                        "install_dir_to (under ~/.local):",
                        install_path,
                    )?),
                );
            }
        }
    }

    Ok(params)
}

fn non_empty(s: &str) -> Result<(), String> {
    if s.is_empty() {
        Err("must not be empty".to_string())
    } else {
        Ok(())
    }
}

fn https_url(s: &str) -> Result<(), String> {
    if s.starts_with("https://") {
        Ok(())
    } else {
        Err("must start with https://".to_string())
    }
}

fn install_path(s: &str) -> Result<(), String> {
    if s.starts_with("~/.local") {
        Ok(())
    } else {
        Err("must start with ~/.local".to_string())
    }
}

#[allow(clippy::too_many_arguments)]
fn build_dep_toml(
    name: &str,
    command: &str,
    installer: Installer,
    version: &str,
    source: Option<&str>,
    params: &BTreeMap<String, toml::Value>,
    platform: &str,
    arch: &str,
) -> String {
    let mut out = String::new();
    out.push_str(&format!("[deps.{name}]\n"));
    out.push_str(&format!("command = \"{command}\"\n\n"));

    let table = format!("[deps.{name}.{platform}.{arch}]");
    out.push_str(&format!("{table}\n"));
    out.push_str(&format!("installer = \"{}\"\n", installer_key(installer)));
    out.push_str(&format!("version = \"{version}\"\n"));
    if let Some(src) = source {
        out.push_str(&format!("source = \"{src}\"\n"));
    }
    out.push('\n');

    if !params.is_empty() {
        let params_table = format!("[deps.{name}.{platform}.{arch}.params]");
        out.push_str(&format!("{params_table}\n"));
        for (k, v) in params {
            out.push_str(&format!("{} = {}\n", k, toml_value_str(v)));
        }
        out.push('\n');
    }

    out
}

fn installer_key(installer: Installer) -> &'static str {
    match installer {
        Installer::System => "system",
        Installer::Brew => "brew",
        Installer::Cask => "cask",
        Installer::Apt => "apt",
        Installer::RepoPackage => "repo_package",
        Installer::Ppa => "ppa",
        Installer::OfficialScript => "official_script",
        Installer::DownloadBinary => "download_binary",
    }
}

fn toml_value_str(value: &toml::Value) -> String {
    match value {
        toml::Value::String(s) => format!("\"{s}\""),
        toml::Value::Array(arr) => {
            let items: Vec<String> = arr
                .iter()
                .map(|v| match v {
                    toml::Value::String(s) => format!("\"{s}\""),
                    other => format!("{other:?}"),
                })
                .collect();
            format!("[{}]", items.join(", "))
        }
        other => format!("{other:?}"),
    }
}

// ── add config ───────────────────────────────────────────────────────────

fn add_config(dry_run: bool) -> Result<(), String> {
    let repo = std::env::current_dir().map_err(|e| format!("cwd: {e}"))?;
    let dotfiles_path = repo.join("dotfiles.toml");

    let existing = config::load_dotfiles(&dotfiles_path).unwrap_or(DotfilesManifest {
        schema_version: None,
        files: Vec::new(),
    });

    // 1. Source
    let source = prompt_required(
        "Source path (relative to repo, e.g. config/ripgreprc):",
        |s| {
            if s.is_empty() {
                return Err("source must not be empty".to_string());
            }
            if s.starts_with('/') || s.starts_with('~') || s.contains('$') || s.contains("..") {
                return Err("source must be a relative path (no /, ~, $, ..)".to_string());
            }
            if existing.files.iter().any(|f| f.source == s) {
                return Err(format!("source '{s}' already in dotfiles.toml"));
            }
            Ok(())
        },
    )?;

    // 2. Target
    let target = prompt_required("Target path (~ or / prefixed, e.g. ~/.ripgreprc):", |s| {
        if s.is_empty() {
            return Err("target must not be empty".to_string());
        }
        if !(s.starts_with('~') || s.starts_with('/')) {
            return Err("target must start with ~ or /".to_string());
        }
        if existing
            .files
            .iter()
            .filter(|f| f.enabled)
            .any(|f| f.target == s)
        {
            return Err(format!("target '{s}' already used by an active entry"));
        }
        Ok(())
    })?;

    // 3. Kind
    let kind_input =
        prompt_default("Kind (file or dir):", "file").map_err(|e| format!("input error: {e}"))?;
    let kind = match kind_input.to_lowercase().as_str() {
        "file" => FileKind::File,
        "dir" => FileKind::Dir,
        other => return Err(format!("invalid kind: {other} (must be file or dir)")),
    };

    // 4. Offer to create source if it doesn't exist
    let source_path = repo.join(&source);
    let create_source = if source_path.exists() {
        false
    } else if dry_run {
        println!("would create source path: {}", source_path.display());
        false
    } else {
        prompt_yn(
            &format!("Source '{}' does not exist. Create it?", source),
            true,
        )?
    };

    // 5. Platforms
    let platforms_input = prompt_default("Platforms (all, mac, linux, or comma-separated):", "all")
        .map_err(|e| format!("input error: {e}"))?;
    let platforms: Vec<String> = if platforms_input == "all" {
        Vec::new()
    } else {
        let parts: Vec<String> = platforms_input
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        for p in &parts {
            if p != "mac" && p != "linux" {
                return Err(format!("invalid platform: {p} (must be mac or linux)"));
            }
        }
        parts
    };

    // 6. Enabled
    let enabled = prompt_yn("Enabled?", true)?;

    // 7. Notes
    let notes = prompt_default("Notes (optional, press Enter to skip):", "")
        .map_err(|e| format!("input error: {e}"))?;
    let notes = if notes.is_empty() { None } else { Some(notes) };

    // Build TOML
    let toml_snippet = build_config_toml(
        &source,
        &target,
        kind,
        &platforms,
        enabled,
        notes.as_deref(),
    );

    println!();
    if dry_run {
        println!("==> add config (dry-run)");
        println!("{toml_snippet}");
        if create_source {
            println!("would create source path: {}", source_path.display());
        }
        println!("==> dry-run complete (no changes made)");
        return Ok(());
    }

    println!("Will add the following to dotfiles.toml:\n");
    println!("{toml_snippet}");
    if create_source {
        println!("Will also create: {}", source_path.display());
    }
    if !confirm("Proceed?")? {
        println!("aborted.");
        return Ok(());
    }

    // Create source path
    if create_source {
        match kind {
            FileKind::File => {
                if let Some(parent) = source_path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("failed to create parent dir: {e}"))?;
                }
                std::fs::write(&source_path, "")
                    .map_err(|e| format!("failed to create source file: {e}"))?;
            }
            FileKind::Dir => {
                std::fs::create_dir_all(&source_path)
                    .map_err(|e| format!("failed to create source directory: {e}"))?;
            }
        }
    }

    // Atomic write
    append_and_validate_dotfiles(&dotfiles_path, &toml_snippet, &repo)?;
    println!("hint: run `dotman check` to verify the updated manifest.");
    Ok(())
}

fn build_config_toml(
    source: &str,
    target: &str,
    kind: FileKind,
    platforms: &[String],
    enabled: bool,
    notes: Option<&str>,
) -> String {
    let kind_str = match kind {
        FileKind::File => "file",
        FileKind::Dir => "dir",
    };

    let mut out = String::new();
    out.push_str("[[files]]\n");
    out.push_str(&format!("source = \"{source}\"\n"));
    out.push_str(&format!("target = \"{target}\"\n"));
    out.push_str(&format!("kind = \"{kind_str}\"\n"));

    if !platforms.is_empty() {
        let items: Vec<String> = platforms.iter().map(|p| format!("\"{p}\"")).collect();
        out.push_str(&format!("platforms = [{}]\n", items.join(", ")));
    }

    if !enabled {
        out.push_str("enabled = false\n");
    }

    if let Some(n) = notes {
        out.push_str(&format!("notes = \"{n}\"\n"));
    }

    out.push('\n');
    out
}

// ── atomic write ─────────────────────────────────────────────────────────

fn append_and_validate_deps(deps_path: &Path, snippet: &str, repo: &Path) -> Result<(), String> {
    let existing = std::fs::read_to_string(deps_path).unwrap_or_default();
    let merged = if existing.trim().is_empty() {
        snippet.to_string()
    } else {
        format!("{existing}\n{snippet}")
    };

    let tmp_path = deps_path.with_extension("toml.tmp");
    std::fs::write(&tmp_path, &merged).map_err(|e| format!("failed to write tmp: {e}"))?;

    let deps = config::load_deps(&tmp_path)?;
    let files = config::load_dotfiles(&repo.join("dotfiles.toml")).unwrap_or(DotfilesManifest {
        schema_version: None,
        files: Vec::new(),
    });
    let host = platform::detect_host()?;
    if let Err(errors) = crate::check::run_check(&deps, &files, &host, repo) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(format!(
            "validation failed:\nerror: {}",
            errors.join("\nerror: ")
        ));
    }

    std::fs::rename(&tmp_path, deps_path).map_err(|e| format!("failed to rename tmp: {e}"))?;
    Ok(())
}

fn append_and_validate_dotfiles(
    dotfiles_path: &Path,
    snippet: &str,
    repo: &Path,
) -> Result<(), String> {
    let existing = std::fs::read_to_string(dotfiles_path).unwrap_or_default();
    let merged = if existing.trim().is_empty() {
        snippet.to_string()
    } else {
        format!("{existing}\n{snippet}")
    };

    let tmp_path = dotfiles_path.with_extension("toml.tmp");
    std::fs::write(&tmp_path, &merged).map_err(|e| format!("failed to write tmp: {e}"))?;

    let files = config::load_dotfiles(&tmp_path)?;
    let deps = config::load_deps(&repo.join("deps.toml")).unwrap_or(DepsManifest {
        schema_version: None,
        deps: BTreeMap::new(),
    });
    let host = platform::detect_host()?;
    if let Err(errors) = crate::check::run_check(&deps, &files, &host, repo) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(format!(
            "validation failed:\nerror: {}",
            errors.join("\nerror: ")
        ));
    }

    std::fs::rename(&tmp_path, dotfiles_path).map_err(|e| format!("failed to rename tmp: {e}"))?;
    Ok(())
}
