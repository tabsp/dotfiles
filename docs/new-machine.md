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

- `git`
- `curl`
- CA certificates
- 编译工具链

macOS 先安装 Xcode Command Line Tools：

```sh
xcode-select --install
```

Linux 根据发行版安装基础工具。如果需要自动安装 Maple Mono NF CN 字体，还要确保
有 `unzip` 和 `fontconfig`。

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

## 5. Rust

安装 Rust toolchain with Cargo：

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

因为 dotfiles 还没有 deploy，当前 fish 需要先临时加入 Cargo 路径：

```fish
fish_add_path "$HOME/.cargo/bin"
cargo --version
```

## 6. 获取仓库

```fish
git clone https://github.com/tabsp/dotfiles.git ~/dotfiles
cd ~/dotfiles
make build
```

## 7. Bootstrap

先预览：

```fish
make bootstrap DRY_RUN=1
```

确认无误后执行：

```fish
make bootstrap
```

`bootstrap` 会读取 `dotman.bootstrap.yaml`：

- 使用 `packages/Brewfile` 安装 Homebrew 管理的 CLI 工具和 macOS cask。
- macOS 通过 Homebrew cask 安装 Ghostty 和 `font-maple-mono-nf-cn`。
- Linux 运行 `packages/install-maple-mono-linux.sh`，把 Maple Mono NF CN 安装到
  `~/.local/share/fonts/MapleMono-NF-CN`。

## 8. Deploy

先预览：

```fish
make deploy DRY_RUN=1
```

确认无误后执行：

```fish
make deploy
```

`deploy` 会读取 `dotman.yaml`，链接 dotfiles、创建目录并运行日常部署相关的
shell 步骤。如果 fish 已经提前创建了 `~/.config/fish`，`dotman` 会先备份它，
再链接仓库里的 fish 配置。

## 9. 恢复私有配置

- 恢复 `~/.gitconfig.local`。
- 恢复 `~/.config/fish/local.d/*.fish`。
- 恢复 SSH/GPG key，并检查权限。

## 10. 手动应用设置

- 登录 1Password、浏览器、GitHub、云同步等账号。
- 配置系统权限，例如终端、编辑器和窗口管理工具的 Accessibility 权限。
- Linux 上按发行版或 [Ghostty 官方安装说明](https://ghostty.org/docs/install/binary)
  手动安装 Ghostty；`deploy` 只负责链接 Ghostty 配置。
- 检查字体、输入法、浏览器扩展和 GUI 应用设置。
- 首次打开 Neovim，让插件和工具完成安装。

## 11. 验证

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
