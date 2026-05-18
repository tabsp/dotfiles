# Manifest Schema

Dotman uses TOML manifests to define dotfiles and dependencies. Each manifest
has a `schema_version` field for compatibility tracking.

## Current Schema Version: 1

### deps.toml

```toml
schema_version = 1  # optional, defaults to 1

[deps.<name>]
command = "<binary-name>"
version_check = { args = ["--version"], regex = "<pattern>" }  # optional

[<name>.<platform>-<arch>]  # or [<name>.default]
installer = "download-binary" | "official-script" | "brew" | "apt" | "repo-package" | "ppa" | "system"
version = "<version>"        # optional
source = "<url-template>"    # for download-binary and official-script
params = { ... }             # installer-specific parameters
distros = ["ubuntu", ...]    # optional distro filter
```

### dotfiles.toml

```toml
schema_version = 1  # optional, defaults to 1

[[files]]
source = "config/<name>"
target = "<target-path>"
```

## Evolution Rules

### When to NOT bump schema_version (backward-compatible)

- Adding new **optional** fields with `#[serde(default)]`.
- Adding new installer types that old manifests don't reference.
- Changing documentation or comments.

### When to bump schema_version (breaking change)

- Adding new **required** fields without a default.
- Renaming existing fields.
- Changing field types (e.g., string → list).
- Removing fields.
- Changing the top-level structure.

### Deprecation Process

1. **Deprecation warning:** Old field continues to work but `dotman check` emits
   a warning for one schema version.
2. **Migration period:** Both old and new formats accepted for at least one
   schema version bump.
3. **Removal:** Old field removed in the next schema version.

### Version Rejection

If a manifest specifies `schema_version = N` where N > supported version:

```
deps.toml: manifest requires schema version N but dotman supports up to 1.
Upgrade dotman or use an older manifest.
```

This prevents silent misbehavior when dotman is too old for the manifest format.
