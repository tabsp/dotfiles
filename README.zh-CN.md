# dotfiles

[English](README.md)

dotman 是一个小型 Rust-based dotfiles 部署工具，用于我的个人 macOS/Linux 环境。
它使用 Dotbot-like 的有序 YAML 配置来链接文件、创建目录并运行设置命令。

## 前置依赖

- 带 Cargo 的 Rust 工具链
- GNU Make
- Git
- curl
- fish shell

## 使用

构建部署工具：

```sh
make build
```

预览部署计划：

```sh
make deploy DRY_RUN=1
```

部署 dotfiles：

```sh
make deploy
```

预览 bootstrap 步骤：

```sh
make bootstrap DRY_RUN=1
```

运行 bootstrap 步骤：

```sh
make bootstrap
```

跳过 shell 命令，例如插件同步：

```sh
make deploy EXCEPT=shell
```

只运行链接步骤：

```sh
make deploy ONLY=link
```

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
      description: Install and sync fish plugins
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
- `shell.stdout` / `shell.stderr`：单条命令的输出覆盖设置。
- `clean`：目前会被解析并在 dry-run 中显示，但非 dry-run 清理尚未实现。

## 本地覆盖

机器相关的路径、token 和临时工具配置不要放进共享仓库。

fish 会加载本地文件：

```text
~/.config/fish/local.d/*.fish
```

新机器首次设置参考 [docs/new-machine.md](docs/new-machine.md)。

## 目录结构

- `config/`：被跟踪的 dotfiles 源文件
- `docs/`：设置说明和手动清单
- `dotman.yaml`：部署步骤
- `dotman.bootstrap.yaml`：bootstrap 步骤
- `packages/`：包清单和安装辅助脚本
- `src/`：Rust 部署工具源码
- `tests/`：CLI 集成测试

## 开发

```sh
make lint
make test
make ci
```
