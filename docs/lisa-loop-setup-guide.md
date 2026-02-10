# Lisa Loop Setup Guide

How to set up your project for lisa-loop completion. This guide assumes you have a codebase and want lisa to manage concurrent Claude Code agents working through tickets using the RDSPI workflow.

---

## 1. Prerequisites

Install these before starting:

- **Zellij** -- the terminal multiplexer lisa runs inside. Install via `cargo install zellij` or your package manager.
- **Claude Code** -- the CLI agent lisa spawns. Install via `npm install -g @anthropic-ai/claude-code`. Verify with `claude --version`.
- **Git repository** -- your project must be a git repo. Lisa uses git for commit serialization across concurrent agents.
- **The lisa plugin** -- either download the `.wasm` file from a release, or build it yourself:

```bash
# Clone lisa and build the plugin
git clone <lisa-repo-url>
cd lisa
cargo build --target wasm32-wasi --release
# Output: target/wasm32-wasi/release/lisa.wasm
```

Copy the `.wasm` file somewhere accessible. You will reference it in your zellij layout.

---

## 2. Directory Structure

Create the following directories in your project root:

```
your-project/
├── CLAUDE.md              # Workflow definition (required)
├── docs/active/
│   ├── tickets/           # Ticket markdown files
│   ├── stories/           # Story markdown files (optional)
│   └── work/              # Phase artifacts (auto-created by agents)
```

Run this from your project root:

```bash
mkdir -p docs/active/tickets docs/active/stories docs/active/work
```

The `work/` directory gets populated automatically. Each ticket gets a subdirectory (`docs/active/work/T-001-01/`) containing its phase artifacts as agents produce them. Do not pre-create these.

---

## 3. CLAUDE.md Template

Create a `CLAUDE.md` in your project root. This is the file every Claude Code agent reads to understand how to work. It has two sections: project-specific context, and the RDSPI workflow definition.

Copy this template and fill in the project-specific parts:

```markdown
# CLAUDE.md

## Project

<!-- Replace this section with your project's description, build commands, and layout. -->

One-paragraph description of what this project is and does.

### Build and Test

\`\`\`bash
# Build
<your-build-command>

# Test
<your-test-command>

# Lint (if applicable)
<your-lint-command>
\`\`\`

### Source Layout

\`\`\`
src/
  main.rs          # Entry point
  lib.rs           # Library root
  ...              # Describe your modules
\`\`\`

### Directory Conventions

\`\`\`
docs/active/tickets/    # Ticket files (markdown with YAML frontmatter)
docs/active/stories/    # Story files (same frontmatter pattern)
docs/active/work/       # Work artifacts, one subdirectory per ticket ID
\`\`\`

---

## RDSPI Workflow

Every ticket passes through five phases in order. No phases are skipped regardless of ticket size.

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

Execute the plan. Track progress in `progress.md`. Commit incrementally.

Follow the plan step by step. After each meaningful unit of work, commit. Update `progress.md` with what has been completed, what remains, and any deviations from the plan. If the plan needs adjustment, document the deviation and rationale before proceeding.

Artifact: `docs/active/work/{ticket-id}/progress.md`

---

## Phase Rules

1. **All five phases always run.** Research, Design, Structure, Plan, Implement. Each phase is cheap (~200 lines, a few minutes). Skipping phases based on ticket size is how context degrades.

2. **~200 lines per artifact.** This is not a hard limit but a forcing function for structured thinking. Enough to be thorough, short enough to review quickly.

3. **Phase transitions.** When a phase completes, update the ticket's `phase` field in its YAML frontmatter. Lisa watches for these changes to update scheduling.

4. **Review points.** Research and Design are high-leverage review points. Reviewing ~200 lines of research or design catches problems before they become thousands of lines of wrong code. Structure and Plan may auto-advance depending on project configuration.

5. **Artifacts are insurance.** If a session crashes or hits limits, the latest artifact plus the ticket is enough to seed a new session at the correct phase.

---

## Ticket Format

Tickets live in `docs/active/tickets/`. Each ticket is a markdown file with YAML frontmatter:

\`\`\`yaml
---
id: T-024-03
story: S-024
title: migrate-climate-calls
type: task
status: open
priority: high
phase: research
depends_on: [T-024-01, T-024-02]
blocks: [T-024-06]
---

## Context

Description of the work and why it matters.

## Acceptance Criteria

- Concrete, verifiable conditions for done.
\`\`\`

Fields:
- `id`: Unique ticket identifier (e.g., `T-024-03`)
- `story`: Parent story ID
- `title`: Kebab-case short name
- `type`: `task` | `bug` | `spike`
- `status`: `open` | `in-progress` | `review` | `done` | `blocked`
- `priority`: `critical` | `high` | `medium` | `low`
- `phase`: `ready` | `research` | `design` | `structure` | `plan` | `implement` | `review` | `done`
- `depends_on`: List of ticket IDs that must complete before this ticket starts
- `blocks`: List of ticket IDs that depend on this ticket

---

## Concurrency

Lisa computes the DAG from ticket dependencies and spawns threads for all tickets whose dependencies are satisfied. Multiple threads work on the same branch. Commit serialization is handled via file locking -- agents do not need to coordinate with each other.

If two tickets modify the same files, that is a missing dependency edge in the DAG. The lock is a safety net, not a substitute for correct dependency modeling.
```

