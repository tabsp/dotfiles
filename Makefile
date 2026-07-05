.PHONY: help build init plan deploy lint rust-lint shell-lint docker-lint nvim-check test ci clean
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
		'  make shell-lint   Run shellcheck when available' \
		'  make docker-lint  Run hadolint when available' \
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

lint: rust-lint shell-lint docker-lint

rust-lint:
	cargo fmt --check
	cargo clippy --all-targets --all-features -- -D warnings

shell-lint:
	@if command -v shellcheck >/dev/null 2>&1; then \
		shellcheck tests/e2e/*.sh tests/e2e/scenarios/*.sh; \
	else \
		echo "shellcheck not found; skipping shell lint"; \
	fi

docker-lint:
	@if command -v hadolint >/dev/null 2>&1; then \
		hadolint tests/e2e/Dockerfile; \
	else \
		echo "hadolint not found; skipping Dockerfile lint"; \
	fi

nvim-check:
	XDG_STATE_HOME=/private/tmp/nvim-state-check XDG_CACHE_HOME=/private/tmp/nvim-cache-check nvim --headless -u config/nvim/init.lua +qa

test:
	cargo test

ci: lint test

clean:
	cargo clean

include tests/e2e/Makefile.target.mk
