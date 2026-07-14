# dotfiles

[English](README.md)

dotman 是一个轻量的 Rust TUI dotfiles 部署工具，用于我的 macOS/Linux 开发环境。
使用 YAML 配置安装软件、链接配置文件、创建目录、清理旧路径、运行设置命令——全部走
Plan -> Review -> Run 流程，支持每台机器独立的状态和持久化运行历史。

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

原地更新已有安装（下载并校验当前平台对应的 GitHub Release 产物）：

```bash
dotman self update
```

然后直接运行。首次启动时，dotman 会初始化默认 dotfiles profile 并进入主菜单；
选择 Deploy、调整 Plan，再查看 Review 后开始 Run：

```sh
dotman
```

这会 clone `https://github.com/tabsp/dotfiles.git` 到
`~/.local/share/dotman/repos/main`，加载其中的 `dotman.yaml`，并打开 TUI。

无人值守模式（CI、脚本部署）：

```sh
dotman deploy --headless
```

Headless 与 TUI 使用相同的执行和结果模型，会输出实时 action 日志与最终
`ran / changed / no change / failed` 汇总、保存运行历史，并在失败或中止时
返回非零退出码。

显式初始化本仓库：

```sh
dotman init https://github.com/tabsp/dotfiles.git --branch main --profile main
dotman deploy
```

## 用法

```sh
dotman                       # TUI 主菜单（首次运行自动 init）
dotman deploy                # TUI: sync → Plan → Review → Run
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

Footer 只显示当前页面的主快捷键：

| 页面         | 主快捷键                                                            |
| ------------ | ------------------------------------------------------------------- |
| 主菜单       | `↑↓` 导航，`Enter` 打开，`q` 退出                                   |
| Plan         | `↑↓` 导航，`Space` 切换，`s` 保存，`r` 进入 Review，`q` 返回        |
| Review       | `↑↓` 滚动，`r` 执行，`q` 返回                                       |
| Run / Result | `↑↓` 滚动，`Tab` 过滤，`Enter` 折叠，暂停时 `f` 跟随，`q` 中止/返回 |
| History      | `↑↓` 导航，`Enter` 打开，`d` 删除，`q` 返回                         |
| 历史回放     | `↑↓` 导航，`Space` 折叠，`q` 返回                                   |

主菜单直达键（`d` 部署、`p` 计划、`h` 历史）、Vim 导航（`j/k`、`gg`、`G`）、
`Home/End`、`PageUp/PageDown`、方向键切换过滤器，以及等价的 `Enter`/`Space`
操作仍然可用，但不在 footer 中提示。

结果名称有明确区分：**Ran** 表示 shell 命令已执行；**Changed** 表示 dotman
完成了安装、创建、链接、备份或清理。错误优先于警告展示；历史保存失败时，
最终运行结果仍保持可见。

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

每台机器的选择状态保存在 `~/.local/share/dotman/selection/`，并按规范化后的
`dotman.yaml` 路径隔离。小幅修改配置不会丢失已有选择；新增 item ID 使用 plan
默认值。

运行日志保存在 `~/.local/share/dotman/runs/<ulid>.json`，可通过
`dotman history` 或 `dotman run <id>` 浏览。

## 工作原理

部署流水线：

```text
解析 profile → 同步 repo（git pull）→ 加载 dotman.yaml → 构建 plan → review → 执行 → 保存历史
```

dotman 的 profile 系统管理 dotfiles 仓库本身（URL、分支、clone 路径、自动同步）。
部署配置（`dotman.yaml`）只描述*部署什么*——安装、链接、创建、shell 命令。

找不到 profile 或配置时会自动触发 auto-init。Headless 使用无交互默认值；
TUI 会在 Run 开始前通过 Plan 和 Review 展示部署变更。

## 本地覆盖

机器特定的路径、令牌和临时工具设置应该放在共享仓库之外。

Fish 从以下路径加载本地文件：

```text
~/.config/fish-local/*.fish
```

新机器的首次设置参考 [docs/new-machine.md](docs/new-machine.md)。

## 开发

```sh
make build
make format        # 格式化所有支持且由 Git 跟踪的文件
make format-check  # 只检查格式，不修改文件
make lint
make test
make config-check  # 执行完整的本地配置验证
make ci
```

提交前先统一格式、执行完整配置检查，再查看最终差异：

```sh
make format
make config-check
git diff
```