The RDSPI Workflow section and everything below it can be copied verbatim. The Project section at the top is what you customize per project. Be specific about build commands, test commands, and source layout -- agents read this to orient themselves.

---

## 4. Writing Tickets

### File Naming

Name ticket files to match their ID: `T-001-01.md`, `T-003-07.md`. The convention is `T-{story number}-{sequence number}`. This is not enforced but keeps things findable.

### Required Frontmatter

Every ticket needs these fields:

```yaml
---
id: T-001-01
title: define-core-types
type: task
status: open
priority: high
phase: ready
depends_on: []
blocks: []
---
```

- `id` -- must be unique across all tickets
- `title` -- kebab-case, descriptive, short
- `type` -- what kind of work: `task`, `bug`, `spike`, `feature`, `chore`
- `status` -- start with `open`
- `priority` -- `critical`, `high`, `medium`, or `low`
- `phase` -- start with `ready` for tickets that should be picked up immediately
- `depends_on` -- list of ticket IDs that must be `done` before this ticket starts
- `blocks` -- list of ticket IDs that cannot start until this ticket finishes

### Optional Frontmatter

- `story` -- parent story ID (e.g., `S-001`). Useful for grouping but not required for DAG computation.

### Body Format

After the frontmatter, write two sections:

```markdown
## Context

What needs to happen and why. Provide enough background for an agent that has
never seen your codebase before. Reference specific files, modules, or patterns
where relevant.

## Acceptance Criteria

- Each criterion is concrete and verifiable
- "All tests pass" is better than "code works"
- "Function returns error for empty input" is better than "handles edge cases"
- Reference specific files or interfaces when possible
```

### Dependency Modeling

This is the most important part of ticket writing. Get the dependencies right and lisa handles parallelism automatically. Get them wrong and agents collide.

**Rule: if two tickets modify the same files, one must depend on the other.**

Think about it this way: two agents working simultaneously on the same file will produce conflicting commits. The `depends_on` / `blocks` edges prevent this.

Example of a correct dependency chain:

```
T-001-01 (define types)      -- no dependencies
  blocks: [T-001-02, T-001-03]

T-001-02 (wire plugin state)  -- depends on types being defined
  depends_on: [T-001-01]
  blocks: [T-001-03]

T-001-03 (end-to-end test)    -- depends on both
  depends_on: [T-001-01, T-001-02]
```

Here, T-001-01 runs first. When it finishes, T-001-02 starts. T-001-03 waits for both. If T-001-02 and T-001-03 touched completely different files, they could run in parallel -- but T-001-03 depends on T-001-02's output, so it waits.

**Tips:**

- When in doubt, add a dependency. False parallelism (two agents stepping on each other) is worse than false serialization (one agent waiting when it could have started).
- `blocks` is the inverse of `depends_on`. Keep both consistent: if A depends on B, then B should list A in its `blocks`. Lisa uses both for DAG computation.
- A ticket with `depends_on: []` and `phase: ready` will be picked up immediately when lisa starts.

