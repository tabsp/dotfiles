# Roadmap Refresh And Agent Queue Reset Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** `docs/superpowers/specs/2026-05-17-p0-roadmap-refresh-and-agent-queue-reset-design.md`

**Goal:** Archive completed foundation items from the Next Queue, populate a
fresh Next Queue with only proposed future work, add a Risk Register, and
verify the agent harness can schedule new work.

**Architecture:** This is a documentation-only epic. The only code change is the
parser fix in `src/agent.rs` (already completed pre-spec). All remaining work
is editing `docs/roadmap.md`.

**Tech Stack:** Markdown, Rust (parser fix only), Makefile (agent harness).

---

## Existing Code Map

- `src/agent.rs`: roadmap parser, agent check logic, phase advancement.
- `docs/roadmap.md`: the target file for all documentation changes.
- `docs/superpowers/agent/state.toml`: agent state tracking.
- `docs/superpowers/agent/handoffs/`: completed epic handoff notes.

## Task: Roadmap Restructuring

**Files:**
- Modify: `docs/roadmap.md`
- Test: `src/agent.rs` (existing `parses_next_queue_section`)

- [ ] **Step 1: Archive completed items to Completed Foundation**

Move all `done` items from `## Next Queue` to `## Completed Foundation` as a
table with columns: Priority, Epic, Category, Outcome, Handoff.

- [ ] **Step 2: Rebuild Next Queue with proposed items only**

Keep only `proposed` items in `## Next Queue`, ordered P0 → P1 → P2.
Remove all `done`, `specified`, `planned`, and `in_progress` items.
The current active epic (P0 - Roadmap Refresh) will be `done` by the end.

- [ ] **Step 3: Add Risk Register section**

Create `## Risk Register` between Next Queue and Completed Foundation.
Populate with entries from Planning Review: installer trust boundary,
recovery ownership inventory, no automated release pipeline.

- [ ] **Step 4: Update Agent Scheduling Rules**

Document the Roadmap Planning Review escape hatch and queue exhaustion behavior.

- [ ] **Step 5: Verify with agent harness**

Run `make agent-check`, `cargo test agent`, `make agent-next`, `make agent-start`.

## Verification Commands

- `cargo test parses_next_queue`
- `cargo test agent`
- `make agent-check`
- `make agent-next`
- `make agent-start EPIC="P0 - Multi-Agent Review Protocol"`

## Test Level

- Unit tests: `cargo test agent` (parser tests)
- Manual verification: `make agent-next`, `make agent-start`, `make agent-check`

## Regression Coverage Expectations

- `## Active Queue` parsing must still work (backward compatibility).
- `make agent-next` must not return completed or deferred items.
- No handoff links or verification records may be lost.
- `make check` must continue to pass.

## Expected Outcomes

- `docs/roadmap.md` has a clean Next Queue with only `proposed` items.
- `docs/roadmap.md` has a `## Risk Register` section with at least 3 entries.
- `docs/roadmap.md` has a `## Completed Foundation` section with all 17+ done epics.
- `make agent-next` returns "P0 - Multi-Agent Review Protocol".
- `make agent-start` can lock the next P0 item.
- `make agent-check` passes for the current epic after advance to `done`.
