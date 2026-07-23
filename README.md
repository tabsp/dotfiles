# dotfiles

[中文](README.zh-CN.md)

[![CI](https://github.com/tabsp/dotfiles/actions/workflows/ci.yml/badge.svg)](https://github.com/tabsp/dotfiles/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/tabsp/dotfiles)](https://github.com/tabsp/dotfiles/releases/latest)
[![License: MIT](https://img.shields.io/github/license/tabsp/dotfiles)](LICENSE)
[![Demo](https://img.shields.io/badge/demo-interactive-brightgreen)](https://dotfiles.tabsp.com/)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux-lightgrey)](https://github.com/tabsp/dotfiles#quick-start)
[![Built With Ratatui](https://ratatui.rs/built-with-ratatui/badge.svg)](https://ratatui.rs/)

dotman is a tiny Rust TUI dotfiles manager for my personal macOS/Linux environment.
It uses a flat YAML config to install software, link config files, create directories,
clean old paths, and run setup commands — all in a Plan -> Review -> Run flow with
per-machine state and persistent run history.

## Preview

Try the interactive demo at [dotfiles.tabsp.com](https://dotfiles.tabsp.com/) — the real
Ratatui TUI rendered in the browser by a side-effect-free Rust/WebAssembly state machine.
Plan, Review, Run, and Replay are fully navigable; nothing ever executes.

![dotman main menu](assets/screenshots/dotman-main-menu.png)

![dotfiles workspace](assets/screenshots/dotfiles-workspace.png)

## What's Inside

| Layer                | Tools                                                                                                |
| -------------------- | ---------------------------------------------------------------------------------------------------- |
| Terminal             | Ghostty                                                                                              |
| Shell                | Fish with numbered `conf.d` modules and Fisher plugins                                               |
| Multiplexer          | tmux with tmux-status; herdr                                                                         |
| Software             | Neovim (LazyVim), Pi coding agent, mise, lazygit, btop, fastfetch, yazi                              |
| Enhancement · prompt | Starship, tealdeer, markdownlint-cli2                                                                |
| Enhancement · cli    | ripgrep, fd, bat, glow, eza, dua-cli, gum, tree-sitter-cli, delta, trash-cli, pay-respects, gitleaks |
| Enhancement · data   | fzf, zoxide, jq, yq, direnv, Atuin                                                                   |
| Font                 | Maple Mono NF                                                                                        |

## Quick Start

Install the latest `dotman` release with the checksum-verifying installer:

```sh
curl -fsSL https://github.com/tabsp/dotfiles/releases/latest/download/install.sh | sh
```

The installer detects macOS/Linux and arm64/x86_64, verifies the release SHA-256,
and installs to `~/.local/bin`. Set `DOTMAN_VERSION=v0.3.3` to pin a release or
`DOTMAN_INSTALL_DIR` to choose another directory.

If Homebrew is already available, install and upgrade through the Tap instead:

```sh
brew install tabsp/tap/dotman
```

Update an installation created by `install.sh` in place:

```bash
dotman self update
```

Homebrew owns installations created through the Tap. Upgrade those with:

```sh
brew upgrade dotman
```

To prevent the two package managers from overwriting each other, `self update`
detects Homebrew Cellar installations and refuses to modify them.

Then run it. On first launch, dotman initializes the default dotfiles profile and
opens the main menu. Choose Deploy, adjust the Plan, then review the selected
actions before starting the Run:

```sh
dotman
```

This clones `https://github.com/tabsp/dotfiles.git` to
`~/.local/share/dotman/repos/main`, loads `dotman.yaml`, and opens the TUI.

For unattended setup (CI, scripted setup):

```sh
dotman deploy --headless
```

Headless mode uses the same execution and result model as the TUI, prints live
action output and a final `ran / changed / no change / failed` summary, saves the
run to history, and exits non-zero for failed or aborted runs.

To initialize this repository explicitly:

```sh
dotman init https://github.com/tabsp/dotfiles.git --branch main --profile main
dotman deploy
```

## Usage

```sh
dotman                       # TUI main menu (auto-inits on first run)
dotman deploy                # TUI: sync → plan → review → run
dotman plan                  # TUI: show plan only, no execution
dotman init [repo]           # initialize a dotfiles profile
dotman sync                  # git pull current profile
dotman status                # show profile and repo state
dotman doctor                # check prerequisites
dotman profile list          # list configured profiles
dotman profile add <name> <repo>
dotman profile remove <name>
dotman history               # TUI: browse past runs
dotman run <ulid>            # TUI: replay a past run
dotman new-link <target> <source>
dotman deploy --headless     # headless: non-interactive deploy
dotman plan --headless       # headless: emit JSON plan to stdout
```

Global options:

```sh
--headless         no prompts; use safe defaults and fail on ambiguity
--bootstrap-git    allow dotman to install git before profile/bootstrap work
--config <path>    use a dotman.yaml directly and bypass profile resolution
--no-init          fail instead of auto-initializing when no config is found
```

## TUI Keys

The footer shows only the primary keys for the current screen:

| Screen       | Primary keys                                                                    |
| ------------ | ------------------------------------------------------------------------------- |
| Main menu    | `↑↓` navigate, `Enter` open, `q` quit                                           |
| Plan         | `↑↓` navigate, `Space` toggle, `s` save, `r` review, `q` back                   |
| Review       | `↑↓` scroll, `r` run, `q` back                                                  |
| Run / Result | `↑↓` scroll, `Tab` filter, `Enter` fold, `f` follow when paused, `q` abort/back |
| History      | `↑↓` navigate, `Enter` open, `d` delete, `q` back                               |
| Run replay   | `↑↓` navigate, `Space` fold, `q` back                                           |

Direct main-menu keys (`d` deploy, `p` plan, `h` history), Vim navigation
(`j/k`, `gg`, `G`), `Home/End`, `PageUp/PageDown`, arrow-key filter switching,
and equivalent `Enter`/`Space` actions remain available as unlisted convenience
keys.

Result labels are intentional: **Ran** means a shell command completed;
**Changed** means dotman installed, created, linked, backed up, or cleaned
something. Errors are shown before warnings, while the final run result remains
visible if saving history fails.

## Configuration

Deployment steps live in `dotman.yaml` in your dotfiles repo:

```yaml
package_managers:
  macos: brew
  ubuntu: brew
  arch: pacman

auto_install_pkg_manager: true

default_shell: fish

install:
  [ghostty, fish, tmux, neovim, lazygit, btop, ripgrep, fzf, starship, atuin]

links:
  ~/.config/fish: config/fish
  ~/.config/nvim: config/nvim
  ~/.config/ghostty: config/ghostty
  ~/.tmux.conf: config/tmux.conf

create:
  - ~/.config/fish-local
  - ~/Workspace/tries

shell:
  - command: fish -lc 'fisher update'
    description: Sync fish plugins
    optional: true
    if: command -v fish >/dev/null 2>&1

clean:
  - target: ~/.config/old-tool
    force: true
```

YAML field reference:

- `package_managers` — per-platform package manager.
- `auto_install_pkg_manager` — if true, attempts to install the package
  manager itself (e.g. Homebrew) before any install steps.
- `default_shell` — login shell to switch to automatically. dotman resolves
  the real path with `command -v` and ensures it is listed in `/etc/shells`.
- `install: [name]` — list of tool names to install. dotman picks the right
  install command for your platform from its internal tool database.
- `links:` — map of target -> source. dotman also accepts list entries with
  `target`, `source`, `backup`, and `relink` for per-link behavior.
- `create:` — directories to ensure exist.
- `shell:` — list of shell commands. Supports `description`,
  `optional: true`, and `if:` (condition guard).
- `clean:` — paths to remove. By default only symlinks are removed; use
  `force: true` to back up and remove regular files/directories.

Profile configuration (repo URL, branch, checkout path, auto-sync) lives at
`~/.config/dotman/config.toml`. dotman manages this automatically — you
don't need to create or edit it by hand.

Per-machine selections are stored under
`~/.local/share/dotman/selection/`, scoped by the normalized `dotman.yaml` path.
Small config edits therefore keep existing choices; newly added item IDs use
their plan defaults.

Run logs are at `~/.local/share/dotman/runs/<ulid>.json` and can be
browsed with `dotman history` or `dotman run <id>`.

## How It Works

The deployment pipeline is:

```text
resolve profile → sync repo (git pull) → load dotman.yaml → build plan → review → execute → save history
```

dotman's profile system manages the dotfiles repo itself (URL, branch, clone
path, auto-sync). The deployment config (`dotman.yaml`) only describes _what_
to deploy — installs, links, creates, shell commands.

Auto-init triggers automatically when no profile or config is found. Headless
mode uses non-interactive defaults. In the TUI, deployment changes are shown in
Plan and Review before Run starts.

## Local Overrides

Machine-specific paths, tokens, and temporary tool setup should stay out of the
shared repository.

### Pi coding agent

Shared Pi instructions, permissions, prompts, plugin settings, and the pinned
plugin catalog live under `config/pi/`. `dotman` links those files into
`~/.pi` and exposes `pi-plugin-stack` in `~/.local/bin`.

Plugin installation is an optional, idempotent deployment action. It runs only
when both `pi` and `jq` are available and the installed packages or
`settings.json.packages` differ from `config/pi/plugins.json`. Run it manually
when needed:

```sh
pi-plugin-stack list
pi-plugin-stack install --dry-run
pi-plugin-stack install
pi-plugin-stack check
```

The catalog accepts exact npm versions only. Installation preserves every
machine-local setting outside `packages` and restores the previous
`settings.json` if a package installation fails.

The global permission policy allows file edits plus common read-only shell,
build, lint, and test commands without prompting. Unknown commands, shell
composition/redirection, in-place transforms, deployment, publication, and
other potentially dangerous operations still require confirmation.

Web Access includes the `librarian` skill and may clone public repositories for
source-backed dependency research. Browser cookie access remains disabled. Pi
Lens uses the shared `config/pi-lens/config.json`, ignores generated dependency
trees, reports actionable warnings, defers formatting until the end of a turn,
and never applies automatic fixes.

The real `~/.pi/agent/settings.json` remains machine-local. The installer
updates only its `packages` field, preserving the selected provider, model, and
other preferences. `models.json`, `auth.json`, API keys, trust decisions,
sessions, caches, and MCP server definitions are never managed by this repo.

Fish loads local-only files from:

```text
~/.config/fish-local/*.fish
```

For first-time setup on a new machine, follow [docs/new-machine.md](docs/new-machine.md).

## Development

```sh
make build
make format        # format all supported tracked files
make format-check  # check formatting without modifying files
make lint
make secret-check  # scan the working tree and Git history for secrets
make test
make config-check  # run the complete local configuration validation
make ci
```

Before committing, format the repository, run the complete configuration
validation, and review the resulting diff:

```sh
make format
make config-check
git diff
```

### Automated releases

Pushing a `vX.Y.Z` tag that matches the version in `Cargo.toml` builds and tests
all four release targets, publishes their archives, checksums, and `install.sh`,
then generates, installs, tests, and pushes `Formula/dotman.rb` to
`tabsp/homebrew-tap`. Manual workflow runs are only for retrying an existing
semantic release tag and build that tag rather than the selected UI branch.

The Homebrew publication uses a write-enabled SSH Deploy Key scoped to the
public `tabsp/homebrew-tap` repository. Its private key is stored in this
repository as the `HOMEBREW_TAP_DEPLOY_KEY` Actions secret; no personal access
token is required.
