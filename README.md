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

### macOS

One tap, three formulae. The formula name is the channel, and installing one is
the whole choice:

| formula | takes |
| --- | --- |
| `lisa` | the newest release that is not a release candidate |
| `lisa-nightly` | the newest release that has soaked for a day |
| `lisa-canary` | the newest release, candidate or not |

Trust the tap once, then install the one you want:

```bash
brew trust johnhkchen/lisa
brew install johnhkchen/lisa/lisa

lisa doctor
```

`brew trust` is Homebrew asking whether you mean to run code from a tap that is
not its own. It is a one-time answer per machine. Lisa needs it because each
formula names the other two, which is what keeps two channels off one box.
Homebrew versions without a `brew trust` command do not need the line.

After that, plain `brew upgrade` keeps the machine on whatever its formula says.
No Lisa command is in that path.

**The three cannot be installed together.** They all provide the same `lisa`, so
each conflicts with the other two and `brew` says so rather than leaving PATH
order to decide which one runs. Changing channel is an uninstall and an install:

```bash
brew uninstall lisa
brew install johnhkchen/lisa/lisa-nightly
```

**Going back to an older version is the one thing Homebrew cannot do.** `brew
switch` is gone, and a formula carries one version, so there is no `lisa=0.4.4`
here the way there is on apt. The way back on a Mac is Lisa's own installer,
naming the release:

```bash
lisa upgrade --tag v0.4.4
```

**If this machine already has `lisa` from this tap**, two things change. Run
`brew trust johnhkchen/lisa` once — without it, the next `brew upgrade` stops
and says the tap is untrusted. And `lisa` stops taking release candidates: it
used to carry whatever shipped last, candidate included, so the machine will now
sit still until the next real release instead of moving every few days. Nothing
is uninstalled and no version is taken away. To keep following candidates, swap
to `lisa-canary` with the two commands above.

Until releases are promoted into nightly on their own, `lisa-nightly` carries
the same release as `lisa`.

### Debian and Ubuntu

One repository, one signing key, three channels. The channel is the word in the
sources line, and that word is the whole choice:

| channel | takes |
| --- | --- |
| `stable` | the newest release that is not a release candidate |
| `nightly` | the newest release that has soaked for a day |
| `canary` | the newest release, candidate or not |

Install the archive key in its own keyring, point at the channel you want, then
install the CLI and its pinned Zellij runtime:

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

channel=stable   # or nightly, or canary
echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/lisa-archive-keyring.gpg] https://johnhkchen.github.io/lisa $channel main" \
  | sudo tee /etc/apt/sources.list.d/lisa.list >/dev/null
sudo apt-get update
sudo apt-get install -y lisa lisa-runtime-zellij

lisa doctor
```

Normal `apt-get update` and `apt-get upgrade` keep both packages on whatever the
channel says. `lisa-runtime-zellij` provides Lisa's pinned runtime at
`/usr/libexec/lisa/zellij`, so apt installs do not need a first-run Zellij
download; it ships in all three channels next to the `lisa` it was built with.

**Changing channel** is that one word and an update. All three suites are signed
by the same key, so nothing new has to be trusted:

```bash
sudo sed -i 's/ stable main/ nightly main/' /etc/apt/sources.list.d/lisa.list
sudo apt-get update
sudo apt-get install --only-upgrade lisa lisa-runtime-zellij
```

**Going back down** a channel — canary or nightly to stable — asks for an older
version than the one on the box. apt calls that a downgrade and will not do it
until you say so:

```bash
apt-cache madison lisa                    # every version this channel offers
sudo apt-get install --allow-downgrades \
  lisa=0.4.4-1 lisa-runtime-zellij=0.4.4-1