---

## 5. Writing Stories

Stories are optional. They group related tickets into a higher-level unit of work.

### Story Format

Stories live in `docs/active/stories/`. Same frontmatter pattern as tickets, but simpler:

```yaml
---
id: S-001
title: plugin-foundation
type: story
status: in_progress
priority: high
---

## Plugin Foundation

High-level description of the goal. What does this story accomplish when all its
tickets are done?

- Bullet points describing the scope
- What capabilities exist after this story completes
- Any constraints or boundaries
```

### How Stories Group Tickets

Tickets reference their parent story via the `story` field:

```yaml
---
id: T-001-03
story: S-001     # <-- links this ticket to story S-001
title: end-to-end-dashboard
...
---
```

Stories do not have `depends_on` or `blocks` fields. Dependency ordering is defined entirely at the ticket level. A story is done when all its tickets are done.

Use stories when you have 3+ related tickets that form a logical unit. For one-off tickets, skip the story.

---

## 6. Running Lisa

### Loading the Plugin in Zellij

Start zellij, then load the lisa plugin. You can do this via a layout file or by loading the plugin directly.

**Option A: Zellij layout file** (recommended for repeatable setup)

Create a `layout.kdl` file:

```kdl
layout {
    pane
    pane {
        plugin location="file:/path/to/lisa.wasm" {
            ticket_dir "docs/active/tickets"
            story_dir  "docs/active/stories"
            work_dir   "docs/active/work"
            max_threads "4"
            auto_advance "false"
        }
    }
}
```

Then start zellij with it:

```bash
zellij --layout layout.kdl
```

**Option B: Load plugin directly in a running session**

```bash
zellij plugin -- file:/path/to/lisa.wasm
```

### Configuration Options

All configuration is passed through the zellij plugin config map:

| Option | Default | Description |
|--------|---------|-------------|
| `ticket_dir` | `docs/active/tickets` | Directory containing ticket markdown files |
| `story_dir` | `docs/active/stories` | Directory containing story markdown files |
| `work_dir` | `docs/active/work` | Directory for phase artifacts |
| `max_threads` | `4` | Maximum concurrent Claude Code sessions |
| `auto_advance` | `false` | Whether to auto-advance phases without human review |

**On `max_threads`:** Start with 2. Four concurrent agents on one branch works but creates more file churn. Go higher only after you have confidence your dependency graph is correct.

**On `auto_advance`:** When false (default), agents park after Research and Design for human review. When true, agents proceed through all phases without stopping. Use `false` until you trust the workflow. Research and Design review catches problems before they become expensive implementation mistakes.

### What to Expect

When lisa loads:

1. It scans `ticket_dir` for all `.md` files and parses their frontmatter.
2. It computes the dependency DAG from `depends_on` / `blocks` fields.
3. It identifies tickets where `status: open`, `phase: ready`, and all dependencies are satisfied.
4. It spawns Claude Code sessions (up to `max_threads`) for those ready tickets.
5. The dashboard renders showing the DAG, active threads, and parked sessions.

---

## 7. Workflow In Practice

### Startup: DAG Computation

When lisa starts, it reads every ticket file and builds the dependency graph. Tickets with no unmet dependencies and `phase: ready` are immediately eligible for scheduling. Lisa spawns sessions for as many as `max_threads` allows, prioritizing by the `priority` field.

### Agent Lifecycle

Each Claude Code session follows this lifecycle:

1. **Session opens.** Lisa runs `claude --dangerously-skip-permissions` in a new zellij pane, pointed at the ticket.
2. **Agent reads context.** The agent reads the ticket file and `CLAUDE.md`. The ticket tells it what to do. CLAUDE.md tells it how (the RDSPI phases, artifact format, build commands).
3. **Agent works through phases.** Starting from the ticket's current `phase`, the agent produces the artifact for each phase, updates the ticket's `phase` field in its frontmatter, and proceeds.
4. **Review points.** After Research and Design (by default), the agent parks -- it has produced its artifact and waits for human review. Lisa detects this and marks the thread as parked on the dashboard.
5. **Human reviews.** You read the artifact (`docs/active/work/T-XXX-XX/research.md` or `design.md`). If it looks good, advance the ticket's phase in its frontmatter and the agent continues. If it needs changes, leave feedback in the session.
6. **Implementation.** The agent follows its plan, commits incrementally, and tracks progress in `progress.md`.
7. **Completion.** The agent sets the ticket's phase to `done`. Lisa marks the thread as completed, recomputes the DAG, and spawns sessions for any tickets that are now unblocked.

