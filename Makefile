.PHONY: help build dev-deploy dev-bootstrap lint test ci clean
.DEFAULT_GOAL := help

DOTMAN := target/debug/dotman
RUST_SOURCES := $(shell find src -name '*.rs')

help:
	@printf '%s\n' \
		'Usage:' \
		'  make build                       Build dotman' \
		'  make dev-deploy                  Deploy with target/debug/dotman' \
		'  make dev-deploy DRY_RUN=1        Preview deployment with target/debug/dotman' \
		'  make dev-deploy ONLY=link        Run only a directive' \
		'  make dev-deploy EXCEPT=shell     Skip a directive' \
		'  make dev-bootstrap               Run bootstrap with target/debug/dotman' \
		'  make dev-bootstrap DRY_RUN=1     Preview bootstrap with target/debug/dotman' \
		'  make lint                        Run rustfmt and clippy checks' \
		'  make test                        Run tests' \
		'  make ci                          Run lint and tests' \
		'  make clean                       Remove build artifacts'

$(DOTMAN): Cargo.toml Cargo.lock $(RUST_SOURCES)
	cargo build

build: $(DOTMAN)

dev-deploy: $(DOTMAN)
	$(DOTMAN) deploy $(if $(filter 1,$(DRY_RUN)),--dry-run,) $(if $(ONLY),--only $(ONLY),) $(if $(EXCEPT),--except $(EXCEPT),)

dev-bootstrap: $(DOTMAN)
	$(DOTMAN) bootstrap $(if $(filter 1,$(DRY_RUN)),--dry-run,) $(if $(ONLY),--only $(ONLY),) $(if $(EXCEPT),--except $(EXCEPT),)

lint:
	cargo fmt --check
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test

ci: lint test

clean:
	cargo clean
