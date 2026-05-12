.PHONY: help bootstrap link doctor shell check lint test ci build build-dotman cargo-preflight
.DEFAULT_GOAL := help

DOTMAN := target/debug/dotman
CONFLICT ?= backup
RUST_SOURCES := $(shell find src -name '*.rs')

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
		'  make shell                       Interactively set fish as the login shell' \
		'  make check                       Validate manifests and host support' \
		'  make lint                        Run cargo fmt and clippy checks' \
		'  make test                        Run Rust tests' \
		'  make ci                          Run local verification suite'

cargo-preflight:
	@command -v cargo >/dev/null 2>&1 || { echo "error: cargo is required to build dotman" >&2; echo "hint: install Rust with rustup: https://rustup.rs/" >&2; exit 1; }
	@command -v rustc >/dev/null 2>&1 || { echo "error: rustc is required to build dotman" >&2; echo "hint: install Rust with rustup: https://rustup.rs/" >&2; exit 1; }
	@if [ "$$(uname -s)" = "Linux" ]; then \
		command -v cc >/dev/null 2>&1 || { \
			echo "error: cc is required to build Rust crates with native dependencies" >&2; \
			echo "hint: on Ubuntu/Debian, install it with: sudo apt-get install -y build-essential" >&2; \
			exit 1; \
		}; \
	elif [ "$$(uname -s)" = "Darwin" ]; then \
		command -v cc >/dev/null 2>&1 || { \
			echo "error: cc is required to build Rust crates with native dependencies" >&2; \
			echo "hint: on macOS, install Xcode Command Line Tools with: xcode-select --install" >&2; \
			exit 1; \
		}; \
	fi

$(DOTMAN): Cargo.toml Cargo.lock $(RUST_SOURCES) | cargo-preflight
	cargo build

build-dotman: cargo-preflight $(DOTMAN)

build: build-dotman

bootstrap: build-dotman
	$(DOTMAN) bootstrap

link: build-dotman
	$(DOTMAN) link --conflict $(CONFLICT) $(if $(filter 1,$(DRY_RUN)),--dry-run,)

doctor: build-dotman
	$(DOTMAN) doctor

shell: build-dotman
	$(DOTMAN) shell

check: build-dotman
	$(DOTMAN) check

lint: cargo-preflight
	@command -v rustfmt >/dev/null 2>&1 || { echo "error: rustfmt is required for make lint" >&2; echo "hint: install it with: rustup component add rustfmt" >&2; exit 1; }
	@cargo clippy --version >/dev/null 2>&1 || { echo "error: clippy is required for make lint" >&2; echo "hint: install it with: rustup component add clippy" >&2; exit 1; }
	cargo fmt --check
	cargo clippy --all-targets --all-features -- -D warnings

test: cargo-preflight
	cargo test

ci: lint check test
