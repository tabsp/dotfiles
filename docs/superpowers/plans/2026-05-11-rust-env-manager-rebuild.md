# Rust Env Manager Rebuild Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a small Rust backend named `dotman` behind `make bootstrap`, `make link`, `make doctor`, and `make check`.

**Architecture:** Keep `make` as the public entry point and implement the behavior in a root-level Rust crate. Store dependency policy in `deps.toml`, managed file mappings in `dotfiles.toml`, and move repo-owned config sources into `config/` while preserving home targets.

**Tech Stack:** Rust, Cargo, TOML via `toml`, serialization via `serde`, CLI parsing via `clap`, tests with Rust unit/integration tests.

**Scope Note:** This plan builds the first runnable Rust slice. It implements the full manifest/check/link/doctor/bootstrap structure and safe package-manager installers. Execution support for `repo_package`, `official_script`, and `download_binary` remains a follow-up plan before the full v1 spec can be called complete.

---

## File Structure

- Create `Cargo.toml`: root Rust crate metadata and dependencies.
- Create `src/main.rs`: CLI entrypoint and top-level dispatch.
- Create `src/config.rs`: TOML loading and typed config structs.
- Create `src/platform.rs`: OS, architecture, and Linux distro detection/normalization.
- Create `src/check.rs`: read-only manifest and environment validation.
- Create `src/link.rs`: dotfile link execution, conflict handling, and dry-run output.
- Create `src/doctor.rs`: machine state checks and version checks.
- Create `src/deps.rs`: dependency installer selection and installed-state checks.
- Create `src/installers.rs`: installer behavior for `system`, `brew`, `cask`, `apt`, `repo_package`, `official_script`, and `download_binary`.
- Create `src/path.rs`: path expansion, repository-boundary checks, and symlink helpers.
- Create `src/output.rs`: consistent stdout/stderr formatting.
- Create `deps.toml`: v1 dependency manifest for current tools.
- Create `dotfiles.toml`: v1 managed-file manifest.
- Create `Makefile`: thin wrapper that performs cargo preflight, builds `dotman`, and calls backend subcommands.
- Move `.config/nvim` to `config/nvim`.
- Move `.config/fish` to `config/fish`.
- Move `.config/ghostty` to `config/ghostty`.
- Move `.tmux.conf` to `config/tmux.conf`.
- Modify `.gitignore`: ignore `target/`.

---

### Task 1: Scaffold Rust Backend And Make Entrypoints

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/output.rs`
- Create: `Makefile`
- Modify: `.gitignore`

- [ ] **Step 1: Create the Cargo manifest**

Create `Cargo.toml`:

```toml
[package]
name = "dotman"
version = "0.1.0"
edition = "2021"

[dependencies]
clap = { version = "4.5", features = ["derive"] }
regex = "1.10"
serde = { version = "1.0", features = ["derive"] }
sha2 = "0.10"
time = { version = "0.3", features = ["formatting", "local-offset", "macros"] }
toml = "0.8"

[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.1"
tempfile = "3.10"
```

- [ ] **Step 2: Add output helpers**

Create `src/output.rs`:

```rust
pub fn progress(message: impl AsRef<str>) {
    println!("==> {}", message.as_ref());
}

pub fn warn(message: impl AsRef<str>) {
    eprintln!("warn: {}", message.as_ref());
}

pub fn error(message: impl AsRef<str>) {
    eprintln!("error: {}", message.as_ref());
}
```

- [ ] **Step 3: Add the initial CLI**

Create `src/main.rs`:

```rust
mod output;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "dotman")]
#[command(about = "Internal dotfiles environment manager")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Bootstrap,
    Link {
        #[arg(long, default_value = "backup")]
        conflict: Conflict,
        #[arg(long)]
        dry_run: bool,
    },
    Doctor,
    Check,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum Conflict {
    Fail,
    Backup,
    Overwrite,
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Bootstrap => {
            output::progress("bootstrap");
            Ok(())
        }
        Command::Link { conflict, dry_run } => {
            output::progress(format!("link conflict={conflict:?} dry_run={dry_run}"));
            Ok(())
        }
        Command::Doctor => {
            output::progress("doctor");
            Ok(())
        }
        Command::Check => {
            output::progress("check");
            Ok(())
        }
    };

    if let Err(err) = result {
        output::error(err);
        std::process::exit(1);
    }
}
```

- [ ] **Step 4: Create the Makefile wrapper**

Create `Makefile`:

```makefile
.PHONY: bootstrap link doctor check build build-dotman cargo-preflight

DOTMAN := target/debug/dotman
CONFLICT ?= backup

cargo-preflight:
	@command -v cargo >/dev/null 2>&1 || { echo "error: cargo is required to build dotman" >&2; exit 1; }

build-dotman: cargo-preflight
	cargo build

build: build-dotman

bootstrap: build-dotman
	$(DOTMAN) bootstrap

link: build-dotman
	$(DOTMAN) link --conflict $(CONFLICT) $(if $(filter 1,$(DRY_RUN)),--dry-run,)

doctor: build-dotman
	$(DOTMAN) doctor

check: build-dotman
	$(DOTMAN) check
```

- [ ] **Step 5: Ignore Rust build output**

Append to `.gitignore`:

```gitignore
target/
```

- [ ] **Step 6: Verify CLI scaffolding**

Run:

```bash
cargo test
make check
make link DRY_RUN=1
```

Expected:

```text
cargo test exits 0
make check exits 0 and prints ==> check
make link DRY_RUN=1 exits 0 and prints ==> link ...
```

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml src/main.rs src/output.rs Makefile .gitignore
git commit -m "feat: scaffold dotman backend"
```

### Task 2: Add Manifest Types And Parsing

**Files:**
- Create: `src/config.rs`
- Modify: `src/main.rs`
- Create: `deps.toml`
- Create: `dotfiles.toml`

- [ ] **Step 1: Add config data structures**

Create `src/config.rs`:

```rust
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct DepsManifest {
    pub deps: BTreeMap<String, Dependency>,
}

#[derive(Debug, Deserialize)]
pub struct Dependency {
    pub command: String,
    #[serde(default)]
    pub version_check: Option<VersionCheck>,
    #[serde(default)]
    pub mac: BTreeMap<String, InstallEntry>,
    #[serde(default)]
    pub linux: BTreeMap<String, InstallEntry>,
}

#[derive(Debug, Deserialize)]
pub struct VersionCheck {
    #[serde(default = "default_version_args")]
    pub args: Vec<String>,
    pub regex: String,
    #[serde(default = "default_version_stream")]
    pub stream: VersionStream,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum VersionStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Deserialize)]
pub struct InstallEntry {
    pub installer: Installer,
    pub version: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub params: BTreeMap<String, toml::Value>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Installer {
    System,
    Brew,
    Cask,
    Apt,
    RepoPackage,
    OfficialScript,
    DownloadBinary,
}

#[derive(Debug, Deserialize)]
pub struct DotfilesManifest {
    pub files: Vec<FileEntry>,
}

#[derive(Debug, Deserialize)]
pub struct FileEntry {
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub kind: Option<FileKind>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub platforms: Vec<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FileKind {
    File,
    Dir,
}

pub fn load_deps(path: &Path) -> Result<DepsManifest, String> {
    let raw = fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    toml::from_str(&raw).map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

pub fn load_dotfiles(path: &Path) -> Result<DotfilesManifest, String> {
    let raw = fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    toml::from_str(&raw).map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

fn default_enabled() -> bool {
    true
}

fn default_version_args() -> Vec<String> {
    vec!["--version".to_string()]
}

fn default_version_stream() -> VersionStream {
    VersionStream::Stdout
}
```

- [ ] **Step 2: Wire the module into the CLI**

Modify `src/main.rs`:

```rust
mod config;
mod output;

use clap::{Parser, Subcommand, ValueEnum};
use std::path::Path;

#[derive(Debug, Parser)]
#[command(name = "dotman")]
#[command(about = "Internal dotfiles environment manager")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Bootstrap,
    Link {
        #[arg(long, default_value = "backup")]
        conflict: Conflict,
        #[arg(long)]
        dry_run: bool,
    },
    Doctor,
    Check,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum Conflict {
    Fail,
    Backup,
    Overwrite,
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Bootstrap => {
            output::progress("bootstrap");
            Ok(())
        }
        Command::Link { conflict, dry_run } => {
            output::progress(format!("link conflict={conflict:?} dry_run={dry_run}"));
            Ok(())
        }
        Command::Doctor => {
            output::progress("doctor");
            Ok(())
        }
        Command::Check => {
            let _deps = config::load_deps(Path::new("deps.toml"))?;
            let _files = config::load_dotfiles(Path::new("dotfiles.toml"))?;
            output::progress("check");
            Ok(())
        }
    };

    if let Err(err) = result {
        output::error(err);
        std::process::exit(1);
    }
}
```

- [ ] **Step 3: Create the initial dependency manifest**

Create `deps.toml`:

```toml
[deps.git]
command = "git"

[deps.git.version_check]
args = ["--version"]
regex = 'git version ([0-9]+\.[0-9]+\.[0-9]+)'
stream = "stdout"

[deps.git.mac.arm64]
installer = "system"
version = "latest"

[deps.git.mac.x86_64]
installer = "system"
version = "latest"

[deps.git.linux.arm64]
installer = "apt"
version = "latest"

[deps.git.linux.arm64.params]
package = "git"

[deps.git.linux.x86_64]
installer = "apt"
version = "latest"

[deps.git.linux.x86_64.params]
package = "git"

[deps.tmux]
command = "tmux"

[deps.tmux.version_check]
args = ["-V"]
regex = 'tmux ([0-9]+\.[0-9]+)'
stream = "stdout"

[deps.tmux.mac.arm64]
installer = "brew"
version = "latest"
source = "https://github.com/tmux/tmux"

[deps.tmux.mac.arm64.params]
package = "tmux"

[deps.tmux.mac.x86_64]
installer = "brew"
version = "latest"
source = "https://github.com/tmux/tmux"

[deps.tmux.mac.x86_64.params]
package = "tmux"

[deps.tmux.linux.arm64]
installer = "apt"
version = "latest"

[deps.tmux.linux.arm64.params]
package = "tmux"

[deps.tmux.linux.x86_64]
installer = "apt"
version = "latest"

[deps.tmux.linux.x86_64.params]
package = "tmux"
```

- [ ] **Step 4: Create the initial managed-file manifest**

Create `dotfiles.toml`:

```toml
[[files]]
source = "config/nvim"
target = "~/.config/nvim"
kind = "dir"

[[files]]
source = "config/fish"
target = "~/.config/fish"
kind = "dir"

[[files]]
source = "config/ghostty"
target = "~/.config/ghostty"
kind = "dir"

[[files]]
source = "config/tmux.conf"
target = "~/.tmux.conf"
kind = "file"
```

- [ ] **Step 5: Verify missing config is parsed**

Run:

```bash
cargo test
make check
```

Expected:

```text
make check fails until config/ exists in a later task
parsing errors should be reported as error: ...
```

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml src/main.rs src/config.rs deps.toml dotfiles.toml
git commit -m "feat: add dotman manifests"
```

### Task 3: Migrate Repository Config Layout

**Files:**
- Move: `.config/nvim` -> `config/nvim`
- Move: `.config/fish` -> `config/fish`
- Move: `.config/ghostty` -> `config/ghostty`
- Move: `.tmux.conf` -> `config/tmux.conf`

- [ ] **Step 1: Move config sources**

Run:

```bash
mkdir -p config
git mv .config/nvim config/nvim
git mv .config/fish config/fish
git mv .config/ghostty config/ghostty
git mv .tmux.conf config/tmux.conf
```

- [ ] **Step 2: Remove empty `.config` directory if git no longer tracks files there**

Run:

```bash
find .config -type f -print
```

Expected:

```text
no output
```

- [ ] **Step 3: Verify manifest paths now exist**

Run:

```bash
test -d config/nvim
test -d config/fish
test -d config/ghostty
test -f config/tmux.conf
make check
```

Expected:

```text
make check should parse manifests; deeper validation may still be incomplete
```

- [ ] **Step 4: Commit**

```bash
git add config .config dotfiles.toml
git commit -m "refactor: move dotfiles sources under config"
```

### Task 4: Implement Platform Detection And Structural Check

**Files:**
- Create: `src/platform.rs`
- Create: `src/check.rs`
- Modify: `src/main.rs`
- Modify: `src/config.rs`

- [ ] **Step 1: Add platform detection**

Create `src/platform.rs`:

```rust
use std::fs;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Host {
    pub platform: Platform,
    pub arch: Arch,
    pub distro: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Platform {
    Mac,
    Linux,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Arch {
    Arm64,
    X86_64,
}

impl Platform {
    pub fn key(self) -> &'static str {
        match self {
            Self::Mac => "mac",
            Self::Linux => "linux",
        }
    }
}

impl Arch {
    pub fn key(self) -> &'static str {
        match self {
            Self::Arm64 => "arm64",
            Self::X86_64 => "x86_64",
        }
    }
}