### Review Points

The highest-leverage moments in the workflow:

- **After Research:** Read `research.md`. Does the agent understand the codebase correctly? Did it find the right files and patterns? Catching a misunderstanding here prevents 4 phases of wrong work.
- **After Design:** Read `design.md`. Is the chosen approach sound? Were alternatives considered? Is the rationale grounded in what Research found?

Structure and Plan are lower-risk review points. With `auto_advance: false`, agents still park there. With `auto_advance: true`, they proceed through to implementation.

### Phase Artifacts

Artifacts live in `docs/active/work/{ticket-id}/` and look like this:

```
docs/active/work/T-001-01/
├── research.md      # ~200 lines: what exists in the codebase
├── design.md        # ~200 lines: options, tradeoffs, decision
├── structure.md     # ~200 lines: file-level blueprint
├── plan.md          # ~200 lines: sequenced implementation steps
└── progress.md      # Updated during implementation
```

Each artifact is a standalone document. You can read any of them without needing to look at the session. They serve three purposes:

1. **Review checkpoints.** Read 200 lines of design instead of reviewing 2000 lines of code.
2. **Crash insurance.** If a session dies, the latest artifact plus the ticket is enough to restart a new session at the right phase.
3. **Knowledge capture.** When work spans multiple days or agents, the artifacts record what was learned and decided.

---

## 8. Notes for `lisa init` (Future)

A `lisa init` command would automate the manual setup described in this guide. Here is what it would do and what to watch for.

### What `lisa init` Would Automate

1. **Create directory structure.** `mkdir -p docs/active/{tickets,stories,work}`.
2. **Generate CLAUDE.md template.** Scaffold the file with project-specific placeholders and the full RDSPI workflow definition. Detect language/framework from the repo (presence of `Cargo.toml`, `package.json`, `go.mod`, etc.) to pre-fill build commands.
3. **Create initial story and tickets from user input.** Interactive prompt: "Describe what you want to build." Parse the response into a story and 2-5 tickets with dependency edges.
4. **Validate setup.** Check that `CLAUDE.md` exists, ticket directory has at least one ticket, all `depends_on` references resolve to real ticket IDs, no circular dependencies in the DAG, and `claude` and `zellij` are on PATH.

### Pain Points During Manual Setup

These are the things that go wrong when setting up by hand -- and the things `lisa init` should prevent:

- **Forgetting `CLAUDE.md`.** Without it, agents have no workflow definition and produce unstructured output. The init command should refuse to proceed without it.
- **Mismatched `depends_on` / `blocks`.** If T-001-02 lists `depends_on: [T-001-01]` but T-001-01 does not list `blocks: [T-001-02]`, the DAG still works (lisa computes from `depends_on`) but the ticket files are inconsistent. The init command should auto-populate `blocks` as the inverse of `depends_on`.
- **Circular dependencies.** Easy to create accidentally when writing tickets by hand. The init command should validate the DAG is acyclic.
- **Tickets with wrong initial phase.** If you write `phase: research` instead of `phase: ready`, the ticket looks like it is already in-progress. Agents get confused. The init command should default new tickets to `phase: ready`.
- **Missing acceptance criteria.** Vague tickets produce vague implementations. The init command should warn if a ticket body has no `## Acceptance Criteria` section.
- **Dependency gaps.** Two tickets that modify the same files but have no dependency edge. The init command cannot fully detect this (it would need to know file modification scope), but it could prompt the user: "Do any of these tickets modify the same files? If so, add a dependency."
- **CLAUDE.md without build commands.** Agents need to know how to build and test. If the Project section has placeholder commands, the init command should warn.
