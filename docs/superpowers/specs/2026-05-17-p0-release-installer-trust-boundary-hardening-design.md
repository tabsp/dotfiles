# Release Installer Trust Boundary Hardening Design

## Goal

Harden `scripts/install.sh` so that checksum verification is mandatory for both
the dotman binary and the dotfiles source archive. The installer must fail
rather than silently skip verification.

## Scope

- Make checksum download mandatory (remove `|| true` fallback).
- Make checksum tool availability mandatory (fail if shasum/sha256sum missing
  when checksum file is present).
- Add checksum verification for the dotfiles source archive.
- Ensure version/tag consistency between binary and source archive downloads.
- Add CLI tests for failure paths (missing checksum, wrong checksum, missing tool).

## Non-Goals

- Changing the version resolution logic (version default remains `0.1.0`).
- Adding GPG or other signature verification.
- Rewriting the installer in a different language.
- Changing the download mechanism (curl/wget).

## Design

### Current trust gaps

1. `curl ... "$CHECKSUM_URL" ... || true` — checksum download failure is silent.
2. `if [ -f "$TMPDIR/$CHECKSUM" ]` — verification skipped if file absent.
3. No checksum verification for dotfiles source archive.
4. No error when checksum tool (shasum/sha256sum) is unavailable.

### Hardened flow

1. Download binary archive → fail on error.
2. Download binary checksum → **fail on error** (remove `|| true`).
3. Verify binary checksum → fail if tool missing or mismatch.
4. Download dotfiles source archive → fail on error.
5. Download dotfiles source checksum → **fail on error**.
6. Verify dotfiles source checksum → fail if tool missing or mismatch.
7. Extract and install.

### Checksum tool detection

```
find_checksum_tool() {
    if command -v shasum >/dev/null 2>&1; then
        echo "shasum -a 256"
    elif command -v sha256sum >/dev/null 2>&1; then
        echo "sha256sum"
    else
        echo ""
    fi
}
```

Detect once, fail early if unavailable.

### Dotfiles source checksum

The dotfiles source archive tag must match the binary version. The checksum
URL pattern:

```
https://github.com/tabsp/dotfiles/releases/download/v${VERSION}/dotfiles-${VERSION}.tar.gz.sha256
```

## Error Handling

- Missing checksum file → `error: failed to download checksum` → exit 1.
- Missing checksum tool → `error: shasum or sha256sum required for verification` → exit 1.
- Checksum mismatch → `error: checksum verification failed` → exit 1 (existing).
- Missing dotfiles source checksum → `error: failed to download dotfiles source checksum` → exit 1.
- Failed archive download → exit 1 (existing `set -e`).

## Verification Strategy

- `cargo test install_script` — existing integration test must still pass.
- New integration tests for failure paths.
- Manual: run `sh scripts/install.sh` with BASE_URL pointing to test artifacts.

## Regression Coverage Expectations

- Existing successful install path must not break.
- `curl` and `wget` download paths must both enforce mandatory checksum.
- macOS `shasum` and Linux `sha256sum` must both work.