pub fn detect_host() -> Result<Host, String> {
    let platform = match std::env::consts::OS {
        "macos" => Platform::Mac,
        "linux" => Platform::Linux,
        other => return Err(format!("unsupported operating system: {other}")),
    };

    let arch = match std::env::consts::ARCH {
        "aarch64" => Arch::Arm64,
        "x86_64" => Arch::X86_64,
        other => return Err(format!("unsupported architecture: {other}")),
    };

    let distro = if platform == Platform::Linux {
        Some(read_linux_distro().unwrap_or_else(|| "unknown".to_string()))
    } else {
        None
    };

    Ok(Host { platform, arch, distro })
}

pub fn distro_supported(host: &Host) -> bool {
    match host.platform {
        Platform::Mac => true,
        Platform::Linux => matches!(host.distro.as_deref(), Some("ubuntu" | "debian")),
    }
}

fn read_linux_distro() -> Option<String> {
    let content = fs::read_to_string("/etc/os-release").ok()?;
    for line in content.lines() {
        if let Some(value) = line.strip_prefix("ID=") {
            return Some(value.trim_matches('"').to_ascii_lowercase());
        }
    }
    None
}
```

- [ ] **Step 2: Add check validation**

Create `src/check.rs`:

```rust
use crate::config::{DepsManifest, DotfilesManifest, FileKind, Installer};
use crate::platform::{distro_supported, Host, Platform};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub fn run_check(deps: &DepsManifest, files: &DotfilesManifest, host: &Host, repo: &Path) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let mut commands = BTreeSet::new();
    let mut active_targets = BTreeSet::new();
    let mut pairs = BTreeSet::new();
    let mut active_files = 0usize;

    if !distro_supported(host) {
        errors.push("unsupported Linux distribution; v1 supports Ubuntu and Debian".to_string());
    }

    for (name, dep) in &deps.deps {
        if !commands.insert(dep.command.clone()) {
            errors.push(format!("duplicate command in deps.toml: {}", dep.command));
        }

        match dep.entries_for(host.platform.key(), host.arch.key()).as_slice() {
            [] => errors.push(format!("dependency {name} has no current-host entry")),
            [entry] => {
                validate_installer_platform(name, entry.installer, host, &mut errors);
                if entry.version != "latest" && dep.version_check.is_none() {
                    errors.push(format!("dependency {name} pins version {} but has no version_check", entry.version));
                }
                validate_https(name, entry.source.as_deref(), &mut errors);
            }
            _ => errors.push(format!("dependency {name} has multiple current-host entries")),
        }
    }

    for file in &files.files {
        let pair = (file.source.clone(), file.target.clone());
        if !pairs.insert(pair) {
            errors.push(format!("duplicate dotfile mapping: {} -> {}", file.source, file.target));
        }

        if !file.enabled {
            continue;
        }
        if !file.platforms.is_empty() && !file.platforms.iter().any(|p| p == host.platform.key()) {
            continue;
        }
        active_files += 1;

        if !active_targets.insert(file.target.clone()) {
            errors.push(format!("duplicate active target: {}", file.target));
        }
        validate_source(repo, &file.source, file.kind, &mut errors);
        validate_target(repo, &file.target, &mut errors);
    }

    if active_files == 0 {
        errors.push("dotfiles.toml has no active file mappings for this host".to_string());
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_installer_platform(name: &str, installer: Installer, host: &Host, errors: &mut Vec<String>) {
    match installer {
        Installer::Brew | Installer::Cask if host.platform != Platform::Mac => {
            errors.push(format!("dependency {name} uses mac-only installer on non-mac host"));
        }
        Installer::Apt | Installer::RepoPackage if host.platform != Platform::Linux => {
            errors.push(format!("dependency {name} uses linux-only installer on non-linux host"));
        }
        _ => {}
    }
}

fn validate_https(name: &str, source: Option<&str>, errors: &mut Vec<String>) {
    if let Some(source) = source {
        if !source.starts_with("https://") {
            errors.push(format!("dependency {name} source must use https://"));
        }
    }
}

fn validate_source(repo: &Path, source: &str, expected: Option<FileKind>, errors: &mut Vec<String>) {
    if source.starts_with('/') || source.starts_with('~') || source.contains('$') || source.split('/').any(|part| part == "..") {
        errors.push(format!("invalid source path: {source}"));
        return;
    }

    let path = repo.join(source);
    if !path.exists() {
        errors.push(format!("source does not exist: {source}"));
        return;
    }

    if let Some(kind) = expected {
        let ok = match kind {
            FileKind::File => path.is_file(),
            FileKind::Dir => path.is_dir(),
        };
        if !ok {
            errors.push(format!("source kind mismatch: {source}"));
        }
    }
}

fn validate_target(repo: &Path, target: &str, errors: &mut Vec<String>) {
    if target.contains('$') {
        errors.push(format!("target must not contain environment variables: {target}"));
    }
    if !(target.starts_with("~/") || target.starts_with('/')) {
        errors.push(format!("target must be absolute or ~-based: {target}"));
    }
    let expanded = expand_home(target);
    if let Some(path) = expanded {
        if path.starts_with(repo) {
            errors.push(format!("target must not point inside repository: {target}"));
        }
    }
}

fn expand_home(path: &str) -> Option<PathBuf> {
    if let Some(rest) = path.strip_prefix("~/") {
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join(rest))
    } else {
        Some(PathBuf::from(path))
    }
}
```

- [ ] **Step 3: Add entry selection helper**

Modify `src/config.rs` and add this method:

```rust
impl Dependency {
    pub fn entries_for(&self, platform: &str, arch: &str) -> Vec<&InstallEntry> {
        match platform {
            "mac" => self.mac.get(arch).into_iter().collect(),
            "linux" => self.linux.get(arch).into_iter().collect(),
            _ => Vec::new(),
        }
    }
}
```

- [ ] **Step 4: Wire check into CLI**

Modify `src/main.rs` to import and call `check`:

```rust
mod check;
mod config;
mod output;
mod platform;

