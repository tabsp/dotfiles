# dotfiles

[中文](README.zh-CN.md)

dotman is a tiny Rust-based dotfiles deployer for my personal macOS/Linux environment.
It uses a Dotbot-like ordered YAML config to link files, create directories, and run setup commands.

## Preview

![Terminal setup preview](assets/screenshots/terminal-preview.png)

## Prerequisites

- curl

The installer can install or update `dotman`, download the published dotfiles
bundle, install Homebrew when missing, install fish through Homebrew, and then
run `dotman bootstrap` and `dotman deploy`.

For first-time setup, install `dotman` and the published dotfiles bundle from
the site:

```sh
sh -c "$(curl -fsSL https://dotfiles.tabsp.com/install.sh)"
```

For unattended setup:

```sh
curl -fsSL https://dotfiles.tabsp.com/install.sh | sh -s -- --yes
```

Fish may live outside the current `PATH` on a fresh machine. The installer
prints the exact `exec .../fish -l` command for the current terminal, and
attempts to change the default login shell for future sessions.

When `dotman.yaml` is not present in the current directory, `dotman` falls back
to `DOTFILES_DIR` and then to the installed bundle in
`~/.local/share/tabsp-dotfiles`.

## Usage

Preview the deployment:

```sh
dotman deploy --dry-run
```

Deploy dotfiles:

```sh
dotman deploy
```

Preview bootstrap steps:

```sh
dotman bootstrap --dry-run
```

Run bootstrap steps:

```sh
dotman bootstrap
```

Skip shell commands such as plugin sync:

```sh
dotman deploy --except shell
```

Run only link steps:

```sh
dotman deploy --only link
```

## Tools

| Tool | Purpose |
|------|---------|
| [fish](https://fishshell.com) | Shell with built-in autosuggestions |
| [starship](https://starship.rs) | Cross-shell prompt |
| [direnv](https://direnv.net) | Per-directory environment variables |
| [mise](https://mise.jdx.dev) | Runtime and tool version manager |
| [fzf](https://github.com/junegunn/fzf) | Fuzzy finder (files, history, zoxide jump) |
| [zoxide](https://github.com/ajeetdsouza/zoxide) | Smarter `cd` with directory ranking |
| [fd](https://github.com/sharkdp/fd) | Fast `find` replacement |
| [ripgrep](https://github.com/BurntSushi/ripgrep) | Fast `grep` replacement |
| [eza](https://github.com/eza-community/eza) | Modern `ls` replacement with icons |
| [bat](https://github.com/sharkdp/bat) | `cat` with syntax highlighting |
| [tealdeer](https://github.com/dbrgn/tealdeer) | Fast `tldr` client |
| [btop](https://github.com/aristocratos/btop) | Resource monitor |
| [fastfetch](https://github.com/fastfetch-cli/fastfetch) | System info display |
| [dua-cli](https://github.com/Byron/dua-cli) | Disk usage analyzer |
| [neovim](https://neovim.io) | Editor |
| [lazygit](https://github.com/jesseduffield/lazygit) | Terminal Git UI |
| [yazi](https://github.com/sachinsenal/yazi) | Terminal file manager |
| [tmux](https://github.com/tmux/tmux) | Terminal multiplexer with Catppuccin theme |
| [ghostty](https://ghostty.org) | GPU-accelerated terminal with Catppuccin Mocha theme |
| [jq](https://github.com/jqlang/jq) + [yq](https://github.com/mikefarah/yq) | JSON/YAML CLI processors |
| [ruby](https://www.ruby-lang.org) | Runtime for `try` experiment manager |

All packages are installed via `brew bundle --file packages/Brewfile` during bootstrap. Fish integrates most tools on startup and defines custom functions: `zi` (fzf+zoxide jump), `ff` (fzf file picker), `y` (yazi with auto-cd), `t` (tmux attach/create). See `config/fish/config.fish`.

## Layout

- `bin/`: user scripts linked into `~/.local/bin` (tmux-status, etc.)
- `config/`: tracked dotfiles for fish, nvim, ghostty, btop, fastfetch, starship, tealdeer, tmux, git
- `docs/`: setup notes and manual checklists
- `dotman.yaml`: deploy steps (link configs, create directories, sync derived state)
- `dotman.bootstrap.yaml`: bootstrap steps (install packages, fonts)
- `packages/`: Brewfile and platform-specific install helpers
- `scripts/`: install and static site build scripts
- `src/`: Rust deployer source
- `tests/`: CLI integration tests

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
      description: Sync fish plugins
      optional: true
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
- `shell.optional`: if `true`, a failed command is reported as a warning and
  later steps continue. Defaults to `false`.
- `shell.stdout` / `shell.stderr`: per-command output overrides.
- `clean`: parsed and shown in dry-runs, but non-dry-run cleanup is not implemented yet.

Deploy is fail-fast for core file operations: link/create failures stop the run.
Network-sensitive sync commands can be marked with `optional: true` so transient
failures do not make the whole deploy fail.

## Local Overrides

Machine-specific paths, tokens, and temporary tool setup should stay out of the
shared repository.

Fish loads local-only files from:

```text
~/.config/fish/local.d/*.fish
```

For first-time setup on a new machine, follow [docs/new-machine.md](docs/new-machine.md).

## Development

```sh
make build
make lint
make test
make ci
```

Run the real Linux install flow in Docker:

```sh
make e2e-linux
make e2e-linux E2E_ARGS="--local --inspect --keep"
```

The E2E script builds `dotman` from the current worktree, serves a local
installer/manifest/bundle inside Docker, runs the install script, and verifies
the installed dotfiles. `--inspect` opens the finished container as the `tester`
user for manual checks.

## Publishing

The website build publishes the installer and runtime bundle:

```sh
scripts/build-site.sh
```

Use this for Vercel:

- Build Command: `scripts/build-site.sh`
- Output Directory: `public`

`public/manifest.json` points at `bundle/latest.tar.gz` on
`dotfiles.tabsp.com` and at dotman binaries from the latest GitHub Release.
Tagging `v*` runs the release workflow that builds those dotman binaries.
