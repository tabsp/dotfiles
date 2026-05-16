# Doctor Summary And Machine Output Design

## Goal

Add a summary line to doctor output and an optional `--json` flag for
machine-readable structured output suitable for automation scripts.

## References

- `src/doctor.rs`: `run_doctor` collects `hard_errors`, `warnings`, `oks` and
  prints them as `error:`, `warn:`, `ok:` lines.
- `src/main.rs:111-119`: `run_doctor` CLI wrapper.

## Scope

- Print a summary line after all doctor checks: `doctor: N ok, M warning(s), K error(s)`.
- Add `--json` flag to the `doctor` subcommand.
- In JSON mode, output a JSON object with `ok`, `warnings`, `errors` arrays
  and a `summary` field.
- Exit code unchanged: 0 if no hard errors, non-zero otherwise.
- Summary is printed to stdout (not stderr) in both modes for easy capture.

## Non-Goals

- Do not change the per-item output format.
- Do not add structured output formats beyond JSON.
- Do not change bootstrap's use of doctor.

## Design

### Human-readable mode (default)

```
ok: bat: command bat
ok: bat: version 0.25.0
warn: rg: version drift installed=14.1.0 expected=14.1.1
error: fd: missing command fd
doctor: 2 ok, 1 warning, 1 error
```

### JSON mode (`--json`)

```json
{
  "ok": ["bat: command bat", "bat: version 0.25.0"],
  "warnings": ["rg: version drift installed=14.1.0 expected=14.1.1"],
  "errors": ["fd: missing command fd"],
  "summary": {"ok": 2, "warnings": 1, "errors": 1}
}
```

### API change

`run_doctor` gains a `json: bool` parameter. When `json` is true, it
serializes the collected items as JSON instead of printing line-by-line.

## Error Handling

- JSON serialization failure is reported as a hard error.
- Existing error/warning/ok collection unchanged.

## Verification Strategy

- `cargo test doctor` — existing doctor tests pass
- `cargo test` — full suite
- `cargo clippy` — zero warnings
