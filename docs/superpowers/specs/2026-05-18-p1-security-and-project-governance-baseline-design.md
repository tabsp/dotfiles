# Security And Project Governance Baseline Design

## Goal

Add minimal security and governance baseline files to the repository: security
policy, trust boundary documentation for remote-code installers, and a release
checklist. This addresses the open risk register item for `official_script`
trust boundaries and closes the governance gap identified in the roadmap.

## Motivation

Risk register: `official_script` HTTPS downloads are executed after download
without documented trust policy (medium severity). The repository also lacks
SECURITY.md, a license file, and a release checklist — all expected by
open-source community standards.

## Scope

- Add `SECURITY.md` with supported versions, reporting process, and trust
  boundaries for installers that execute remote code.
- Add `LICENSE` file (MIT, matching the project's implicit licensing).
- Add `docs/release-checklist.md` covering pre-release verification steps.
- Document trust policy for `official_script` and `download_binary` installers.
- Close the linked risk register item.

## Non-Goals

- Do not change installer behavior or add runtime enforcement of trust policies.
- Do not add a full vulnerability disclosure program or bug bounty.
- Do not add contribution guides (deferred to future work).

## Design

### SECURITY.md

Standard sections:
- **Supported Versions:** `0.1.x` (current), policy for future versions.
- **Reporting a Vulnerability:** email or private GitHub advisory.
- **Trust Boundaries:**
  - `official_script`: remote HTTPS scripts are downloaded and executed
    locally. Scripts run with user privileges. Users should review deps.toml
    entries before running bootstrap.
  - `download_binary`: pre-built binaries from HTTPS URLs are checksum-
    verified before installation. Users should verify checksum sources.
  - `system` / `apt` / `brew`: delegate to OS package manager trust model.
- **Scope:** dotman CLI, installer script, release artifacts.

### LICENSE

MIT License. Copyright notice with project name and year.

### docs/release-checklist.md

Checklist for maintainers before tagging a release:
- [ ] `make ci` passes
- [ ] `make smoke-test` passes
- [ ] `CHANGELOG.md` updated
- [ ] Version bumped in `Cargo.toml`
- [ ] `docs/release-policy.md` reviewed for compatibility
- [ ] Release artifacts workflow triggered with correct tag

### docs/trust-boundaries.md

Document the trust model for each installer type:
| Installer | Trust Model | User Action Required |
|-----------|------------|---------------------|
| download_binary | Checksum-verified HTTPS download | Review deps.toml URL and sha256 |
| official_script | Remote HTTPS script execution | Review script source before bootstrap |
| brew / cask | Delegates to Homebrew | Trust Homebrew's package verification |
| apt / system / repo_package | Delegates to OS package manager | Trust OS package signing |
| ppa | Adds third-party APT repository | Review PPA source before bootstrap |

### Risk Register

Close the `official_script` trust boundary risk item. The trust policy
documentation addresses the risk by making the trust model explicit.

### Verification Strategy

- Review SECURITY.md, LICENSE, release-checklist.md, trust-boundaries.md for
  correctness and completeness.
- `cargo test` passes (no code changes).
- `make check` passes.

### Regression Coverage Expectations

- No code changes; all existing tests pass.

## Error Handling

Not applicable — no code changes. Documentation files are hand-reviewed for
correctness.
