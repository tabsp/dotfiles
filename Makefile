.PHONY: help build dev-deploy dev-bootstrap e2e-linux e2e-image lint test ci clean
.DEFAULT_GOAL := help

DOTMAN := target/debug/dotman
RUST_SOURCES := $(shell find src -name '*.rs')

E2E_IMAGE ?= dotman-e2e:latest

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
		'  make e2e-image                   Build the Docker image used by E2E tests' \
		'  make e2e-linux                   Run real Linux install E2E in Docker' \
		'  make e2e-linux E2E_ARGS="--manual --keep"  Manual E2E testing' \
		'  make lint                        Run rustfmt and clippy checks' \
		'  make test                        Run tests' \
		'  make ci                          Run lint and tests' \
		'  make clean                       Remove build artifacts' \
		'  E2E_IMAGE=... make e2e-linux     Override the base Docker image'

E2E_IMAGE ?= dotman-e2e:latest

$(DOTMAN): Cargo.toml Cargo.lock $(RUST_SOURCES)
	cargo build

build: $(DOTMAN)

dev-deploy: $(DOTMAN)
	$(DOTMAN) deploy $(if $(filter 1,$(DRY_RUN)),--dry-run,) $(if $(ONLY),--only $(ONLY),) $(if $(EXCEPT),--except $(EXCEPT),)

dev-bootstrap: $(DOTMAN)
	$(DOTMAN) bootstrap $(if $(filter 1,$(DRY_RUN)),--dry-run,) $(if $(ONLY),--only $(ONLY),) $(if $(EXCEPT),--except $(EXCEPT),)

e2e-image:
	docker build -t $(E2E_IMAGE) -f scripts/e2e-image.dockerfile scripts/

e2e-linux: e2e-image
	E2E_DOCKER_IMAGE=$(E2E_IMAGE) scripts/e2e-linux.sh $(E2E_ARGS)

lint:
	cargo fmt --check
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test

ci: lint test

clean:
	cargo clean