use clap::{Parser, Subcommand, ValueEnum};
use std::path::Path;

#[derive(Debug, Parser)]
#[command(name = "dotman")]
#[command(about = "Internal dotfiles environment manager")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Bootstrap,
    Link {
        #[arg(long, default_value = "backup")]
        conflict: Conflict,
        #[arg(long)]
        dry_run: bool,
    },
    Doctor,
    Check,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum Conflict {
    Fail,
    Backup,
    Overwrite,
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Bootstrap => {
            output::progress("bootstrap");
            Ok(())
        }
        Command::Link { conflict, dry_run } => {
            output::progress(format!("link conflict={conflict:?} dry_run={dry_run}"));
            Ok(())
        }
        Command::Doctor => {
            output::progress("doctor");
            Ok(())
        }
        Command::Check => run_check(),
    };

    if let Err(err) = result {
        output::error(err);
        std::process::exit(1);
    }
}

fn run_check() -> Result<(), String> {
    let deps = config::load_deps(Path::new("deps.toml"))?;
    let files = config::load_dotfiles(Path::new("dotfiles.toml"))?;
    let host = platform::detect_host()?;
    match check::run_check(&deps, &files, &host, Path::new(".")) {
        Ok(()) => Ok(()),
        Err(errors) => Err(errors.join("\nerror: ")),
    }
}
```

- [ ] **Step 5: Verify check behavior**

Run:

```bash
cargo test
make check
```

Expected:

```text
cargo test exits 0
make check exits 0 after Task 3 moved config paths
```

- [ ] **Step 6: Commit**

```bash
git add src/config.rs src/platform.rs src/check.rs src/main.rs
git commit -m "feat: validate dotman manifests"
```

### Task 5: Implement Link And Dry-Run

**Files:**
- Create: `src/path.rs`
- Create: `src/link.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add path utilities**

Create `src/path.rs`:

```rust
use std::path::{Path, PathBuf};

pub fn expand_home(path: &str) -> Result<PathBuf, String> {
    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var_os("HOME").ok_or_else(|| "HOME is not set".to_string())?;
        Ok(PathBuf::from(home).join(rest))
    } else {
        Ok(PathBuf::from(path))
    }
}

pub fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    let parent = path.parent().ok_or_else(|| format!("target has no parent: {}", path.display()))?;
    if parent.exists() && !parent.is_dir() {
        return Err(format!("target parent is not a directory: {}", parent.display()));
    }
    std::fs::create_dir_all(parent).map_err(|err| format!("failed to create {}: {err}", parent.display()))
}

pub fn which(command: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(command);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
```

- [ ] **Step 2: Add link implementation**

Create `src/link.rs`:

```rust
use crate::config::DotfilesManifest;
use crate::path::{ensure_parent_dir, expand_home};
use crate::platform::Host;
use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Conflict {
    Fail,
    Backup,
    Overwrite,
}

pub fn run_link(files: &DotfilesManifest, host: &Host, repo: &Path, conflict: Conflict, dry_run: bool) -> Result<(), String> {
    let actions = plan(files, host, repo, conflict)?;
    if dry_run {
        print_dry_run(&actions);
        if actions.iter().any(|action| matches!(action.kind, ActionKind::WouldFail)) {
            return Err("dry-run: would fail".to_string());
        }
        return Ok(());
    }

    for action in actions {
        apply_action(action)?;
    }
    Ok(())
}

#[derive(Debug)]
struct Action {
    kind: ActionKind,
    source: PathBuf,
    target: PathBuf,
    reason: Option<String>,
}

#[derive(Debug, Eq, PartialEq)]
enum ActionKind {
    WouldLink,
    WouldBackup,
    WouldOverwrite,
    WouldFail,
}

fn plan(files: &DotfilesManifest, host: &Host, repo: &Path, conflict: Conflict) -> Result<Vec<Action>, String> {
    let mut actions = Vec::new();
    for entry in &files.files {
        if !entry.enabled {
            continue;
        }
        if !entry.platforms.is_empty() && !entry.platforms.iter().any(|p| p == host.platform.key()) {
            continue;
        }

        let source = repo.join(&entry.source);
        let target = expand_home(&entry.target)?;
        let conflict_reason = if target.exists() || target.is_symlink() {
            if is_expected_symlink(&target, &source) {
                None
            } else {
                Some(describe_conflict(&target, &source))
            }
        } else {
            None
        };
        let kind = match conflict_reason {
            None => ActionKind::WouldLink,
            Some(_) => match conflict {
                Conflict::Fail => ActionKind::WouldFail,
                Conflict::Backup => ActionKind::WouldBackup,
                Conflict::Overwrite => ActionKind::WouldOverwrite,
            },
        };

        actions.push(Action {
            kind,
            source,
            target,
            reason: conflict_reason,
        });
    }
    Ok(actions)
}

fn apply_action(action: Action) -> Result<(), String> {
    ensure_parent_dir(&action.target)?;
    match action.kind {
        ActionKind::WouldFail => Err(format!("target conflict: {}", action.target.display())),
        ActionKind::WouldBackup => {
            let backup = unique_backup_path(&action.target);
            fs::rename(&action.target, &backup).map_err(|err| format!("failed to backup {}: {err}", action.target.display()))?;
            unix_fs::symlink(&action.source, &action.target)
                .map_err(|err| format!("failed to link {}: {err}", action.target.display()))
        }
        ActionKind::WouldOverwrite => {
            remove_existing(&action.target)?;
            unix_fs::symlink(&action.source, &action.target)
                .map_err(|err| format!("failed to link {}: {err}", action.target.display()))
        }
        ActionKind::WouldLink => {
            if is_expected_symlink(&action.target, &action.source) {
                Ok(())
            } else {
                unix_fs::symlink(&action.source, &action.target)
                    .map_err(|err| format!("failed to link {}: {err}", action.target.display()))
            }
        }
    }
}

fn is_expected_symlink(target: &Path, source: &Path) -> bool {
    fs::read_link(target).map(|actual| actual == source).unwrap_or(false)
}

fn describe_conflict(target: &Path, source: &Path) -> String {
    if let Ok(actual) = fs::read_link(target) {
        return format!("symlink points to {}, expected {}", actual.display(), source.display());
    }
    if target.is_dir() {
        "target is an existing directory".to_string()
    } else if target.is_file() {
        "target is an existing file".to_string()
    } else {
        "target exists with unsupported file type".to_string()
    }
}

fn remove_existing(path: &Path) -> Result<(), String> {
    if path.is_dir() && !path.is_symlink() {
        fs::remove_dir_all(path).map_err(|err| format!("failed to remove directory {}: {err}", path.display()))
    } else {
        fs::remove_file(path).map_err(|err| format!("failed to remove {}: {err}", path.display()))
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
    now.format(&format).unwrap_or_else(|_| "19700101000000".to_string())
}

fn print_dry_run(actions: &[Action]) {
    for wanted in [ActionKind::WouldFail, ActionKind::WouldOverwrite, ActionKind::WouldBackup, ActionKind::WouldLink] {
        for action in actions.iter().filter(|action| action.kind == wanted) {
            println!("{:?}: {} -> {}", action.kind, action.source.display(), action.target.display());
            if let Some(reason) = &action.reason {
                println!("  reason: {reason}");
            }
        }
    }
    if actions.iter().any(|action| action.kind == ActionKind::WouldFail) {
        println!("dry-run: would fail");
    } else {
        println!("dry-run: success");
    }
}
```

