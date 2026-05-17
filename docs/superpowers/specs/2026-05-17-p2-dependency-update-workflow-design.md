# Dependency Update Workflow Design

## Goal

Provide a `dotman update` command and documented workflow for checking and
updating pinned download_binary metadata in deps.toml.

## Scope

- Add `dotman update` subcommand that lists all download_binary dependencies
  with their pinned versions, URLs, and SHA256 digests.
- Add `--check` flag to compare pinned versions against latest GitHub releases
  (for deps hosted on GitHub).
- Add `make update-deps-check` and `make update-deps-list` targets.
- Document the manual update workflow in README.

## Non-Goals

- Do not automatically modify deps.toml (manual review required).
- Do not support non-GitHub release sources for `--check`.
- Do not add update support for non-download_binary installers.

## Design

### `dotman update`

```
dotman update          # list all download_binary deps
dotman update --check  # compare against latest GitHub releases
```

Output format (list):
```
eza: v0.20.0 (linux x86_64)
  url: https://github.com/eza-community/eza/releases/download/v0.20.0/eza_x86_64-unknown-linux-gnu.tar.gz
  sha256: abc123...
```

Output format (--check):
```
eza: v0.20.0 → v0.21.0 (update available)
fzf: v0.60.0 (up to date)
```

### Implementation

Parse deps.toml, find DownloadBinary entries, extract version/url/sha256.
For `--check`, parse GitHub release URLs to extract owner/repo, call
`https://api.github.com/repos/{owner}/{repo}/releases/latest`, compare tags.

### Make targets

```makefile
update-deps-list: build-dotman
	$(DOTMAN) update

update-deps-check: build-dotman
	$(DOTMAN) update --check
```

## Error Handling

- Invalid deps.toml: report parse error.
- Network error during --check: report per-dep failure, continue.
- GitHub API rate limit: report and suggest GITHUB_TOKEN.

## Verification Strategy

- `cargo test` — full suite
- `cargo clippy` — zero warnings
- Manual: `cargo run -- update` in repo root
