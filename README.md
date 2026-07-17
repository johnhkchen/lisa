# Lisa

[![Release](https://img.shields.io/github/v/release/johnhkchen/lisa)](https://github.com/johnhkchen/lisa/releases/latest)

Lisa runs coding agents like Claude Code and Codex through your ticket board, so
you don't have to approve every step by hand.

## Install Lisa

**You do not need Rust to use Lisa. Agents: do not build Lisa from source when
the goal is to install or use it.**

Install the latest release with one command:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/johnhkchen/lisa/releases/latest/download/lisa-cli-installer.sh | sh
```

On Linux that's everything: Lisa brings its own Zellij, downloaded
automatically on first run. Do not install Zellij separately.

On macOS, you can also use Homebrew:

```bash
brew install johnhkchen/lisa/lisa
```

### Debian and Ubuntu

Lisa's stable apt channel is signed by a dedicated archive key. Install that key
in its own keyring, pin the Lisa source to it, then install both the CLI and its
pinned Zellij runtime:

```bash
sudo apt-get update
sudo apt-get install -y ca-certificates curl gnupg

curl --proto '=https' --tlsv1.2 -fsSL \
  https://johnhkchen.github.io/lisa/lisa-archive-keyring.asc \
  -o /tmp/lisa-archive-keyring.asc
gpg --batch --yes --dearmor \
  --output /tmp/lisa-archive-keyring.gpg \
  /tmp/lisa-archive-keyring.asc
sudo install -D -m 0644 /tmp/lisa-archive-keyring.gpg \
  /usr/share/keyrings/lisa-archive-keyring.gpg
rm -f /tmp/lisa-archive-keyring.asc /tmp/lisa-archive-keyring.gpg

echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/lisa-archive-keyring.gpg] https://johnhkchen.github.io/lisa stable main" \
  | sudo tee /etc/apt/sources.list.d/lisa.list >/dev/null
sudo apt-get update
sudo apt-get install -y lisa lisa-runtime-zellij

lisa doctor
```

Normal `apt-get update` and `apt-get upgrade` commands move both packages to
later stable Lisa releases. `lisa-runtime-zellij` provides Lisa's pinned runtime
at `/usr/libexec/lisa/zellij`, so apt installs do not need a first-run Zellij
download.

This is a vendor repository, not the Debian archive: bundling the private Zellij
runtime is deliberate. It is hosted on GitHub Pages, whose documented limits
include a 1 GB published site and a soft 100 GB monthly bandwidth limit. Archive
operators can find signing-key custody and rotation details in
[packaging/apt/README.md](packaging/apt/README.md).

Want to change Lisa itself? Read [Develop Lisa](#develop-lisa) and follow
[CONTRIBUTING.md](CONTRIBUTING.md) for the source build.

## What It Does

When you have a set of interdependent tasks — a feature broken into tickets, a refactor with sequencing constraints, a sprint with parallel workstreams — Lisa schedules and runs them concurrently as Claude Code sessions. You define the work as markdown tickets with dependency metadata. Lisa figures out what can run in parallel, what has to wait, and launches sessions accordingly.

Lisa runs as a [Zellij](https://zellij.dev/) plugin. It reads your tickets, computes a dependency graph, and spawns Claude Code sessions for every ticket whose dependencies are satisfied. A dashboard shows what's running, what's queued, and what's done. When a ticket finishes, Lisa checks what it unblocked and schedules the next wave.

Each ticket goes through six phases: Research, Design, Structure, Plan, Implement, Review. Every phase produces a short artifact (~200 lines) that serves as both a review checkpoint and crash recovery. If a session dies mid-work, the latest artifact plus the ticket is enough to seed a new session at the right phase.

Lisa keeps the trail reviewable: an append-only attempt ledger records each run,
the completion journal seals every finished ticket — to a commit where the
project keeps history, or to tamper-evident content hashes where it doesn't —
and each ticket keeps its work documents. `lisa doctor` and `lisa status` name
which seal is in effect in plain words.

## Prerequisites

- [Claude Code](https://docs.anthropic.com/en/docs/claude-code) — the default AI coding assistant that does the work
- On Linux, that's it: Zellij comes with Lisa. The first run downloads Lisa's
  own pinned copy (about 15 MB) — there is nothing for you to install. On
  macOS, install [Zellij](https://zellij.dev/) yourself: `brew install zellij`.

Claude Code is the default and only required agent client. Lisa can alternatively
drive [Codex](https://developers.openai.com/codex) — see
[Codex client](#codex-client-experimental) below.

After installing Lisa, run `lisa doctor` to verify everything is in place. `lisa
doctor` checks the dependencies for your *selected* client (Claude by default).

## Quick Start

Initialize your project:

```bash
cd your-project
lisa init
```

This creates the ticket directories and a `CLAUDE.md` tailored to your project.

In a folder that isn't already a repository, `lisa init` offers to keep
**project history** — undo for finished work, plus a record of what the agents
did. Accepting sets everything up itself; nothing to configure. In scripts and
agent-driven runs, pass the choice as a flag:

```bash
lisa init --with-history   # recommended: finished work is undoable
lisa init --no-history     # finished work is recorded in Lisa's journal but won't be undoable
```

A project already inside a repository is left exactly as found — no offer, no
changes.

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
| `scheduling.max_threads` | `2` | Maximum concurrent agent sessions |
| `agent.client` | `claude` | Which agent client the loop drives (`claude` or `codex`) |
| `guards.completion` | `auto` | How finished work is sealed: `auto` picks the strongest seal the project supports, `commit` requires history and fails hard without it, `journal` records hash-stamped journal entries only |
| `triage.enabled` | `true` | When a ticket parks needing a decision, let an agent read the blocker first and attach a plain suggestion with prepared steps |
| `triage.timeout_secs` | `120` | How long that triage pass may run before Lisa gives up on it (the park itself is never delayed) |

## Codex client (experimental)

By default Lisa drives Claude Code. It can alternatively drive
[Codex](https://developers.openai.com/codex), OpenAI's native agent CLI. Claude
and Codex are the only supported clients today; broader protocol support (ACP) is
future work, not available yet.

**A project that never opts in behaves exactly as before** — the default is
`claude`, and Claude Code's launch, prompt, and `lisa doctor` output are
unchanged.

### Selecting Codex

Persistently, in `.lisa.toml`:

```toml
[agent]
client = "codex"
```

Or per run, with a flag that overrides `.lisa.toml`:

```bash
lisa loop --client codex
```

Precedence is `--client` > `.lisa.toml [agent].client` > default (`claude`).

### Prerequisites

- The `codex` binary on `PATH`:

  ```bash
  npm i -g @openai/codex
  ```

- **Version pinning caveat.** Codex's CLI flags, hooks, and trust model can drift
  between releases. `lisa doctor` reports the installed `codex --version` so you
  can confirm what you're running.
- **Directory trust.** A native Codex session can block on an interactive
  directory-trust prompt. When Codex is selected, `lisa doctor` and `lisa loop`
  pre-seed `trust_level = "trusted"` for the project in `$CODEX_HOME/config.toml`
  (default `~/.codex/config.toml`), best-effort.

Run `lisa doctor` after selecting Codex (and after every `codex` upgrade) to
verify the binary, version, and trust seeding.

### What runs in the pane

A Codex ticket launches the official interactive Codex TUI with its initial RDSPI
prompt, just as the Claude path launches Claude Code. Lisa-generated hooks in
`.codex/hooks.json` translate `Stop`, `SessionStart[clear]`, and `PostToolUse`
into the same `.lisa/signals/` files the scheduler consumes. A reused pane stays
inside Codex: Lisa sends `/clear`, waits for the clear hook, then types the next
ticket prompt. Review follow-ups are typed into the live composer too.

In mixed-provider loops, Lisa prefers a pane already running the requested
client. If all released panes belong to the other provider, it safely recycles
one: `/exit` returns the pane to its shell, then Lisa launches the correct fresh
CLI after a short grace period. Running or human-blocked panes are never evicted.

Codex reads `AGENTS.md` for project context (Claude reads `CLAUDE.md`). `lisa
init` scaffolds both files plus both clients' hook configuration; `AGENTS.md`
points at `CLAUDE.md` as the single source of truth. Re-run `lisa init` in an
existing project before its first native Codex loop.

The lower-level `lisa agent-exec` / `codex exec --json` path remains available
for diagnostics and explicitly headless automation, but `lisa loop` no longer
uses its JSON renderer for Codex panes.

## How It Works

### Workflow

Every ticket passes through six phases in order:

1. **Research** — Map the relevant codebase. What exists, where, how it connects.
2. **Design** — Explore options, evaluate tradeoffs, choose an approach with rationale.
3. **Structure** — Define file-level changes, module boundaries, public interfaces.
4. **Plan** — Sequence implementation steps with testing strategy.
5. **Implement** — Execute the plan, commit meaningful ticket-owned units through Lisa's isolated transaction, track progress.
6. **Review** — Summarize changes, test coverage, and open concerns, then wait for Lisa to confirm completion.

Each phase produces a ~200-line artifact in `docs/active/work/{ticket-id}/`. These are review checkpoints — catching a bad design at 200 lines is cheaper than catching it at 2,000 lines of wrong code.

### Atomic completion

Agents never use the shared ordinary Git index as a handoff. During Implement,
each meaningful source unit is committed with `lisa commit-ticket` and exact
repository-relative `--include` paths; ordinary `git add`, broad `git add -A`,
and ordinary `git commit` are outside the generated workflow. Existing staged
entries owned by a human or another tool remain staged and cannot enter a ticket
commit.

After `review.md` is written, the agent stays on that ticket. Lisa prepares both
Done frontmatter fields and commits the ticket plus its work artifacts through
the same isolated transaction. The seat is released, provenance is published,
and dependents become eligible only after Lisa receives and verifies that commit
receipt.

If the completion transaction fails, Lisa fails closed — and bounded: a failure
only an operator can fix is retried a small fixed number of times, then the
ticket parks under **Waiting on you** with one plain sentence naming what to do
(the full technical detail stays in the journal). No completion failure can
churn silently, and none leaves a state you can't recover from the dashboard or
`lisa unblock`.

Projects without history don't lose the seal — they get a different one. Where
a repository exists, finished work is **commit-sealed** exactly as above. Where
none exists (and you declined the history offer), completion is
**journal-sealed**: gated on the same review verdict plus SHA-256 hashes of the
ticket and every work artifact, fail-closed and tamper-evident, just not
undoable. The tier is pinned once per run, recorded on every ledger row, and
never switches silently — `lisa doctor` and `lisa status` say which one you
have.

### Scheduling

Tickets declare dependencies via the `depends_on` field. Lisa computes a DAG, topologically sorts it, and schedules all tickets whose dependencies are satisfied. As tickets complete, newly unblocked tickets are scheduled automatically.

### Concurrency

Multiple Claude and Codex sessions work in parallel on the same branch. Lisa's
ticket commands serialize ref movement and build commits in isolated alternate
indexes, so the shared ordinary index is never a ticket mailbox. Sessions do not
coordinate commit timing, but they must declare exact owned paths. If two tickets
modify the same files, that is a missing dependency edge in the DAG; transaction
isolation is a safety boundary, not a substitute for correct dependencies.

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

Scaffold a project for Lisa: creates ticket directories, `CLAUDE.md`, `AGENTS.md` (a pointer to `CLAUDE.md` for the Codex client), RDSPI workflow, hooks, and `.lisa.toml`.

```bash
lisa init                 # Initialize current directory
lisa init --with-history  # Answer the project-history offer: yes (undoable finished work)
lisa init --no-history    # Answer it: no (journal record only)
lisa init --dry-run       # Preview what would be created
lisa init --path ../other-project
```

In a folder with no repository, init offers project history (see Quick Start).
Interactive runs can answer the prompt; scripts and agents must pass
`--with-history` or `--no-history`. Existing repositories are never modified.

Re-running `lisa init` is conservative. Lisa replaces a static workflow or hook
template only when its exact contents match a known Lisa version; customized,
unreadable, or otherwise unclassifiable files are preserved and shown as safety
skips. Structured TOML and JSON targets keep their format-aware merge behavior
and preserve unrelated project settings.

`.lisa/.gitignore` has a stricter append-only contract: init preserves every
existing line in place and adds only missing Lisa-required rules. Project rules
are never deleted, reordered, or rewritten.

Both dry runs and real runs label creates, updates, no-ops, and safety skips. A
successful real run also prints `Files changed`, the exact set of files whose
contents it created or updated. Inspect those reported files before your next
commit.

### `lisa validate`

Check that tickets parse correctly, the DAG has no cycles or missing dependencies, and the project structure is sound.

```bash
lisa validate
lisa validate --check-tools   # Also verify zellij and claude are on PATH
```

### `lisa loop`

Launch a Zellij session with the Lisa plugin. Schedules and runs agent sessions based on the ticket DAG.

```bash
lisa loop
lisa loop --max-threads 4        # Override concurrent session limit
lisa loop --client codex         # Drive Codex instead of Claude (overrides .lisa.toml)
lisa loop --dry-run              # Show what would launch without starting
```

### `lisa status`

Inspect the DAG offline: tickets, dependencies, execution waves, and scheduling
readiness. Anything that needs your decision appears first under **Waiting on
you**, each with one plain sentence you can act on (or paste to your coding
agent). Reviewer observations that didn't stop the work appear under **Notes
for you**. The completion-seal line says how finished work is being recorded.

```bash
lisa status
```

### `lisa doctor`

Check the selected agent (`claude` or `codex`, per `.lisa.toml`) and the Zellij
runtime Lisa will use, and report which completion seal the project resolves —
including the exact fix when commit sealing is configured but unavailable. When
Codex is selected, this also prepares directory trust for unattended
`codex exec`.

```bash
lisa doctor
```

### `lisa unblock`

Re-open a parked ticket after you've handled its ask. Lisa runs the blocker's
own check first and tells you if the fix isn't in place yet.

```bash
lisa unblock T-001-01
```

### `lisa notes`

Read and clear the **Notes for you** queue — reviewer observations filed
against finished work that kept moving.

```bash
lisa notes              # List unread notes
lisa notes ack T-001-01 # Mark a ticket's note as read
```

### `lisa proposal`

When a parked ticket carries a first-responder suggestion, apply its prepared
steps or dismiss it. Both are recorded. Applying runs exactly the steps shown
in `lisa status` — read them there first.

```bash
lisa proposal apply T-001-01
lisa proposal dismiss T-001-01
```

### `lisa setup-guide`

Print LLM-friendly setup instructions for the current project. Useful for seeding a Claude Code session with project context.

```bash
lisa setup-guide
```

## Develop Lisa

Changing Lisa itself requires a source build. Follow
[CONTRIBUTING.md](CONTRIBUTING.md) for setup, test commands, and how to submit
changes.

## License

MIT