- [ ] **Step 3: Wire link into CLI**

Modify `src/main.rs` to map CLI conflict into `link::Conflict` and call `link::run_link`.

Expected relevant code:

```rust
mod link;
mod path;

fn run_link(conflict: Conflict, dry_run: bool) -> Result<(), String> {
    let deps = config::load_deps(Path::new("deps.toml"))?;
    let files = config::load_dotfiles(Path::new("dotfiles.toml"))?;
    let host = platform::detect_host()?;
    match check::run_check(&deps, &files, &host, Path::new(".")) {
        Ok(()) => {}
        Err(errors) => return Err(errors.join("\nerror: ")),
    }
    let conflict = match conflict {
        Conflict::Fail => link::Conflict::Fail,
        Conflict::Backup => link::Conflict::Backup,
        Conflict::Overwrite => link::Conflict::Overwrite,
    };
    link::run_link(&files, &host, Path::new("."), conflict, dry_run)
}
```

- [ ] **Step 4: Verify dry-run and link**

Run:

```bash
cargo test
cargo run -- link --dry-run --conflict backup
```

Expected:

```text
dry-run output grouped by action
exit 0 when no would-fail actions exist
```

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/main.rs src/link.rs src/path.rs
git commit -m "feat: link managed dotfiles"
```

### Task 6: Implement Doctor

**Files:**
- Create: `src/doctor.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add doctor implementation**

Create `src/doctor.rs`:

```rust
use crate::config::{DepsManifest, DotfilesManifest, VersionStream};
use crate::path::{expand_home, which};
use crate::platform::Host;
use regex::Regex;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn run_doctor(deps: &DepsManifest, files: &DotfilesManifest, host: &Host, repo: &Path) -> Result<(), String> {
    let mut hard_errors = Vec::new();
    let mut warnings = Vec::new();
    let mut oks = Vec::new();

    for (name, dep) in &deps.deps {
        let entries = dep.entries_for(host.platform.key(), host.arch.key());
        if entries.is_empty() {
            continue;
        }
        if which(&dep.command).is_none() {
            hard_errors.push(format!("{name}: missing command {}", dep.command));
            continue;
        }
        oks.push(format!("{name}: command {}", dep.command));

        if let Some(check) = &dep.version_check {
            match read_version(&dep.command, check) {
                Ok(version) => {
                    let expected = &entries[0].version;
                    if expected == "latest" {
                        oks.push(format!("{name}: version {version}"));
                    } else if expected != &version {
                        warnings.push(format!("{name}: version drift installed={version} expected={expected}"));
                    } else {
                        oks.push(format!("{name}: version {version}"));
                    }
                }
                Err(err) => hard_errors.push(format!("{name}: {err}")),
            }
        }
    }

    for file in &files.files {
        if !file.enabled {
            continue;
        }
        if !file.platforms.is_empty() && !file.platforms.iter().any(|p| p == host.platform.key()) {
            continue;
        }
        let target = expand_home(&file.target)?;
        let expected = repo.join(&file.source);
        match fs::read_link(&target) {
            Ok(actual) if actual == expected => oks.push(format!("link {} -> {}", target.display(), expected.display())),
            Ok(actual) => hard_errors.push(format!("wrong link {} -> {}, expected {}", target.display(), actual.display(), expected.display())),
            Err(_) if target.exists() => hard_errors.push(format!("target exists but is not a symlink: {}", target.display())),
            Err(_) => hard_errors.push(format!("missing target symlink: {}", target.display())),
        }
    }

    for err in &hard_errors {
        eprintln!("error: {err}");
    }
    for warning in &warnings {
        eprintln!("warn: {warning}");
    }
    for ok in &oks {
        println!("ok: {ok}");
    }

    if hard_errors.is_empty() {
        Ok(())
    } else {
        Err(format!("doctor found {} hard error(s)", hard_errors.len()))
    }
}

fn read_version(command: &str, check: &crate::config::VersionCheck) -> Result<String, String> {
    let output = Command::new(command)
        .args(&check.args)
        .output()
        .map_err(|err| format!("failed to run version check: {err}"))?;
    if !output.status.success() {
        return Err(format!("version check exited {}", output.status));
    }
    let bytes = match check.stream {
        VersionStream::Stdout => output.stdout,
        VersionStream::Stderr => output.stderr,
    };
    let text = String::from_utf8_lossy(&bytes);
    let regex = Regex::new(&check.regex).map_err(|err| format!("invalid version regex: {err}"))?;
    regex
        .captures(&text)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| "version output did not match regex".to_string())
}

```

