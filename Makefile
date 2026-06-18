.PHONY: help build deploy bootstrap lint test ci clean
.DEFAULT_GOAL := help

DOTMAN := target/debug/dotman
RUST_SOURCES := $(shell find src -name '*.rs')

help:
	@printf '%s\n' \
		'Usage:' \
		'  make build                       Build dotman' \
		'  make deploy                      Deploy dotfiles from dotman.yaml' \
		'  make deploy DRY_RUN=1            Preview deployment' \
		'  make deploy ONLY=link            Run only a directive' \
		'  make deploy EXCEPT=shell         Skip a directive' \
		'  make bootstrap                   Run bootstrap steps from dotman.bootstrap.yaml' \
		'  make bootstrap DRY_RUN=1         Preview bootstrap steps' \
		'  make lint                        Run rustfmt and clippy checks' \
		'  make test                        Run tests' \
		'  make ci                          Run lint and tests' \
		'  make clean                       Remove build artifacts'

$(DOTMAN): Cargo.toml Cargo.lock $(RUST_SOURCES)
	cargo build

build: $(DOTMAN)

deploy: $(DOTMAN)
	$(DOTMAN) deploy $(if $(filter 1,$(DRY_RUN)),--dry-run,) $(if $(ONLY),--only $(ONLY),) $(if $(EXCEPT),--except $(EXCEPT),)

bootstrap: $(DOTMAN)
	$(DOTMAN) bootstrap $(if $(filter 1,$(DRY_RUN)),--dry-run,) $(if $(ONLY),--only $(ONLY),) $(if $(EXCEPT),--except $(EXCEPT),)

lint:
	cargo fmt --check
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test

ci: lint test

clean:
	cargo clean
