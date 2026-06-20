# dotfiles

[中文](README.zh-CN.md)

dotman is a tiny Rust-based dotfiles deployer for my personal macOS/Linux environment.
It uses a Dotbot-like ordered YAML config to link files, create directories, and run setup commands.

## Preview

![Terminal setup preview](assets/screenshots/terminal-preview.png)

## Prerequisites

- Rust toolchain with Cargo
- GNU Make
- Git
- curl
- fish shell

## Usage

Build the deployer:

```sh
make build
```

Preview the deployment:

```sh
make deploy DRY_RUN=1
```

Deploy dotfiles:

```sh
make deploy
```

Preview bootstrap steps:

```sh
make bootstrap DRY_RUN=1
```

Run bootstrap steps:

```sh
make bootstrap
```

Skip shell commands such as plugin sync:

```sh
make deploy EXCEPT=shell
```

Run only link steps:

```sh
make deploy ONLY=link
```

## Configuration

Deployment steps live in `dotman.yaml`. Bootstrap steps live in
`dotman.bootstrap.yaml`.

Supported directives:

- `defaults`
- `link`
- `create`
- `shell`
- `clean`: planned / dry-run placeholder

Example:

```yaml
- defaults:
    link:
      create: true
      relink: true
      relative: true
    shell:
      stdout: true
      stderr: true

- link:
    ~/.config/fish: config/fish
    ~/.config/nvim: config/nvim

- create:
    - ~/.config/fish/local.d

- shell:
    - command: fish -lc 'type -q fisher; or curl -sL https://raw.githubusercontent.com/jorgebucaran/fisher/main/functions/fisher.fish | source; and fisher update'
      description: Install and sync fish plugins
```

Field reference:

- `defaults.link.create`: create missing parent directories for link targets.
- `defaults.link.relink`: replace an existing symlink when it points somewhere else.
- `defaults.link.backup`: move an existing conflicting target aside before linking.
- `defaults.link.relative`: create relative symlinks.
- `defaults.shell.stdout`: inherit stdout from shell commands.
- `defaults.shell.stderr`: inherit stderr from shell commands.
- `link`: maps target paths to source paths. A link item can also use `path` plus
  per-item `create`, `relink`, `backup`, `relative`, and `if` overrides.
- `create`: creates directories, following existing symlinked path components.
- `shell.command`: command to run through `sh -c`.
- `shell.description`: human-readable label shown in logs.
- `shell.if`: shell condition that must succeed before the command runs.
- `shell.stdout` / `shell.stderr`: per-command output overrides.
- `clean`: parsed and shown in dry-runs, but non-dry-run cleanup is not implemented yet.

## Local Overrides

Machine-specific paths, tokens, and temporary tool setup should stay out of the
shared repository.

Fish loads local-only files from:

```text
~/.config/fish/local.d/*.fish
```

For first-time setup on a new machine, follow [docs/new-machine.md](docs/new-machine.md).

## Layout

- `bin/`: user scripts linked into `~/.local/bin`
- `config/`: tracked source dotfiles
- `docs/`: setup notes and manual checklists
- `dotman.yaml`: deploy steps
- `dotman.bootstrap.yaml`: bootstrap steps
- `packages/`: package manifests and install helpers
- `src/`: Rust deployer source
- `tests/`: CLI integration tests

## Development

```sh
make lint
make test
make ci
```