- [ ] **Step 2: Wire doctor into CLI**

Modify `src/main.rs`:

```rust
mod doctor;

fn run_doctor() -> Result<(), String> {
    let deps = config::load_deps(Path::new("deps.toml"))?;
    let files = config::load_dotfiles(Path::new("dotfiles.toml"))?;
    let host = platform::detect_host()?;
    match check::run_check(&deps, &files, &host, Path::new(".")) {
        Ok(()) => doctor::run_doctor(&deps, &files, &host, Path::new(".")),
        Err(errors) => Err(errors.join("\nerror: ")),
    }
}
```

- [ ] **Step 3: Verify doctor**

Run:

```bash
cargo test
cargo run -- doctor
```

Expected:

```text
prints ok lines for available commands and links
returns non-zero if links are not yet applied
```

- [ ] **Step 4: Commit**

```bash
git add src/main.rs src/doctor.rs
git commit -m "feat: add dotman doctor"
```

### Task 7: Implement Dependency Installed-State Checks And Bootstrap

**Files:**
- Create: `src/deps.rs`
- Create: `src/installers.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add installer installed-state checks**

Create `src/installers.rs`:

```rust
use crate::config::{InstallEntry, Installer};
use crate::path::which;
use std::process::Command;

pub fn is_installed(command: &str, entry: &InstallEntry) -> Result<bool, String> {
    match entry.installer {
        Installer::System => Ok(which(command).is_some()),
        Installer::Brew => package_command("brew", &["list", "--formula", package(entry)?]),
        Installer::Cask => package_command("brew", &["list", "--cask", package(entry)?]),
        Installer::Apt | Installer::RepoPackage => package_command("dpkg", &["-s", package(entry)?]),
        Installer::OfficialScript => {
            if let Some(path) = string_param(entry, "install_to") {
                Ok(crate::path::expand_home(path)?.exists())
            } else {
                Ok(which(command).is_some())
            }
        }
        Installer::DownloadBinary => {
            let path = string_param(entry, "install_to").ok_or_else(|| "download_binary missing install_to".to_string())?;
            Ok(crate::path::expand_home(path)?.exists())
        }
    }
}

pub fn install_missing(command: &str, entry: &InstallEntry) -> Result<(), String> {
    if is_installed(command, entry)? {
        return Ok(());
    }
    match entry.installer {
        Installer::System => Err(format!("missing system command: {command}")),
        Installer::Brew => run("brew", &["install", package(entry)?]),
        Installer::Cask => run("brew", &["install", "--cask", package(entry)?]),
        Installer::Apt => run("sudo", &["apt-get", "install", "-y", package(entry)?]),
        Installer::RepoPackage => Err("repo_package installer execution is deferred from the first runnable slice".to_string()),
        Installer::OfficialScript => Err("official_script installer execution is deferred from the first runnable slice".to_string()),
        Installer::DownloadBinary => Err("download_binary installer execution is deferred from the first runnable slice".to_string()),
    }
}

fn package(entry: &InstallEntry) -> Result<&str, String> {
    string_param(entry, "package").ok_or_else(|| "missing package param".to_string())
}

fn string_param<'a>(entry: &'a InstallEntry, key: &str) -> Option<&'a str> {
    entry.params.get(key)?.as_str()
}

fn package_command(command: &str, args: &[&str]) -> Result<bool, String> {
    let status = Command::new(command)
        .args(args)
        .status()
        .map_err(|err| format!("failed to run {command}: {err}"))?;
    Ok(status.success())
}

fn run(command: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(command)
        .args(args)
        .status()
        .map_err(|err| format!("failed to run {command}: {err}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{command} exited {status}"))
    }
}

```

- [ ] **Step 2: Add dependency bootstrap**

Create `src/deps.rs`:

```rust
use crate::config::DepsManifest;
use crate::installers;
use crate::platform::Host;

pub fn install_missing(deps: &DepsManifest, host: &Host) -> Result<(), String> {
    for (name, dep) in &deps.deps {
        let entries = dep.entries_for(host.platform.key(), host.arch.key());
        let Some(entry) = entries.first() else {
            continue;
        };
        println!("==> dependency {name}");
        installers::install_missing(&dep.command, entry)?;
    }
    Ok(())
}
```

- [ ] **Step 3: Wire bootstrap**

Modify `src/main.rs`:

```rust
mod deps;
mod installers;

fn run_bootstrap() -> Result<(), String> {
    let deps_manifest = config::load_deps(Path::new("deps.toml"))?;
    let files = config::load_dotfiles(Path::new("dotfiles.toml"))?;
    let host = platform::detect_host()?;
    match check::run_check(&deps_manifest, &files, &host, Path::new(".")) {
        Ok(()) => {}
        Err(errors) => return Err(errors.join("\nerror: ")),
    }
    deps::install_missing(&deps_manifest, &host)?;
    link::run_link(&files, &host, Path::new("."), link::Conflict::Backup, false)?;
    doctor::run_doctor(&deps_manifest, &files, &host, Path::new("."))
}
```

- [ ] **Step 4: Verify bootstrap up to supported installers**

Run:

```bash
cargo test
cargo run -- bootstrap
```

Expected:

```text
bootstrap installs only missing system/brew/cask/apt tools supported so far
unsupported installer entries fail clearly
```

- [ ] **Step 5: Commit**

```bash
git add src/main.rs src/deps.rs src/installers.rs
git commit -m "feat: bootstrap missing dependencies"
```

### Task 8: Add Test Harness And Defer Unsafe Installers

**Files:**
- Create: `tests/common/mod.rs`
- Create: `tests/cli_check.rs`
- Create: `tests/cli_link.rs`
- Create: `tests/cli_doctor.rs`
- Modify: `deps.toml`

- [ ] **Step 1: Add shared integration test helpers**

Create `tests/common/mod.rs`:

```rust
pub fn current_host_table(dep: &str) -> String {
    let platform = if cfg!(target_os = "macos") { "mac" } else { "linux" };
    let arch = if cfg!(target_arch = "aarch64") { "arm64" } else { "x86_64" };
    format!("[deps.{dep}.{platform}.{arch}]")
}

