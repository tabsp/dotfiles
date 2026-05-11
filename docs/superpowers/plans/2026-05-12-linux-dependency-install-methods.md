# Linux Dependency Install Methods Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:executing-plans` to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Spec:** `docs/superpowers/specs/2026-05-12-linux-dependency-install-methods-design.md`

**Goal:** Implement the reviewed Linux dependency install methods in `dotman`
and `deps.toml`: Ubuntu PPA support, distro-filtered dependency entries,
portable user-local PATH setup, and pinned Linux binary downloads.

**Architecture:** Keep `make` as the user entry point and keep `dotman` as the
Rust backend. Extend the current manifest model with `distros` and `ppa`, then
reuse the existing installer validation, dependency selection, and bootstrap
flow. Do not run `make bootstrap` during verification unless explicitly
requested because it can install system packages.

**Tech Stack:** Rust 2024, `serde`/`toml`, existing `reqwest` download path,
existing archive extraction code, fish shell configuration, Makefile targets.

---

## Existing Code Map

- `src/config.rs`: manifest data model. Add `InstallEntry.distros` and
  `Installer::Ppa` here. Keep raw platform/arch entry lookup separate from
  distro filtering so `check` can produce precise diagnostics.
- `src/check.rs`: static manifest validation. Add `distros` rules and `ppa`
  parameter/platform validation here.
- `src/deps.rs`: bootstrap dependency loop. Select the current-host entry by
  first getting raw platform/arch entries and then applying distro filters.
- `src/installers.rs`: installed-state checks and install execution. Add `ppa`
  install behavior and `official_script.args` leading-`~/` expansion here.
- `src/platform.rs`: host detection already exposes `Host.distro`; do not use
  `distro_supported()` for PPA Ubuntu validation.
- `config/fish/config.fish`: portable PATH setup for `$HOME/.local/bin` and
  `$HOME/.cargo/bin`.
- `deps.toml`: update Linux installer choices and add pinned binary entries.
- `tests/common/mod.rs`: add helpers for Linux-specific manifest snippets.
- `tests/cli_check.rs`: CLI-level validation tests.
- `src/installers.rs` unit tests: installer helper tests that do not need to
  invoke real `sudo`, network, or package managers.

## Pinned Linux Binary Data

Use these exact values when updating `deps.toml`.

### `nvim`

- Version: `0.12.2`
- `version_check.args`: `["--version"]`
- `version_check.stream`: `stdout`
- `version_check.regex`: `NVIM v?([0-9]+\.[0-9]+\.[0-9]+)`
- x86_64 URL:
  `https://github.com/neovim/neovim/releases/download/v0.12.2/nvim-linux-x86_64.tar.gz`
- x86_64 SHA256:
  `31cf85945cb600d96cdf69f88bc68bec814acbff50863c5546adef3a1bcef260`
- x86_64 `archive_kind`: `tar.gz`
- x86_64 `binary_path`: `nvim-linux-x86_64/bin/nvim`
- arm64 URL:
  `https://github.com/neovim/neovim/releases/download/v0.12.2/nvim-linux-arm64.tar.gz`
- arm64 SHA256:
  `f697d4e4582b6e4b5c3c26e76e06ce26efa08ba1768e03fd2733fcc422bb0490`
- arm64 `archive_kind`: `tar.gz`
- arm64 `binary_path`: `nvim-linux-arm64/bin/nvim`
- `install_to`: `~/.local/bin/nvim`

### `yazi`

- Version: `26.5.6`
- `version_check.args`: `["--version"]`
- `version_check.stream`: `stdout`
- `version_check.regex`: `yazi ([0-9]+\.[0-9]+\.[0-9]+)`
- x86_64 URL:
  `https://github.com/sxyazi/yazi/releases/download/v26.5.6/yazi-x86_64-unknown-linux-gnu.zip`
- x86_64 SHA256:
  `1c9096f0a83b8102c194385f644cdeff93cc8269426163c9d033041ebd537bd2`
