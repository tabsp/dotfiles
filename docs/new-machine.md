# New Machine Setup

这份清单用于新机器初始化。核心思路是先安装 `dotman`，然后让 dotman 自己完成 profile
初始化、仓库 clone、配置加载和部署。

账号登录、SSH/GPG、系统权限和私有配置仍然需要人工处理。

## 1. 旧机器备份

- 确认 SSH/GPG key 可以恢复，或已经迁移到安全位置。
- 保存 `~/.gitconfig.local`。
- 保存 `~/.config/fish-local/*.fish` 中的本地私有配置。
- 确认需要迁移的应用数据、字体、浏览器配置和登录态。

## 2. 系统基础工具

- `curl`
- CA certificates
- `git`（可选；缺失时必须在 headless 部署中加入 `--bootstrap-git`）

macOS 先安装 Xcode Command Line Tools：

```sh
xcode-select --install
```

## 3. 安装 dotman

通过 latest release 二进制安装：

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

确认安装结果：

```sh
dotman --version
```

## 4. 首次部署

直接运行 dotman。首次运行时找不到 profile 会自动初始化：

```sh
dotman
```

这会使用默认 dotfiles 仓库 (`https://github.com/tabsp/dotfiles.git`)，
clone 到 `~/.local/share/dotman/repos/main`，写入
`~/.config/dotman/config.toml`，加载其中的 `dotman.yaml`，然后进入 TUI 主菜单。
在主菜单选择 Deploy，调整 Plan 中的选择，在 Review 中检查实际 action，再按 `r`
开始 Run。完成后确认最终的 Ran、Changed、No Change、Skipped 和 Failed 统计。

### 无交互部署

在脚本、CI 或远程 bootstrap 场景中使用 `--headless`：

```sh
dotman deploy --headless
```

Headless 会输出实时 action 日志和最终统计，将结果写入 History，并在执行失败或中止时
返回非零退出码。历史保存失败也会返回非零，并明确提示检查磁盘空间和数据目录权限。

如果 git 不存在：

```sh
dotman deploy --headless --bootstrap-git
```

如果你希望找不到 profile 时直接失败，而不是自动初始化：

```sh
dotman deploy --headless --no-init
```

如果只想验证某个本地配置文件，不走 profile：

```sh
dotman plan --headless --config ./dotman.yaml
```

### 显式初始化本仓库

```sh
dotman init https://github.com/tabsp/dotfiles.git --branch main --profile main
dotman deploy
```

`init` 会 clone 仓库、验证配置、写入 profile，并打印可部署步骤摘要；它不会执行部署。
随后运行 `dotman deploy` 进入 Plan -> Review -> Run。之后可以用：

```sh
dotman profile list
dotman status
dotman sync
```

检查当前 profile 和仓库状态。

## 5. Fish 生效

如果 deploy 过程中安装了 fish 并修改了默认 shell，重新登录或手动切换：

macOS：

```sh
exec /opt/homebrew/bin/fish -l
```

Linux：

```sh
exec /home/linuxbrew/.linuxbrew/bin/fish -l
```

如果默认 shell 没有生效，先确认 fish 路径和当前 shell：

```sh
command -v fish
echo "$SHELL"
```

再检查账号记录中的登录 shell。

macOS：

```sh
dscl . -read "/Users/$USER" UserShell
```

Linux：

```sh
getent passwd "$USER" | cut -d: -f7
```

## 6. 恢复私有配置

- 恢复 `~/.gitconfig.local`。
- 恢复 `~/.config/fish-local/*.fish`。
- 恢复 SSH/GPG key，并检查权限。

## 7. 手动应用设置

- 登录 1Password、浏览器、GitHub、云同步等账号。
- 配置系统权限，例如终端、编辑器和窗口管理工具的 Accessibility 权限。
- Ghostty 安装由内置工具数据库按平台处理：macOS 使用 Homebrew cask，Arch 使用
  pacman，Fedora 使用 COPR，Ubuntu 使用社区安装脚本。其他 Linux 发行版按
  [Ghostty 官方安装说明](https://ghostty.org/docs/install/binary) 手动安装；
  `deploy` 仍会负责链接 Ghostty 配置。
- 机器相关的 Ghostty 设置（窗口大小、透明度等）可以创建
  `~/.config/ghostty/config.local`。
- 检查字体、输入法、浏览器扩展和 GUI 应用设置。
- 首次打开 Neovim，让插件和工具完成安装。

## 8. 验证

```sh
dotman status
dotman doctor
dotman history
```

`dotman history` 应显示刚完成的运行；打开记录后可逐 action 查看状态、错误和已保存输出。
Plan 选择保存在 `~/.local/share/dotman/selection/`，同一配置文件的小幅修改不会清空
已有选择。

检查 Maple Mono 字体：

```sh
fc-list | grep -i "Maple Mono"
```

macOS 如果没有 `fc-list`，可以用字体册或直接在 Ghostty 中选择
`Maple Mono NF CN` 验证。
