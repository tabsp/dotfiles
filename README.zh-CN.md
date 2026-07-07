# dotfiles

[English](README.md)

dotman 是一个轻量的 Rust TUI dotfiles 部署工具，用于我的 macOS/Linux 开发环境。
使用 YAML 配置安装软件、链接配置文件、运行设置命令——全部走 Plan → Confirm → Run 流程，支持每台机器独立的状态和持久化运行历史。

## 预览

![Terminal setup preview](assets/screenshots/terminal-preview.png)

## 快速开始

通过 Homebrew（或其他任意方式）安装 `dotman`：

```sh
brew install tabsp/tap/dotman
```

然后直接运行。首次运行 dotman 会自动用默认 dotfiles 仓库初始化：

```sh
dotman
```

这会 clone `https://github.com/tabsp/dotfiles.git` 到 `~/.local/share/dotman/repos/main`，
加载其中的 `dotman.yaml`，并打开 TUI plan 视图。

无人值守模式（CI、脚本部署）：

```sh
dotman deploy --headless
```

使用自己的 dotfiles 仓库：

```sh
dotman init https://github.com/you/dotfiles.git
dotman deploy
```

## 用法

```sh
dotman                       # TUI 主菜单（首次运行自动 init）
dotman deploy                # TUI: sync → plan → 确认 → 执行
dotman plan                  # TUI: 仅展示 plan，不执行
dotman init [repo]           # 初始化 dotfiles profile
dotman sync                  # git pull 当前 profile
dotman status                # 查看 profile 和仓库状态
dotman doctor                # 检查系统前置依赖
dotman profile list          # 列出已配置的 profile
dotman history               # TUI: 浏览历史运行记录
dotman run <ulid>            # TUI: 重放历史运行
dotman deploy --headless     # headless: 无交互部署
dotman plan --headless       # headless: 输出 JSON plan
```

## TUI 快捷键

| 键 | 功能 |
| --- | --- |
| `↑↓` 或 `j k` | 导航 |
| `space` | 切换当前步骤 |
| `a` / `n` | 全选 / 全不选 |
| `1-6` | 折叠/展开层级 |
| `s` | 保存选择到状态文件 |
| `r` | 执行 |
| `e` | 返回 plan 视图（从结果页） |
| `q` 或 `Esc` | 返回 / 退出 |

## 配置

部署步骤写在 dotfiles 仓库的 `dotman.yaml` 中：

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

字段说明：

- `package_managers` — 按平台指定包管理器。
- `auto_install_pkg_manager` — 如果为 true，在安装任何工具前先尝试安装包管理器（如 Homebrew）。
- `default_shell` — 自动切换登录 shell。dotman 会用 `command -v` 解析实际路径，并确保路径在 `/etc/shells` 中。
- `install: [name]` — 要安装的工具名列表。dotman 从内置工具数据库中选择对应平台的安装命令。
- `links:` — target → source 映射。dotman 处理相对/绝对路径、备份和重链接。
- `create:` — 需要确保存在的目录。
- `shell:` — 要执行的 shell 命令列表。支持 `description`、`optional: true` 和 `if:` 条件。

Profile 配置（仓库 URL、branch、checkout 路径、自动同步）保存在
`~/.config/dotman/config.toml`。dotman 自动管理此文件，不需要手动创建或编辑。

每台机器的选择状态保存在 `~/.local/share/dotman/state.toml`，首次运行
会根据层级策略自动应用默认选择。

运行日志保存在 `~/.local/share/dotman/runs/<ulid>.json`，可通过
`dotman history` 或 `dotman run <id>` 浏览。

## 工作原理

部署流水线：

```
解析 profile → 同步 repo（git pull）→ 加载 dotman.yaml → 构建 plan → 确认 → 执行 → 保存历史
```

dotman 的 profile 系统管理 dotfiles 仓库本身（URL、分支、clone 路径、自动同步）。
部署配置（`dotman.yaml`）只描述*部署什么*——安装、链接、创建、shell 命令。

找不到 profile 或配置时，auto-init 自动触发。headless 模式下使用全部默认值；
交互模式下会提示你确认。

## 状态

### 已实现

- 80 个测试通过（`cargo test`）
- `cargo build --release` 生成 ~4MB 二进制
- `cargo clippy --all-targets -- -D warnings` 和 `cargo fmt --check` 通过
- TUI：MainMenu / PlanView / ConfirmView / RunView / ResultView /
  HistoryView / RunReplay，Catppuccin Mocha 主题和 Nerd Font 图标
- 首次运行自动 init（clone 仓库、写入 profile、加载配置）
- Profile 管理（添加/删除/列出，支持多个 dotfiles 仓库）
- Git 引导安装（headless 模式下 `--bootstrap-git` 自动安装 git）
- Headless 模式（`--headless`），适合脚本、CI 和远程 bootstrap
- Plan 选择持久化到 `~/.local/share/dotman/state.toml`
- RunView 实时显示子进程 stdout/stderr 输出，支持终止当前进程组的中断
- 工具数据库有 17 个条目，覆盖实际使用的工具
- `dotman plan --headless` 输出 JSON
- `dotman deploy --headless` 端到端运行
- 安装操作的步骤级重试（5s/10s/20s 指数退避）
- `dotman new-link <target> <source>` 更新 `dotman.yaml`
- `dotman history` 和 `dotman run <ulid>` 浏览和重放历史运行

### 尚未实现

- **E2E 覆盖**：新的 profile-based bootstrap 流程的端到端测试尚未实现。

## 本地覆盖

机器特定的路径、令牌和临时工具设置应该放在共享仓库之外。

Fish 从以下路径加载本地文件：

```text
~/.config/fish/local.d/*.fish
```

新机器的首次设置参考 [docs/new-machine.md](docs/new-machine.md)。

## 开发

```sh
make build
make lint
make test
make ci
```
