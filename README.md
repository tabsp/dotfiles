# dotfiles

Personal dotfiles managed by `dotman`, a small Rust dotfiles deployer inspired
by Dotbot's ordered configuration model.

## Prerequisites

- Rust toolchain with Cargo
- GNU Make
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

Skip shell commands such as plugin sync:

```sh
make deploy EXCEPT=shell
```

Run only link steps:

```sh
make deploy ONLY=link
```

## Configuration

Deployment steps live in `dotman.yaml`.

Supported directives:

- `defaults`
- `link`
- `create`
- `shell`
- `clean` (dry-run placeholder only)

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

- create:
    - ~/.config/fish/local.d

- shell:
    - command: fish -lc 'fisher update'
      description: Sync fish plugins
      stdout: true
      stderr: true
```

## Local Overrides

Machine-specific paths, tokens, and temporary tool setup should stay out of the
shared repository.

Fish loads local-only files from:

```text
~/.config/fish/local.d/*.fish
```

## Layout

- `config/`: tracked source dotfiles
- `dotman.yaml`: deploy steps
- `src/`: Rust deployer source
- `tests/`: CLI integration tests

## Development

```sh
make lint
make test
make ci
```
