.PHONY: help build init plan deploy lint nvim-check test ci clean
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
		'  make lint         Run rustfmt and clippy checks' \
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

lint:
	cargo fmt --check
	cargo clippy --all-targets --all-features -- -D warnings

nvim-check:
	XDG_STATE_HOME=/private/tmp/nvim-state-check XDG_CACHE_HOME=/private/tmp/nvim-cache-check nvim --headless -u config/nvim/init.lua +qa

test:
	cargo test

ci: lint test

clean:
	cargo clean
