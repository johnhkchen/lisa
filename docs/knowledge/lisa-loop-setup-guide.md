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
git clone https://github.com/johnhkchen/lisa
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
├── CLAUDE.md              # Project-specific context (required)
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

Create a `CLAUDE.md` in your project root. This file contains **only project-specific context** -- the RDSPI workflow definition ships with lisa in `docs/rdspi-workflow.md` and is injected into agent context automatically when sessions are spawned. You do not need to copy the workflow into every project.

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

The RDSPI workflow definition is in docs/rdspi-workflow.md and is injected into agent context by lisa automatically.
```

Be specific about build commands, test commands, and source layout -- agents read this to orient themselves. The workflow phases, phase rules, ticket format, and concurrency model are all defined in `docs/rdspi-workflow.md` which lisa references when spawning each Claude session.

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
---
```

- `id` -- must be unique across all tickets. Use only alphanumeric characters and hyphens (the ID is used as a directory name for work artifacts)
- `title` -- kebab-case, descriptive, short
- `type` -- what kind of work: `task`, `bug`, `spike`, `feature`, `chore`
- `status` -- start with `open`
- `priority` -- `critical`, `high`, `medium`, or `low`
- `phase` -- start with `ready` for tickets that should be picked up immediately
- `depends_on` -- list of ticket IDs that must be `done` before this ticket starts

### Optional Frontmatter

- `story` -- parent story ID (e.g., `S-001`). Useful for grouping but not required for DAG computation.
- `blocks` -- list of ticket IDs that cannot start until this ticket finishes. This is computed automatically by lisa from `depends_on` edges, so you do not need to maintain it. If present, it will be parsed, but omitting it is recommended to avoid inconsistency.

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

Think about it this way: two agents working simultaneously on the same file will produce conflicting commits. The `depends_on` edges prevent this.

Use `depends_on` as the primary mechanism for declaring dependencies. Lisa computes the full DAG -- including reverse edges (which tickets are blocked by a given ticket) -- from `depends_on` alone. You do not need to maintain a `blocks` field; it is optional and computed.

Example of a correct dependency chain:

```
T-001-01 (define types)      -- no dependencies
  depends_on: []

T-001-02 (wire plugin state)  -- depends on types being defined
  depends_on: [T-001-01]

T-001-03 (end-to-end test)    -- depends on both
  depends_on: [T-001-01, T-001-02]
```

Here, T-001-01 runs first. When it finishes, T-001-02 starts. T-001-03 waits for both. If T-001-02 and T-001-03 touched completely different files, they could run in parallel -- but T-001-03 depends on T-001-02's output, so it waits.

**Tips:**

- When in doubt, add a dependency. False parallelism (two agents stepping on each other) is worse than false serialization (one agent waiting when it could have started).
- You only need `depends_on`. Lisa computes reverse edges (blocked-by) automatically. The `blocks` field is accepted if present but is not required and not recommended -- maintaining both sides of every edge is busywork and a source of inconsistency.
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
tickets: [T-001-01, T-001-02, T-001-03]
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

### Optional: `tickets` Field

Stories can include a `tickets` field listing their child ticket IDs:

```yaml
tickets: [T-001-01, T-001-02, T-001-03]
```

This field is optional and informational -- it is not used for DAG computation. The authoritative link between a ticket and its story is the `story` field on each ticket. The `tickets` field on a story is a convenience for scanning story scope at a glance without grepping the tickets directory.

`lisa init` and future `lisa add-ticket` commands could maintain this field automatically.

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
            max_threads "2"  // concurrency cap; creates 4 panes (2x)
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
| `max_threads` | `2` | Maximum concurrent sessions (pane count is 2x) |
| `auto_advance` | `false` | Whether to auto-advance phases without human review |

**On `max_threads`:** Start with 2. The layout creates `2 * max_threads` panes so finishing sessions can wind down while new ones start immediately. Only `max_threads` tickets run concurrently. Go higher only after you have confidence your dependency graph is correct.

**On `auto_advance`:** When false (default), agents park after Research and Design for human review. When true, agents proceed through all phases without stopping. Use `false` until you trust the workflow. Research and Design review catches problems before they become expensive implementation mistakes.

### What to Expect

When lisa loads:

