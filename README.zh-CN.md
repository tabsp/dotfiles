# dotfiles

[English](README.md)

个人 dotfiles，由 `dotman` 管理。`dotman` 是一个小型 Rust 配置部署工具，
配置模型参考了 Dotbot 的有序步骤列表。

## 前置依赖

- 带 Cargo 的 Rust 工具链
- GNU Make
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

跳过 shell 命令，例如插件同步：

```sh
make deploy EXCEPT=shell
```

只运行链接步骤：

```sh
make deploy ONLY=link
```

## 配置

部署步骤写在 `dotman.yaml` 中。

支持的指令：

- `defaults`
- `link`
- `create`
- `shell`
- `clean`（目前只在 dry-run 中占位）

示例：

```yaml
- defaults:
    link:
      create: true
      relink: true
      relative: true

- link:
    ~/.config/fish: config/fish
    ~/.config/nvim: config/nvim

- create:
    - ~/.config/fish/local.d

- shell:
    - command: fish -lc 'fisher update'
      description: Sync fish plugins
      stdout: true
      stderr: true
```

## 本地覆盖

机器相关的路径、token 和临时工具配置不要放进共享仓库。

fish 会加载本地文件：

```text
~/.config/fish/local.d/*.fish
```

## 目录结构

- `config/`：被跟踪的 dotfiles 源文件
- `dotman.yaml`：部署步骤
- `src/`：Rust 部署工具源码
- `tests/`：CLI 集成测试

## 开发

```sh
make lint
make test
make ci
```