pub fn current_host_params_table(dep: &str) -> String {
    let platform = if cfg!(target_os = "macos") { "mac" } else { "linux" };
    let arch = if cfg!(target_arch = "aarch64") { "arm64" } else { "x86_64" };
    format!("[deps.{dep}.{platform}.{arch}.params]")
}

pub fn non_current_host_table(dep: &str) -> String {
    let platform = if cfg!(target_os = "macos") { "mac" } else { "linux" };
    let arch = if cfg!(target_arch = "aarch64") { "x86_64" } else { "arm64" };
    format!("[deps.{dep}.{platform}.{arch}]")
}
```

- [ ] **Step 2: Add CLI check integration tests**

Create `tests/cli_check.rs`:

```rust
mod common;

use common::{current_host_params_table, current_host_table, non_current_host_table};
use predicates::prelude::*;

#[test]
fn check_rejects_missing_manifests() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .arg("check")
        .assert()
        .failure()
        .stderr(predicates::str::contains("error:"));
}

#[test]
fn check_reports_missing_current_host_entry() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("deps.toml"), format!(r#"
[deps.git]
command = "git"

{}
installer = "system"
version = "latest"
"#, non_current_host_table("git"))).expect("deps");
    std::fs::write(temp.path().join("dotfiles.toml"), "files = []\n").expect("dotfiles");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .arg("check")
        .assert()
        .failure()
        .stderr(predicate::str::contains("has no current-host entry"));
}