- x86_64 `archive_kind`: `zip`
- x86_64 `binary_path`: `yazi-x86_64-unknown-linux-gnu/yazi`
- arm64 URL:
  `https://github.com/sxyazi/yazi/releases/download/v26.5.6/yazi-aarch64-unknown-linux-gnu.zip`
- arm64 SHA256:
  `c38b07961e7fc4c76503fd0f4a1b4bd0b379a99835b818cd899b0315c728e1e1`
- arm64 `archive_kind`: `zip`
- arm64 `binary_path`: `yazi-aarch64-unknown-linux-gnu/yazi`
- `install_to`: `~/.local/bin/yazi`

### `eza`

- Version: `0.23.4`
- `version_check.args`: `["--version"]`
- `version_check.stream`: `stdout`
- `version_check.regex`:
  `eza(?: - A modern, maintained replacement for ls)? v?([0-9]+\.[0-9]+\.[0-9]+)`
- x86_64 URL:
  `https://github.com/eza-community/eza/releases/download/v0.23.4/eza_x86_64-unknown-linux-gnu.tar.gz`
- x86_64 SHA256:
  `0c38665440226cd8bef5d1d4f3bc6ff77c927fb0d68b752739105db7ab5b358d`
- x86_64 `archive_kind`: `tar.gz`
- x86_64 `binary_path`: `eza`
- arm64 URL:
  `https://github.com/eza-community/eza/releases/download/v0.23.4/eza_aarch64-unknown-linux-gnu.tar.gz`
- arm64 SHA256:
  `366e8430225f9955c3dc659b452150c169894833ccfef455e01765e265a3edda`
- arm64 `archive_kind`: `tar.gz`
- arm64 `binary_path`: `eza`
- `install_to`: `~/.local/bin/eza`

### `lazygit`

- Version: `0.61.1`
- `version_check.args`: `["--version"]`
- `version_check.stream`: `stdout`
- `version_check.regex`: `version=([0-9]+\.[0-9]+\.[0-9]+)`
- x86_64 URL:
  `https://github.com/jesseduffield/lazygit/releases/download/v0.61.1/lazygit_0.61.1_linux_x86_64.tar.gz`
- x86_64 SHA256:
  `1b91e660700f2332696726b635202576b543e2bc49b639830dccd26bc5160d5d`
- x86_64 `archive_kind`: `tar.gz`
- x86_64 `binary_path`: `lazygit`
- arm64 URL:
  `https://github.com/jesseduffield/lazygit/releases/download/v0.61.1/lazygit_0.61.1_linux_arm64.tar.gz`
- arm64 SHA256:
  `20b1abb2bee5dfd46173b9047353eb678bc51a23839e821958d0b1863ab1655e`
- arm64 `archive_kind`: `tar.gz`
- arm64 `binary_path`: `lazygit`
- `install_to`: `~/.local/bin/lazygit`

### `fzf`

- Version: `0.72.0`
- `version_check.args`: `["--version"]`
- `version_check.stream`: `stdout`
- `version_check.regex`: `([0-9]+\.[0-9]+\.[0-9]+)`
- x86_64 URL:
  `https://github.com/junegunn/fzf/releases/download/v0.72.0/fzf-0.72.0-linux_amd64.tar.gz`
- x86_64 SHA256:
  `0e58e4bd0b3c5d68c56b54c460a6863d0de79633ed18d388575a960ab447b006`
- x86_64 `archive_kind`: `tar.gz`
- x86_64 `binary_path`: `fzf`
- arm64 URL:
  `https://github.com/junegunn/fzf/releases/download/v0.72.0/fzf-0.72.0-linux_arm64.tar.gz`
- arm64 SHA256:
  `a0a5b50730f568c5f08b8dbba1e6e598db253e1856d371290086786b889b996b`
- arm64 `archive_kind`: `tar.gz`
- arm64 `binary_path`: `fzf`
- `install_to`: `~/.local/bin/fzf`

---

## Task 1: Add Manifest Support for `distros` and `ppa`

**Files:**

