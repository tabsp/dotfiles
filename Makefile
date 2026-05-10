.PHONY: bootstrap link doctor check build build-dotman cargo-preflight

DOTMAN := target/debug/dotman
CONFLICT ?= backup

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
