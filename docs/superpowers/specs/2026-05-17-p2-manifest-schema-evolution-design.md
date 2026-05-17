# Manifest Schema Evolution Design

## Goal

Document `deps.toml` and `dotfiles.toml` schemas explicitly so users and
maintainers can understand the format without reading Rust deserialization code.

## Scope

- Create `docs/manifest-schema.md` documenting:
  - `deps.toml`: top-level structure, `[deps.<name>]` entries, installer types,
    per-platform/per-arch keys, required and optional params per installer.
  - `dotfiles.toml`: `[[files]]` entries, source/target/kind/enabled/platforms.
  - Field types, defaults, and validation rules.
  - Compatibility rules (what can change without breaking existing manifests).
- Link the schema doc from `README.md`.

## Non-Goals

- Do not change the Rust deserialization or validation code.
- Do not add a JSON Schema or formal schema language.
- Do not enforce schema versioning (future epic: Manifest Defaults).

## Design

Schema documentation lives in `docs/manifest-schema.md`. It is a Markdown
reference that describes every field, its type, default value, and any
validation constraints applied by `make check`.

### Structure

```markdown
# Manifest Schema

## deps.toml

### Top-level

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| deps | table | yes | Map of dependency name to Dependency. |

### Dependency

| Key | Type | Required | Default | Description |
|-----|------|----------|---------|-------------|
| command | string | yes | — | CLI command name for `which` lookup. |
| version_check | table | no | none | Version verification config. |
| mac | table | no | {} | Per-architecture entries for macOS. |
| linux | table | no | {} | Per-architecture entries for Linux. |

... (and so on for all fields)
```

## Error Handling

N/A — documentation-only change.

## Verification Strategy

- `make check` — no regressions
- `cargo test` — full suite
- Manual: review `docs/manifest-schema.md` for completeness
