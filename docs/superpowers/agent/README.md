# Roadmap Agent Runtime

This directory contains repo-local state and templates for deterministic roadmap
agent workflow commands.

Tracked files:

- `README.md`
- `templates/spec.md`
- `templates/plan.md`
- `templates/handoff.md`
- finished handoffs under `handoffs/`

Ignored mutable files:

- `state.toml`
- `current-handoff.md`

Use `make agent-init` to recreate missing mutable state.
