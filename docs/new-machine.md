# New Machine Setup

这份清单用于新机器初始化。`dotman bootstrap` 负责可自动执行的安装步骤；
账号登录、SSH/GPG、系统权限和私有配置仍然需要人工处理。

执行 `exec fish -l` 之后，后续命令默认在 fish 中执行。

## 1. 旧机器备份

- 确认 SSH/GPG key 可以恢复，或已经迁移到安全位置。
- 保存 `~/.gitconfig.local`。
- 保存 `~/.config/fish/local.d/*.fish` 中的本地私有配置。
- 确认需要迁移的应用数据、字体、浏览器配置和登录态。

## 2. 系统基础工具

先安装基础工具，这部分不由 `dotman` 自动处理：

- `curl`
- CA certificates

macOS 先安装 Xcode Command Line Tools：

```sh
xcode-select --install
```

Linux 根据发行版安装基础工具。如果需要自动安装 Maple Mono NF CN 字体，还要确保有
`unzip` 和 `fontconfig`。

## 3. Homebrew

如果这台机器使用 Homebrew，先按官方方式安装：

```sh
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

安装后，确保当前 shell 能找到 `brew`。按机器实际路径选择一个执行：

```sh
eval "$(/opt/homebrew/bin/brew shellenv)"
eval "$(/usr/local/bin/brew shellenv)"
eval "$(/home/linuxbrew/.linuxbrew/bin/brew shellenv)"
```

`dotman bootstrap` 不负责安装 Homebrew；它只会在 `brew` 已可用时运行
`brew bundle --file packages/Brewfile`。

## 4. Fish

fish 是手动前置条件，不由 `packages/Brewfile` 安装。可以手动用 Homebrew 安装：

```sh
brew install fish
```

把 fish 设置为登录 shell，并让当前终端立刻进入 fish。下面这段命令在当前系统
shell 中执行，适用于 macOS 和 Linux：

```sh
grep -Fx "$(command -v fish)" /etc/shells || command -v fish | sudo tee -a /etc/shells
chsh -s "$(command -v fish)"
exec fish -l
```

如果 `chsh` 对新窗口没有立刻生效，重新登录系统后再确认。当前终端可以继续通过
`exec fish -l` 进入 fish。

## 5. 安装 dotman 和 dotfiles bundle

从发布站点安装 `dotman`，并下载最新 dotfiles bundle 到
`~/.local/share/tabsp-dotfiles`：

```sh
curl -fsSL https://dotfiles.tabsp.com/install.sh | sh
```

安装脚本会先执行 dry-run，确认后才运行真正的 bootstrap/deploy。确认过流程后，
也可以使用无交互模式：

```sh
curl -fsSL https://dotfiles.tabsp.com/install.sh | sh -s -- --yes
```

`dotman` 会安装到 `~/.local/bin/dotman`。安装脚本首轮运行时会用绝对路径调用它；
如果当前 shell 还没有把 `~/.local/bin` 加进 `PATH`，脚本结束时会给出提示。

## 6. Bootstrap

先预览：

```fish
dotman bootstrap --dry-run
```

确认无误后执行：

```fish
dotman bootstrap
```

`bootstrap` 会读取 `dotman.bootstrap.yaml`：

- 使用 `packages/Brewfile` 安装 Homebrew 管理的 CLI 工具和 macOS cask。
- 如果 `try` 尚不可用，通过 Homebrew 的 Ruby 安装 `try-cli`。
- macOS 通过 Homebrew cask 安装 Ghostty 和 `font-maple-mono-nf-cn`。
- Linux 运行 `packages/install-maple-mono-linux.sh`，把 Maple Mono NF CN 安装到
  `~/.local/share/fonts/MapleMono-NF-CN`。

## 7. Deploy

先预览：

```fish
dotman deploy --dry-run
```

确认无误后执行：

```fish
dotman deploy
```

`deploy` 会读取 `dotman.yaml`，链接 dotfiles、创建目录，并运行依赖配置文件的
同步步骤，例如 fish plugins 和 tealdeer pages。如果 fish 已经提前创建了
`~/.config/fish`，`dotman` 会先备份它，再链接仓库里的 fish 配置。
核心文件操作失败会停止本次 deploy；受网络影响的同步步骤标记为 `optional: true`，
失败时只会显示 warning，并继续后续步骤。

## 8. 恢复私有配置

- 恢复 `~/.gitconfig.local`。
- 恢复 `~/.config/fish/local.d/*.fish`。
- 恢复 SSH/GPG key，并检查权限。

## 9. 手动应用设置

- 登录 1Password、浏览器、GitHub、云同步等账号。
- 配置系统权限，例如终端、编辑器和窗口管理工具的 Accessibility 权限。
- Linux 上按发行版或 [Ghostty 官方安装说明](https://ghostty.org/docs/install/binary)
  手动安装 Ghostty；`deploy` 只负责链接 Ghostty 配置。如需机器相关的
  Ghostty 设置（窗口大小、透明度等），创建 `~/.config/ghostty/config.local`，
  仓库配置会自动加载它。
- 检查字体、输入法、浏览器扩展和 GUI 应用设置。
- 首次打开 Neovim，让插件和工具完成安装。

## 10. 验证

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
