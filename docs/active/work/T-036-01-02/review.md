# Review — T-036-01-02: plain, verb-forward command help

Handoff document. What changed, how it was verified, and what a human reviewer
should know.

## What changed

One source file, one commit (`2890401c`), through `lisa commit-ticket`.

**`crates/lisa-cli/src/main.rs`** — twelve `///` doc comments on the `Commands`
enum variants rewritten into plain, verb-forward English. Nothing else: no
`about =` line (owned by T-036-01-01), no `#[command(...)]` attribute, no field,
no `main()` match arm. The `git diff` is **comment-only** (verified: every added
/removed line is a `///` line).

No files created or deleted. No test added (T-036-01-03 owns that).

### Operator commands (the AC-gated five)

| Command  | Before | After |
|----------|--------|-------|
| init     | Initialize a project for lisa-loop completion | **Set up a project to run with Lisa.** |
| validate | Validate ticket DAG and project setup | **Check your tickets and project setup for problems before a run.** |
| status   | Show DAG status: tickets, dependencies, execution waves, scheduling readiness | **Show which tickets are ready to run and which are waiting, and why.** |
| doctor   | Check that all runtime dependencies are installed | **Check that the tools Lisa needs are installed.** |
| loop     | Launch zellij with the Lisa plugin for DAG-driven task scheduling | **Start a run: work through the ready tickets, in parallel where they don't collide.** |

### Internal commands (reworded, not jargon-gated)

| Command         | After |
|-----------------|-------|
| agent-exec      | Run Codex and turn its output into Lisa's pane signals. *(multi-line body kept verbatim)* |
| capture-usage   | Record a Claude session's token usage from its Stop-hook payload on stdin, … |
| commit-ticket   | Commit this ticket's own files without touching the repo's ordinary git index. |
| complete-ticket | Mark a ticket done and commit its files in one step. |
| setup-guide     | Print setup instructions for an agent to follow. |
| hooks-guide     | Print the guide for wiring up Claude Code hooks. |
| version         | Print Lisa's version. |

## Acceptance criteria — met

> Each operator subcommand's `///`-derived help reads as a plain action (Loop no
> longer says 'DAG-driven task scheduling'); `lisa init|validate|status|doctor|
> loop --help` each describe what running the command does; no operator-facing
> help string contains the banned jargon terms.

- ✅ Each operator help opens with an imperative action ("Set up…", "Check…",
  "Show…", "Start a run…"). Verified by reading each `lisa <cmd> --help` first
  line off the built release binary.
- ✅ Loop no longer says "DAG-driven task scheduling"; it now leads with "Start a
  run" — the epic's "one command that launches a run".
- ✅ Jargon ban: grep of the operator command list for
  `dag|orchestrat|scheduling|leverage|solutions` returned **zero hits**.

## Verification performed

- `cargo build -p lisa-cli --release` — clean.
- Rendered `lisa --help` and each `lisa <cmd> --help` — new plain lines present;
  `agent-exec --help` still carries its LISA_PANE_ID / `codex exec` body.
- All 12 subcommands resolve (`<cmd> --help` exits 0) — command set unchanged.
- `cargo test --workspace` — **286 passed, 0 failed**. (No existing test reads
  help; this is a no-regression check.)
- `git status` on `crates/lisa-cli/src/main.rs` — clean post-commit; nothing
  left staged, modified, or untracked.

## Test coverage & gaps

- **No automated help-surface test in this ticket — by design.** The story
  assigns the regression lock (assert no banned jargon + command set unchanged)
  to **T-036-01-03**, whose test file is disjoint from `main.rs`. Adding it here
  would collide with that ticket's ownership. Until T-036-01-03 lands, the
  jargon-clean state is guarded only by manual verification above, not a test.
- The copy itself is the artifact; it was verified by running the real binary,
  which is the meaningful check for help text.

## Open concerns / notes for the reviewer

- **Subjective copy.** The exact wording (e.g. Loop's concurrency clause) is a
  brand-voice judgment; design.md records the options weighed and why A won for
  each command. If a reviewer prefers different phrasing, it's a one-line edit.
- **Per-flag help untouched.** `--path`, `--dry-run`, etc. keep their existing
  `///` help — the AC scopes to subcommand-level help, so flag descriptions were
  intentionally left alone. Not a gap, a scope boundary.
- **Internal commands kept precise nouns** (`Stop-hook`, `codex`, ordinary git
  index) — these name real machinery an agent debugging hooks relies on, and the
  jargon ban is operator-facing only. Deliberate, per design.md Decision 2.

## Critical issues needing human attention

None. Comment-only change, all tests green, command set and dispatch provably
unchanged.
