.PHONY: help bootstrap link doctor check lint test ci build build-dotman cargo-preflight
.DEFAULT_GOAL := help

DOTMAN := target/debug/dotman
CONFLICT ?= backup

help:
	@printf '%s\n' \
		'Usage:' \
		'  make build                       Build the Rust backend' \
		'  make bootstrap                   Install dependencies, link dotfiles, and run doctor' \
		'  make link                        Link managed dotfiles' \
		'  make link DRY_RUN=1              Preview link actions' \
		'  make link CONFLICT=fail          Fail on target conflicts' \
		'  make link CONFLICT=backup        Back up target conflicts before linking' \
		'  make link CONFLICT=overwrite     Overwrite target conflicts before linking' \
		'  make doctor                      Inspect installed commands and managed links' \
		'  make check                       Validate manifests and host support' \
		'  make lint                        Run cargo fmt and clippy checks' \
		'  make test                        Run Rust tests' \
		'  make ci                          Run local verification suite'

cargo-preflight:
	@command -v cargo >/dev/null 2>&1 || { echo "error: cargo is required to build dotman" >&2; exit 1; }

build-dotman: cargo-preflight
	cargo build

build: build-dotman

bootstrap: build-dotman
	$(DOTMAN) bootstrap

link: build-dotman
	$(DOTMAN) link --conflict $(CONFLICT) $(if $(filter 1,$(DRY_RUN)),--dry-run,)

doctor: build-dotman
	$(DOTMAN) doctor

check: build-dotman
	$(DOTMAN) check

lint: cargo-preflight
	cargo fmt --check
	cargo clippy --all-targets --all-features -- -D warnings

test: cargo-preflight
	cargo test

ci: lint check test
