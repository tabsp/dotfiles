# Manifest Defaults Design

## Goal

Allow deps.toml entries to share common metadata via a `default` section,
reducing repetition across platform and architecture entries.

## Scope

- Add optional `[deps.<name>.default]` section to the schema.
- When a per-arch entry is missing a field present in `default`, the default
  value is used.
- Per-arch values always override defaults when both are present.
- Update `entries_for` / `entries_for_host` to merge defaults.
- Update `docs/manifest-schema.md` to document the `default` section.
- Update `make check` to validate the new section.

## Non-Goals

- Do not add nested default inheritance (e.g., platform-level defaults).
- Do not change the file-level format (still TOML).
- Do not add dotfiles.toml defaults.

## Design

### Schema

```toml
[deps.nvim]
command = "nvim"

[deps.nvim.default]
installer = "download_binary"
version = "v0.10.0"

[deps.nvim.mac.arm64]
# inherits installer and version from default
url = "https://.../nvim-macos-arm64.tar.gz"
sha256 = "abc..."
archive_kind = "tar.gz"
binary_path = "nvim-macos-arm64/bin/nvim"
install_to = "~/.local/bin/nvim"

[deps.nvim.mac.x86_64]
url = "https://.../nvim-macos-x86_64.tar.gz"
sha256 = "def..."
archive_kind = "tar.gz"
binary_path = "nvim-macos-x86_64/bin/nvim"
install_to = "~/.local/bin/nvim"
```

### Merge strategy

1. If `default` exists, each per-arch entry inherits from it.
2. `installer` and `version` are taken from default if not present in the
   per-arch entry.
3. All other fields (`url`, `sha256`, `archive_kind`, `binary_path`, etc.)
   come from the per-arch entry directly (no default merging — they are
   params, not standalone fields).
4. `params` map is merged: default params + per-arch params, with per-arch
   winning on key collision.
5. If no `default` section exists, behavior is unchanged.

### API changes

`Dependency` gains `default: Option<InstallEntry>`. `entries_for` returns
merged entries: for each arch key, if default exists, merge the params and
use default's installer/version as fallback.

### Example: Before vs After

Before (4 full entries):
```toml
[deps.nvim.mac.arm64]
installer = "download_binary"
version = "v0.10.0"
url = "...-arm64.tar.gz"
...
```

After (1 default + 4 arch-specific entries):
```toml
[deps.nvim.default]
installer = "download_binary"
version = "v0.10.0"

[deps.nvim.mac.arm64]
url = "...-arm64.tar.gz"
...
```

## Error Handling

- Missing required fields after merge still fail `make check`.
- `default` with no per-arch entries is allowed (no-op).
- `default.installer` mismatch with per-arch entry is not an error —
  per-arch always wins.

## Verification Strategy

- `cargo test config` — merge logic tests
- `cargo test` — full suite
- `make check` — validates default sections
- `cargo clippy` — zero warnings
