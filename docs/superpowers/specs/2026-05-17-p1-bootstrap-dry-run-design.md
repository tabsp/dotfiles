# Bootstrap Dry Run Design

## Goal

Add `--dry-run` to `bootstrap` so first-time setup can be previewed before any
package installs or filesystem changes occur.

## References

- `src/main.rs`: existing `Link { dry_run }` flag (line 39) and `run_bootstrap` (line 120).
- `src/link.rs`: `print_dry_run` and `run_link` with dry_run parameter.
- `src/deps.rs`: `install_missing` iterates dependencies and calls installers.
- `src/installers.rs`: `install_missing` and `is_installed` — status check vs install.

## Scope

- Add `--dry-run` flag to `Command::Bootstrap`.
- In dry-run mode, `run_bootstrap` prints what each step would do without
  mutating the system:
  1. `check` — run as normal (validates config, no side effects).
  2. `deps` — print each dependency that would be installed.
  3. `link` — pass `dry_run=true` to `run_link` (already supported).
  4. `doctor` — skip (it reports state, not meaningful in dry-run).
  5. Post-bootstrap hints — skip.
- Exit with code 0 on successful preview, non-zero if `check` fails.

## Non-Goals

- Do not add dry-run to individual installer commands (`install_download_binary`
  etc.). The deps step just reports which deps are missing.
- Do not change `doctor` behavior.

## Design

### CLI

```
dotman bootstrap --dry-run
```

The `Bootstrap` variant gains `dry_run: bool` with `#[arg(long)]`.

### Flow

1. Parse CLI.
2. `run_bootstrap(dry_run)`.
3. `check::run_check` — always runs (no mutations).
4. If dry_run: iterate deps, call `is_installed` for each, print status.
5. If not dry_run: `deps::install_missing` as before.
6. `link::run_link(..., dry_run)` — passes flag through.
7. If not dry_run: `doctor::run_doctor`, print hints.
8. If dry_run: skip doctor and hints.

### Output format

```
==> bootstrap (dry-run)
==> check
==> dependencies
would install: ripgrep
would install: fd
already installed: bat
==> link
would link: ~/.config/nvim -> ...
==> dry-run complete (no changes made)
```

## Error Handling

- `check` failures are reported normally and exit with error.
- Missing-host entries for deps are reported as errors.
- Link conflicts in dry-run are reported but do not fail (matching existing
  `link --dry-run` behavior).

## Verification Strategy

- `cargo test` — full test suite
- `cargo clippy` — zero warnings
- Manual: `cargo run -- bootstrap --dry-run` in repo root