- Modify: `src/config.rs`
- Modify: `src/check.rs`
- Modify: `src/deps.rs`
- Modify: `tests/common/mod.rs`
- Modify: `tests/cli_check.rs`

- [ ] **Step 1: Add failing check tests for distro filtering**

Add tests to `tests/cli_check.rs` that create Linux manifests explicitly rather
than using the current host helper. These tests should exercise the validation
without running installers.

Add helper snippets to `tests/common/mod.rs`:

```rust
pub fn minimal_dotfiles() -> &'static str {
    r#"
[[files]]
source = "config/tmux.conf"
target = "~/.tmux.conf"
kind = "file"
"#
}

pub fn write_minimal_dotfiles(repo: &std::path::Path) {
    std::fs::create_dir_all(repo.join("config")).expect("config");
    std::fs::write(repo.join("config/tmux.conf"), "set -g mouse on\n").expect("source");
    std::fs::write(repo.join("dotfiles.toml"), minimal_dotfiles()).expect("dotfiles");
}
```

Add CLI tests:

```rust
#[test]
fn check_rejects_distros_on_mac_entry() {
    let temp = tempfile::tempdir().expect("tempdir");
    common::write_minimal_dotfiles(temp.path());
    std::fs::write(
        temp.path().join("deps.toml"),
        r#"
[deps.fish]
command = "fish"

[deps.fish.mac.arm64]
installer = "brew"
version = "latest"
distros = ["ubuntu"]

[deps.fish.mac.arm64.params]
package = "fish"
"#,
    )
    .expect("deps");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .arg("check")
        .assert()
        .failure()
        .stderr(predicate::str::contains("distros is only valid on linux entries"));
}
```

Add unit tests in `src/config.rs` instead of introducing a public library only
for integration-test access:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{Arch, Host, Platform};

    fn entry_with_distros(distros: Option<Vec<&str>>) -> InstallEntry {
        InstallEntry {
            installer: Installer::Ppa,
            version: "latest".to_string(),
            source: None,
            distros: distros.map(|items| items.into_iter().map(str::to_string).collect()),
            params: BTreeMap::new(),
        }
    }

    fn linux_host(distro: &str) -> Host {
        Host {
            platform: Platform::Linux,
            arch: Arch::X86_64,
            distro: Some(distro.to_string()),
        }
    }

    #[test]
    fn entry_without_distros_matches_any_distro() {
        assert!(entry_with_distros(None).matches_distro(&linux_host("debian")));
    }

    #[test]
    fn entry_with_matching_distro_matches() {
        assert!(entry_with_distros(Some(vec!["ubuntu"])).matches_distro(&linux_host("ubuntu")));
    }

    #[test]
    fn entry_with_non_matching_distro_does_not_match() {
        assert!(!entry_with_distros(Some(vec!["ubuntu"])).matches_distro(&linux_host("debian")));
    }

    #[test]
    fn empty_distros_matches_no_distro() {
        assert!(!entry_with_distros(Some(vec![])).matches_distro(&linux_host("ubuntu")));
    }
}
```

- [ ] **Step 2: Run tests and verify they fail for the expected reason**

Run:

```sh
cargo test distros check_rejects_distros_on_mac_entry
```

Expected before implementation:

- The tests fail because `distros` is not modeled, `ppa` is unknown, or
  `matches_distro` does not exist yet.

- [ ] **Step 3: Extend the manifest model**

In `src/config.rs`, add:

```rust
#[derive(Debug, Deserialize)]
pub struct InstallEntry {
    pub installer: Installer,
    pub version: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub distros: Option<Vec<String>>,
    #[serde(default)]
    pub params: BTreeMap<String, toml::Value>,
}
```

Add `Ppa` to `Installer`:

```rust
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Installer {
    System,
    Brew,
    Cask,
    Apt,
    RepoPackage,
    Ppa,
    OfficialScript,
    DownloadBinary,
}
```

Keep `Dependency::entries_for(platform, arch)` as the raw platform/arch lookup.
Do not hide distro filtering inside it, because `check` must distinguish these
two cases:

- no entry exists for the current platform/architecture
- an entry exists for the current platform/architecture but is filtered out by
  `distros`

Add a filtered helper instead:

```rust
impl Dependency {
    pub fn entries_for_host<'a>(
        &'a self,
        platform: &str,
        arch: &str,
        host: &crate::platform::Host,
    ) -> Vec<&'a InstallEntry> {
        self.entries_for(platform, arch)
            .into_iter()
            .filter(|entry| entry.matches_distro(host))
            .collect()
    }
}

