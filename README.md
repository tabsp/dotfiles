# dotfiles

[中文](README.zh-CN.md)

dotman is a tiny Rust-based TUI dotfiles deployer for my personal macOS/Linux environment.
It uses a flat YAML config to install software, link config files, create directories,
clean old paths, and run setup commands — all in a Plan -> Confirm -> Run flow with
per-machine state and persistent run history.

## Preview

![dotman main menu](assets/screenshots/dotman-main-menu.png)

![dotfiles workspace](assets/screenshots/dotfiles-workspace.png)

## Quick Start

Install the latest `dotman` release binary:

```sh
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) target="aarch64-apple-darwin" ;;
  Darwin-x86_64) target="x86_64-apple-darwin" ;;
  Linux-aarch64) target="aarch64-unknown-linux-gnu" ;;
  Linux-x86_64) target="x86_64-unknown-linux-gnu" ;;
  *) echo "unsupported platform: $(uname -s)-$(uname -m)" >&2; exit 1 ;;
esac

mkdir -p ~/.local/bin
export PATH="$HOME/.local/bin:$PATH"
curl -fsSL "https://github.com/tabsp/dotfiles/releases/latest/download/dotman-${target}.tar.gz" |
  tar -xz -C ~/.local/bin dotman
```

Then run it. On first launch, dotman auto-initializes the default dotfiles profile:

```sh
dotman
```

This clones `https://github.com/tabsp/dotfiles.git` to
`~/.local/share/dotman/repos/main`, loads `dotman.yaml`, and opens the TUI.

For unattended setup (CI, scripted setup):

```sh
dotman deploy --headless
```

To initialize this repository explicitly:

```sh
dotman init https://github.com/tabsp/dotfiles.git --branch main --profile main
dotman deploy
```

## Usage

```sh
dotman                       # TUI main menu (auto-inits on first run)
dotman deploy                # TUI: sync → plan → confirm → run
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

## 0.2 Release Notes

The 0.2 line is the first profile-based release. Notable changes:

- First-run auto-init: clone repo, write profile config, load `dotman.yaml`
- Profile management: `init`, `sync`, `status`, `profile list/add/remove`
- Headless deployment for scripts, CI, and remote bootstrap
- `--bootstrap-git` for installing git before profile setup when needed
- `--config <path>` for direct config testing without profiles
- TUI main menu, plan, confirm, run, result, history, and run replay views
- Plan selections persisted in `~/.local/share/dotman/state.toml`
- Persistent run history in `~/.local/share/dotman/runs/<ulid>.json`
- Live stdout/stderr streaming during execution, with abort support
- Step-level install retry with 5s/10s/20s backoff
- Link map/list config formats with backup/relink support
- `clean` actions for removing stale symlinks or backed-up paths
- `new-link <target> <source>` helper for updating `dotman.yaml`
- Embedded tool DB with 25 named tools plus default package templates
- E2E scenarios for profile lifecycle, repo sync, runtime deploy, history,
  failure behavior, sudo prompt, new-link, and install branches
- 113 tests pass (`cargo test`)
- `cargo build --release` produces a small static-feeling CLI binary
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` clean

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