```

Old versions stay in the pool, so that same command is also how you go back to a
release that worked. `apt-cache madison lisa` is where the exact version strings
come from.

This is a vendor repository, not the Debian archive: bundling the private Zellij
runtime is deliberate. It is hosted on GitHub Pages, whose documented limits
include a 1 GB published site and a soft 100 GB monthly bandwidth limit. Archive
operators can find signing-key custody and rotation details in
[packaging/apt/README.md](packaging/apt/README.md).

Want to change Lisa itself? Read [Develop Lisa](#develop-lisa) and follow
[CONTRIBUTING.md](CONTRIBUTING.md) for the source build.

## Keep Lisa current

`lisa upgrade` moves this machine to the release it asked for:

```bash
lisa upgrade                      # move to what this machine's channel says
lisa upgrade --channel nightly    # pick a channel and move, in one command
lisa upgrade --tag v0.4.4         # go back to an exact release
lisa upgrade --dry-run            # say what would happen and change nothing
```

Every green `main` is tagged, so there is one train of releases and three ways to
subscribe to it:

| channel | takes | who it is for |
| --- | --- | --- |
| `canary` | the newest release, prerelease or not | a machine you are developing on |
| `nightly` | the newest release, prerelease or not, once it has soaked | a machine that runs real work with nothing at stake |
| `stable` | the newest release that is not a prerelease | everything else |

A machine that has never picked is treated as `stable`, and `lisa upgrade` says
so rather than pretending someone chose.

**Soak** is why `nightly` is not just `canary` a day later. A release becomes
eligible for `nightly` once it is **24 hours old**, and only the newest release
is ever a candidate: anything below it has been superseded, whether or not the
one above it has soaked yet. So a release candidate that a hotfix replaces twenty
minutes later is never installed anywhere. If the newest release has not aged out
yet, `lisa upgrade` says how much longer and leaves the machine where it is.

The channel is a property of the machine, not of a project, so it lives in a
per-user file:

- Linux: `~/.config/lisa/config.toml`
- macOS: `~/Library/Application Support/io.johnhkchen.lisa/config.toml`

```toml
channel = "nightly"
# Hours a release must age before the nightly channel will take it.
soak_hours = 24
```

`lisa upgrade --channel <name>` writes that file for you; `soak_hours` is there
to edit when 24 hours is the wrong wait. (A project's `.lisa.toml` also has a
`version` field. That records the Lisa that set the *project* up and has nothing
to do with channels.)

**To find out where a machine stands, run `lisa doctor`.** It reports Lisa itself
as one row — the channel this box is on, the version installed, and the version
that channel resolves to right now — and when the two differ it names the command
that settles the gap. A machine that is level says so once and stays quiet, a
machine that has never picked a channel is reported as *unset* rather than
silently counted as stable, and a machine that cannot reach the release list says
that instead of claiming to be current. Being behind is something to know, not a
refusal: `doctor` still exits the way it always did.

```bash
lisa doctor          # read it here
lisa doctor --json   # collect it from every box: data.lisa.state is level, behind, ahead, waiting or unresolved
```

Two things `upgrade` deliberately does not do. With no network it stops, says it
could not read the release list, and leaves the installed Lisa in place — it
never guesses. And on a machine where Homebrew or apt owns `lisa`, it refuses to
write over their file and prints the command that does move them
(`brew upgrade lisa`, `apt-get install --only-upgrade lisa`). On those machines
the channel is the package that is installed — the formula name or the suite in
the sources line — and `brew upgrade` or `apt-get upgrade` is the whole of
keeping it current. See [macOS](#macos) and
[Debian and Ubuntu](#debian-and-ubuntu). The per-user channel file below is for
machines with no package to ask: the one-command install above, and source
builds.

**An upgrade never lands under a run.** If any Zellij session is up on the
machine, `lisa upgrade` stops rather than swap the binary a live loop is calling,
names the sessions holding it, and offers `--anyway` for when you know better.

### A machine that upgrades itself

A machine that waits to be upgraded by hand drifts. On a box that runs background
work — real work, with nothing in front of anyone — put the upgrade on a schedule
and let it meet each release before you do:

```bash
lisa nightly install --project ~/work/some-board   # channel nightly + a nightly job
lisa nightly status                                # where does this box stand?
lisa nightly status --json                         # the same answer for a script
lisa nightly uninstall                             # off the schedule, channel kept
```

On macOS that writes a launchd job at
`~/Library/LaunchAgents/io.johnhkchen.lisa.nightly.plist`, which runs one cycle at
04:30 and again at 05:30 and 06:30 in case the machine was busy. A cycle skips
entirely while the machine is working, moves only when the nightly channel has a
release that has soaked, and then checks the new release against a board you name
— `lisa doctor` under the version that just landed, which is what catches a
Homebrew Zellij that has drifted out of the supported range. Anything that fails
is written down, exits non-zero, goes to the system log and a notification, and is
handed to `alert_command` if you set one. Every alarm carries the way back:

```bash
lisa upgrade --tag v0.4.4    # one command, back to a release that worked
```

Each cycle is recorded next to the channel — `nightly/health.json` for the last
one and `nightly/history.jsonl` for all of them — and `lisa nightly status` reads
it: it fails when the last cycle failed, when the record has gone stale because
nothing is running the job, and when the box has been too busy to move for three
nights. Silence is a finding, not a pass. The full arrangement, and what to check
after a release lands, is in
[docs/knowledge/mac-mini-nightly.md](docs/knowledge/mac-mini-nightly.md).

## What It Does

When you have a set of interdependent tasks — a feature broken into tickets, a refactor with sequencing constraints, a sprint with parallel workstreams — Lisa schedules and runs them concurrently as Claude Code sessions. You define the work as markdown tickets with dependency metadata. Lisa figures out what can run in parallel, what has to wait, and launches sessions accordingly.

Lisa runs as a [Zellij](https://zellij.dev/) plugin. It reads your tickets, computes a dependency graph, and spawns Claude Code sessions for every ticket whose dependencies are satisfied. A dashboard shows what's running, what's queued, and what's done. When a ticket finishes, Lisa checks what it unblocked and schedules the next wave.

Each ticket goes `ready → implement → review → done`. The agent does the work and commits it as it goes, then writes one review document and a machine-readable verdict at the end. If a session dies mid-work, what it already committed is on the branch and the next session carries on from there.

Lisa keeps the trail reviewable: an append-only attempt ledger records each run,
the completion journal seals every finished ticket — to a commit where the
project keeps history, or to tamper-evident content hashes where it doesn't —
and each ticket keeps its work documents. Every ticket runs in its own agent
session, and what each one cost is recorded too: token usage joins the ledger
after the session ends, and `lisa status` shows it per ticket — with an honest
"not yet joined" line instead of a made-up number when a capture hasn't landed.
`lisa doctor` and `lisa status` name which seal is in effect in plain words.

## Prerequisites

- [Claude Code](https://docs.anthropic.com/en/docs/claude-code) — the default AI coding assistant that does the work
- On Linux, that's it: Zellij comes with Lisa. The first run downloads Lisa's
  own pinned copy (about 15 MB) — there is nothing for you to install. On
  macOS, install [Zellij](https://zellij.dev/) yourself: `brew install zellij`.

Claude Code is the default and only required agent client. Lisa can alternatively
drive [Codex](https://developers.openai.com/codex) — see
[Codex client](#codex-client) below.

After installing Lisa, run `lisa doctor` to verify everything is in place. `lisa
doctor` checks the dependencies for your *selected* client (Claude by default).

## Quick Start

Initialize your project:

```bash
cd your-project
lisa init
```

This creates the ticket directories, the workflow document Lisa hands your
agents, the hooks, and `.lisa.toml`. Your own agent context file is yours to
write.

In a folder that isn't already a repository, `lisa init` keeps **project
history** when the machine supports it — undo for finished work, plus a record
of what the agents did. If it can't, Lisa uses its journal instead and says so.
Interactive runs still offer the choice. Use the flags only when you want to
override the automatic decision:

```bash
lisa init --with-history   # require history: finished work is undoable
lisa init --no-history     # force the journal: finished work won't be undoable
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
| `version` | the installed Lisa version | Tracks the Lisa version used to set up this project. |
| `dirs.tickets` | `docs/active/tickets` | Chooses where Lisa reads ticket files. |
| `dirs.stories` | `docs/active/stories` | Chooses where Lisa reads story files. |
| `dirs.work` | `docs/active/work` | Chooses where Lisa keeps work records. |
| `runtime.zellij` | `managed` | Chooses how Lisa starts Zellij. |
| `agent.client` | `claude` | Chooses which coding agent Lisa drives. Omit it to detect agents on PATH; claude is the default when both are installed. |
| `agent.model` | `opus` | Chooses which model that agent runs. Omit it to use whatever the agent runs by default. |
| `guards.completion` | `auto` | Controls how finished work is sealed. auto picks the strongest your project supports. |
| `triage.enabled` | `true` | Lets Lisa inspect work that needs you before asking for help. |
| `triage.timeout_secs` | `120` | Limits how long Lisa can inspect work that needs you. |
| `scheduling.max_threads` | `2` | Limits how many coding agents can work at once. |
| `scheduling.review_timeout_secs` | `600` | Limits how long Lisa waits for review to finish. |
| `scheduling.session_timeout_secs` | `3600` | Limits how long one coding-agent session can run. |
| `scheduling.wind_down_secs` | `300` | Sets aside time for an agent to wrap up before its session ends. |
| `scheduling.assignment_ack_timeout_secs` | `30` | Limits how long Lisa waits for an agent to accept assigned work. |
| `scheduling.phase_timeouts` | `{}` | Limits how long each kind of work can run. |
| `scheduling.provider_caps` | `{}` | Limits how many agents of each kind can work at once. |

