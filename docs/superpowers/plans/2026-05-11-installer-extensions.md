# Installer Extensions Implementation Plan

**Goal:** Implement executable support for `download_binary`,
`official_script`, and `repo_package` under the constraints defined in:

- `docs/superpowers/specs/2026-05-10-rust-env-manager-rebuild-design.md`
- `docs/superpowers/specs/2026-05-11-installer-extensions-design.md`

This plan uses the same level of detail as the main rebuild plan: explicit
crate versions, concrete file skeletons, fixture strategy, verification
commands, and expected outcomes.

## Task 1: Add shared helper modules and dependencies

**Files:**
- Modify: `Cargo.toml`
- Create: `src/http.rs`
- Create: `src/archive.rs`
- Create: `src/process.rs`
- Modify: `src/installers.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Expand Cargo dependencies**

Update `Cargo.toml` like this:

```toml
[dependencies]
clap = { version = "4.5", features = ["derive"] }
flate2 = "1.0"
regex = "1.10"
reqwest = { version = "0.12", default-features = false, features = ["blocking", "rustls-tls"] }
serde = { version = "1.0", features = ["derive"] }
sha2 = "0.10"
tar = "0.4"
tempfile = "3.10"
time = { version = "0.3", features = ["formatting", "local-offset", "macros"] }
toml = "0.8"
xz2 = "0.1"
zip = { version = "2.2", default-features = false, features = ["deflate"] }

[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.1"
tiny_http = "0.12"
```

Notes:

- move `tempfile` from `[dev-dependencies]` to `[dependencies]` because
  runtime installers need it
- use `reqwest` blocking client rather than `ureq` because redirect policy,
  TLS handling, and testability are stronger

- [ ] **Step 2: Add HTTP helper**

Create `src/http.rs` with this shape:

```rust
use reqwest::blocking::Client;
use reqwest::redirect::Policy;
use std::time::Duration;

pub struct DownloadedFile {
    pub final_url: String,
    pub bytes: Vec<u8>,
}

pub fn download_https(url: &str, allow_redirects: bool) -> Result<DownloadedFile, String> {
    validate_manifest_https(url)?;
    let client = Client::builder()
        .redirect(if allow_redirects { Policy::limited(10) } else { Policy::none() })
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|err| format!("failed to build http client: {err}"))?;
    let response = client.get(url).send().map_err(|err| format!("download failed: {err}"))?;
    let final_url = response.url().to_string();
    validate_final_https(&final_url)?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("download returned {status}"));
    }
    let bytes = response.bytes().map_err(|err| format!("failed to read response body: {err}"))?;
    Ok(DownloadedFile { final_url, bytes: bytes.to_vec() })
}

pub fn validate_manifest_https(url: &str) -> Result<(), String> { /* ... */ }
pub fn validate_final_https(url: &str) -> Result<(), String> { /* ... */ }
```

- [ ] **Step 3: Add archive helper**

Create `src/archive.rs` with this shape:

```rust
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

pub enum ArchiveKind {
    Raw,
    TarGz,
    TarXz,
    Zip,
}

pub fn parse_archive_kind(value: &str) -> Result<ArchiveKind, String> { /* ... */ }

pub fn unpack(bytes: &[u8], kind: &ArchiveKind, dest: &Path) -> Result<Option<PathBuf>, String> {
    match kind {
        ArchiveKind::Raw => {
            let path = dest.join("downloaded-binary");
            fs::write(&path, bytes).map_err(|err| format!("failed to write raw payload: {err}"))?;
            Ok(Some(path))
        }
        ArchiveKind::TarGz => { /* tar::Archive + flate2::read::GzDecoder */ }
        ArchiveKind::TarXz => { /* tar::Archive + xz2::read::XzDecoder */ }
        ArchiveKind::Zip => { /* zip::ZipArchive */ }
    }
}
```

- [ ] **Step 4: Add child-process helper**

Create `src/process.rs` with this shape:

```rust
use std::process::{Command, Output};

pub fn run_capture(command: &str, args: &[&str]) -> Result<Output, String> { /* ... */ }

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

