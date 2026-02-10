# Ralph: Design Document

## Problem

The RDSPI workflow (Research → Design → Structure → Plan → Implement) is project-agnostic, but every existing implementation couples it to a specific project's toolchain. A TypeScript DAG tool works for one project but requires rewriting or dragging Node into the next. A bash Ralph loop works until it doesn't, and the lessons learned die with the project. Worktree commands in a Justfile get abandoned when a new repo starts. The workflow knowledge gets trapped in project-specific implementation details and requires reimplementation every time.

The existing single-thread Ralph model compounds this. It assumes linear execution: claim a ticket, work it, finish, move on. This wastes human attention (watching one agent work when you could be reviewing another's output) and wastes agent time (blocking on review while other independent tickets sit idle). The editable YAML DAG file drifts because agents treat it as a shortcut, editing the file directly instead of ticket frontmatter. VC-backed wrapper products add abstraction layers optimized for demos, not for power users SSHing from a phone to manage four concurrent agents.

## Core Insight

The workflow is independent of any project. The tool that implements it should be too.

## What We're Building

Ralph is a standalone zellij plugin — a Rust program compiled to WASM — that implements the RDSPI workflow as a DAG-driven concurrent scheduler. It carries between projects as a single `.wasm` file with zero project-specific dependencies. A project's only responsibility is having tickets in the right format.

## Design Principles

**No project-language dependency.** Ralph doesn't care if the codebase is TypeScript, Rust, or Python. It reads markdown frontmatter and manages terminal panes. It never touches project code.

**Single source of truth.** The DAG is computed on demand from ticket frontmatter. There is no DAG file. Agents cannot corrupt what doesn't exist.

**Five phases always run.** Research, Design, Structure, Plan, Implement. Each phase is cheap (~200 lines, a few minutes). Skipping phases based on ticket size is how context degrades. Bad research produces thousands of bad lines of code. Bad design produces hundreds. Review early, not late.

**Concurrent threads, not sequential execution.** Ralph reads the DAG, finds all tickets whose dependencies are satisfied, and spins up threads for each. Threads park at review points while other work continues. Human attention is the scheduler, not the bottleneck.

**Review points, not auto-termination.** When Claude finishes a phase and waits for input, that's a feature. The human reviews when ready. Meanwhile, Ralph has already started the next independent ticket.

## Architecture

### Ralph as Zellij Plugin

Ralph runs as a WASM plugin inside zellij. It renders a dashboard pane showing the DAG, thread states, and phase progress. It uses zellij's plugin API to:

- **Spawn command panes** (`open_command_pane`) for each Claude Code thread — one session per ticket, running `claude --dangerously-skip-permissions`. The agent reads the ticket and workflow definition to know what to do. No prompt injection from Ralph.
- **Detect phase completion** via `FileSystemUpdate` events (when artifact files appear or the ticket's `phase` field changes) and `CommandPaneExited` events (when a session ends).
- **Run background commands** (`run_command`) for DAG computation, artifact validation, commit serialization — anything that shouldn't block the UI.
- **Track pane state** via `PaneUpdate` events, knowing which threads are alive, which are waiting for review, and which have finished.
- **Manage layout** by opening panes near the plugin, naming them, and organizing them within zellij's native pane/tab system.

### Why Zellij, Not Tmux

Tmux + bash scripts is where every previous attempt at this has topped out. You get something working, it's fragile, it's not portable, and the state management is all ad-hoc file watches and polling. Zellij provides:

- A real event system (pane lifecycle, filesystem changes, command completion).
- A plugin API with typed Rust bindings — not `tmux send-keys` string manipulation.
- Session persistence with automatic serialization and crash recovery.
- Native layout management (tabs, panes, floating panes, stacking).
- A single binary with no runtime dependencies beyond the OS.

The plugin is a compiled, testable, distributable artifact — not scattered shell scripts.

### Ticket Format

Tickets are both the unit of work and the prompt. The agent reads the ticket to understand what to do and what phase it's in. Phase transitions are driven by updating the ticket's `phase` field — the ticket itself enforces the workflow.

Tickets live in a configurable directory (default: `docs/active/tickets/`). Format:

```yaml
---
id: T-024-03
story: S-024
title: migrate-climate-calls
type: task
status: open
priority: high
phase: research  # ready | research | design | structure | plan | implement | review | done
depends_on: [T-024-01, T-024-02]
blocks: [T-024-06]
---

## Context

The climate API calls are scattered across three services and use inconsistent
error handling. They need to be consolidated into a single climate service module.

## Acceptance Criteria

- All climate API calls route through `src/services/climate.ts`
- Retry logic with exponential backoff on all external calls
- Existing tests pass, new integration tests for the consolidated service
```

The generic phase definitions (what Research means, what Design means, what artifacts to produce) live in the project's `CLAUDE.md` or a workflow definition file that Ralph points to. The ticket provides the specific task. The agent reads both — the workflow tells it how to work, the ticket tells it what to work on.

This means Ralph doesn't need to inject phase prompts. It opens a session, the agent reads the ticket and the workflow definition, and proceeds. When a phase completes, the ticket's `phase` field advances and the agent continues. Ralph's role is scheduling and lifecycle, not prompt engineering.

Stories live alongside (default: `docs/active/stories/`). Same frontmatter pattern with their own dependency fields.

### DAG Computation

No file. Ralph reads all ticket files, parses frontmatter, resolves `depends_on` / `blocks` relationships, and produces an in-memory graph. This runs as a background command via `run_command` (calling a small script or doing it in-plugin via filesystem scanning). The graph determines what can run in parallel and what must wait.

### Phase Artifacts

Each ticket gets a work directory created on demand: `docs/active/work/T-NNN-XX/`

Contents per phase:

| Phase | Artifact | Purpose |
|-------|----------|---------|
| Research | `research.md` | Descriptive map of codebase. What exists, where, how it connects. Not prescriptive. |
| Design | `design.md` | Options explored, tradeoffs evaluated, decision with rationale. |
| Structure | `structure.md` | File-level changes, architecture, component boundaries, ordering. |
| Plan | `plan.md` | Sequenced implementation steps, testing strategy, verification criteria. |
| Implement | `progress.md` | Tracks implementation across potentially multiple sessions. |

Each phase produces a ~200-line artifact not as a compaction boundary, but as structured output that forces disciplined thinking and provides review leverage. The full context remains available in the session throughout.

### Concurrency and Commit Serialization

Multiple threads can work on the same branch because independent tickets (by DAG definition) shouldn't be modifying the same files. The dangerous window is just `git add && git commit` — two agents doing that simultaneously produces a dirty index error.

Solution: a commit lock. `flock` (or equivalent) on a lockfile. Each agent's commit step acquires the lock, adds, commits, releases. Milliseconds of actual contention. Agents don't need to know about each other — Ralph wraps the commit mechanism and serialization happens transparently.

If two tickets do touch the same files, that's a dependency the DAG should express. The lock is a safety net, not a substitute for correct dependency modeling.

### Thread Lifecycle

A thread is one Claude Code session that runs a single ticket through all five phases. With Opus 4.6's 1M context window, a full ticket cycle — Research through Implement — fits comfortably in one session without compaction.

Ralph opens a session and points it at the ticket. The agent reads the ticket (the task context) and the workflow definition in CLAUDE.md (the phase instructions). It knows what phase it's in, what that phase requires, and what artifact to produce. When it finishes a phase, it updates the ticket's `phase` field and continues to the next — or parks if the phase requires human review.

The ticket drives the transitions. Ralph watches for `phase` field changes to update its dashboard and scheduling decisions, but it doesn't need to inject prompts or orchestrate phase-by-phase. The agent and the ticket handle that together.

Artifacts are still produced at every phase boundary. They serve as:

- **Review checkpoints.** The human reads `research.md` or `design.md` as a file, not by interrogating the session. Reviewing ~200 lines of specs is higher leverage than reviewing thousands of lines of generated code.
- **Insurance.** If a session hits context limits, crashes, or needs to be restarted, the latest artifact plus the ticket is enough to seed a new session at the right phase.
- **Handoff documents.** If work spans multiple days or the human wants a different agent to take over, the artifacts capture the accumulated understanding.

The artifacts are not compaction boundaries. They are the written record of each phase's conclusions, produced because structured thinking produces better output — not because the context window demands it.

### Agent Teams

When `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS` is enabled, Ralph can leverage swarms for Research and Design phases — spawn parallel agents exploring different aspects of the codebase or advocating for different design options. Not for Structure, Plan, or Implement, where sequential reasoning matters and file modification conflicts arise.

This is an enhancement, not a requirement for v1.

### Observability

The Ralph dashboard pane shows:

- The DAG with dependency edges and current status of each ticket.
- Active threads: which ticket, which phase, how long running.
- Parked threads: waiting for human review, with the artifact path.
- Recent activity log: phase completions, commits, errors.
- Quick-jump to any thread's pane.

This replaces `just dag-status`, `just ralph-status`, and `just ralph-logs` with a single live view.

## What the Project Provides

Ralph requires only:

1. **Ticket files** in the expected frontmatter format, in a configured directory.
2. **A git repository** (for commit serialization).
3. **Claude Code installed** on the host.
4. **Zellij** as the terminal multiplexer.

No language runtime. No package manager. No project-specific configuration beyond the ticket directory path.

## What Ralph Provides

1. DAG computation from ticket frontmatter.
2. Concurrent thread scheduling based on dependency resolution.
3. Thread lifecycle management — spawning sessions, tracking phase progress, detecting completion.
4. Commit serialization across concurrent threads.
5. A live dashboard with full workflow visibility.
6. Session persistence (via zellij) across disconnects and reconnects.
7. Portability across projects as a single `.wasm` file.

## Testability

The system has clear testing boundaries:

**Pure logic (unit testable, no zellij needed):**
- DAG computation: given ticket frontmatter, produce correct dependency graph.
- Scheduler decisions: given ticket states and DAG, determine which threads to spawn.
- Artifact validation: given a work directory, determine if a phase artifact is complete.
- Ticket state parsing: given a ticket file, correctly extract phase, dependencies, status.

**Integration (needs zellij plugin harness):**
- Pane lifecycle: spawning command panes, receiving exit events, managing state.
- Filesystem watching: detecting artifact creation and ticket state changes.
- Background command execution: running git operations, DAG computation scripts.

**End-to-end (needs Claude Code):**
- Does the Research phase prompt actually produce a useful `research.md`?
- Does compaction preserve the right information across phase transitions?
- Do concurrent agents on the same branch cause conflicts in practice?

The first category is cheap and fast. The second needs a zellij test environment. The third is expensive and is where experimentation matters most.

## Open Questions

- **Context limits in practice**: A full RDSPI cycle on a large ticket should fit in 1M tokens. If it doesn't, the artifacts provide a restart point. Monitor whether this assumption holds across real workloads.
- **Parallelism limits**: Two concurrent threads on one branch is safe with the commit lock. Four may create enough file churn to confuse agents. Needs empirical testing.
- **Agent teams ROI**: Do Research/Design swarms actually improve output quality, or just burn tokens?
- **Worktree integration**: Ralph could manage worktrees for parallel implementation across stories (not just tickets). Deferred until single-branch concurrency is proven.
- **Plugin distribution**: `.wasm` file via GitHub releases, or a zellij plugin registry if one matures.
- **Review gating**: Which phases require human approval before advancing, and which auto-advance? Research and Design are high-leverage review points. Structure and Plan may be safe to auto-advance in many cases. This should be configurable per project or per ticket.

## Non-Goals

- Replacing Claude Code. Ralph orchestrates it, doesn't wrap it.
- Building a project management UI. The dashboard shows workflow state, not Jira.
- Supporting non-Claude agents. The phase prompts and hooks are Claude Code-specific.
- Running without zellij. The plugin API is the foundation, not an optional integration.