impl InstallEntry {
    pub fn matches_distro(&self, host: &crate::platform::Host) -> bool {
        let Some(distros) = &self.distros else {
            return true;
        };
        if host.platform != crate::platform::Platform::Linux || distros.is_empty() {
            return false;
        }
        let Some(distro) = host.distro.as_deref() else {
            return false;
        };
        distros.iter().any(|item| item == distro)
    }
}
```

Use `entries_for_host(host.platform.key(), host.arch.key(), host)` in
`src/deps.rs`. Do not keep the current `continue` behavior when no entry
matches. If `deps::install_missing` is called after `check`, this should be
unreachable; if it is called directly in the future, it should fail clearly
instead of silently skipping a dependency:

```rust
let entries = dep.entries_for_host(host.platform.key(), host.arch.key(), host);
let Some(entry) = entries.first() else {
    let detail = if host.platform == crate::platform::Platform::Linux {
        format!(
            " for distro {}",
            host.distro.as_deref().unwrap_or("unknown")
        )
    } else {
        String::new()
    };
    return Err(format!("dependency {name} has no current-host entry{detail}"));
};
```

- [ ] **Step 4: Validate `distros` and PPA platform constraints**

In `src/check.rs`:

- Reject `distros` when validating any macOS entry before current-host entry
  selection. This must scan all manifest entries, not only entries selected for
  the current host.
- Treat `Installer::Ppa` as Linux-only.
- Treat `Installer::Ppa` as Ubuntu-only when it is the selected current-host
  entry.
- When no current-host entry matches on Linux and there is a same-arch Linux
  entry filtered by distro, report:

```text
dependency fish has no current-host entry for distro debian
```

Implement current-host selection in `run_check` in two phases:

```rust
let raw_entries = dep.entries_for(host.platform.key(), host.arch.key());
let entries: Vec<_> = raw_entries
    .iter()
    .copied()
    .filter(|entry| entry.matches_distro(host))
    .collect();

