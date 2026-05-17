# Multi-Agent Review Protocol Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** `docs/superpowers/specs/2026-05-17-p0-multi-agent-review-protocol-design.md`

**Goal:** Formalize the multi-agent review protocol as documentation and
templates. Update the review template with clear role descriptions, read-only
constraints, and coordinator synthesis rules. Document trigger criteria and
integration with existing roadmap workflows.

**Architecture:** Documentation-only epic. Updates to two template files and
AGENTS.md. No code changes.

**Tech Stack:** Markdown.

---

## Existing Code Map

- `docs/superpowers/agent/templates/multi-agent-review.md`: existing review
  template skeleton.
- `docs/superpowers/agent/templates/roadmap-review.md`: roadmap planning
  review template that references multi-agent review.
- `AGENTS.md` (repo root): references multi-agent review for safety-sensitive
  epics.
- `docs/roadmap.md`: epic definition and trigger criteria.

## Task: Formalize Multi-Agent Review Protocol

**Files:**
- Modify: `docs/superpowers/agent/templates/multi-agent-review.md`
- Modify: `docs/superpowers/agent/templates/roadmap-review.md`
- Modify: `AGENTS.md`

- [ ] **Step 1: Enhance the multi-agent review template**

Add explicit role descriptions, read-only constraint language, and coordinator
synthesis instructions to the existing template.

- [ ] **Step 2: Update roadmap review template**

Reference the multi-agent review protocol in roadmap-review.md for trigger
criteria.

- [ ] **Step 3: Update AGENTS.md**

Ensure the Multi-Agent Review section clearly references the protocol and
trigger criteria.

- [ ] **Step 4: Add plan link to roadmap and advance**

Update roadmap entry with plan path, advance to `planned`, then `in_progress`.

## Verification Commands

- `make agent-check`
- `cat docs/superpowers/agent/templates/multi-agent-review.md | grep -c "read-only"`

## Test Level

- Manual review: inspect template files for required sections and constraint
  language.
- `make agent-check` passes for the current epic.

## Regression Coverage Expectations

- Existing template sections must not be removed.
- AGENTS.md must still reference multi-agent review.
- The protocol must remain documentation-only (no harness enforcement).

## Expected Outcomes

- `multi-agent-review.md` template has clear role descriptions, read-only
  constraints, and coordinator synthesis rules.
- `roadmap-review.md` template references the multi-agent review protocol.
- `AGENTS.md` Multi-Agent Review section is clear and actionable.
- `make agent-check` passes.
