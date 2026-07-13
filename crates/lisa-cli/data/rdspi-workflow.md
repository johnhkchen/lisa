## RDSPI Workflow

Every ticket passes through six phases in order. No phases are skipped regardless of ticket size. Complete all phases in a single continuous pass — do not stop between phases.

### Research

Map the codebase. Produce `research.md` (~200 lines).

Descriptive, not prescriptive. What exists, where, how it connects. Identify the files, modules, patterns, and boundaries relevant to the ticket. Surface assumptions and constraints. Do not propose solutions.

Artifact: `docs/active/work/{ticket-id}/research.md`

### Design

Explore options, evaluate tradeoffs, decide with rationale. Produce `design.md` (~200 lines).

Enumerate viable approaches. Assess each against the codebase reality from Research. Choose one and explain why. Document what was rejected and why. The decision must be grounded in the research, not assumptions.

Artifact: `docs/active/work/{ticket-id}/design.md`

### Structure

Define file-level changes, architecture, and component boundaries. Produce `structure.md` (~200 lines).

Specify which files are created, modified, or deleted. Define module boundaries, public interfaces, and internal organization. Establish the ordering of changes where it matters. This is the blueprint -- not code, but the shape of the code.

Artifact: `docs/active/work/{ticket-id}/structure.md`

### Plan

Sequence the implementation steps. Produce `plan.md` (~200 lines).

Break the work into ordered steps that can be executed and verified independently where possible. Define the testing strategy: what gets unit tests, what needs integration tests, what the verification criteria are. Each step should be small enough to commit atomically.

Artifact: `docs/active/work/{ticket-id}/plan.md`

### Implement

Execute the plan. Track progress in `progress.md`. Commit meaningful units through Lisa's isolated transaction.

Follow the plan step by step. Update `progress.md` with what has been completed, what remains, and any deviations from the plan. If the plan needs adjustment, document the deviation and rationale before proceeding.

For each meaningful implementation unit, run `lisa commit-ticket --ticket-id <ticket-id> --message <message> --include <exact-repository-relative-path>...`. Pass only paths owned by this ticket. Never use the ordinary index for ticket work: do not run ordinary `git add`, broad `git add -A`, or ordinary `git commit`, and do not leave staged changes for another command or process to consume. Before finishing Review, ensure every ticket-owned source change is committed through `lisa commit-ticket` and no ticket-owned source file remains staged, modified, or untracked.

Artifact: `docs/active/work/{ticket-id}/progress.md`

### Review

Self-assess the completed work. Produce `review.md` (~200 lines).

Summarize what changed: files created, modified, or deleted. Evaluate test coverage and flag gaps. Surface open concerns, TODOs, or known limitations. Flag critical issues that need human attention. This is the handoff document — what a human reviewer needs to understand the work without reading every diff.

Alongside `review.md`, write `review-disposition.json` with exactly one of these JSON shapes: `{"disposition":"pass","reason":null}` when the work is ready to complete, or `{"disposition":"block","reason":"<non-empty actionable reason>"}` when it is not. A pass with a reason, or a block without a non-empty reason, is invalid.

After writing both Review artifacts, remain on the current ticket and wait. Do not edit phase/status, publish completion yourself, or start another ticket. Lisa prepares Done, commits the ticket and work artifacts through the isolated transaction, and confirms that completion commit before releasing the seat or scheduling dependents.

Artifacts:

- `docs/active/work/{ticket-id}/review.md`
- `docs/active/work/{ticket-id}/review-disposition.json`

---

## Phase Rules

1. **All six phases always run.** Research, Design, Structure, Plan, Implement, Review. Each phase is cheap (~200 lines, a few minutes). Skipping phases based on ticket size is how context degrades.

2. **~200 lines per artifact.** This is not a hard limit but a forcing function for structured thinking. Enough to be thorough, short enough to review quickly.

3. **Phase transitions.** Lisa detects completed artifacts and advances the ticket's `phase` field in the YAML frontmatter automatically. Do not update phase or status fields manually — just produce the artifact and continue to the next phase.

4. **High-leverage phases.** Research and Design artifacts are the best return on review time. Reviewing ~200 lines of research or design catches problems before they become thousands of lines of wrong code. Structure and Plan may auto-advance depending on project configuration.

5. **Artifacts are insurance.** If a session crashes or hits limits, the latest artifact plus the ticket is enough to seed a new session at the correct phase.

6. **Completion is commit-gated.** The agent makes ticket-owned source changes durable through `lisa commit-ticket`, but Lisa alone writes Done and publishes completion. A failed completion commit leaves the ticket, seat, and dependents in place for a safe retry.

---

## Ticket Format

Tickets live in `docs/active/tickets/`. Each ticket is a markdown file with YAML frontmatter:

```yaml
---
id: T-024-03
story: S-024
title: migrate-climate-calls
type: task
status: open
priority: high
phase: ready
depends_on: [T-024-01, T-024-02]
---

## Context

Description of the work and why it matters.

## Acceptance Criteria

- Concrete, verifiable conditions for done.
```

Fields:
- `id`: Unique ticket identifier (e.g., `T-024-03`)
- `story`: Parent story ID
- `title`: Kebab-case short name
- `type`: `task` | `bug` | `spike`
- `status`: `open` | `in-progress` | `review` | `done` | `blocked`
- `priority`: `critical` | `high` | `medium` | `low`
- `phase`: `ready` | `research` | `design` | `structure` | `plan` | `implement` | `review` | `done`
- `depends_on`: List of ticket IDs that must complete before this ticket starts
- `blocks`: *(optional)* List of ticket IDs that depend on this ticket. Lisa computes this automatically from `depends_on`, so you do not need to maintain it by hand

---

## Concurrency

Lisa computes the DAG from ticket dependencies and spawns threads for all tickets whose dependencies are satisfied. Multiple threads work on the same branch. `lisa commit-ticket` and Lisa's final completion command serialize commits while using an isolated Git index, so unrelated entries already staged in the ordinary index remain untouched and uncommitted.

If two tickets modify the same files, that is a missing dependency edge in the DAG. The isolated transaction is a safety boundary, not a substitute for correct dependency modeling or exact `--include` ownership.