match entries.as_slice() {
    [] if host.platform == Platform::Linux && !raw_entries.is_empty() => {
        let distro = host.distro.as_deref().unwrap_or("unknown");
        errors.push(format!(
            "dependency {name} has no current-host entry for distro {distro}"
        ));
    }
    [] => errors.push(format!("dependency {name} has no current-host entry")),
    [entry] => {
        validate_installer_platform(name, entry.installer, host, &mut errors);
        validate_installer_params(name, entry, repo, &mut errors);
        if entry.version != "latest" && dep.version_check.is_none() {
            errors.push(format!(
                "dependency {name} pins version {} but has no version_check",
                entry.version
            ));
        }
        validate_https(name, entry.source.as_deref(), &mut errors);
    }
    _ => errors.push(format!("dependency {name} has multiple current-host entries")),
}
```

Add a separate manifest-wide validation helper:

```rust
fn validate_distros_scope(name: &str, dep: &crate::config::Dependency, errors: &mut Vec<String>) {
    for entry in dep.mac.values() {
        if entry.distros.is_some() {
            errors.push(format!(
                "dependency {name} distros is only valid on linux entries"
            ));
        }
    }
}
```

Call `validate_distros_scope(name, dep, &mut errors)` before entry selection.

- [ ] **Step 5: Run targeted tests**

Run:

```sh
cargo test distros check_rejects_distros_on_mac_entry
```

Expected:

- Distro matching unit tests pass.
- The macOS `distros` rejection CLI test passes.
- The filtered-distro path reports `for distro <detected-distro>` when a raw
  same-platform/same-arch entry exists but does not match the detected distro.
- `deps::install_missing` hard-fails if called directly and no filtered
  current-host entry exists.

---

## Task 2: Implement `ppa` Installer

**Files:**

- Modify: `src/installers.rs`
- Modify: `src/check.rs`
- Modify: `tests/cli_check.rs`
- Add unit tests in `src/installers.rs`

- [ ] **Step 1: Add failing validation tests for PPA params**

In `tests/cli_check.rs`, add tests for:

- missing `ppa`
- missing `package`
- non-string `bootstrap_package`

Use a current-host Linux entry in unit-level check tests when possible. If using
CLI tests on macOS, the test should still assert param validation is surfaced
through `check`.

Expected stderr fragments:

```text
dependency fish missing required param ppa
dependency fish missing required param package
dependency fish param bootstrap_package must be string
```

- [ ] **Step 2: Implement PPA param validation**

In `validate_installer_params`:

```rust
Installer::Ppa => {
    require_string_param(name, entry, "ppa", errors);
    require_string_param(name, entry, "package", errors);
    validate_optional_string_param(name, entry, "bootstrap_package", errors);
}
```

In `validate_installer_platform`, reject `Ppa` outside Linux. Also reject PPA on
Linux when `host.distro.as_deref() != Some("ubuntu")`.

- [ ] **Step 3: Add installer helpers without invoking real system commands in tests**

In `src/installers.rs`, add:

```rust
fn bootstrap_package(entry: &InstallEntry) -> &str {
    string_param(entry, "bootstrap_package").unwrap_or("software-properties-common")
}
```

Add unit tests:

```rust
#[test]
fn ppa_bootstrap_package_defaults_to_software_properties_common() {
    let entry = fake_entry_with(&[]);
    assert_eq!(bootstrap_package(&entry), "software-properties-common");
}

#[test]
fn ppa_bootstrap_package_can_be_overridden() {
    let entry = fake_entry_with(&[(
        "bootstrap_package",
        toml::Value::String("custom-package".to_string()),
    )]);
    assert_eq!(bootstrap_package(&entry), "custom-package");
}
```

- [ ] **Step 4: Implement PPA installed-state and install execution**

Update `is_installed`:

```rust
Installer::Apt | Installer::RepoPackage | Installer::Ppa => {
    package_command("dpkg", &["-s", package(entry)?])
}
```

Update `install_missing`:

```rust
Installer::Ppa => install_ppa(entry, host),
```

Add:

```rust
fn install_ppa(entry: &InstallEntry, host: &Host) -> Result<(), String> {
    if host.platform != Platform::Linux || host.distro.as_deref() != Some("ubuntu") {
        return Err("ppa supports Ubuntu Linux only".to_string());
    }

    let package = package(entry)?;
    let ppa = required_string(entry, "ppa")?;
    let bootstrap = bootstrap_package(entry);

    if !package_command("dpkg", &["-s", bootstrap])? {
        run_capture_checked("sudo", &["apt-get", "install", "-y", bootstrap])?;
    }

    run_capture_checked("sudo", &["add-apt-repository", "-y", ppa])?;
    run_capture_checked("sudo", &["apt-get", "update"])?;
    run_capture_checked("sudo", &["apt-get", "install", "-y", package])
}
```

- [ ] **Step 5: Run targeted tests**

Run:

```sh
cargo test ppa_bootstrap_package check_validates_ppa
```

Expected:

- PPA helper tests pass.
- PPA validation tests pass.

---

## Task 3: Expand `official_script.args` Leading-Home Support

**Files:**

- Modify: `src/installers.rs`

- [ ] **Step 1: Add failing unit tests for arg expansion**

In `src/installers.rs` tests, add:

```rust
#[test]
fn expand_script_arg_expands_leading_home() {
    let home = std::env::var("HOME").expect("HOME");
    assert_eq!(
        expand_script_arg("~/.local/bin").expect("expand"),
        format!("{home}/.local/bin")
    );
}

