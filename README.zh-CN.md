# dotfiles

[English](README.md)

dotman 是一个小型 Rust-based dotfiles 部署工具，用于我的个人 macOS/Linux 环境。
它使用 Dotbot-like 的有序 YAML 配置来链接文件、创建目录并运行设置命令。

## 效果预览

![终端环境预览](assets/screenshots/terminal-preview.png)

## 前置依赖

- curl

安装脚本会负责安装或更新 `dotman`，并下载发布好的 dotfiles bundle。完整首跑自动化
（包管理器、fish、登录 shell）仍是后续计划；v0.2 alpha 重点是 TUI 的
Plan → Confirm → Run 流程。

首次设置时，从网站安装 `dotman` 和发布好的 dotfiles bundle：

```sh
curl -fsSL https://dotfiles.tabsp.com/install | sh
```

无交互安装：

```sh
curl -fsSL https://dotfiles.tabsp.com/install | sh -s -- --yes
```

`--yes` 会用 `dotman --auto plan` 验证安装后的 bundle；目前不会自动执行完整 deploy。

如果当前目录没有 `dotman.yaml`，`dotman` 会自动 fallback 到 `DOTFILES_DIR`，
再 fallback 到安装好的 `~/.local/share/tabsp-dotfiles` bundle。

## 使用

TUI 是主入口。`dotman` 无参数打开主菜单：

```sh
dotman                  # TUI 主菜单
dotman deploy           # TUI: plan → 确认 → 跑 deploy
dotman bootstrap        # TUI: plan → 确认 → 跑 bootstrap
dotman plan             # TUI: 只看 plan
dotman history          # TUI: 历史 run
dotman run <ulid>       # TUI: 回放某次 run
dotman --auto deploy    # headless: plan → 自动确认 → 跑（脚本用）
```

只查看 plan 不执行：

```sh
dotman plan
```

Headless 模式（脚本用）：

```sh
dotman --auto deploy
```

## TUI 键位

| 键 | 动作 |
| --- | --- |
| `↑↓` 或 `j k` | 上下导航 |
| `space` | 切换当前 step |
| `a` / `n` | 全选 / 全不选 |
| `s` | 把 selection 存到 state |
| `r` | 跑 |
| `i` | 看当前 step 的 detail |
| `e` | 从 result 回到 plan view |
| `q` 或 `Esc` | 退回 / 退出 |

## 配置

部署步骤写在 `dotman.yaml`（和可选的 `dotman.bootstrap.yaml`，专门放 bootstrap 命令）：

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

字段说明：

- `package_managers` — 每平台包管理器（用来查工具的安装命令）。工具名本身列在 `install:` 里；具体的安装命令在 dotman 内部的工具库（TOML，约 20 条，编译进 binary）。
- `install: [name]` — 要装的工具名列表。dotman 自动选对平台的安装命令。
- `links:` — target → source 映射。dotman 处理相对/绝对路径、备份、relink 等情况。
- `create:` — 确保存在的目录。
- `shell:` — 要跑的 shell 命令列表。支持 `description`、`optional: true`（失败只警告不中断）、`if:`（条件守卫）。

State（per-machine selection）在 `~/.local/share/dotman/state.toml`——首次跑按 layer 策略自动应用默认（terminal/shell/multiplexer 是 pick-one，software/enhancement 是 all）。

Run log 在 `~/.local/share/dotman/runs/<ulid>.json`，可以用 `dotman history` 或 `dotman run <id>` 浏览。

## 工具

