# New Machine Setup

这份清单用于新机器初始化。核心思路是先安装 `dotman`，然后让 dotman 自己完成仓库
clone、配置加载和部署。

账号登录、SSH/GPG、系统权限和私有配置仍然需要人工处理。

## 1. 旧机器备份

- 确认 SSH/GPG key 可以恢复，或已经迁移到安全位置。
- 保存 `~/.gitconfig.local`。
- 保存 `~/.config/fish/local.d/*.fish` 中的本地私有配置。
- 确认需要迁移的应用数据、字体、浏览器配置和登录态。

## 2. 系统基础工具

- `curl`
- CA certificates

macOS 先安装 Xcode Command Line Tools：

```sh
xcode-select --install
```

## 3. 安装 dotman

通过 Homebrew：

```sh
brew install tabsp/tap/dotman
```

或从 GitHub Release 下载二进制放到 `~/.local/bin/`。

## 4. 首次部署

直接运行 dotman。首次运行时找不到 profile 会自动初始化：

```sh
dotman
```

这会使用默认 dotfiles 仓库 (`https://github.com/tabsp/dotfiles.git`)，
clone 到 `~/.local/share/dotman/repos/main`，加载其中的 `dotman.yaml`，
生成 plan 并进入 TUI 确认界面。

### 无交互部署

在脚本、CI 或远程 bootstrap 场景中使用 `--headless`：

```sh
dotman deploy --headless
```

如果 git 不存在：

```sh
dotman deploy --headless --bootstrap-git
```

### 自定义仓库

```sh
dotman init https://github.com/you/dotfiles.git
dotman deploy
```

init 会 clone 仓库、写入 profile 配置，然后展示 plan 预览。确认后用 `dotman deploy`
执行部署。

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

如果默认 shell 没有生效：

```sh
command -v fish
getent passwd "$USER" | cut -d: -f7
echo "$SHELL"
```

## 6. 恢复私有配置

- 恢复 `~/.gitconfig.local`。
- 恢复 `~/.config/fish/local.d/*.fish`。
- 恢复 SSH/GPG key，并检查权限。

## 7. 手动应用设置

- 登录 1Password、浏览器、GitHub、云同步等账号。
- 配置系统权限，例如终端、编辑器和窗口管理工具的 Accessibility 权限。
- Linux 上按发行版或 [Ghostty 官方安装说明](https://ghostty.org/docs/install/binary)
  手动安装 Ghostty；`deploy` 只负责链接 Ghostty 配置。机器相关的 Ghostty 设置
  （窗口大小、透明度等）可以创建 `~/.config/ghostty/config.local`。
- 检查字体、输入法、浏览器扩展和 GUI 应用设置。
- 首次打开 Neovim，让插件和工具完成安装。

## 8. 验证

```sh
dotman status
dotman doctor
```

检查 Maple Mono 字体：

```sh
fc-list | grep -i "Maple Mono"
```

macOS 如果没有 `fc-list`，可以用字体册或直接在 Ghostty 中选择
`Maple Mono NF CN` 验证。