1. It scans `ticket_dir` for all `.md` files and parses their frontmatter.
2. It computes the dependency DAG from `depends_on` fields (and optional `blocks` fields if present).
3. It identifies tickets where `status: open`, `phase: ready`, and all dependencies are satisfied.
4. It spawns Claude Code sessions (up to `max_threads`) for those ready tickets.
5. The dashboard renders showing the DAG, active threads, and parked sessions.

---

## 7. Workflow In Practice

### Startup: DAG Computation

When lisa starts, it reads every ticket file and builds the dependency graph. Tickets with no unmet dependencies and `phase: ready` are immediately eligible for scheduling. Lisa spawns sessions for as many as `max_threads` allows, prioritizing by the `priority` field.

### Agent Lifecycle

Each Claude or Codex session follows this lifecycle:

1. **Session opens.** Lisa runs `claude --dangerously-skip-permissions` in a new zellij pane, pointed at the ticket.
2. **Agent reads context.** The agent reads the real ticket file, its provider context (`CLAUDE.md` or `AGENTS.md`), and `docs/knowledge/rdspi-workflow.md`. The ticket tells it what to do; the context file supplies project build/layout guidance; the workflow defines phase artifacts and the atomic Git contract.
3. **Agent works through phases.** Starting from the ticket's current `phase`, the agent produces the artifact for each phase. Lisa detects each artifact and advances phase frontmatter; the agent does not edit phase or status.
4. **Review points.** After Research and Design (by default), the agent parks -- it has produced its artifact and waits for human review. Lisa detects this and marks the thread as parked on the dashboard.
5. **Human reviews.** You read the artifact (`docs/active/work/T-XXX-XX/research.md` or `design.md`). Lisa has already recorded the artifact-driven phase transition; do not edit phase/status for the normal flow. If changes are needed, leave feedback in the session; otherwise let the configured continuation/auto-advance behavior proceed.
6. **Implementation.** The agent follows its plan, tracks progress in `progress.md`, and commits meaningful ticket-owned units only through `lisa commit-ticket` with exact repository-relative `--include` paths. It does not stage ticket work in the ordinary Git index.
7. **Completion.** The agent writes `review.md` and waits. Lisa prepares phase/status Done and commits the ticket plus all work artifacts through its isolated transaction. Only a verified commit receipt completes the thread, releases its provider seat, and unblocks dependents.

### Atomic completion and recovery

Lisa constructs ticket commits in an alternate Git index while holding the
repository's Lisa commit lock. A pre-existing staged file owned by a human or
another tool remains byte-for-byte staged and is excluded from the ticket
commit. Generated agent instructions prohibit ordinary `git add`, broad
`git add -A`, ordinary `git commit`, and staged handoff between commands.

