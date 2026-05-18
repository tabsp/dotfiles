# Trust Boundaries

This document describes the trust model for each installer type in dotman.
Users should review these boundaries before adding dependencies to `deps.toml`
or running `dotman bootstrap`.

## Installer Trust Models

| Installer | Trust Model | User Action |
|-----------|------------|-------------|
| `download_binary` | HTTPS download + mandatory SHA-256 checksum verification. Both binary and checksum must download and verify successfully. | Review `url` and `sha256` in deps.toml. Verify checksum source is the official project release. |
| `official_script` | Remote HTTPS script is downloaded and executed with user privileges. Script runs as the current user with full access to user-writable paths. | Review the script source at the URL before adding to deps.toml. Verify the domain is the official project domain. |
| `brew` | Delegates to Homebrew. Dotman runs `brew install`. Homebrew verifies bottle signatures and formula integrity. | Trust Homebrew's package verification. Review formula source if concerned. |
| `cask` | Delegates to Homebrew Cask. Downloads from vendor URLs with SHA-256 verification where available. | Trust Homebrew Cask's verification. Review cask source if concerned. |
| `apt` | Delegates to APT. Dotman runs `apt-get install`. APT verifies package signatures via GPG. | Trust Debian/Ubuntu package signing. |
| `system` | Delegates to OS package manager. Dotman runs the system's native install command. | Trust the OS package manager's verification. |
| `repo_package` | Adds a third-party APT repository, then installs via `apt-get`. Repository signing key is added to APT keyring. | Review the repository URL and signing key before adding to deps.toml. |
| `ppa` | Adds a Launchpad PPA, then installs via `apt-get`. PPA packages are signed by Launchpad. | Review the PPA source before adding to deps.toml. |

## Install Script

The release install script (`scripts/install.sh`) downloads dotman binaries and
dotfiles source archives from GitHub Releases. Both downloads are verified with
mandatory SHA-256 checksums. Checksum download failures are fatal.

## Manifest Validation

`dotman check` validates `deps.toml` and `dotfiles.toml` before any machine
state changes:

- All URLs must use HTTPS.
- Required parameters are validated per installer type.
- Platform-specific entries are validated for OS/distro constraints.
- `install_to` paths must be under `$HOME/.local/bin/`.

## Reporting

See [SECURITY.md](../SECURITY.md) for vulnerability reporting.
