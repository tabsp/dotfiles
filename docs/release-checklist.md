# Release Checklist

Checklist for maintainers before tagging a new release.

## Pre-release

- [ ] `make ci` passes (lint → check → test)
- [ ] `make smoke-test` passes (build → name → checksum → install → verify)
- [ ] `CHANGELOG.md` updated with release notes
- [ ] Version bumped in `Cargo.toml`
- [ ] `docs/release-policy.md` reviewed for compatibility (breaking changes
  called out)
- [ ] Pre-1.0: MINOR bumps may include breaking changes; document in changelog

## Release

- [ ] Tag pushed: `git tag v<version> && git push origin v<version>`
- [ ] **Release Artifacts** workflow triggered with tag
- [ ] Release artifacts attached to GitHub Release
- [ ] Install script verified: `curl -fsSL <raw-install-url> | DOTMAN_VERSION=<version> sh`