fn truncate_bytes(bytes: &[u8]) -> String { /* max 8 KiB */ }
```

- [ ] **Step 5: Add installer cleanup helper**

Extend `src/installers.rs` with a helper like:

```rust
fn cleanup_temp(temp: tempfile::TempDir, succeeded: bool) -> Result<(), String> {
    match temp.close() {
        Ok(()) => Ok(()),
        Err(err) if succeeded => {
            eprintln!("warn: failed to remove temporary directory: {err}");
            Ok(())
        }
        Err(err) => Err(format!("failed to remove temporary directory: {err}")),
    }
}
```

- [ ] **Step 6: Wire helper modules into CLI**

Modify `src/main.rs`:

```rust
mod archive;
mod http;
mod process;
```

- [ ] **Step 7: Verify helper integration**

Run:

```bash
cargo test
```

Expected:

```text
Cargo resolves the new crates
test build exits 0
```

## Task 2: Implement `download_binary`

**Files:**
- Modify: `src/installers.rs`
- Modify: `src/check.rs`
- Create: `tests/download_binary.rs`

- [ ] **Step 1: Add `download_binary` installer code path**

Extend `src/installers.rs` with helpers like:

```rust
fn install_download_binary(command: &str, entry: &InstallEntry) -> Result<(), String> {
    let url = required_string(entry, "url")?;
    let sha256 = required_string(entry, "sha256")?;
    let archive_kind = crate::archive::parse_archive_kind(required_string(entry, "archive_kind")?)?;
    let binary_path = required_string(entry, "binary_path")?;
    let install_to = crate::path::expand_home(required_string(entry, "install_to")?)?;

    match existing_install_state(&install_to)? {
        ExistingInstall::Installed => return Ok(()),
        ExistingInstall::Invalid(kind) => {
            return Err(format!("download_binary invalid install_to={} kind={kind}", install_to.display()));
        }
        ExistingInstall::Missing => {}
    }

    let temp = tempfile::tempdir().map_err(|err| format!("failed to create temp dir: {err}"))?;
    let result = (|| {
    let downloaded = crate::http::download_https(url, true)?;
    verify_sha256(&downloaded.bytes, sha256)?;
    let payload = crate::archive::unpack(&downloaded.bytes, &archive_kind, temp.path())?;
    // `binary_path` is still required by the main spec even when `archive_kind = "raw"`.
    // The raw branch ignores it at runtime but validation still requires the field.
    let binary = resolve_binary_path(temp.path(), payload.as_deref(), &archive_kind, binary_path)?;
    install_binary(&binary, &install_to)?;
    Ok(())
    })();
    cleanup_temp(temp, result.is_ok())
}
```

- [ ] **Step 2: Add `download_binary` manifest validation**

Extend `src/check.rs` with installer-specific checks:

```rust
match entry.installer {
    Installer::DownloadBinary => {
        require_https_param(name, entry, "url", &mut errors);
        require_string_param(name, entry, "sha256", &mut errors);
        require_string_param(name, entry, "archive_kind", &mut errors);
        require_string_param(name, entry, "binary_path", &mut errors);
        require_string_param(name, entry, "install_to", &mut errors);
        validate_archive_kind(name, entry, &mut errors);
    }
    _ => {}
}
```

- [ ] **Step 3: Add HTTP fixture strategy**

Use `tiny_http` in `tests/download_binary.rs` to create a local test server with
handlers for:

- `/raw/example`
- `/zip/example`
- `/tar-gz/example`
- `/tar-xz/example`
- `/redirect/example`

Each handler should write exact bytes for a known fixture so SHA256 assertions
can be deterministic.

- [ ] **Step 4: Add `download_binary` tests**

Create `tests/download_binary.rs` covering:

- raw payload install
- `zip` archive install
- `tar.gz` archive install
- `tar.xz` archive install
- bad SHA256 failure
- redirect to another `https` URL on the same local server
- pre-existing invalid `install_to` (directory or non-executable file)

Use temp directories for:

- repo root
- fake home
- install target

- [ ] **Step 5: Verify `download_binary`**

Run:

```bash
cargo test --test download_binary
cargo test
```

Expected:

```text
download_binary tests pass
full cargo test still exits 0
```

## Task 3: Implement `official_script`

**Files:**
- Modify: `src/installers.rs`
- Modify: `src/check.rs`
- Create: `tests/official_script.rs`

- [ ] **Step 1: Add `official_script` installer code path**

Extend `src/installers.rs` with helpers like:

```rust
fn install_official_script(command: &str, entry: &InstallEntry) -> Result<(), String> {
    let script_url = required_string(entry, "script_url")?;
    let args = string_list_param(entry, "args")?;
    let install_to = optional_install_to(entry)?;

    if let Some(path) = &install_to {
        match existing_install_state(path)? {
            ExistingInstall::Installed => return Ok(()),
            ExistingInstall::Invalid(kind) => {
                return Err(format!("official_script invalid install_to={} kind={kind}", path.display()));
            }
            ExistingInstall::Missing => {}
        }
    } else if crate::path::which(command).is_some() {
        return Ok(());
    }

    let temp = tempfile::tempdir().map_err(|err| format!("failed to create temp dir: {err}"))?;
    let script_path = temp.path().join("install.sh");
    let result = (|| {
        let downloaded = crate::http::download_https(script_url, false)?;
        std::fs::write(&script_path, &downloaded.bytes).map_err(|err| format!("failed to write script: {err}"))?;
        set_executable(&script_path)?;
        let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
        let output = crate::process::run_capture(script_path.to_str().unwrap(), &arg_refs)?;
        if !output.status.success() {
            return Err(with_child_output("official_script failed", &output));
        }
        Ok(())
    })();
    cleanup_temp(temp, result.is_ok())
}
```

- [ ] **Step 2: Add `official_script` manifest validation**

Extend `src/check.rs`:

```rust
match entry.installer {
    Installer::OfficialScript => {
        require_https_param(name, entry, "script_url", &mut errors);
        validate_string_array_param(name, entry, "args", &mut errors);
        validate_optional_install_to(name, entry, &mut errors);
    }
    _ => {}
}
```

- [ ] **Step 3: Add script fixture strategy**

Create fake script bodies in `tests/official_script.rs` using the same pattern
already used in doctor tests:

```rust
std::fs::write(&script, "#!/bin/sh\nexit 0\n")?;
std::fs::write(&script_fail, "#!/bin/sh\necho boom >&2\nexit 12\n")?;
```

Serve them over `tiny_http` so the installer still exercises
`download -> temp file -> execute`.

- [ ] **Step 4: Add `official_script` tests**

Create `tests/official_script.rs` covering:

- successful install when `install_to` becomes executable
- command fallback success when `install_to` is absent
- non-zero script exit
- bad script URL
- invalid existing `install_to`

- [ ] **Step 5: Verify `official_script`**

Run:

```bash
cargo test --test official_script
cargo test
```

Expected:

```text
official_script tests pass
full cargo test still exits 0
```

## Task 4: Implement `repo_package`

**Files:**
- Modify: `src/installers.rs`
- Modify: `src/check.rs`
- Create: `tests/repo_package.rs`

- [ ] **Step 1: Add `repo_package` installer code path**

Extend `src/installers.rs` with helpers like:

```rust
fn install_repo_package(command: &str, entry: &InstallEntry) -> Result<(), String> {
    let package = required_string(entry, "package")?;
    let repo_url = required_string(entry, "repo_url")?;
    let repo_key_url = required_string(entry, "repo_key_url")?;
    let repo_channel = required_string(entry, "repo_channel")?;
    let repo_components = string_list_param(entry, "repo_components")?;

    if package_command("dpkg", &["-s", package])? {
        return Ok(());
    }

    let keyring_path = format!("/usr/share/keyrings/{package}-dotman.gpg");
    let sources_path = format!("/etc/apt/sources.list.d/{package}-dotman.list");
    let source_line = format!(
        "deb [signed-by={}] {} {} {}\n",
        keyring_path,
        repo_url,
        repo_channel,
        repo_components.join(" ")
    );

    let key_bytes = fetch_repo_key(repo_key_url)?;
    write_privileged_if_changed(&keyring_path, &key_bytes)?;
    write_privileged_if_changed(&sources_path, source_line.as_bytes())?;
    run_sudo("apt-get", &["update"])?;
    run_sudo("apt-get", &["install", "-y", package])
}
```

- [ ] **Step 2: Add `repo_package` manifest validation**

Extend `src/check.rs`:

```rust
match entry.installer {
    Installer::RepoPackage => {
        require_string_param(name, entry, "package", &mut errors);
        require_https_param(name, entry, "repo_url", &mut errors);
        require_https_param(name, entry, "repo_key_url", &mut errors);
        require_string_param(name, entry, "repo_channel", &mut errors);
        require_non_empty_string_array(name, entry, "repo_components", &mut errors);
    }
    _ => {}
}
```

- [ ] **Step 3: Add `repo_package` test strategy**

`tests/repo_package.rs` should stay unit-heavy and non-destructive:

- test derived keyring path
- test derived sources path
- test generated `deb [signed-by=...] ...` line
- test `dpkg -s` precheck skip path by stubbing command execution behind a
  small function boundary
- test distro rejection by constructing unsupported `Host`
- test armored key path chooses `gpg --dearmor`

Do not attempt real `sudo apt-get` integration in this task.

- [ ] **Step 4: Verify `repo_package`**

Run:

```bash
cargo test --test repo_package
cargo test
```

Expected:

```text
repo_package tests pass
full cargo test still exits 0
```

## Task 5: Integrate into bootstrap and manifests

**Files:**
- Modify: `deps.toml`
- Modify: `src/deps.rs`
- Modify: `src/doctor.rs`

- [ ] **Step 1: Keep default manifest runnable**

Do not add active default entries that force the current local bootstrap path
through unimplemented or privileged-only installers.

Default `deps.toml` should remain runnable on the current repo and continue to
use only:

- `system`
- `brew`
- `cask`
- `apt`

- [ ] **Step 2: Add commented representative examples**

Append non-active examples to `deps.toml` for:

- one `download_binary` Linux entry
- one `official_script` macOS entry
- one `repo_package` Linux entry

Use them as documented examples only, commented out so bootstrap behavior for
the current repo remains unchanged.

- [ ] **Step 3: Verify integration**

Run:

```bash
cargo test
make check
make doctor
make link DRY_RUN=1
make bootstrap
```

Expected:

```text
cargo test passes
make check exits 0
make doctor exits 0 on the now-correct local links
make link DRY_RUN=1 prints grouped WouldLink output
make bootstrap remains idempotent for the current default manifest
```

## Task 6: Commit sequence

- [ ] Commit shared helpers
- [ ] Commit `download_binary`
- [ ] Commit `official_script`
- [ ] Commit `repo_package`
- [ ] Commit integration updates
