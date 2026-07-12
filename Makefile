.PHONY: help build init plan deploy lint lint-tools rust-lint shell-lint fish-check docker-lint action-lint nvim-check test ci clean
.DEFAULT_GOAL := help

DOTMAN := target/debug/dotman
RUST_SOURCES := $(shell find src -name '*.rs')

help:
	@printf '%s\n' \
		'Usage:' \
		'  make build        Build dotman' \
		'  make init         Initialize default dotman profile' \
		'  make plan         Show deployment plan in headless mode' \
		'  make deploy       Deploy in headless mode' \
		'  make lint         Run Rust, shell, and Dockerfile checks' \
		'  make rust-lint    Run rustfmt and clippy checks' \
		'  make shell-lint   Run ShellCheck' \
		'  make fish-check   Check Fish syntax and PATH scope' \
		'  make docker-lint  Run Hadolint' \
		'  make action-lint  Check GitHub Actions workflows' \
		'  make nvim-check   Check Neovim config loads headlessly' \
		'  make test         Run tests' \
		'  make ci           Run lint and tests' \
		'  make clean        Remove build artifacts'

$(DOTMAN): Cargo.toml Cargo.lock $(RUST_SOURCES)
	cargo build

build: $(DOTMAN)

init: $(DOTMAN)
	$(DOTMAN) init

plan: $(DOTMAN)
	$(DOTMAN) plan --headless

deploy: $(DOTMAN)
	$(DOTMAN) deploy --headless

lint: lint-tools rust-lint shell-lint fish-check docker-lint action-lint

lint-tools:
	@missing=""; \
	for tool in fish shellcheck hadolint actionlint jq nvim stylua; do \
		command -v "$$tool" >/dev/null 2>&1 || missing="$$missing $$tool"; \
	done; \
	if test -n "$$missing"; then \
		echo "missing lint tools:$$missing" >&2; \
		echo "install with: brew install fish shellcheck hadolint actionlint jq neovim stylua" >&2; \
		exit 1; \
	fi

rust-lint:
	cargo fmt --check
	cargo clippy --all-targets --all-features -- -D warnings

shell-lint:
	shellcheck bin/tmux-status tests/e2e/*.sh tests/e2e/scenarios/*.sh

fish-check:
	fish -n config/fish/config.fish config/fish/conf.d/*.fish config/fish/functions/*.fish
	@tmp="$$(mktemp -d /private/tmp/dotfiles-fish-check.XXXXXX)"; \
	trap 'rm -rf "$$tmp"' EXIT; \
	mkdir -p "$$tmp/home/.local/bin" "$$tmp/home/.cargo/bin" "$$tmp/config/fish"; \
	env -u MISE_SHELL -u __MISE_DIFF -u __MISE_ORIG_PATH -u __MISE_SESSION \
	  HOME="$$tmp/home" XDG_CONFIG_HOME="$$tmp/config" fish --no-config -c \
	  'fish_add_path --global $$HOME/.local/bin $$HOME/.cargo/bin'; \
	env -u MISE_SHELL -u __MISE_DIFF -u __MISE_ORIG_PATH -u __MISE_SESSION \
	  HOME="$$tmp/home" XDG_CONFIG_HOME="$$tmp/config" fish --no-config -c \
	  'not set -q fish_user_paths; or test (count $$fish_user_paths) -eq 0'

docker-lint:
	hadolint tests/e2e/Dockerfile tests/e2e/Dockerfile.production tests/e2e/Dockerfile.sudo

action-lint:
	actionlint

nvim-check:
	jq empty config/nvim/lazy-lock.json
	stylua --check config/nvim
	@tmp="$$(mktemp -d "$${TMPDIR:-/tmp}/dotfiles-nvim-check.XXXXXX")"; \
	trap 'rm -rf "$$tmp"' EXIT; \
	XDG_STATE_HOME="$$tmp/state" XDG_CACHE_HOME="$$tmp/cache" \
	  nvim --headless -u config/nvim/init.lua +qa

test:
	cargo test

ci: lint nvim-check test
	cargo test --test tui_pty -- --nocapture

clean:
	cargo clean

include tests/e2e/Makefile.target.mk
