# Lisa

[![Release](https://img.shields.io/github/v/release/johnhkchen/lisa)](https://github.com/johnhkchen/lisa/releases/latest)

DAG-driven concurrent task scheduling for AI-assisted development.

## What It Does

When you have a set of interdependent tasks — a feature broken into tickets, a refactor with sequencing constraints, a sprint with parallel workstreams — Lisa schedules and runs them concurrently as Claude Code sessions. You define the work as markdown tickets with dependency metadata. Lisa figures out what can run in parallel, what has to wait, and launches sessions accordingly.

Lisa runs as a [Zellij](https://zellij.dev/) plugin. It reads your tickets, computes a dependency graph, and spawns Claude Code sessions for every ticket whose dependencies are satisfied. A dashboard shows what's running, what's queued, and what's done. When a ticket finishes, Lisa checks what it unblocked and schedules the next wave.

Each ticket goes through five phases: Research, Design, Structure, Plan, Implement. Every phase produces a short artifact (~200 lines) that serves as both a review checkpoint and crash recovery. If a session dies mid-work, the latest artifact plus the ticket is enough to seed a new session at the right phase.

## Prerequisites

- [Claude Code](https://docs.anthropic.com/en/docs/claude-code) — the AI coding assistant that does the work
- [Zellij](https://zellij.dev/) — terminal multiplexer that hosts Lisa as a plugin

After installing Lisa, run `lisa doctor` to verify everything is in place.

## Install

### Shell installer (recommended)

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/johnhkchen/lisa/releases/latest/download/lisa-cli-installer.sh | sh
```

### Homebrew (macOS)

```bash
brew install johnhkchen/lisa/lisa
```

### From crates.io (coming soon)

```bash
cargo install lisa-cli
```

> **Note:** `lisa-cli` is not yet published to crates.io. For now, use the shell installer, Homebrew, or build from source. When available, building from crates.io will require the `wasm32-wasip1` Rust target (`rustup target add wasm32-wasip1`) because the WASM plugin is compiled and embedded during the build.

### From source

```bash
git clone https://github.com/johnhkchen/lisa
cd lisa
rustup target add wasm32-wasip1
just install
```

Requires the [Rust toolchain](https://rustup.rs/) and [just](https://github.com/casey/just).

## Quick Start

Initialize your project:

```bash
cd your-project
lisa init
```

This creates the ticket directories and a `CLAUDE.md` tailored to your project.

Create a ticket in `docs/active/tickets/`:

```yaml
---
id: T-001-01
title: Add user authentication
type: task
status: open
phase: ready
priority: high
depends_on: []
---

## Context

Add JWT-based authentication to the API. The `/login` endpoint should
accept email/password and return a signed token.

## Acceptance Criteria

- POST /login returns a JWT on valid credentials
- Protected routes reject requests without a valid token
```

Launch Lisa:

```bash
lisa loop
```

Lisa opens a Zellij session with a dashboard. It picks up all tickets in `ready` phase whose dependencies are satisfied and starts Claude Code sessions for each one.

By default Lisa runs 2 concurrent sessions. To run more:

```bash
# One-off: pass a flag
lisa loop --max-threads 4

# Persistent: edit .lisa.toml
```

```toml
# .lisa.toml
[scheduling]
max_threads = 4
```

The `--max-threads` flag overrides `.lisa.toml` for that run.

## Configuration

`lisa init` creates a `.lisa.toml` in your project root:

```toml
[dirs]
tickets = "docs/active/tickets"
stories = "docs/active/stories"
work = "docs/active/work"

[scheduling]
max_threads = 2
```

| Key | Default | Description |
|-----|---------|-------------|
| `dirs.tickets` | `docs/active/tickets` | Where Lisa reads ticket files |
| `dirs.stories` | `docs/active/stories` | Where Lisa reads story files |
| `dirs.work` | `docs/active/work` | Where phase artifacts are written |
| `scheduling.max_threads` | `2` | Maximum concurrent Claude Code sessions |

## How It Works

### Workflow

Every ticket passes through five phases in order:

1. **Research** — Map the relevant codebase. What exists, where, how it connects.
2. **Design** — Explore options, evaluate tradeoffs, choose an approach with rationale.
3. **Structure** — Define file-level changes, module boundaries, public interfaces.
4. **Plan** — Sequence implementation steps with testing strategy.
5. **Implement** — Execute the plan, commit incrementally, track progress.

Each phase produces a ~200-line artifact in `docs/active/work/{ticket-id}/`. These are review checkpoints — catching a bad design at 200 lines is cheaper than catching it at 2,000 lines of wrong code.

### Scheduling

Tickets declare dependencies via the `depends_on` field. Lisa computes a DAG, topologically sorts it, and schedules all tickets whose dependencies are satisfied. As tickets complete, newly unblocked tickets are scheduled automatically.

### Concurrency

Multiple Claude Code sessions work in parallel on the same branch. Commit serialization is handled via file locking — sessions don't need to coordinate with each other. If two tickets modify the same files, that's a missing dependency edge in the DAG.

## Project Layout

```
crates/
  lisa-core/       Shared types, ticket parsing, DAG computation
  lisa-plugin/     Zellij WASM plugin (scheduler, dashboard, plugin entry)
  lisa-cli/        CLI binary (lisa init, lisa validate, lisa loop, lisa doctor)

docs/
  active/
    tickets/       Ticket files (markdown with YAML frontmatter)
    stories/       Story files (grouping related tickets)
    work/          Phase artifacts, one subdirectory per ticket
  knowledge/
    rdspi-workflow.md   Workflow definition (injected into agent context)
```

## CLI Reference

### `lisa init`

Scaffold a project for Lisa: creates ticket directories, `CLAUDE.md`, RDSPI workflow, hooks, and `.lisa.toml`.

```bash
lisa init              # Initialize current directory
lisa init --dry-run    # Preview what would be created
lisa init --path ../other-project
```

### `lisa validate`

Check that tickets parse correctly, the DAG has no cycles or missing dependencies, and the project structure is sound.

```bash
lisa validate
lisa validate --check-tools   # Also verify zellij and claude are on PATH
```

### `lisa loop`

Launch a Zellij session with the Lisa plugin. Schedules and runs Claude Code sessions based on the ticket DAG.

```bash
lisa loop
lisa loop --max-threads 4     # Override concurrent session limit
lisa loop --dry-run            # Show what would launch without starting
```

### `lisa status`

Inspect the DAG offline: tickets, dependencies, execution waves, and scheduling readiness.

```bash
lisa status
```

### `lisa doctor`

Verify that all runtime dependencies (Zellij, Claude Code, wasm32-wasip1 target) are installed.

```bash
lisa doctor
```

### `lisa setup-guide`

Print LLM-friendly setup instructions for the current project. Useful for seeding a Claude Code session with project context.

```bash
lisa setup-guide
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for build instructions, test commands, and how to submit changes.

## License

MIT
