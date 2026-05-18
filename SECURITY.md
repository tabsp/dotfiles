# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | ✅ |

Pre-1.0 versions receive security fixes. Breaking changes are called out in the
changelog.

## Reporting a Vulnerability

To report a security vulnerability, please open a
[private security advisory](https://github.com/tabsp/dotfiles/security/advisories/new)
on GitHub. Do not open a public issue.

We aim to acknowledge reports within 48 hours and provide an initial assessment
within 5 business days.

## Trust Boundaries

Dotman manages machine state. The following installers execute third-party code
or download third-party binaries. Users should understand these trust boundaries
before running `dotman bootstrap`.

### official_script

HTTPS scripts are downloaded and executed with user privileges. The script URL
and arguments are defined in `deps.toml`. Before running bootstrap with
`official_script` entries, review the script source and verify the URL points
to the official project domain.

### download_binary

Pre-built binaries are downloaded from HTTPS URLs and checksum-verified with
SHA-256 before installation. The URL and expected checksum are defined in
`deps.toml`. Both the binary download and checksum download must succeed for
installation to proceed.

### System Package Managers (brew, apt, system, repo_package, ppa)

These installers delegate trust to the operating system's package manager.
Dotman does not verify package signatures or repository authenticity beyond
what the package manager provides.

### Install Script

The release install script (`scripts/install.sh`) downloads dotman binaries and
dotfiles source archives from GitHub Releases with mandatory SHA-256 checksum
verification. Both the binary and source archive checksums are verified before
installation.

## Scope

This policy covers:

- The `dotman` CLI binary and its dependencies
- The release install script (`scripts/install.sh`)
- The release artifact workflow (`.github/workflows/release-artifacts.yml`)
- Manifest validation (`deps.toml`, `dotfiles.toml`)

Out of scope:

- Third-party tools installed by dotman
- Operating system vulnerabilities in package managers
- User-modified manifest files
