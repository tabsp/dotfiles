# dotfiles

[中文](README.zh-CN.md)

dotman is a tiny Rust-based TUI dotfiles deployer for my personal macOS/Linux environment.
It uses a flat YAML config to install software, link config files, and run setup commands — all in a Plan → Confirm → Run flow with per-machine state and persistent run history.

## Preview

![Terminal setup preview](assets/screenshots/terminal-preview.png)

## Prerequisites

- curl

The installer downloads `dotman` and the latest dotfiles bundle. Full
first-run automation (package manager, fish, login shell) is still planned;
v0.2 alpha focuses on the TUI Plan → Confirm → Run flow.

For first-time setup, install `dotman` and the published dotfiles bundle from
the site:

```sh
curl -fsSL https://dotfiles.tabsp.com/install | sh
```

This installs the binary and expands the bundle to
`~/.local/share/tabsp-dotfiles`.

For unattended setup (CI, scripted setup):

```sh
curl -fsSL https://dotfiles.tabsp.com/install | sh -s -- --yes
```

`--yes` verifies the installed bundle with `dotman --auto plan`; it does not
run a full deploy yet.

## Usage

The TUI is the primary interface. `dotman` with no args opens the main menu.

```sh
dotman                  # TUI main menu
dotman deploy           # TUI: plan → confirm → run deploy
dotman bootstrap        # TUI: plan → confirm → run bootstrap
dotman plan             # TUI: show plan only, no execution
dotman history          # TUI: browse past runs
dotman run <ulid>       # TUI: replay a past run
dotman --auto deploy    # headless: plan → auto-confirm → run (for scripts)
```

## TUI Keys

| Key | Action |
| --- | --- |
| `↑↓` or `j k` | Navigate |
| `space` | Toggle current step |
| `a` / `n` | Select all / none |
| `s` | Save selection to state |
| `r` | Run |
| `i` | Detail for current step |
| `e` | Back to plan view (from result) |
| `q` or `Esc` | Back / quit |

## Configuration

Deployment steps live in `dotman.yaml` (and optionally `dotman.bootstrap.yaml`
for bootstrap-specific commands).

```yaml
package_managers:
  macos: brew
  ubuntu: brew
  arch: pacman

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

- `package_managers` — per-platform package manager (used to look up the
  right install command for each tool). Tools themselves are listed by name
  in `install:`; the actual install commands live in dotman's internal
  tool database (TOML, ~20 entries, ships compiled in).
- `install: [name]` — list of tool names to install. dotman picks the right
  install command for your platform.
- `links:` — map of target → source. dotman handles relative/absolute,
  backup, and relink based on the source state.
- `create:` — directories to ensure exist.
- `shell:` — list of shell commands to run. Supports `description`,
  `optional: true` (warn on failure, don't abort), and `if:` (condition
  guard).

State (per-machine selection) lives at
`~/.local/share/dotman/state.toml` — first-run defaults are applied
automatically based on layer strategy (pick-one for terminal/shell/multiplexer,
all for software/enhancement).

Run logs are at `~/.local/share/dotman/runs/<ulid>.json` and can be
browsed with `dotman history` or `dotman run <id>`.

## Status (v0.2 alpha)

This is a working rewrite of dotman but not yet production-ready. The
architecture and core data flow are complete; the items below are known
gaps that need real-world testing or follow-up work.

### Known bugs

- No known blocking bug after local unit/lint checks and Docker E2E smoke.

### Not yet implemented

- **FirstRunScreen**: when no `dotman.yaml` is found, dotman errors out.
  A first-run wizard (auto-install package manager, clone repo) was
  planned but not implemented.
- **Subprocess live pipe**: RunView receives execution events and captured
  command output, but long-running subprocess stdout/stderr is still delivered
  after each action completes rather than line-by-line while the process runs.
- **Visual mockups / screenshots**: no PNG mockups in `assets/screenshots/`.

### What works

- 39 unit tests pass (`cargo test`)
- `cargo build --release` produces a 3.7MB binary
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` clean
- TUI flow includes MainMenu / PlanView / ConfirmView / RunView /
  ResultView / HistoryView / RunReplay with Catppuccin Mocha theme
  and Nerd Font icons
- Explicit TUI subcommands open their target screens directly
- Plan selections persist to `~/.local/share/dotman/state.toml`
- RunView shows execution events, captured command output, and supports
  cooperative abort between actions
- Tool DB has 17 entries covering the user's actual tools
- `dotman --auto plan` parses the real `dotman.yaml` and emits JSON
- `dotman --auto deploy` runs end-to-end (loads config → builds plan →
  executes → saves run log)
- Per-step retry for install actions (5s/10s/20s exponential backoff)
- `dotman new-link <target> <source>` updates the `links:` map in
  `dotman.yaml`
- `make e2e-linux` passes the installer/bundle/plan Docker smoke test
- `dotman history` and `dotman run <ulid>` browse and replay past runs
- State persistence to `~/.local/share/dotman/state.toml`

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

`make e2e-linux` runs the real Linux install flow in Docker.
