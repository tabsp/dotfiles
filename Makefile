.PHONY: help build init plan deploy format format-check format-tools lint lint-tools rust-lint shell-lint fish-check docker-lint action-lint portability-check nvim-check config-check config-check-tools test ci clean
.DEFAULT_GOAL := help

DOTMAN := target/debug/dotman
RUST_SOURCES := $(shell find src -name '*.rs')
NVIM_TEST_LUA := $(wildcard tests/nvim*.lua)
NVIM_DATA_HOME := $(or $(XDG_DATA_HOME),$(HOME)/.local/share)/nvim
PRETTIER ?= $(or $(shell command -v prettier 2>/dev/null),$(NVIM_DATA_HOME)/mason/bin/prettier)
PRETTIER_FILES := $(shell git ls-files --cached --others --exclude-standard -- '*.md' '*.json' '*.jsonc' '*.yaml' '*.yml')

help:
	@printf '%s\n' \
		'Usage:' \
		'  make build        Build dotman' \
		'  make init         Initialize default dotman profile' \
		'  make plan         Show deployment plan in headless mode' \
		'  make deploy       Deploy in headless mode' \
		'  make format       Format all supported tracked files' \
		'  make format-check Check repository-wide formatting' \
		'  make lint         Run Rust, shell, and Dockerfile checks' \
		'  make rust-lint    Run rustfmt and clippy checks' \
		'  make shell-lint   Run ShellCheck' \
		'  make fish-check   Check Fish syntax and PATH scope' \
		'  make docker-lint  Run Hadolint' \
		'  make action-lint  Check GitHub Actions workflows' \
		'  make portability-check Check for host-specific absolute paths' \
		'  make nvim-check   Check Neovim config loads headlessly' \
		'  make config-check Check all managed configuration files' \
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

format-tools:
	@test -x "$(PRETTIER)" || { \
		echo "missing formatter: prettier" >&2; \
		echo "install with Mason (:MasonInstall prettier) or add prettier to PATH" >&2; \
		exit 1; \
	}
	@command -v stylua >/dev/null 2>&1 || { echo "missing formatter: stylua" >&2; exit 1; }
	@command -v cargo >/dev/null 2>&1 || { echo "missing formatter: cargo" >&2; exit 1; }

format: format-tools
	cargo fmt
	stylua config/nvim
	stylua --config-path config/nvim/stylua.toml $(NVIM_TEST_LUA)
	$(PRETTIER) --write $(PRETTIER_FILES)

format-check: format-tools
	cargo fmt --check
	stylua --check config/nvim
	stylua --check --config-path config/nvim/stylua.toml $(NVIM_TEST_LUA)
	$(PRETTIER) --check $(PRETTIER_FILES)

lint: lint-tools format-check rust-lint shell-lint fish-check docker-lint action-lint portability-check

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
	@set -e; \
	tmp="$$(mktemp -d "$${TMPDIR:-/tmp}/dotfiles-fish-check.XXXXXX")"; \
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

portability-check:
	@private_tmp='/private''/tmp'; \
	user_home='/Users/''[[:alnum:]_.-]+'; \
	matches="$$(git grep -nE -e "$$private_tmp" -e "$$user_home" || true)"; \
	if test -n "$$matches"; then \
		echo "host-specific absolute paths found:" >&2; \
		echo "$$matches" >&2; \
		exit 1; \
	fi

nvim-check:
	jq empty config/nvim/lazy-lock.json
	stylua --check config/nvim
	stylua --check --config-path config/nvim/stylua.toml $(NVIM_TEST_LUA)
	@set -e; \
	tmp="$$(mktemp -d "$${TMPDIR:-/tmp}/dotfiles-nvim-check.XXXXXX")"; \
	trap 'rm -rf "$$tmp"' EXIT; \
	export XDG_CONFIG_HOME="$(CURDIR)/config" NVIM_CONFIG_CHECK_ROOT="$(CURDIR)/config/nvim"; \
	XDG_STATE_HOME="$$tmp/state" XDG_CACHE_HOME="$$tmp/cache" \
	  nvim --headless -u "$(CURDIR)/config/nvim/init.lua" -l tests/nvim_check.lua; \
	for mode in direct dirchange; do \
	  NVIM_SESSION_CHECK="$$mode" XDG_STATE_HOME="$$tmp/state" XDG_CACHE_HOME="$$tmp/cache" \
	    nvim --headless -u "$(CURDIR)/config/nvim/init.lua" README.md -l tests/nvim_session_check.lua; \
	done; \
	NVIM_SESSION_CHECK=directory XDG_STATE_HOME="$$tmp/state" XDG_CACHE_HOME="$$tmp/cache" \
	  nvim --headless -u "$(CURDIR)/config/nvim/init.lua" . -l tests/nvim_session_check.lua; \
	XDG_STATE_HOME="$$tmp/state" XDG_CACHE_HOME="$$tmp/cache" \
	  nvim --headless -u "$(CURDIR)/config/nvim/init.lua" --startuptime "$$tmp/startup.log" +qa; \
	awk 'NF && $$1 ~ /^[0-9.]+$$/ { total=$$1 } END { \
	  printf "Neovim startup: %.1fms\n", total; \
	  if (total > 500) { print "Neovim startup exceeded 500ms budget" > "/dev/stderr"; exit 1 } \
	}' "$$tmp/startup.log"

config-check-tools:
	@missing=""; \
	for tool in cargo fish shellcheck jq nvim stylua yq tmux; do \
		command -v "$$tool" >/dev/null 2>&1 || missing="$$missing $$tool"; \
	done; \
	if test -n "$$missing"; then \
		echo "missing config check tools:$$missing" >&2; \
		exit 1; \
	fi

config-check: config-check-tools format-check shell-lint fish-check portability-check nvim-check test $(DOTMAN)
	yq '.' dotman.yaml >/dev/null
	$(DOTMAN) plan --headless >/dev/null
	@tmp="$$(mktemp -d "$${TMPDIR:-/tmp}/dotfiles-tmux-check.XXXXXX")"; \
	socket="$$tmp/tmux.sock"; \
	trap 'tmux -S "$$socket" kill-server >/dev/null 2>&1 || true; rm -rf "$$tmp"' EXIT; \
	tmux -S "$$socket" -f "$(CURDIR)/config/tmux.conf" new-session -d -s config-check; \
	test "$$(tmux -S "$$socket" show-options -gv default-terminal)" = tmux-256color; \
	test "$$(tmux -S "$$socket" show-options -gv status-interval)" = 5

test:
	cargo test

ci: lint nvim-check test
	cargo test --test tui_pty -- --nocapture

clean:
	cargo clean

include tests/e2e/Makefile.target.mk