## Codex client

By default Lisa drives Claude Code. It can alternatively drive
[Codex](https://developers.openai.com/codex), OpenAI's native agent CLI —
full boards run end-to-end on either client, through the same scheduling,
sealing, and usage accounting. Claude and Codex are the only supported clients
today; broader protocol support (ACP) is future work, not available yet.

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

A Codex ticket launches the official interactive Codex TUI with its initial ticket
prompt, just as the Claude path launches Claude Code. Lisa-generated hooks in
`.codex/hooks.json` translate `Stop`, `SessionStart[clear]`, and `PostToolUse`
into the same `.lisa/signals/` files the scheduler consumes. Every ticket gets a
fresh session: when a ticket finishes, Lisa exits the TUI, waits a short grace
period for the shell to return, and launches the next ticket's CLI fresh — so
each session carries exactly one ticket's identity from start to end. Review
follow-ups are typed into the live composer of the ticket's own session.

In mixed-provider loops, Lisa prefers a pane already running the requested
client, and the same exit-then-fresh boundary safely hands a pane from one
provider to the other. Running or human-blocked panes are never evicted.

Codex reads `AGENTS.md` for project context and Claude reads `CLAUDE.md`. Both
files are yours to write: `lisa init` scaffolds both clients' hook configuration
and leaves your context files alone. Re-run `lisa init` in an existing project
before its first native Codex loop.

The lower-level `lisa agent-exec` / `codex exec --json` path remains available
for diagnostics and explicitly headless automation, but `lisa loop` no longer
uses its JSON renderer for Codex panes.

## How It Works

### Workflow

A ticket has four states, and an agent works two of them:

1. **Implement** — Do the work. Commit meaningful ticket-owned units through Lisa's isolated transaction as they finish.
2. **Review** — Summarize what changed, what it covers, and what is still open, then wait for Lisa to confirm completion.

That produces two files in `docs/active/work/{ticket-id}/`: `review.md` for a human, and `review-disposition.json` — pass or block, with a reason and an optional read-only check — for Lisa. Nothing is written before the work exists; the diff and the commits are the record of what happened.

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
`lisa unblock`. And when the work itself did get committed but the finishing
record didn't, Lisa stops retrying after a couple of tries and tells you to run
`lisa already-done` — which checks history for that work before it settles
anything.

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
    lisa-workflow.md    How a ticket moves (injected into agent context)
```

## CLI Reference

### `lisa init`

Scaffold a project for Lisa: creates ticket directories, `docs/knowledge/lisa-workflow.md`, hooks, and `.lisa.toml`.

On a project an older Lisa set up, it also clears out what that Lisa left behind: the workflow document under its old name, the `CLAUDE.md` and `AGENTS.md` Lisa used to write for you, and a `.lisa.toml` setting Lisa stopped reading. It removes a file only when the bytes are still exactly as Lisa wrote them. Change one line and the file is yours — kept as it is, and named in the list so you can see Lisa looked and left it. Your board is never rewritten. Run `lisa init --dry-run` first to read the whole list before anything happens.

What init leaves behind on purpose is [`lisa clean`](#lisa-clean)'s to offer, and yours to say yes to.

```bash
lisa init                 # Initialize with the strongest history mode available
lisa init --with-history  # Require project history (undoable finished work)
lisa init --no-history    # Override with a journal record only
lisa init --dry-run       # Preview what would be created and what would be removed
lisa init --path ../other-project
```

In a folder with no repository, interactive init offers project history (see
Quick Start). Without an interactive answer, init keeps history when the machine
supports it and otherwise uses Lisa's journal. The history flags override that
decision. Existing repositories are never modified.

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
lisa loop --headless             # Run on a host with no terminal at all
```

`--headless` is for a machine that cannot show you a window — a container, a
server you reach with `ssh -T`, a GitHub Codespace. Zellij will not start
without a terminal, so Lisa opens one nobody is watching; the agents still get
their panes, and the dashboard is replaced by `lisa status` and `lisa status
--json` from wherever you are. Everywhere with a terminal keeps the ordinary
run. See [docs/knowledge/headless-board.md](docs/knowledge/headless-board.md).

### `lisa status`

Inspect the DAG offline: tickets, dependencies, execution waves, and scheduling
readiness. Anything that needs your decision appears first under **Waiting on
you**, each with one plain sentence you can act on (or paste to your coding
agent). Reviewer observations that didn't stop the work appear under **Notes
for you**. **Token usage** lists what each completed ticket cost once its
session's capture joins the ledger — never a fabricated zero for a missing
capture. The completion-seal line says how finished work is being recorded.

If a run stopped without shutting down, the seats it was working still look
busy. **Seats held by a run that is gone** names them, says how Lisa knows, and
points at `lisa release-seats`. If more than one run is holding this board at
once, that is named too — with each one's stop command, because a board with two
schedulers on it reads exactly like a healthy one otherwise.

```bash
lisa status
```

### `lisa doctor`

Check the selected agent (`claude` or `codex`, per `.lisa.toml`) and the Zellij
runtime Lisa will use, and report which completion seal the project resolves —
including the exact fix when commit sealing is configured but unavailable. When
Codex is selected, this also prepares directory trust for unattended
`codex exec`.

The first row is Lisa itself: which channel this machine is on, what it has
installed, and what that channel resolves to. See
[Keep Lisa current](#keep-lisa-current) for what the channels take.

```bash
lisa doctor
lisa doctor --json   # the same answer for a script; see `lisa json-guide`
```

`lisa doctor` also says how this project differs from the Lisa you have
installed, and names the exact command that settles each difference:
`lisa init` for what Lisa can bring forward on its own, `lisa clean` for what
only removal will fix, and the one-line edit for anything in a file that is
yours.

### `lisa clean`

Remove what an older Lisa left behind. `lisa doctor` reports, `lisa init` brings
forward what it can prove is safe, and `lisa clean` is where you say *yes, remove
it* to the rest.

**A bare run removes nothing.** It prints the list — every file, and why Lisa
believes it can go — and stops. Read it, then run it again with `--remove`.

```bash
lisa clean            # print the list, change nothing
lisa clean --remove   # carry the list out
```

Three kinds of thing are on the list:

- Documents an older Lisa left behind that `lisa init` looked at and declined —
  a workflow document you edited, a `CLAUDE.md` Lisa generated that something
  still points at.
- The old workflow's notes — `research.md`, `design.md`, `structure.md`,
  `plan.md`, `progress.md` — in the work folder of a ticket your board records as
  **done**. Nothing else in that folder is touched: your own notes stay, and so
  does `review.md`.
- Lisa's own working folders under `.lisa/attempts/`, for tickets that are done.

And what is never on it, under any flag: your board (`docs/active/tickets/`,
`docs/active/stories/`), your `.lisa.toml`, anything Lisa did not write, the work
of any ticket that is not done, and anything a symlink points to outside your
project. Lisa removes files; it never rewrites what is inside one.

### `lisa release-seats`

Free the seats a run left behind when it stopped without shutting down.

When a run dies — the machine swaps, the terminal is killed, the laptop sleeps
and never wakes — it can't put its own seats back. The tickets it was working
keep looking busy, `lisa status` keeps counting them as in progress, and
anything reading that board keeps believing it. Running `lisa loop` again does
clear it, by reassigning the tickets, which is a strange thing to have to do
when the board is telling you a run is already going.

**A bare run frees nothing.** It prints which seats it believes are free, what
each one was working, and the evidence — then stops. Read it, then run it again
with `--release`.

```bash
lisa release-seats            # print the list and the evidence, change nothing
lisa release-seats --release  # free the listed seats
```

Lisa frees a seat only when two things agree: no scheduler has said it was
running for longer than this project allows, **and** nothing has stirred in
Lisa's signal folder for the same stretch. While a run is there — even one
detached in the background, or sitting quietly on a question — the command says
so and frees nothing. A seat Lisa isn't sure about stays held, because handing
out a seat somebody is working is the mistake that costs you.

The refusal tells you which of the two it is. *A run is working these seats*
means your panes are writing and something is genuinely being done. *A scheduler
is holding these seats* means a run exists but nothing has moved for a while —
so the message names that run and the command that ends it, rather than leaving
you with a sentence you can't act on.

### `lisa reset-ticket`

Put a ticket back on the board when nothing is working on it.

Lisa moves a ticket to **implement** the moment it hands it to an agent. If that
agent never really starts — its pane was still held by the agent before it, say —
the ticket sits there claiming to be under way with nobody on it, and the field
fix used to be opening the file and editing `phase:` by hand: the one line every
assignment tells agents never to touch.

**A bare run changes nothing.** It prints which tickets it would move, how many
attempts each has spent, and how many of those never started a session at all.

```bash
lisa reset-ticket T-062-01-03           # print the plan, change nothing
lisa reset-ticket T-062-01-03 --apply   # put it back on the board
```

A reset moves `phase` back to `ready` and `status` back to `open`, and touches
nothing else — committed work stays, attempt history stays, and a ticket your
board records as **done** is never a candidate. While a run is going, Lisa says
so and changes nothing: press `r` in the Lisa pane instead, where the scheduler
can release the seat in the same breath. If the run holding the board is one you
can't see, the refusal names it and `lisa schedulers` ends it.

### `lisa heal-panes`

Ask a running loop to put back a coding pane it lost.

`lisa loop` lays out twice as many coding panes as it runs tickets, so a pane
finishing up never blocks a new ticket from starting. Those panes are made once,
at launch. A pane that dies afterwards — its shell exited, its terminal crashed,
Lisa closed it on an agent that went silent — used to be gone for the rest of the
session, and the run carried on at less concurrency than it was asked for with
every screen still reading healthy. On one board that was four panes down to two.

The loop watches for this on its own now: it counts its coding panes whenever
Zellij says the panes changed, puts back what is missing, and says so in its own
title (`myproject · 3/4 panes`). This command is the door for whoever notices
first — a monitoring script, a person reading the tab.

```bash
lisa heal-panes            # ask, and read the answer
lisa heal-panes --json     # the same answer for another program
```

It creates nothing. It leaves the ask in the project and the loop decides: the
plugin runs inside the Zellij server and is the only thing that can put a pane
back where the layout wanted it. You get one of three answers — **healed**,
**already fine**, or a **refusal** that says what to do instead. If nothing
answers at all, that is reported as nothing answering, which means something
different: no loop is running here, or its dashboard has stopped ticking.

Regeneration is bounded. Three panes in ten minutes and the loop stops asking,
says so once, and carries on with the panes it has — a pane that dies the instant
it is made must not become a loop. Restarting the run is the way back.

### `lisa schedulers`

Show every run holding this board, and stop one that outlived its pane.

Closing a loop's pane does not stop the run. `lisa loop` starts a Zellij
*client*; the part that hands out tickets lives in the Zellij *server*, and the
server keeps going. Close the pane, close the window, kill the `lisa loop`
process — the run carries on holding seats and reading your panes' signals, with
nothing on screen to say so. Start another loop and you have two of them, each
seeing about half of what your agents write and quietly eating the other half.

```bash
lisa schedulers                              # who is running this board
lisa schedulers --stop fascinating-drum      # end that one
```

Each line names the run, when it started, when it last checked in, its Zellij
server, and the exact command that stops it. `--stop` runs that command for you.
It ends the whole session, agent panes included, so you always name which one —
and Lisa refuses the session your own terminal is sitting in, because a command
that closes the window it is printing to can't tell you what it did.

Starting a second run on a board that already has one is refused, with the first
one named. That is the only thing standing between a quiet afternoon and two
schedulers splitting your signals between them.

### `lisa unblock`

Re-open a parked ticket after you've handled its ask. Lisa runs the blocker's
own check first and tells you if the fix isn't in place yet.

```bash
lisa unblock T-001-01
```

When the check declines, Lisa shows its work: the command as recorded, the
folder it ran in, what it exited with, and what it printed — labelled as the
check's words, not Lisa's finding. A check that stopped before it could look
says so, instead of reading as a verdict on what you did.

If you've done the ask and checked it yourself, you're never stuck behind a
check that won't agree:

```bash
lisa unblock T-001-01 --override-check
```

That reopens the ticket and writes down that you overrode the check, so a forced
unblock stays tellable apart from one that passed on its own.

### `lisa already-done`

Finish a ticket whose work is already saved in your project's history. Now and
then Lisa commits a ticket's work and then can't record the finishing touch —
the ticket sits waiting while the work itself is safely in history. This settles
it: Lisa looks for that work, and if it's really there, marks the ticket done
and writes the record.

If Lisa can't find the work in history, nothing changes and it says so. Your
word isn't enough — the commit has to be there.

```bash
lisa already-done T-001-01
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

### `lisa upgrade`

Move this machine to the release its channel names, or to an exact one. The
channels and the soak window are described in
[Keep Lisa current](#keep-lisa-current).

```bash
lisa upgrade                    # what this machine's channel says
lisa upgrade --channel stable   # pick a channel and move, in one command
lisa upgrade --tag v0.4.4       # go back to an exact release
lisa upgrade --dry-run          # say what would happen and change nothing
```

Every run names the version it is moving from and the one it is moving to before
it moves. It stops instead of guessing when it cannot read the release list, and
it leaves a Homebrew- or apt-installed `lisa` to its package manager.

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
