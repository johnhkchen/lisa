# Research — T-036-01-02: plain, verb-forward command help

Descriptive map of the surface this ticket touches. No solutions here.

## What the ticket asks

Rewrite each operator command's `///`-derived help into plain, verb-forward
kitchen-table English, and reword the internal commands' help too, so that
`lisa <cmd> --help` states *the action of running the command* rather than a
category term. Acceptance hinges on: (a) each operator subcommand's help reads
as a plain action; (b) `init|validate|status|doctor|loop --help` each describe
what running the command does; (c) no operator-facing help string contains the
banned jargon terms.

## Where the help text lives

All of it is in a single file: `crates/lisa-cli/src/main.rs`. clap 4.5's derive
API turns two things into help:

1. `#[command(about = "…")]` on `struct Cli` → the top-level about-line. **Owned
   by T-036-01-01, already rewritten** to "Runs your coding agents through a
   project's tickets." Out of scope here; do not touch.
2. The `///` doc comment on each `enum Commands` variant → that subcommand's
   help. The **first line** becomes the short help shown in the parent
   `lisa --help` command list; the **whole comment** becomes the long help shown
   by `lisa <cmd> --help`. This ticket owns these `///` comments.

No other file renders help. There is no man page, README help block, or shell
completion in scope (the story's honest boundary names those as deferred).

## The twelve variants and their current help (verbatim)

Operator commands (foregrounded, `display_order` 0–4 from T-036-01-01):

| Variant  | Current `///` first line |
|----------|--------------------------|
| Init     | `Initialize a project for lisa-loop completion` |
| Validate | `Validate ticket DAG and project setup` |
| Status   | `Show DAG status: tickets, dependencies, execution waves, scheduling readiness` |
| Doctor   | `Check that all runtime dependencies are installed` |
| Loop     | `Launch zellij with the Lisa plugin for DAG-driven task scheduling` |

Hook/plumbing commands (trailing, `display_order` 20–23):

| Variant        | Current `///` (summary) |
|----------------|-------------------------|
| AgentExec      | `Run Codex under Lisa's legacy JSON signal/rendering wrapper.` + 3-sentence body about LISA_PANE_ID/LISA_TICKET_ID, `codex exec --json`, signal files. |
| CaptureUsage   | `Capture Claude session token usage from a Stop-hook payload on stdin, writing .lisa/claude/<ticket>.usage.json for the provenance ledger.` |
| CommitTicket   | `Commit ticket-owned paths without using the repository's ordinary index.` |
| CompleteTicket | `Mark a ticket done and commit its loop-owned files atomically.` |

Hidden commands (`hide = true`, not in the primary listing but reachable):

| Variant     | Current `///` first line |
|-------------|--------------------------|
| SetupGuide  | `Output LLM-friendly setup instructions for this project` |
| HooksGuide  | `Output the hooks setup guide for agents configuring Claude Code hooks` |
| Version     | `Print version information` |

## The banned jargon terms (constraint source)

Consolidated from the epic E-036, story S-036-01, and user-global CLAUDE.md
brand voice:

- **`DAG` / `DAG-driven`** — appears in Validate, Status, Loop today. The
  epic explicitly calls this out as the rot to remove.
- **`orchestration` / `orchestrate`** — brand-banned; not currently present but
  must not be introduced.
- **`concurrent task scheduling` / "scheduling" as a category** — the epic's
  named category frame to retire. Status leans on it ("scheduling readiness",
  "execution waves").
- **`leverage`, `solutions`, "case study", "build log", "research release",
  "deployment architecture"** — brand-banned generally; none present, keep out.

Scope nuance: the AC bans these terms specifically in **operator-facing** help.
The four hook commands and three hidden commands are machinery-facing, not
operator-facing, so the hard ban is on the five operator strings; the internal
commands are *reworded for plainness* but may retain precise technical nouns
(e.g. `Stop-hook`, `codex exec`) that name real machinery an agent/operator
debugging hooks needs.

## What each operator command actually does (so the copy can be truthful)

Grounded in the `main()` dispatch and the module each arm calls:

- **Init** (`init::run_init`) — sets up a project to run under Lisa: writes/updates
  `.lisa.toml`, `CLAUDE.md`, the embedded RDSPI workflow, hook wiring. Has
  `--dry-run` and `--path`.
- **Validate** (`init::run_validate`) — checks the ticket set and project setup
  are well-formed (dependency edges resolve, no cycles, files parse); `--check-tools`
  additionally checks `zellij`/`claude` are on PATH.
- **Status** (`status::run_status`) — prints which tickets are ready, which are
  waiting on dependencies, and the readiness picture, without launching anything.
- **Doctor** (`doctor::run_doctor`) — checks the external tools Lisa needs are
  installed and reachable.
- **Loop** (`loop_cmd::run_loop`) — the command that starts a run: launches
  zellij with the Lisa plugin, which works ready tickets concurrently where they
  don't collide. Has `--max-threads`, `--client`, `--dry-run`.

## Constraints & assumptions carried into Design

- **Diff must stay comment-only.** Editing only `///` lines keeps every match
  arm, flag, and field untouched — the story's scope boundary. No variant
  reordering (that was T-036-01-01's job and is settled).
- **First line does double duty.** Because clap uses the first `///` line for
  the parent-list short help *and* as the opening of `<cmd> --help`, the first
  line must stand alone as a plain action sentence.
- **T-036-01-03 will lock this.** A sibling ticket adds a native test asserting
  no banned jargon in operator help and that the command set is unchanged. This
  ticket should leave the strings in a state that test can pin cleanly.
- **Assumption:** rewording the *inner* field/arg `///` comments (e.g. `--path`
  help) is out of scope — the AC and story speak to the subcommand-level help
  (the variant `///`), not per-flag descriptions. Leave flag help as-is.
- **Assumption:** the hidden commands' help is still worth de-jargoning for the
  power user who runs `lisa version`/`setup-guide --help`, but they are not
  gating AC items; treat as best-effort plainness, not a jargon-ban target.
