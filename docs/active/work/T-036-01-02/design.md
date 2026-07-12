# Design — T-036-01-02: plain, verb-forward command help

The ticket is pure copy: decide the exact `///` string for each subcommand. The
only real decisions are (1) the voice/shape each line takes and (2) how far to
rewrite the machinery commands. Everything grounds in Research.

## Guiding rule for every line

From the user-global brand voice: **labels orient by what you'd DO with it**,
verb-forward, kitchen-table plain. So each operator help opens with an imperative
verb naming the action of running the command, states the plain thing it does,
and drops all category nouns (`DAG`, `scheduling`, `orchestration`). The first
line must stand alone (Research: it is reused as the parent-list short help).

## Decision 1 — the five operator strings

For each I list the options considered and the choice.

### Init — currently "Initialize a project for lisa-loop completion"

- A. **"Set up a project to run with Lisa."** Plain, verb-forward, says the
  outcome (the project becomes runnable). Drops "lisa-loop completion" (opaque).
- B. "Initialize Lisa in a project." "Initialize" is faintly technical and
  reads back the command name; weaker than "Set up".
- C. "Create Lisa's config and workflow files in a project." Accurate but leads
  with artifacts, not the action the operator wants.

**Choice: A.** "Set up a project to run with Lisa." An operator reads it as
"this is the one I run first." Truthful to `init::run_init` (writes config +
workflow + hooks). No jargon.

### Validate — currently "Validate ticket DAG and project setup"

- A. **"Check your tickets and project setup for problems before a run."** Verb
  "Check", plain "problems", states *when* you'd use it. Kills "DAG".
- B. "Validate the ticket graph and project setup." Still leans on "Validate"
  (echoes name) and "graph" is DAG by another word.
- C. "Look for broken ticket dependencies and setup mistakes." Good, but
  "dependencies" is mild jargon and it under-sells the setup check.

**Choice: A.** Names the action (check), the object (tickets + setup), and the
payoff (catch problems before a run). "before a run" quietly points at `loop`.

### Status — currently "Show DAG status: tickets, dependencies, execution waves, scheduling readiness"

- A. **"Show which tickets are ready to run and which are waiting, and why."**
  Verb "Show"; replaces "DAG status/execution waves/scheduling readiness" with
  the plain question an operator actually asks. "and why" covers the
  dependency-blocked reason without the word "dependency".
- B. "Show what Lisa would run next and what's blocking the rest." Also good;
  "blocking" is slightly more jargon-y than "waiting".
- C. "List every ticket and its state." True but flattens the ready-vs-waiting
  distinction that is the whole value of the command.

**Choice: A.** It is the heaviest-jargon line today (three banned frames in one
string) and A removes all three while staying honest to `status::run_status`.

### Doctor — currently "Check that all runtime dependencies are installed"

- A. **"Check that the tools Lisa needs are installed."** "runtime dependencies"
  → "the tools Lisa needs" — kitchen-table, same meaning.
- B. "Check your setup can run Lisa." Vague; overlaps Validate's framing.
- C. "Check that zellij and claude are installed." Too specific — names today's
  tools and would rot as the tool set changes.

**Choice: A.** Minimal, plain, verb-forward, and it distinguishes cleanly from
Validate (Doctor = external tools; Validate = tickets/setup).

### Loop — currently "Launch zellij with the Lisa plugin for DAG-driven task scheduling"

- A. **"Start a run: work through the ready tickets, in parallel where they
  don't collide."** Leads with the payoff verb ("Start a run"), then the plain
  truth about concurrency. Drops "zellij/plugin/DAG-driven task scheduling" —
  the operator doesn't care that it's zellij; they care that it starts working
  tickets.
- B. "Start working the tickets Lisa can run right now." Cleaner but loses the
  concurrency truth that makes Loop distinct from a serial runner.
- C. "Launch the Lisa dashboard and start running tickets." Reintroduces the
  implementation noun (dashboard/zellij) the brand voice says to drop.

**Choice: A.** This is *the* command that starts a run (the epic's "one command
that launches a run"), so its help should say "Start a run" in the first breath.
The concurrency clause is the one place detail earns its keep — it is what Loop
does that nothing else does. The design for the sibling about-line kept
concurrency off the masthead precisely so it could land *here*.

## Decision 2 — how far to rewrite the machinery commands

The story says "reword the internal commands' help"; the AC's hard jargon ban is
operator-only (Research). So:

- **Approach chosen: de-jargon lightly, keep precise machinery nouns.** Rewrite
  each internal command's first line into a plain "what running this does"
  sentence, but keep the real identifiers an agent debugging hooks needs
  (`Stop-hook`, `codex`, `.lisa/…` paths, ticket-owned). Do not inflate them to
  operator prose they will never be read as — they are invoked by machinery.
- Rejected: **full kitchen-table rewrite** of the four hook commands. Overreach —
  it would strip the exact terms (`--include`, ordinary index, Stop-hook payload)
  that make the command's contract legible to the agent that actually calls it,
  buying no operator legibility (they're hidden/trailing).
- Rejected: **leave internal help untouched.** The story explicitly asks to
  reword them; and AgentExec/CommitTicket today carry faintly category-ish
  phrasing worth plaining.

### Internal rewrites (first line)

- **AgentExec** — keep the existing precise multi-line body; soften the opening
  from "legacy JSON signal/rendering wrapper" toward "Run Codex and turn its
  output into Lisa's pane signals." Body (env vars, `codex exec --json`) stays.
- **CaptureUsage** — "Record a Claude session's token usage from its Stop-hook
  payload" — plainer verb, keeps Stop-hook/path facts.
- **CommitTicket** — "Commit this ticket's own files without touching the
  repo's ordinary git index." — "touching" is plainer than "using"; keeps the
  index fact that is the whole point.
- **CompleteTicket** — "Mark a ticket done and commit its files in one step." —
  "atomically" → "in one step".
- **SetupGuide** — "Print setup instructions for an agent to follow." — drops
  "LLM-friendly" buzz.
- **HooksGuide** — "Print the guide for wiring up Claude Code hooks." — plain.
- **Version** — "Print Lisa's version." — already fine; tiny tidy.

## Scope discipline (what this ticket does NOT change)

- No `about =` line (T-036-01-01), no `display_order`/`hide` attributes, no
  variant reordering. Comment lines only.
- No per-flag `///` help (e.g. `--path`, `--dry-run`) — out of AC scope.
- No test (T-036-01-03 owns the regression lock). This ticket must leave the
  strings jargon-clean so that test pins cleanly.
- `main()`, dispatch, module list, and every field untouched → comment-only diff.

## Verification plan (previewing Plan)

Build the release binary; confirm each `lisa init|validate|status|doctor|loop
--help` opens with the new plain sentence and that `lisa --help`'s command list
shows the new short lines. Grep the operator strings for the banned terms
(`DAG`, `orchestrat`, `scheduling`, `leverage`, `solutions`) → zero hits. Run
`cargo test --workspace` (no test reads help yet, so this is a no-regression
check). Confirm all 12 subcommands still resolve.
