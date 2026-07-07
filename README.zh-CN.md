# dotfiles

[English](README.md)

dotman 是一个轻量的 Rust TUI dotfiles 部署工具，用于我的 macOS/Linux 开发环境。
使用 YAML 配置安装软件、链接配置文件、创建目录、清理旧路径、运行设置命令——全部走
Plan -> Confirm -> Run 流程，支持每台机器独立的状态和持久化运行历史。

## 预览

![dotman main menu](assets/screenshots/dotman-main-menu.png)

![dotfiles workspace](assets/screenshots/dotfiles-workspace.png)

## 快速开始

下载 latest release 二进制安装 `dotman`：

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

然后直接运行。首次启动时，dotman 会自动初始化默认 dotfiles profile：

```sh
dotman
```

这会 clone `https://github.com/tabsp/dotfiles.git` 到
`~/.local/share/dotman/repos/main`，加载其中的 `dotman.yaml`，并打开 TUI。

无人值守模式（CI、脚本部署）：

```sh
dotman deploy --headless
```

显式初始化本仓库：

```sh
dotman init https://github.com/tabsp/dotfiles.git --branch main --profile main
dotman deploy
```

## 用法

```sh
dotman                       # TUI 主菜单（首次运行自动 init）
dotman deploy                # TUI: sync → plan → 确认 → 执行
dotman plan                  # TUI: 仅展示 plan，不执行
dotman init [repo]           # 初始化一个 dotfiles profile
dotman sync                  # git pull 当前 profile
dotman status                # 查看 profile 和仓库状态
dotman doctor                # 检查系统前置依赖
dotman profile list          # 列出已配置的 profile
dotman profile add <name> <repo>
dotman profile remove <name>
dotman history               # TUI: 浏览历史运行记录
dotman run <ulid>            # TUI: 重放历史运行
dotman new-link <target> <source>
dotman deploy --headless     # headless: 无交互部署
dotman plan --headless       # headless: 输出 JSON plan
```

全局选项：

```sh
--headless         无交互模式；使用安全默认值，遇到歧义直接失败
--bootstrap-git    允许 dotman 在 profile/bootstrap 前先安装 git
--config <path>    直接使用某个 dotman.yaml，跳过 profile 解析
--no-init          找不到配置时直接失败，而不是自动初始化
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
    if: command -v fish >/dev/null 2>&1

clean:
  - target: ~/.config/old-tool
    force: true
```

字段说明：

- `package_managers` — 按平台指定包管理器。
- `auto_install_pkg_manager` — 如果为 true，在安装任何工具前先尝试安装包管理器（如 Homebrew）。
- `default_shell` — 自动切换登录 shell。dotman 会用 `command -v` 解析实际路径，并确保路径在 `/etc/shells` 中。
- `install: [name]` — 要安装的工具名列表。dotman 从内置工具数据库中选择对应平台的安装命令。
- `links:` — target -> source 映射。也支持 list 写法，可为单个链接设置
  `target`、`source`、`backup`、`relink`。
- `create:` — 需要确保存在的目录。
- `shell:` — 要执行的 shell 命令列表。支持 `description`、`optional: true` 和 `if:` 条件。
- `clean:` — 要清理的路径。默认只移除 symlink；使用 `force: true` 时会先备份再移除普通文件/目录。

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

## 0.2 Release Notes

0.2 是第一个基于 profile 的版本，主要变化：

- 首次运行自动 init：clone 仓库、写入 profile 配置、加载 `dotman.yaml`
- Profile 管理：`init`、`sync`、`status`、`profile list/add/remove`
- Headless 部署，适合脚本、CI 和远程 bootstrap
- `--bootstrap-git`：需要时先安装 git，再继续 profile 初始化
- `--config <path>`：直接测试某个配置文件，不走 profile
- TUI 主菜单、plan、confirm、run、result、history、run replay 视图
- Plan 选择持久化到 `~/.local/share/dotman/state.toml`
- 运行历史持久化到 `~/.local/share/dotman/runs/<ulid>.json`
- 执行时实时显示 stdout/stderr，并支持中断
- 安装步骤支持 5s/10s/20s 退避重试
- `links` 支持 map/list 两种格式，并支持 backup/relink
- `clean` action 可清理旧 symlink 或备份后移除旧路径
- `new-link <target> <source>` 辅助更新 `dotman.yaml`
- 内置工具数据库包含 25 个命名工具和默认包管理器模板
- E2E 场景覆盖 profile lifecycle、repo sync、runtime deploy、history、
  failure behavior、sudo prompt、new-link、install branches
- 113 个测试通过（`cargo test`）
- `cargo build --release` 生成轻量 CLI 二进制
- `cargo clippy --all-targets -- -D warnings` 和 `cargo fmt --check` 通过

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
