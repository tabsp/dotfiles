# dotfiles

[中文](README.zh-CN.md)

dotman is a tiny Rust-based TUI dotfiles deployer for my personal macOS/Linux environment.
It uses a flat YAML config to install software, link config files, and run setup commands — all in a Plan → Confirm → Run flow with per-machine state and persistent run history.

## Preview

![Terminal setup preview](assets/screenshots/terminal-preview.png)

## Quick Start

Install `dotman` via Homebrew (or any other method):

```sh
brew install tabsp/tap/dotman
```

Then run it. The first time, dotman will auto-initialize with the default dotfiles repo:

```sh
dotman
```

This clones `https://github.com/tabsp/dotfiles.git` to `~/.local/share/dotman/repos/main`,
loads the deployment config from `dotman.yaml`, and opens the TUI plan view.

For unattended setup (CI, scripted setup):

```sh
dotman deploy --headless
```

For a custom dotfiles repo:

```sh
dotman init https://github.com/you/dotfiles.git
dotman deploy
```

## Usage

```sh
dotman                       # TUI main menu (auto-inits on first run)
dotman deploy                # TUI: sync → plan → confirm → run
dotman plan                  # TUI: show plan only, no execution
dotman init [repo]           # initialize dotfiles profile
dotman sync                  # git pull current profile
dotman status                # show profile and repo state
dotman doctor                # check prerequisites
dotman profile list          # list configured profiles
dotman history               # TUI: browse past runs
dotman run <ulid>            # TUI: replay a past run
dotman deploy --headless     # headless: non-interactive deploy
dotman plan --headless       # headless: emit JSON plan to stdout
```

## TUI Keys

| Key | Action |
| --- | --- |
| `↑↓` or `j k` | Navigate |
| `space` | Toggle current step |
| `a` / `n` | Select all / none |
| `1-6` | Fold/unfold layer |
| `s` | Save selection to state |
| `r` | Run |
| `e` | Back to plan view (from result) |
| `q` or `Esc` | Back / quit |

## Configuration

Deployment steps live in `dotman.yaml` in your dotfiles repo:

```yaml
package_managers:
  macos: brew
  ubuntu: brew
  arch: pacman

auto_install_pkg_manager: true

default_shell: fish

install: [ghostty, fish, tmux, neovim, lazygit, btop, ripgrep, fzf, starship]

links:
  ~/.config/fish:    config/fish
  ~/.config/nvim:    config/nvim
  ~/.config/ghostty: config/ghostty
  ~/.tmux.conf:      config/tmux.conf

create:
  - ~/.config/fish/local.d
  - ~/Workspace/tries

shell:
  - command: fish -lc 'fisher update'
    description: Sync fish plugins
    optional: true
```

YAML field reference:

- `package_managers` — per-platform package manager.
- `auto_install_pkg_manager` — if true, attempts to install the package
  manager itself (e.g. Homebrew) before any install steps.
- `default_shell` — login shell to switch to automatically. dotman resolves
  the real path with `command -v` and ensures it is listed in `/etc/shells`.
- `install: [name]` — list of tool names to install. dotman picks the right
  install command for your platform from its internal tool database.
- `links:` — map of target → source. dotman handles relative/absolute,
  backup, and relink based on source state.
- `create:` — directories to ensure exist.
- `shell:` — list of shell commands. Supports `description`,
  `optional: true`, and `if:` (condition guard).

Profile configuration (repo URL, branch, checkout path, auto-sync) lives at
`~/.config/dotman/config.toml`. dotman manages this automatically — you
don't need to create or edit it by hand.

State (per-machine selection) lives at
`~/.local/share/dotman/state.toml` — first-run defaults are applied
automatically based on layer strategy.

Run logs are at `~/.local/share/dotman/runs/<ulid>.json` and can be
browsed with `dotman history` or `dotman run <id>`.

## How It Works

The deployment pipeline is:

```
resolve profile → sync repo (git pull) → load dotman.yaml → build plan → confirm → execute → save history
```

dotman's profile system manages the dotfiles repo itself (URL, branch, clone
path, auto-sync). The deployment config (`dotman.yaml`) only describes *what*
to deploy — installs, links, creates, shell commands.

Auto-init triggers automatically when no profile or config is found. In
headless mode all defaults are used; in interactive mode you're prompted to
confirm.

## Status

### What works

- 80 tests pass (`cargo test`)
- `cargo build --release` produces a ~4MB binary
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` clean
- TUI: MainMenu / PlanView / ConfirmView / RunView / ResultView /
  HistoryView / RunReplay with Catppuccin Mocha theme and Nerd Font icons
- First-run auto-init (clone repo, write profile, load config)
- Profile management (add/remove/list, multiple dotfiles repos)
- Git bootstrap (auto-install git on macOS/Linux in headless mode with
  `--bootstrap-git`)
- Headless mode (`--headless`) for scripts, CI, and remote bootstrap
- Plan selections persist to `~/.local/share/dotman/state.toml`
- RunView shows execution events, live streaming stdout/stderr from
  subprocesses, with abort that kills the current process group
- Tool DB has 17 entries covering the user's actual tools
- `dotman plan --headless` emits JSON
- `dotman deploy --headless` runs end-to-end
- Per-step retry for install actions (5s/10s/20s exponential backoff)
- `dotman new-link <target> <source>` updates `dotman.yaml`
- `dotman history` and `dotman run <ulid>` browse and replay past runs

### Not yet implemented

- E2E coverage for early bootstrap and auto-clone is still in progress.

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
