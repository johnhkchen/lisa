# Research — T-036-01-01: about-line and operator/internal grouping

## Ticket in one line

Replace the top-level `--help` about-line with a plain, operator-first sentence
(no `DAG-driven` / `orchestration` / category jargon) and reorder the command
listing so the five operator commands lead and the hook/plumbing commands are
set apart — using only `help_heading`/`display_order`/`hide`, no subcommand
nesting. Every one of the 12 subcommands must still resolve and run.

## Where the help surface lives

All of the `--help` output is derived by clap from a single file:
`crates/lisa-cli/src/main.rs`.

- Top-level about-line: the `#[command(... about = "Lisa - DAG-driven concurrent
  task scheduling", version)]` attribute on `struct Cli` (main.rs:18–23).
- Per-command summaries: the `///` doc comment on each variant of
  `enum Commands` (main.rs:29–174). clap renders the first paragraph of each
  doc comment as the command's short help in the `Commands:` section.
- Dispatch: `fn main()` matches on `cli.command` (main.rs:176–329). The match
  arms and every subcommand's logic are entirely independent of the help
  attributes — reordering variants or adding grouping attributes cannot change
  behavior.

No other file contributes to `--help`. There is no man-page generator, no
`clap_complete` wiring in this crate, and no README block mirrored from help.

## The command set (12 subcommands)

Declaration order in `enum Commands` today:

| # | Variant         | kebab name        | Role                                  |
|---|-----------------|-------------------|---------------------------------------|
| 1 | Init            | `init`            | operator                              |
| 2 | Validate        | `validate`        | operator                              |
| 3 | Status          | `status`          | operator                              |
| 4 | SetupGuide      | `setup-guide`     | agent-facing guide emitter            |
| 5 | HooksGuide      | `hooks-guide`     | agent-facing guide emitter            |
| 6 | Doctor          | `doctor`          | operator                              |
| 7 | Version         | `version`         | redundant with `-V`/`--version`       |
| 8 | AgentExec       | `agent-exec`      | hook/plumbing (machinery-invoked)     |
| 9 | CaptureUsage    | `capture-usage`   | hook/plumbing (Stop-hook stdin)       |
|10 | CommitTicket    | `commit-ticket`   | hook/plumbing (isolated commit txn)   |
|11 | CompleteTicket  | `complete-ticket` | hook/plumbing (Lisa completion)       |
|12 | Loop            | `loop`            | operator (starts a run)               |

clap also injects a built-in `help` subcommand (display_order defaults last).

The epic/story name the operator five as **init, validate, status, doctor,
loop** and the hook/plumbing four as **agent-exec, capture-usage, commit-ticket,
complete-ticket**. The remaining three — **setup-guide, hooks-guide, version** —
are what the AC says must be "explicitly classified".

## Which commands the machinery depends on (must not disappear)

`agent-exec`, `capture-usage`, `commit-ticket`, and `complete-ticket` are
invoked by hooks and the loop launcher, not typed by operators. Their kebab
names and flags are a contract:
- `commit-ticket` is the exact command this very assignment tells the agent to
  run (`lisa commit-ticket --ticket-id … --include …`).
- `capture-usage` runs from a Claude Stop-hook on stdin.
- `complete-ticket` is Lisa's own completion command.
- `agent-exec` is the legacy Codex JSON wrapper.
`hide = true` removes a command from the listing but leaves it fully resolvable
(verified below), so hiding is safe for these contracts; renaming/removing is
not, and is explicitly out of scope.

## Clap 4.5 capability findings (empirically verified)

Crate uses `clap = { version = "4", features = ["derive"] }`; resolved to
`clap 4.5.57` in `Cargo.lock`. I prototyped against clap 4.5 in a scratch crate:

1. **`help_heading` is NOT accepted on a subcommand variant.** clap errors and
   suggests `next_help_heading`. But `next_help_heading` on a variant compiles
   yet does **not** split the `Commands:` section into separate headings — it
   governs *argument* grouping, not the subcommand listing. So a true
   per-group heading for subcommands is unreachable in the derive API without
   nesting (a separate `#[derive(Subcommand)]` enum under a parent command),
   which the AC forbids ("no subcommand nesting").
2. **`display_order` on a variant works** and sorts the `Commands:` listing by
   that integer regardless of declaration order. Verified `init` (order 0)
   renders before `loop` (order 1) even when declared after it.
3. **`hide = true` works**: the command vanishes from `--help` but `lisa <cmd>`
   still parses and runs (`exit=0`), and `lisa <cmd> --help` still prints its
   own help.

Consequence: the AC's two offered treatments — "appear together **ahead of**"
vs. "under a **distinct heading from**" — collapse to one feasible path in the
derive API: **ahead of**, via `display_order`. The "distinct heading" branch
would require nesting and is therefore out.

## Full-layout prototype (12 real commands)

Prototyped the real command set with operator commands at `display_order` 0–4,
hook commands at 20–23, and setup-guide/hooks-guide/version hidden. Rendered:

```
Commands:
  init  validate  status  doctor  loop         (operators, in that order)
  agent-exec  capture-usage  commit-ticket  complete-ticket
  help
```

All three hidden commands still resolved (`setup-guide`/`hooks-guide`/`version`
each `exit=0`). This is the shape the design will lock.

## Existing tests / boundaries

`crates/lisa-cli/tests/` holds `atomic_provider_contract.rs`,
`real_zellij_delivery_boundary.rs`, and `fixtures/`. None assert on `--help`.
The help-surface regression test is deferred to T-036-01-03 (its own file,
disjoint from main.rs). This ticket writes no test.

## Constraints & assumptions carried into Design

- **Scope seam:** this ticket owns only the about-line and the grouping/ordering
  attributes. The per-command verb-forward copy rewrite is T-036-01-02; the
  jargon in e.g. `loop`'s doc comment ("DAG-driven task scheduling") is left for
  that ticket. The AC scopes the jargon ban here to *the about-line only*.
- **Behavior-free:** only `about=`, `///` copy (about-line only), and
  `#[command(...)]` grouping attributes may change. `main()` and flags untouched.
- **Brand voice** (user-global CLAUDE.md): plain kitchen-table English,
  verb-forward, warm host; no `orchestration`/`DAG-driven`/category jargon.
- Assumption: the four named hook commands should stay **visible** (AC says they
  "appear … ahead of[which implies listed]"), while the leftover three may be
  hidden as their "explicit classification".
