# Release Artifact Matrix Design

## Goal

Build the supported `dotman` release artifacts on native GitHub-hosted runners
so the install script can fetch tarballs for macOS Apple Silicon, macOS Intel,
and Linux x86_64 from a single tagged release.

## Scope

- Add a manual GitHub Actions workflow for release artifacts.
- Build the existing `make release-check` target on native runners:
  - `macos-15` for `aarch64-apple-darwin`
  - `macos-15-intel` for `x86_64-apple-darwin`
  - `ubuntu-24.04` for `x86_64-unknown-linux-gnu`
- Checkout an explicit tag input before building.
- Upload tarballs and checksum files as workflow artifacts.
- Optionally publish the generated files to the GitHub Release for that tag.
- Keep local `make release` behavior unchanged.

## Non-Goals

- Do not move or recreate existing Git tags.
- Do not add local cross-compilation toolchains such as `cross`, Zig, or custom
  sysroots.
- Do not add package-registry publishing.
- Do not sign artifacts.
- Do not add Windows or Linux arm64 release artifacts.

## Design

### Workflow Trigger

Add `.github/workflows/release-artifacts.yml` with `workflow_dispatch` inputs:

- `tag`: required release tag, for example `v0.1.0`.
- `publish_release`: boolean, default `true`.

The workflow checks out the exact tag with `ref: ${{ inputs.tag }}`. This keeps
release artifacts tied to immutable source state and avoids relying on whatever
is currently on `main`.

### Build Matrix

The build job uses a matrix of native GitHub-hosted runners. Each job runs:

```sh
make release-check
```

`make release-check` already reads the Cargo version, detects the Rust host
target, builds the optimized binary, creates the tarball, and verifies the
checksum. Native runner selection makes the detected host target match the
artifact target.

### Artifact Upload

Each matrix job uploads both files from `dist/`:

- `dotman-{target}-{version}.tar.gz`
- `dotman-{target}-{version}.tar.gz.sha256`

Artifacts are named `dotman-{target}` so failed or partial runs are easy to
inspect.

### GitHub Release Publishing

A publish job runs after all matrix builds succeed when `publish_release` is
true. It downloads all workflow artifacts and uploads the tarballs and checksum
files to the GitHub Release named by `tag`.

The workflow may create the release if it does not exist, but it must not move
the tag. The tag is the source of truth.

## Error Handling

- Missing or invalid tag: checkout fails before any artifact is produced.
- Build or test failure: the affected matrix job fails and publish does not run.
- Checksum failure: `make release-check` fails before upload.
- Release upload failure: build artifacts remain available on the workflow run
  for manual recovery.

## Verification Strategy

- Validate workflow YAML structure locally where possible.
- Run `make ci`.
- Run `make release-check` on the current host.
- After merge, manually dispatch the workflow for `v0.1.0` and confirm all
  three artifacts are present.

## Regression Coverage Expectations

- Existing CI behavior remains unchanged.
- Local `make release` and `make release-check` keep the current host-only
  behavior.
- The install script artifact naming stays aligned with `docs/release-policy.md`.