#[test]
fn expand_script_arg_keeps_env_var_literal() {
    assert_eq!(
        expand_script_arg("$HOME/.local/bin").expect("expand"),
        "$HOME/.local/bin"
    );
}

#[test]
fn expand_script_arg_keeps_embedded_tilde_literal() {
    assert_eq!(
        expand_script_arg("prefix~/path").expect("expand"),
        "prefix~/path"
    );
}
```

- [ ] **Step 2: Implement arg expansion**

Add:

```rust
fn expand_script_arg(arg: &str) -> Result<String, String> {
    if arg.starts_with("~/") {
        return crate::path::expand_home(arg)
            .map(|path| path.to_string_lossy().into_owned());
    }
    Ok(arg.to_string())
}

fn expand_script_args(args: Vec<String>) -> Result<Vec<String>, String> {
    args.iter().map(|arg| expand_script_arg(arg)).collect()
}
```

In `install_official_script`, change:

```rust
let args = string_array_param(entry, "args")?;
```

to:

```rust
let args = expand_script_args(string_array_param(entry, "args")?)?;
```

- [ ] **Step 3: Run targeted tests**

Run:

```sh
cargo test expand_script_arg
```

Expected:

- All three arg expansion tests pass.

---

## Task 4: Update Fish PATH Configuration

**Files:**

- Modify: `config/fish/config.fish`

- [ ] **Step 1: Replace user-local PATH setup**

Replace:

```fish
fish_add_path ~/.local/bin
```

with:

```fish
set -l local_bin "$HOME/.local/bin"
if test -d $local_bin
    fish_add_path $local_bin
end

set -l cargo_bin "$HOME/.cargo/bin"
if test -d $cargo_bin
    fish_add_path $cargo_bin
end
```

Do not create directories from fish config. Keep the existing `/opt/homebrew`
and opencode `$HOME/.opencode/bin` logic.

- [ ] **Step 2: Verify fish syntax**

Run:

```sh
fish --no-config --no-execute config/fish/config.fish
```

Expected:

- Exit code `0`.
- No syntax errors.

---

## Task 5: Update `deps.toml`

**Files:**

- Modify: `deps.toml`

- [ ] **Step 1: Keep foundational tools simple**

Keep:

- `git` Linux arm64/x86_64 as `apt`, version `latest`, package `git`
- `tmux` Linux arm64/x86_64 as `apt`, version `latest`, package `tmux`

Add `make`. On Linux, install with `apt`. On macOS, use `system` entries so
`check` can validate that `make` exists without trying to install or manage it.

```toml
[deps.make]
command = "make"

[deps.make.version_check]
args = ["--version"]
regex = 'GNU Make ([0-9]+\.[0-9]+(?:\.[0-9]+)?)'
stream = "stdout"

[deps.make.mac.arm64]
installer = "system"
version = "latest"

[deps.make.mac.x86_64]
installer = "system"
version = "latest"

[deps.make.linux.arm64]
installer = "apt"
version = "latest"

[deps.make.linux.arm64.params]
package = "make"

[deps.make.linux.x86_64]
installer = "apt"
version = "latest"

[deps.make.linux.x86_64.params]
package = "make"
```

Do not add a macOS package-manager installer for `make`.

- [ ] **Step 2: Move fish Linux entries to Ubuntu PPA**

Use this shape for both Linux arches:

```toml
[deps.fish.linux.x86_64]
installer = "ppa"
version = "latest"
source = "https://fishshell.com/"
distros = ["ubuntu"]

[deps.fish.linux.x86_64.params]
ppa = "ppa:fish-shell/release-4"
package = "fish"
```

Repeat for `linux.arm64`.

- [ ] **Step 3: Add ghostty Linux Ubuntu PPA entries**

Use this shape for both Linux arches:

```toml
[deps.ghostty.linux.x86_64]
installer = "ppa"
version = "latest"
source = "https://ghostty.org/"
distros = ["ubuntu"]

