# New Machine Setup

这份清单用于新机器初始化。默认路径是直接运行网站安装脚本，让它安装/更新
`dotman`、下载 dotfiles bundle、安装 Homebrew/fish，并执行 bootstrap/deploy。
账号登录、SSH/GPG、系统权限和私有配置仍然需要人工处理。

安装脚本不会直接把当前父 shell 切换成 fish。它会在结束时打印当前终端可用的
`exec .../fish -l` 命令；执行后，当前终端才会进入 fish。默认登录 shell 的修改在
重新登录后生效。

## 1. 旧机器备份

- 确认 SSH/GPG key 可以恢复，或已经迁移到安全位置。
- 保存 `~/.gitconfig.local`。
- 保存 `~/.config/fish/local.d/*.fish` 中的本地私有配置。
- 确认需要迁移的应用数据、字体、浏览器配置和登录态。

## 2. 系统基础工具

先确保系统有基础下载工具：

- `curl`
- CA certificates

macOS 先安装 Xcode Command Line Tools：

```sh
xcode-select --install
```

Linux 根据发行版安装基础工具。安装脚本会通过 Homebrew 安装主要 CLI 工具；Linux
字体安装还需要 `unzip` 和 `fontconfig`，缺失时 bootstrap 会报出具体依赖。

## 3. 一键安装

从发布站点安装 `dotman`，并下载最新 dotfiles bundle 到
`~/.local/share/tabsp-dotfiles`：

```sh
curl -fsSL https://dotfiles.tabsp.com/install | sh
```

这个写法会保留当前终端作为 TTY，安装脚本可以正常显示交互确认和动态进度。
安装脚本会先执行 dry-run，确认后才运行真正的 bootstrap/deploy。确认过流程后，也可以使用无交互模式：

```sh
curl -fsSL https://dotfiles.tabsp.com/install | sh -s -- --yes
```

安装脚本会：

- 安装或更新 `~/.local/bin/dotman`。
- 安装或更新 `~/.local/share/tabsp-dotfiles`。
- 检测缺失的 Homebrew、fish 和 shell 配置，通过统一的多选提示询问要安装哪些；默认选择全部。`--yes` 模式会自动安装全部。
- 尝试把 fish 加入 `/etc/shells` 并改为默认登录 shell。
- 预览并执行 `dotman bootstrap` 和 `dotman deploy`。

如果当前 shell 还没有把 `~/.local/bin` 加进 `PATH`，脚本结束时会给出提示。结束提示
也会打印绝对路径形式的后续命令，例如：

```sh
~/.local/bin/dotman bootstrap
~/.local/bin/dotman deploy
```

## 4. Fish 生效确认

安装脚本成功修改默认 shell 时，会提示：

```sh
Default shell changed to fish.
New login sessions will start fish after you log out and back in.
```

当前终端需要执行脚本打印的绝对路径命令，例如 Linuxbrew 场景：

```sh
exec /home/linuxbrew/.linuxbrew/bin/fish -l
```

macOS 手动让 fish 生效时，按机器实际路径选择一个：

```sh
sudo grep -Fx /opt/homebrew/bin/fish /etc/shells || echo /opt/homebrew/bin/fish | sudo tee -a /etc/shells
chsh -s /opt/homebrew/bin/fish
exec /opt/homebrew/bin/fish -l
```

```sh
sudo grep -Fx /usr/local/bin/fish /etc/shells || echo /usr/local/bin/fish | sudo tee -a /etc/shells
chsh -s /usr/local/bin/fish
exec /usr/local/bin/fish -l
```

如果默认 shell 没有生效，先检查：

```sh
command -v fish
getent passwd "$USER" | cut -d: -f7
echo "$SHELL"
ps -p $$ -o comm=
```

Linuxbrew 的 fish 通常在 `/home/linuxbrew/.linuxbrew/bin/fish`。可以手动修复：

```sh
sudo grep -Fx /home/linuxbrew/.linuxbrew/bin/fish /etc/shells || echo /home/linuxbrew/.linuxbrew/bin/fish | sudo tee -a /etc/shells
sudo chsh -s /home/linuxbrew/.linuxbrew/bin/fish "$USER"
exec /home/linuxbrew/.linuxbrew/bin/fish -l
```

## 5. Bootstrap

先预览：

```fish
dotman --auto bootstrap 2>&1 | head -50
```

确认无误后执行：

```fish
dotman bootstrap
```

`bootstrap` 会读取 `dotman.yaml` 的 `install:` 列表：

- 使用 `packages/Brewfile` 安装 Homebrew 管理的 CLI 工具和 macOS cask。
- 如果 `try` 尚不可用，通过 Homebrew 的 Ruby 安装 `try-cli`。
- macOS 通过 Homebrew cask 安装 Ghostty 和 `font-maple-mono-nf-cn`。
- Linux 运行 `packages/install-maple-mono-linux.sh`，把 Maple Mono NF CN 安装到
  `~/.local/share/fonts/MapleMono-NF-CN`。

## 6. Deploy

先预览：

```fish
dotman plan
```

`dotman plan` 在 TUI 里显示 plan 但不执行。检查无误后：

确认无误后执行：

```fish
dotman deploy
```

`deploy` 会读取 `dotman.yaml`，链接 dotfiles、创建目录，并运行依赖配置文件的
同步步骤，例如 fish plugins 和 tealdeer pages。如果 fish 已经提前创建了
`~/.config/fish`，`dotman` 会先备份它，再链接仓库里的 fish 配置。
核心文件操作失败会停止本次 deploy；受网络影响的同步步骤标记为 `optional: true`，
失败时只会显示 warning，并继续后续步骤。

## 7. 恢复私有配置

- 恢复 `~/.gitconfig.local`。
- 恢复 `~/.config/fish/local.d/*.fish`。
- 恢复 SSH/GPG key，并检查权限。

## 8. 手动应用设置

- 登录 1Password、浏览器、GitHub、云同步等账号。
- 配置系统权限，例如终端、编辑器和窗口管理工具的 Accessibility 权限。
- Linux 上按发行版或 [Ghostty 官方安装说明](https://ghostty.org/docs/install/binary)
  手动安装 Ghostty；`deploy` 只负责链接 Ghostty 配置。如需机器相关的
  Ghostty 设置（窗口大小、透明度等），创建 `~/.config/ghostty/config.local`，
  仓库配置会自动加载它。
- 检查字体、输入法、浏览器扩展和 GUI 应用设置。
- 首次打开 Neovim，让插件和工具完成安装。

## 9. 验证

检查 Homebrew bundle：

```fish
brew bundle check --file packages/Brewfile
```

检查 Maple Mono 字体：

```fish
fc-list | grep -i "Maple Mono"
```

macOS 如果没有 `fc-list`，可以用字体册或直接在 Ghostty 中选择
`Maple Mono NF CN` 验证。
