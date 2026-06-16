# Dotman Lite Direction

## Goal

Refocus `dotman` into a small dotfiles deployer inspired by Dotbot's
configuration model.

`dotman` should answer one question:

> How do files in this repository safely appear at their expected locations in
> the user's home directory?

## Non-Goals

`dotman` should no longer manage the broader machine environment.

Out of scope:

- dependency installation
- platform support matrices
- doctor/status/reconcile workflows
- release installer hardening
- recovery inventory scanning
- roadmap or agent workflow automation
- plugin systems for arbitrary new behavior
- template engines

## Configuration Model

Use YAML because the desired model is an ordered list of heterogeneous steps.
TOML can represent the same shape, but it is awkward for this use case.

Example:

```yaml
- defaults:
    link:
      create: true
      relink: true
      relative: true

- link:
    ~/.config/fish: config/fish
    ~/.config/nvim: config/nvim
    ~/.config/ghostty: config/ghostty
    ~/.config/yazi: config/yazi
    ~/.config/starship.toml: config/starship.toml
    ~/.config/lazygit: config/lazygit
    ~/.tmux.conf: config/tmux.conf

- create:
    - ~/.config/fish/local.d

- shell:
    - command: fish -lc 'fisher update'
      description: Sync fish plugins
      stdout: true
      stderr: true
```

## Directives

### defaults

Sets defaults for later directives. First version only needs `link` defaults.

Supported link defaults:

- `create`
- `relink`
- `backup`
- `relative`

### link

Creates symlinks from target paths to repository paths.

Short form:

```yaml
- link:
    ~/.config/fish: config/fish
```

Expanded form:

```yaml
- link:
    ~/.config/fish:
      path: config/fish
      create: true
      relink: true
      backup: true
      relative: true
      if: test "$(uname)" = Darwin
```

Initial supported fields:

- `path`
- `create`
- `relink`
- `backup`
- `relative`
- `if`

Do not implement `force`, `hardlink`, `glob`, or `ignore-missing` in the first
version. They expand the safety surface before the simplified model has settled.

### create

Creates directories needed by tools or local overrides.

Initial supported fields:

- path list short form
- optional `mode` can be deferred

### shell

Runs explicit post-deploy commands. Shell steps must be easy to skip.

Required CLI controls:

```sh
dotman deploy --only link
dotman deploy --except shell
dotman deploy --dry-run
```

Dry-run must print shell commands without executing them.

### clean

Optional and deferrable.

If added, keep it narrow: only remove broken symlinks that point into this
repository. Do not add a state database.

## CLI

Initial CLI:

```sh
dotman deploy
dotman deploy --dry-run
dotman deploy --only link
dotman deploy --except shell
```

Possible later commands:

```sh
dotman list
dotman undeploy
```

Avoid reintroducing environment management commands.

## Migration Plan

1. Add `dotman.yaml` beside the current manifests.
2. Implement YAML parsing and `deploy --dry-run`.
3. Implement `link` with safe conflict behavior.
4. Add `create`.
5. Add `shell` with `--except shell` support.
6. Move README install/update docs to the new deploy flow.
7. Remove legacy dependency, doctor, status, recovery, release, and update
   commands once deploy covers daily use.

## Local Overrides

Local-only machine configuration should live outside tracked generated outputs.

For fish, prefer a tracked hook that sources local files:

```fish
for file in ~/.config/fish/local.d/*.fish
    source $file
end
```

Do not store machine-specific paths, tokens, or one-off tool installations in
the shared repo configuration.