| 工具 | 用途 |
| ---- | ---- |
| [fish](https://fishshell.com) | 自带自动补全的 shell |
| [starship](https://starship.rs) | 跨 shell 的提示符 |
| [direnv](https://direnv.net) | 按目录加载环境变量 |
| [mise](https://mise.jdx.dev) | 运行环境和工具版本管理器 |
| [fzf](https://github.com/junegunn/fzf) | 模糊查找（文件、历史、zoxide 跳转） |
| [zoxide](https://github.com/ajeetdsouza/zoxide) | 智能 `cd`，带目录权重排序 |
| [fd](https://github.com/sharkdp/fd) | 快速 `find` 替代 |
| [ripgrep](https://github.com/BurntSushi/ripgrep) | 快速 `grep` 替代 |
| [eza](https://github.com/eza-community/eza) | 现代 `ls` 替代，带图标 |
| [bat](https://github.com/sharkdp/bat) | 带语法高亮的 `cat` |
| [tealdeer](https://github.com/dbrgn/tealdeer) | 快速 `tldr` 客户端 |
| [btop](https://github.com/aristocratos/btop) | 资源监控 |
| [fastfetch](https://github.com/fastfetch-cli/fastfetch) | 系统信息展示 |
| [dua-cli](https://github.com/Byron/dua-cli) | 磁盘使用分析 |
| [neovim](https://neovim.io) | 编辑器 |
| [lazygit](https://github.com/jesseduffield/lazygit) | 终端 Git UI |
| [yazi](https://github.com/sachinsenal/yazi) | 终端文件管理器 |
| [tmux](https://github.com/tmux/tmux) | 终端复用器，Catppuccin 主题 |
| [ghostty](https://ghostty.org) | GPU 加速终端，Catppuccin Mocha 主题 |
| [jq](https://github.com/jqlang/jq) + [yq](https://github.com/mikefarah/yq) | JSON/YAML 命令行处理器 |
| [ruby](https://www.ruby-lang.org) | `try` 实验管理器的运行时 |

所有包在 bootstrap 时通过 `brew bundle --file packages/Brewfile` 安装。Fish 启动时自动集成大部分工具，并定义自定义函数：`zi`（fzf+zoxide 跳转）、`ff`（fzf 文件选择）、`y`（yazi 并自动 cd）、`t`（tmux 附加/创建）。参见 `config/fish/config.fish`。

## 目录结构

- `bin/`：链接到 `~/.local/bin` 的用户脚本（tmux-status 等）
- `config/`：被跟踪的 dotfiles 源文件（fish、nvim、ghostty、btop、fastfetch、starship、tealdeer、tmux、git）
- `docs/`：设置说明和手动清单
- `dotman.yaml`：部署步骤（链接配置、创建目录、同步衍生状态）
- `dotman.bootstrap.yaml`：bootstrap 步骤（安装包、字体）
- `packages/`：Brewfile 和平台相关的安装辅助脚本
- `scripts/`：安装脚本和静态站点构建脚本
- `src/`：Rust 部署工具源码
- `tests/`：CLI 集成测试

## 配置

部署步骤写在 `dotman.yaml` 中。bootstrap 步骤写在
`dotman.bootstrap.yaml` 中。

支持的指令：

- `defaults`
- `link`
- `create`
- `shell`
- `clean`: planned / dry-run placeholder

示例：

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

字段说明：

- `defaults.link.create`：为链接目标自动创建缺失的父目录。
- `defaults.link.relink`：目标已是 symlink 但指向不对时，替换为新的链接。
- `defaults.link.backup`：目标冲突时，先备份原目标再创建链接。
- `defaults.link.relative`：创建相对 symlink。
- `defaults.shell.stdout`：默认继承 shell 命令的 stdout。
- `defaults.shell.stderr`：默认继承 shell 命令的 stderr。
- `link`：把目标路径映射到源路径；单个链接项也可以使用 `path`，并覆盖
  `create`、`relink`、`backup`、`relative` 和 `if`。
- `create`：创建目录，并跟随路径中已有的 symlink 组件。
- `shell.command`：通过 `sh -c` 执行的命令。
- `shell.description`：日志中显示的人类可读步骤名称。
- `shell.if`：命令运行前必须成功的 shell 条件。
- `shell.optional`：为 `true` 时，命令失败只记录为 warning，后续步骤继续执行。
  默认是 `false`。
- `shell.stdout` / `shell.stderr`：单条命令的输出覆盖设置。
- `clean`：目前会被解析并在 dry-run 中显示，但非 dry-run 清理尚未实现。

`deploy` 对核心文件操作采用 fail-fast：link/create 失败会停止本次运行。受网络影响的
同步命令可以标记为 `optional: true`，临时失败不会导致整个 deploy 失败。

## 本地覆盖

机器相关的路径、token 和临时工具配置不要放进共享仓库。

fish 会加载本地文件：

```text
~/.config/fish/local.d/*.fish
```

新机器首次设置参考 [docs/new-machine.md](docs/new-machine.md)。

## 开发

```sh
make build
make lint
make test
make ci
```

在 Docker 中运行真实 Linux 安装流程：

```sh
make e2e-linux
make e2e-linux E2E_ARGS="--local --inspect --keep"
```

E2E 脚本会基于当前 worktree 构建 `dotman`，在 Docker 内提供本地
installer/manifest/bundle，执行安装脚本，并验证安装后的 dotfiles。`--inspect` 会以
`tester` 用户进入完成后的容器，方便人工验收。

## 状态（v0.2 alpha）

这是 dotman 的工作版本重写，但还**没到生产可用**。架构和核心数据流完整；下面列出的是已知缺口，需要真实环境测试或后续工作。

### 已知 bug

- 本地 unit/lint 检查和 Docker E2E smoke 之后，暂无已知阻塞 bug。

### 还没做的

- **FirstRunScreen**：找不到 `dotman.yaml` 时直接报错。首跑向导（自动装包管理器 + clone 仓库）plan 里有但没做
- **子进程逐行实时 pipe**：RunView 已接入执行事件和捕获到的命令输出，但长时间运行的子进程 stdout/stderr 仍是在单个 action 结束后进入 TUI，不是进程运行时逐行进入
- **visual mockup 截图**：`assets/screenshots/` 还没有 PNG

### 能跑的

- 39 个 unit tests 通过（`cargo test`）
- `cargo build --release` 产出 3.7MB binary
- `cargo clippy --all-targets -- -D warnings` 和 `cargo fmt --check` 干净
- TUI flow 已包含 MainMenu / PlanView / ConfirmView / RunView / ResultView / HistoryView / RunReplay，Catppuccin Mocha 主题 + Nerd Font 图标
- 显式 TUI 子命令会直接进入对应 screen
- Plan 选择会持久化到 `~/.local/share/dotman/state.toml`
- RunView 会显示执行事件、捕获到的命令输出，并支持 action 边界的协作式 abort
- 工具 db 17 条，覆盖用户实际用的所有工具
- `dotman --auto plan` 已能解析真实 `dotman.yaml` 并输出 JSON
- `dotman --auto deploy` 端到端跑通（读 config → build plan → 执行 → 存 run log）
- install 步的 retry（5s/10s/20s exponential backoff）
- `dotman new-link <target> <source>` 会更新 `dotman.yaml` 的 `links:` map
- `make e2e-linux` 已通过 installer/bundle/plan Docker smoke test
- `dotman history` 和 `dotman run <ulid>` 浏览 / 回放历史
- State 持久化到 `~/.local/share/dotman/state.toml`

## 发布

网站构建会发布安装脚本和运行时 bundle：

```sh
scripts/build-site.sh
```

Vercel 使用：

- Build Command：`scripts/build-site.sh`
- Output Directory：`public`

`public/manifest.json` 会指向 `dotfiles.tabsp.com` 上的
`bundle/latest.tar.gz`，以及 GitHub 最新 Release 里的 dotman 二进制。
推送 `v*` tag 会触发 release workflow 构建这些 dotman 二进制。
