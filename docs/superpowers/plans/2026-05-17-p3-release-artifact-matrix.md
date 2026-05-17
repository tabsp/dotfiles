# Release Artifact Matrix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a manual GitHub Actions workflow that builds and optionally publishes all supported `dotman` release artifacts for a tag.

**Architecture:** Keep local Makefile behavior host-only and move multi-platform release production to GitHub Actions native runners. A matrix build job runs `make release-check` on each supported host, uploads artifacts, and a publish job attaches them to the release tag when requested.

**Tech Stack:** GitHub Actions, Rust stable toolchain, GNU Make, tar, shasum, `actions/upload-artifact`, `actions/download-artifact`, `softprops/action-gh-release`.

---

## File Structure

- Modify: `docs/roadmap.md` to add the planned roadmap epic.
- Create: `docs/superpowers/specs/2026-05-17-p3-release-artifact-matrix-design.md` for the release matrix design.
- Create: `docs/superpowers/plans/2026-05-17-p3-release-artifact-matrix.md` for this implementation checklist.
- Create: `.github/workflows/release-artifacts.yml` for the manual release workflow.
- Modify: `README.md` to document the manual release-artifact workflow.

## Existing Code Map

- `Makefile`: `release` and `release-check` already build, package, and verify
  the current host artifact.
- `.github/workflows/ci.yml`: existing GitHub Actions workflow for local CI on
  Ubuntu.
- `scripts/install.sh`: maps host OS and architecture to the supported release
  artifact target triples.
- `docs/release-policy.md`: defines artifact naming and supported release
  target triples.
- `README.md`: documents install and developer commands.

## Task 1: Add the Release Artifacts Workflow

**Files:**
- Create: `.github/workflows/release-artifacts.yml`

- [ ] **Step 1: Create the workflow**

```yaml
name: Release Artifacts

on:
  workflow_dispatch:
    inputs:
      tag:
        description: Release tag to build, for example v0.1.0
        required: true
        type: string
      publish_release:
        description: Upload artifacts to the GitHub Release for this tag
        required: true
        default: true
        type: boolean

permissions:
  contents: write

jobs:
  build:
    name: Build ${{ matrix.target }}
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - target: aarch64-apple-darwin
            os: macos-15
          - target: x86_64-apple-darwin
            os: macos-15-intel
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-24.04
    steps:
      - uses: actions/checkout@v4
        with:
          ref: ${{ inputs.tag }}

      - uses: actions-rust-lang/setup-rust-toolchain@v1

      - run: make release-check

      - uses: actions/upload-artifact@v4
        with:
          name: dotman-${{ matrix.target }}
          path: |
            dist/dotman-${{ matrix.target }}-*.tar.gz
            dist/dotman-${{ matrix.target }}-*.tar.gz.sha256
          if-no-files-found: error

  publish:
    name: Publish GitHub Release
    if: ${{ inputs.publish_release }}
    needs: build
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/download-artifact@v4
        with:
          path: dist
          merge-multiple: true

      - uses: softprops/action-gh-release@v2
        with:
          tag_name: ${{ inputs.tag }}
          files: |
            dist/*.tar.gz
            dist/*.tar.gz.sha256
```

- [ ] **Step 2: Verify workflow paths match generated artifact names**

Run: `grep -n "dotman-.*tar.gz" .github/workflows/release-artifacts.yml`

Expected: paths use `dist/dotman-${{ matrix.target }}-*.tar.gz` and matching `.sha256`.

## Task 2: Document Manual Release Artifact Generation

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Add release maintainer notes**

Add a short subsection under the install or development command docs:

```markdown
### Release artifacts

`make release-check` builds and verifies the current host artifact locally.
For a tagged release, run the **Release Artifacts** GitHub Actions workflow with
the release tag to build the supported macOS and Linux tarballs on native
runners and optionally attach them to the GitHub Release.
```

- [ ] **Step 2: Confirm README references the workflow**

Run: `rg -n "Release Artifacts|make release-check" README.md`

Expected: both strings are present.

## Task 3: Verify

**Files:**
- No source edits.

- [ ] **Step 1: Check workflow YAML by parsing with Ruby**

Run: `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/release-artifacts.yml"); puts "ok"'`

Expected: `ok`

- [ ] **Step 2: Run full local CI**

Run: `make ci`

Expected: rustfmt passes, clippy passes with `-D warnings`, manifest check passes, and all Rust tests pass.

- [ ] **Step 3: Run release packaging verification**

Run: `make release-check`

Expected: current host tarball is created and checksum verification prints `OK`.

- [ ] **Step 4: Commit**

```sh
git add docs/roadmap.md docs/superpowers/specs/2026-05-17-p3-release-artifact-matrix-design.md docs/superpowers/plans/2026-05-17-p3-release-artifact-matrix.md .github/workflows/release-artifacts.yml README.md
git commit -m "ci: add release artifact matrix workflow"
```

## Verification Commands

- `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/release-artifacts.yml"); puts "ok"'`
- `rg -n "Release Artifacts|make release-check" README.md`
- `make ci`
- `make release-check`
- After merge and push: manually dispatch **Release Artifacts** for `v0.1.0`.

## Expected Outcomes

- `.github/workflows/release-artifacts.yml` exposes a manual workflow with
  `tag` and `publish_release` inputs.
- The workflow builds the three documented targets on native runners.
- Each target uploads its tarball and `.sha256` file as a workflow artifact.
- The publish job attaches artifacts to the GitHub Release when requested.
- Local release behavior remains host-only.

## Test Level

- Documentation and CI workflow change.
- No Rust source changes.
- Local verification covers workflow YAML parseability, documentation references,
  full Rust CI, and current-host release packaging.
- Cross-runner behavior is verified after push by manually dispatching the
  workflow for the intended release tag.

## Regression Coverage Expectations

- Existing `.github/workflows/ci.yml` remains unchanged.
- `make ci` continues to pass locally.
- `make release-check` continues to produce the current-host tarball and verify
  its checksum.
- Artifact names remain compatible with `scripts/install.sh` and
  `docs/release-policy.md`.

## Self-Review

- Spec coverage: workflow trigger, matrix, artifact upload, publish behavior,
  error handling, and verification are covered by tasks.
- Placeholder scan: no TBD/TODO placeholders.
- Type consistency: target triples match `docs/release-policy.md` and
  `scripts/install.sh`.