#[test]
fn check_aggregates_manifest_errors() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(temp.path().join("config/a")).expect("config");
    std::fs::write(temp.path().join("deps.toml"), format!(r#"
[deps.one]
command = "same"
{}
installer = "brew"
version = "1.0.0"
{}
package = "one"

[deps.two]
command = "same"
{}
installer = "brew"
version = "latest"
source = "http://example.invalid"
{}
package = "two"
"#, current_host_table("one"), current_host_params_table("one"), current_host_table("two"), current_host_params_table("two"))).expect("deps");
    std::fs::write(temp.path().join("dotfiles.toml"), r#"
[[files]]
source = "config/a"
target = "relative-target"
kind = "dir"
"#).expect("dotfiles");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .arg("check")
        .assert()
        .failure()
        .stderr(predicate::str::contains("duplicate command"))
        .stderr(predicate::str::contains("pins version"))
        .stderr(predicate::str::contains("source must use https"))
        .stderr(predicate::str::contains("target must start with ~/"));
}
```

- [ ] **Step 3: Add CLI link integration tests**

Create `tests/cli_link.rs`:

```rust
mod common;

use common::current_host_table;
use predicates::prelude::*;

fn write_link_manifests(repo: &std::path::Path, target: &str) {
    std::fs::create_dir_all(repo.join("config")).expect("config dir");
    std::fs::write(repo.join("config/tmux.conf"), "set -g mouse on\n").expect("source");
    std::fs::write(repo.join("deps.toml"), format!(r#"
[deps.git]
command = "git"
{}
installer = "system"
version = "latest"
"#, current_host_table("git"))).expect("deps");
    std::fs::write(repo.join("dotfiles.toml"), format!(r#"
[[files]]
source = "config/tmux.conf"
target = "{target}"
kind = "file"
"#)).expect("dotfiles");
}

#[test]
fn link_dry_run_reports_conflict_reason_and_fails() {
    let repo = tempfile::tempdir().expect("repo");
    let home = tempfile::tempdir().expect("home");
    write_link_manifests(repo.path(), "~/.tmux.conf");
    std::fs::write(home.path().join(".tmux.conf"), "user file\n").expect("target");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(repo.path())
        .env("HOME", home.path())
        .args(["link", "--dry-run", "--conflict", "fail"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("WouldFail"))
        .stdout(predicate::str::contains("reason: target is an existing file"));
}

#[test]
fn link_conflict_backup_creates_backup_and_symlink() {
    let repo = tempfile::tempdir().expect("repo");
    let home = tempfile::tempdir().expect("home");
    write_link_manifests(repo.path(), "~/.tmux.conf");
    std::fs::write(home.path().join(".tmux.conf"), "user file\n").expect("target");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(repo.path())
        .env("HOME", home.path())
        .args(["link", "--conflict", "backup"])
        .assert()
        .success();

    assert!(std::fs::read_link(home.path().join(".tmux.conf")).is_ok());
    let backups = std::fs::read_dir(home.path())
        .expect("home entries")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().contains(".backup."))
        .count();
    assert_eq!(backups, 1);
}

#[test]
fn link_conflict_overwrite_replaces_target() {
    let repo = tempfile::tempdir().expect("repo");
    let home = tempfile::tempdir().expect("home");
    write_link_manifests(repo.path(), "~/.tmux.conf");
    std::fs::write(home.path().join(".tmux.conf"), "user file\n").expect("target");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(repo.path())
        .env("HOME", home.path())
        .args(["link", "--conflict", "overwrite"])
        .assert()
        .success();

    assert!(std::fs::read_link(home.path().join(".tmux.conf")).is_ok());
}
```

- [ ] **Step 4: Add CLI doctor integration tests**

Create `tests/cli_doctor.rs`:

```rust
mod common;

use common::current_host_table;
use predicates::prelude::*;

#[test]
fn doctor_returns_nonzero_for_missing_link() {
    let repo = tempfile::tempdir().expect("repo");
    let home = tempfile::tempdir().expect("home");
    std::fs::create_dir_all(repo.path().join("config")).expect("config dir");
    std::fs::write(repo.path().join("config/tmux.conf"), "set -g mouse on\n").expect("source");
    std::fs::write(repo.path().join("deps.toml"), format!(r#"
[deps.git]
command = "git"
{}
installer = "system"
version = "latest"
"#, current_host_table("git"))).expect("deps");
    std::fs::write(repo.path().join("dotfiles.toml"), r#"
[[files]]
source = "config/tmux.conf"
target = "~/.tmux.conf"
kind = "file"
"#).expect("dotfiles");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(repo.path())
        .env("HOME", home.path())
        .arg("doctor")
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing target symlink"));
}

#[test]
fn doctor_warns_for_version_drift_but_keeps_exit_zero() {
    let repo = tempfile::tempdir().expect("repo");
    let home = tempfile::tempdir().expect("home");
    let bin = tempfile::tempdir().expect("bin");
    let fake = bin.path().join("fakecmd");
    std::fs::write(&fake, "#!/bin/sh\necho fakecmd 2.0.0\n").expect("fake command");
    make_executable(&fake);
    std::fs::create_dir_all(repo.path().join("config")).expect("config dir");
    std::fs::write(repo.path().join("config/tmux.conf"), "set -g mouse on\n").expect("source");
    std::os::unix::fs::symlink(repo.path().join("config/tmux.conf"), home.path().join(".tmux.conf")).expect("link");
    std::fs::write(repo.path().join("deps.toml"), format!(r#"
[deps.fake]
command = "fakecmd"
[deps.fake.version_check]
args = ["--version"]
regex = 'fakecmd ([0-9]+\.[0-9]+\.[0-9]+)'
{}
installer = "system"
version = "1.0.0"
"#, current_host_table("fake"))).expect("deps");
    std::fs::write(repo.path().join("dotfiles.toml"), r#"
[[files]]
source = "config/tmux.conf"
target = "~/.tmux.conf"
kind = "file"
"#).expect("dotfiles");

    let path = format!("{}:{}", bin.path().display(), std::env::var("PATH").unwrap_or_default());
    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(repo.path())
        .env("HOME", home.path())
        .env("PATH", path)
        .arg("doctor")
        .assert()
        .success()
        .stderr(predicate::str::contains("version drift"));
}

fn make_executable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path).expect("metadata").permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms).expect("chmod");
}

```

- [ ] **Step 5: Keep unsafe installers out of the first runnable manifest**

Keep `repo_package`, `official_script`, and `download_binary` supported by the schema and check validation, but do not put them into the default `deps.toml` until their execution paths have dedicated tests. Use `system`, `brew`, `cask`, and `apt` entries for the first runnable cut.

Use the safe `git` and `tmux` manifest shape from Task 2, with `mac.arm64`, `mac.x86_64`, `linux.arm64`, and `linux.x86_64` entries for each tool.

- [ ] **Step 6: Add explicit follow-up notes for deferred installers**

Add this comment block to the bottom of `deps.toml`:

```toml
# Deferred from the first runnable cut:
# - repo_package execution needs repository mutation tests on Ubuntu/Debian.
# - official_script execution needs temp-script tests and install_to marker tests.
# - download_binary execution needs archive extraction and sha256 tests.
```

- [ ] **Step 7: Run integration tests**

Run:

```bash
cargo test
```

Expected:

```text
unit and integration tests pass
```

- [ ] **Step 8: Commit**

```bash
git add Cargo.lock tests deps.toml
git commit -m "test: add dotman cli checks"
```

### Task 9: Expand Dependency Manifest For Current Tools

**Files:**
- Modify: `deps.toml`

- [ ] **Step 1: Add package-manager based entries**

Add entries for tools that can use `system`, `brew`, `cask`, or `apt` in v1:

```toml
[deps.fish]
command = "fish"

[deps.fish.version_check]
args = ["--version"]
regex = 'fish, version ([0-9]+\.[0-9]+\.[0-9]+)'
stream = "stdout"

[deps.fish.mac.arm64]
installer = "brew"
version = "latest"
source = "https://fishshell.com/"

[deps.fish.mac.arm64.params]
package = "fish"

[deps.ghostty]
command = "ghostty"

[deps.ghostty.mac.arm64]
installer = "cask"
version = "latest"
source = "https://ghostty.org/"

[deps.ghostty.mac.arm64.params]
package = "ghostty"
```

- [ ] **Step 2: Keep non-package-manager tools out until installer execution exists**

Do not add active entries for these tools in v1 until their selected installers are implemented:

```text
yazi
zoxide
starship
eza
lazygit
```

The first runnable Rust cut should prefer correctness over pretending unsupported installers work.

- [ ] **Step 3: Run manifest verification**

Run:

```bash
cargo test
make check
make link DRY_RUN=1
```

Expected:

```text
cargo test passes
make check passes
make link DRY_RUN=1 prints grouped dry-run output
```

- [ ] **Step 4: Commit**

```bash
git add deps.toml
git commit -m "feat: add initial dependency manifest"
```

### Task 10: Documentation And Final Verification

**Files:**
- Create: `README.md`
- Modify: `docs/superpowers/specs/2026-05-10-rust-env-manager-rebuild-design.md` only if implementation reveals a necessary correction.

- [ ] **Step 1: Add README**

Create `README.md`:

```markdown
# dotfiles

Personal dotfiles managed by `make` and the internal Rust backend `dotman`.

## Commands

- `make build`: build the Rust backend without running any environment action.
- `make bootstrap`: build `dotman`, check manifests, install missing dependencies, link dotfiles, and run doctor.
- `make link`: link managed files from `dotfiles.toml`.
- `make link DRY_RUN=1`: preview link actions.
- `make link CONFLICT=fail`: fail on target conflicts.
- `make link CONFLICT=backup`: backup target conflicts before linking.
- `make link CONFLICT=overwrite`: overwrite target conflicts before linking.
- `make doctor`: inspect current machine state.
- `make check`: validate manifests and host support.

## Layout

- `config/`: source dotfiles.
- `dotfiles.toml`: managed file mappings.
- `deps.toml`: dependency installer policy.
- `src/`: Rust backend source.

## Development Dependencies

- Rust toolchain with `cargo`.
- GNU Make.
- Git.

## CI

CI is deferred from the first runnable slice. The first CI target should run `cargo test` and `make check` on macOS plus Ubuntu/Debian, covering both `arm64` and `x86_64` where runners are available.
```

- [ ] **Step 2: Run final verification**

Run:

```bash
cargo test
make check
make link DRY_RUN=1
git status --short
```

Expected:

```text
cargo test passes
make check passes
dry-run output is grouped
only intended docs changes are uncommitted before final commit
```

- [ ] **Step 3: Commit**

```bash
git add README.md docs/superpowers/specs/2026-05-10-rust-env-manager-rebuild-design.md
git commit -m "docs: document dotman workflow"
```
