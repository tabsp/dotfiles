# CI Automation Design

## Goal

Add repository CI configuration that runs the local verification suite on push
and pull request, and documents platform-specific gaps.

## Scope

- Add `.github/workflows/ci.yml` that runs `make ci` on push/PR to `main`.
- Use `ubuntu-latest` as the primary runner.
- Document macOS-only limitations (e.g., `make check` validates macOS host
  entries by default; CLI integration tests assume macOS host).
- Add a `rust-toolchain.toml` to pin the stable toolchain.

## Non-Goals

- Do not add macOS runners (GitHub Actions macOS minutes are limited).
- Do not add cross-platform matrix builds.
- Do not change `make ci` behavior.

## Design

### Workflow

```yaml
name: CI
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
jobs:
  ci:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          components: rustfmt, clippy
      - run: make ci
```

### Platform gaps

- `make check` validates `deps.toml` entries against the detected host. On
  Ubuntu CI, macOS-only entries will be skipped (not errors).
- CLI integration tests that depend on `$HOME` symlinks or macOS-specific
  behavior are skipped or parameterized on the host platform.
- Document these gaps in the workflow file as comments.

### rust-toolchain.toml

Pin the stable Rust toolchain so CI and local builds use the same version.

## Error Handling

- CI failure on any step blocks merge.
- `make check` failures indicate manifest issues.
- `cargo test` failures indicate regression.

## Verification Strategy

- Push to trigger CI (manual).
- `make ci` passes locally.