Completion is fail-closed. If the final transaction cannot commit, reconcile,
or verify its exact paths, Lisa keeps the ticket in Review, retains the current
Claude/Codex seat, emits no Done provenance, and leaves dependents blocked. Read
the dashboard/activity Git error, repair the reported path overlap, lock, author
configuration, or repository state, then let the normal stopped/idle signal (or
the manual completion action) retry. Do not work around the failure by staging
the ticket in the ordinary index.

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
├── progress.md      # Updated during implementation
└── review.md        # Completed changes, coverage, and open concerns
```

Each artifact is a standalone document. You can read any of them without needing to look at the session. They serve three purposes:

1. **Review checkpoints.** Read 200 lines of design instead of reviewing 2000 lines of code.
2. **Crash insurance.** If a session dies, the latest artifact plus the ticket is enough to restart a new session at the right phase.
3. **Knowledge capture.** When work spans multiple days or agents, the artifacts record what was learned and decided.

---

## 8. Archiving Completed Work

When a story is complete -- all its tickets are `done` -- move its files out of the active directory into an archive.

### Archive Directory Structure

```
docs/archive/
├── stories/      # Completed story files
├── tickets/      # Completed ticket files
└── work/         # Phase artifacts (optional)
```

Create these directories alongside `docs/active/`:

```bash
mkdir -p docs/archive/{stories,tickets,work}
```

### What to Move

Archive per-story, not per-ticket. When a story finishes, move all of its files together:

1. The story file: `docs/active/stories/S-001.md` --> `docs/archive/stories/S-001.md`
2. All its ticket files: `docs/active/tickets/T-001-*.md` --> `docs/archive/tickets/`
3. Optionally, work artifacts: `docs/active/work/T-001-*/` --> `docs/archive/work/`

Keep work artifacts if you want historical reference for how decisions were made. Delete them if they are just noise -- the git history has everything if you need it later.

### Why This Matters

Lisa only scans `docs/active/`. Archived files are invisible to the plugin. Moving completed work out of the active directory keeps the DAG small, reduces scan time, and makes the dashboard show only what is in flight.

Do not archive individual tickets while a story is still in progress. A ticket being `done` does not mean it should leave the active directory -- other tickets in the same story may reference it in their `depends_on` fields, and lisa needs to see those done tickets to know the dependency is satisfied.

### Note for `lisa init`

The `lisa init` command should create archive directories alongside active directories: `docs/archive/{stories,tickets,work}`.

---

## 9. Mid-Flight Ticket Modifications

Sometimes requirements change while an agent is working on a ticket. What you do depends on how far the agent has progressed.

### If the Ticket is in Research or Design

Edit freely. The agent re-reads the ticket context at each phase boundary. Changes to acceptance criteria, context, or scope will be picked up when the next phase starts. No special action needed.

### If the Ticket is in Structure, Plan, or Implement

The agent has already committed to an approach based on the earlier phases. Research and Design artifacts reflect the old requirements. Changing the ticket now means the agent is building against stale context.

The right action:

1. **Stop the agent's session.** Close the pane in zellij.
2. **Update the ticket.** Edit the frontmatter and body with the new requirements.
3. **Reset the phase.** Set `phase: ready` in the ticket's frontmatter.
4. **Delete stale work artifacts.** Remove `docs/active/work/{ticket-id}/` -- the research, design, structure, and plan files are based on old requirements.
5. **Let lisa pick it up.** Lisa will see the ticket as ready and a fresh agent will start from Research.

This is intentionally aggressive. A few minutes of re-running Research and Design is cheaper than an agent implementing against wrong requirements. The early phases are fast. Wrong implementations are expensive.

### Trivial Edits

If the change is small and does not invalidate the design -- fixing a typo in acceptance criteria, adding a minor clarification -- use judgment. You do not need to reset for edits that do not change what the agent is building.

---

## 10. Notes for `lisa init` (Future)

A `lisa init` command would automate the manual setup described in this guide. Here is what it would do and what to watch for.

### What `lisa init` Would Automate

1. **Create directory structure.** `mkdir -p docs/active/{tickets,stories,work}`.
2. **Generate CLAUDE.md template.** Scaffold the file with project-specific placeholders (the RDSPI workflow ships separately in `docs/rdspi-workflow.md`). Detect language/framework from the repo (presence of `Cargo.toml`, `package.json`, `go.mod`, etc.) to pre-fill build commands.
3. **Create initial story and tickets from user input.** Interactive prompt: "Describe what you want to build." Parse the response into a story and 2-5 tickets with dependency edges.
4. **Validate setup.** Check that `CLAUDE.md` exists, ticket directory has at least one ticket, all `depends_on` references resolve to real ticket IDs, no circular dependencies in the DAG, and `claude` and `zellij` are on PATH.

### Pain Points During Manual Setup

These are the things that go wrong when setting up by hand -- and the things `lisa init` should prevent:

- **Forgetting `CLAUDE.md`.** Without it, agents have no project context (build commands, source layout) and produce disoriented output. The init command should refuse to proceed without it.
- **Redundant `blocks` fields.** The `blocks` field is no longer required -- lisa computes reverse edges from `depends_on` alone. The init command should not generate `blocks` fields. If existing tickets have `blocks` fields, they are accepted but ignored for DAG computation in favor of the canonical `depends_on` edges.
- **Circular dependencies.** Easy to create accidentally when writing tickets by hand. The init command should validate the DAG is acyclic.
- **Tickets with wrong initial phase.** If you write `phase: research` instead of `phase: ready`, the ticket looks like it is already in-progress. Agents get confused. The init command should default new tickets to `phase: ready`.
- **Missing acceptance criteria.** Vague tickets produce vague implementations. The init command should warn if a ticket body has no `## Acceptance Criteria` section.
- **Dependency gaps.** Two tickets that modify the same files but have no dependency edge. The init command cannot fully detect this (it would need to know file modification scope), but it could prompt the user: "Do any of these tickets modify the same files? If so, add a dependency."
- **CLAUDE.md without build commands.** Agents need to know how to build and test. If the Project section has placeholder commands, the init command should warn.