[deps.ghostty.linux.x86_64.params]
ppa = "ppa:mkasberg/ghostty-ubuntu"
package = "ghostty"
```

Repeat for `linux.arm64`.

- [ ] **Step 4: Add zoxide and starship official script entries**

For `zoxide`, add macOS brew entries and Linux official script entries:

```toml
[deps.zoxide]
command = "zoxide"

[deps.zoxide.version_check]
args = ["--version"]
regex = 'zoxide ([0-9]+\.[0-9]+\.[0-9]+)'
stream = "stdout"

[deps.zoxide.linux.x86_64]
installer = "official_script"
version = "latest"
source = "https://github.com/ajeetdsouza/zoxide"

[deps.zoxide.linux.x86_64.params]
script_url = "https://raw.githubusercontent.com/ajeetdsouza/zoxide/main/install.sh"
install_to = "~/.local/bin/zoxide"
```

Repeat Linux entry for `arm64`. Add macOS `brew` entries for both mac arches.

For `starship`, add macOS brew entries and Linux official script entries:

```toml
[deps.starship]
command = "starship"

[deps.starship.version_check]
args = ["--version"]
regex = 'starship ([0-9]+\.[0-9]+\.[0-9]+)'
stream = "stdout"

[deps.starship.linux.x86_64]
installer = "official_script"
version = "latest"
source = "https://starship.rs/"

[deps.starship.linux.x86_64.params]
script_url = "https://starship.rs/install.sh"
args = ["-y", "-b", "~/.local/bin"]
install_to = "~/.local/bin/starship"
```

Repeat Linux entry for `arm64`. Add macOS `brew` entries for both mac arches.

- [ ] **Step 5: Add pinned binary entries**

Add `nvim`, `yazi`, `eza`, `lazygit`, and `fzf` using the data from
**Pinned Linux Binary Data**. Each tool must include:

- `command`
- `version_check`
- macOS brew entries for arm64 and x86_64
- Linux arm64 and x86_64 `download_binary` entries
- fixed `version`
- `source`
- `url`
- `sha256`
- `archive_kind`
- `binary_path`
- `install_to`

- [ ] **Step 6: Run manifest validation**

Run:

```sh
make check
```

Expected:

- Exit code `0`.

---

## Task 6: Full Verification

**Files:**

- No new edits expected.

- [ ] **Step 1: Run formatting and linting**

Run:

```sh
make lint
```

Expected:

- `cargo fmt --check` passes.
- `cargo clippy` passes.
- Shell lint targets pass if they are part of `make lint`.

- [ ] **Step 2: Run tests**

Run:

```sh
make test
```

Expected:

- All Rust tests pass.

- [ ] **Step 3: Run static check**

Run:

```sh
make check
```

Expected:

- Exit code `0`.

- [ ] **Step 4: Do not run bootstrap by default**

Do not run:

```sh
make bootstrap
```

unless the user explicitly requests it. It may install packages through `sudo`,
Homebrew, official scripts, or binary downloads.

---

## Task 7: Commit

**Files:**

- Commit only the implementation files changed by this plan.

- [ ] **Step 1: Review status**

Run:

```sh
git status --short
```

Expected:

- Only files from this plan are modified.

- [ ] **Step 2: Commit after verification passes**

Run:

```sh
git add src/config.rs src/check.rs src/deps.rs src/installers.rs tests/common/mod.rs tests/cli_check.rs config/fish/config.fish deps.toml docs/superpowers/plans/2026-05-12-linux-dependency-install-methods.md
git commit -m "feat: define linux dependency installers"
```

Expected:

- Commit succeeds.

---

## Self-Review

- Spec coverage: PPA installer, distro filtering, `make` steady-state entry,
  fish PATH portability, official script `~/` arg expansion, fixed binary
  versions, SHA256, and `version_check` are all mapped to tasks.
- Deferred by spec: Debian fallback installers, automatic latest version
  resolution, upgrade planning, optional yazi enhancement dependencies.
- Safety: verification excludes `make bootstrap` by default.
- Known execution risk: binary archive `binary_path` values should be confirmed
  during implementation if a download/extraction test is added, but the
  manifest values above are the planned source of truth for v1.
