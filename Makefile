.PHONY: help bootstrap link doctor shell check lint test ci build build-dotman \
             cargo-preflight \
             agent-init agent-next agent-start agent-status agent-check agent-review-check agent-handoff agent-template \
             agent-advance agent-record-verification agent-finish agent-set-roadmap-status \
             uninstall release release-check smoke-test
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
		'  make agent-init                  Initialize roadmap agent runtime state' \
		'  make agent-next                  Print next eligible roadmap epic' \
		'  make agent-start                 Lock one roadmap epic for work' \
		'  make agent-status                Print current agent state' \
		'  make agent-check                 Validate workflow consistency' \
		'  make agent-review-check          Validate multi-agent review document exists' \
		'  make agent-handoff               Create or validate handoff notes' \
		'  make agent-template              Create spec or plan from template' \
		'  make agent-advance               Advance active epic phase' \
		'  make agent-record-verification   Record verification evidence' \
		'  make agent-finish                Finish active epic after verification' \
		'  make check                       Validate manifests and host support' \
		'  make lint                        Run cargo fmt and clippy checks' \
		'  make test                        Run Rust tests' \
		'  make ci                          Run local verification suite' \
		'  make release                     Build release binary tarball and checksum' \
		'  make release-check               Build release and verify checksum' \
		'  make smoke-test                 End-to-end release install chain validation' \
		'  make uninstall                   Remove dotman binary and list managed state' \

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

release: cargo-preflight
	@mkdir -p dist
	@VERSION=$$(awk -F'"' '/^version/{print $$2; exit}' Cargo.toml); \
	TARGET=$$(rustc -vV | grep host | cut -d ' ' -f2); \
	echo "==> building dotman $${VERSION} for $${TARGET}"; \
	cargo build --release; \
	cp target/release/dotman dist/dotman; \
	tar -czf "dist/dotman-$${TARGET}-$${VERSION}.tar.gz" -C dist dotman; \
	rm dist/dotman; \
	cd dist && shasum -a 256 "dotman-$${TARGET}-$${VERSION}.tar.gz" > "dotman-$${TARGET}-$${VERSION}.tar.gz.sha256"; \
	echo "==> release: dist/dotman-$${TARGET}-$${VERSION}.tar.gz"

release-check: release
	@VERSION=$$(awk -F'"' '/^version/{print $$2; exit}' Cargo.toml); \
	TARGET=$$(rustc -vV | grep host | cut -d ' ' -f2); \
	cd dist && shasum -a 256 -c "dotman-$${TARGET}-$${VERSION}.tar.gz.sha256"

smoke-test: release-check
	@VERSION=$$(awk -F'"' '/^version/{print $$2; exit}' Cargo.toml); \
	TARGET=$$(rustc -vV | grep host | cut -d ' ' -f2); \
	scripts/smoke-test.sh "$$VERSION" "$$TARGET"


uninstall:
	@echo "==> removing dotman binary..."
	@rm -f target/debug/dotman target/release/dotman
	@rm -f "$$HOME/.local/bin/dotman"
	@echo "==> dotman removed"
	@echo ""
	@echo "Remaining managed state (use dotman status to inspect):"
	@echo "  $$HOME/.local/bin/   - installed tools and symlinks"
	@echo "  $$HOME/.config/...   - linked dotfiles"
	@echo "  *.dotman-backup      - backup directories from conflict resolution"
	@echo "  *.dotman-staging     - stale staging directories"
	@echo ""
	@echo "Run 'dotman status' for a full inventory of managed state."
	@echo "Run 'dotman cleanup --execute' to remove stale backup/staging dirs."
	@echo "See docs/recovery.md for full uninstall instructions."
update-deps-list: build-dotman
	$(DOTMAN) update

update-deps-check: build-dotman
	$(DOTMAN) update --check

agent-init: build-dotman
	$(DOTMAN) agent init

agent-next: build-dotman
	$(DOTMAN) agent next

agent-start: build-dotman
	$(DOTMAN) agent start --epic "$(EPIC)" --work-kind $(or $(WORK_KIND),roadmap) $(if $(EXCEPTION_REASON),--exception-reason "$(EXCEPTION_REASON)",)

agent-status: build-dotman
	$(DOTMAN) agent status

agent-check: build-dotman
	$(DOTMAN) agent check

agent-review-check: build-dotman
	$(DOTMAN) agent review-check

agent-handoff: build-dotman
	$(DOTMAN) agent handoff --mode $(MODE) $(if $(SECTION),--section $(SECTION),) $(if $(VALUE),--value $(VALUE),)

agent-template: build-dotman
	$(DOTMAN) agent template --kind $(KIND)

agent-advance: build-dotman
	$(DOTMAN) agent advance --phase $(PHASE)

agent-record-verification: build-dotman
	$(DOTMAN) agent record-verification --command "$(COMMAND)" --result $(RESULT) --summary "$(SUMMARY)"

agent-set-roadmap-status: build-dotman
	$(DOTMAN) agent set-roadmap-status --status $(STATUS)

agent-finish: build-dotman
	$(DOTMAN) agent finish
