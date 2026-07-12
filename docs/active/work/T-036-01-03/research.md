# Research — T-036-01-03: lock-help-surface-regression-test

Descriptive map of the ground this ticket sits on. What exists, where, and the
constraints a regression test must respect. No solutions proposed here.

## The ticket in one line

Add a native `cargo test` that pins the now-legible `--help` surface so three
things cannot silently regress: (a) the full command set (all 12 subcommands
still resolve), (b) the operator/hook split (the four machinery-invoked commands
stay set apart from or hidden out of the primary operator listing), and (c) the
jargon-free copy (about-line + operator help carry no banned category terms).

This is the third and last ticket of S-036-01. Its two predecessors are `done`:
T-036-01-01 settled the about-line and the grouping/ordering attributes;
T-036-01-02 rewrote the per-command copy. This ticket only *locks* their output.

## The surface under test — `crates/lisa-cli/src/main.rs`

All help derives from clap 4 derive macros on two items:

- The top-level `#[derive(Parser)] struct Cli` with
  `#[command(name = "lisa", about = "Runs your coding agents through a project's tickets.", version)]`.
- The `#[derive(Subcommand)] enum Commands` — 12 variants, each with a `///`
  doc comment (its help line) plus `#[command(...)]` grouping attributes.

The 12 subcommands and their current treatment (main.rs:29–186):

| Variant          | kebab name       | Treatment            | Class     |
|------------------|------------------|----------------------|-----------|
| Init             | init             | `display_order = 0`  | operator  |
| Validate         | validate         | `display_order = 1`  | operator  |
| Status           | status           | `display_order = 2`  | operator  |
| Doctor           | doctor           | `display_order = 3`  | operator  |
| Loop             | loop             | `display_order = 4`  | operator  |
| AgentExec        | agent-exec       | `display_order = 20` | hook      |
| CaptureUsage     | capture-usage    | `display_order = 21` | hook      |
| CommitTicket     | commit-ticket    | `display_order = 22` | hook      |
| CompleteTicket   | complete-ticket  | `display_order = 23` | hook      |
| SetupGuide       | setup-guide      | `hide = true`        | internal  |
| HooksGuide       | hooks-guide      | `hide = true`        | internal  |
| Version          | version          | `hide = true`        | internal  |

Note the enum *declaration* order (Init, Validate, Status, SetupGuide,
HooksGuide, Doctor, Version, AgentExec, …, Loop) is NOT the display order;
`display_order` reshuffles the rendered listing. The test must key off rendered
output, not declaration order.

The "four hook commands" named by the AC are the machinery-invoked contract
commands: **agent-exec, capture-usage, commit-ticket, complete-ticket**. They are
visible but banded low-priority (20–23) so they trail the operator block. The
three `hide = true` variants (setup-guide, hooks-guide, version) are dropped from
the listing entirely but still resolve.

## What the rendered help actually looks like (release binary, verified)

`lisa --help` prints:

```
Runs your coding agents through a project's tickets.

Usage: lisa <COMMAND>

Commands:
  init             Set up a project to run with Lisa
  validate         Check your tickets and project setup for problems before a run
  status           Show which tickets are ready to run and which are waiting, and why
  doctor           Check that the tools Lisa needs are installed
  loop             Start a run: work through the ready tickets, in parallel where they don't collide
  agent-exec       Run Codex and turn its output into Lisa's pane signals
  capture-usage    Record a Claude session's token usage from its Stop-hook payload on stdin, writing `.lisa/claude/<ticket>.usage.json` for the provenance ledger
  commit-ticket    Commit this ticket's own files without touching the repo's ordinary git index
  complete-ticket  Mark a ticket done and commit its files in one step
  help             Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

Observations relevant to the test:

- Operator block (init…loop) renders first, hook block (agent-exec…
  complete-ticket) after, then clap's auto-generated `help`. The
  operator-before-hook ordering is the observable form of "set apart".
- The three hidden commands do not appear, yet `lisa version`, `lisa
  setup-guide --help`, `lisa hooks-guide --help` all resolve (exit 0). Verified:
  `lisa version` prints `lisa 0.4.0-rc.6`.
- clap injects a synthetic `help` pseudo-command into the listing. Any "count
  the commands" assertion that scans the Commands block must exclude `help`, or
  it will count 10 visible + not see the 3 hidden ones. Counting rendered lines
  is therefore the wrong primitive; per-command *resolution* is the right one.

## How subcommands "resolve" — the executable contract

`main()` (main.rs:188–341) has one match arm per variant; a variant that failed
to parse would never dispatch. The machinery depends on the four hook commands by
name (hooks call `lisa capture-usage`; the loop launcher and RDSPI flow call
`lisa commit-ticket` / `complete-ticket` / `agent-exec`). "Resolves" for a test
means: `lisa <name> --help` exits 0 and clap recognizes `<name>` as a real
subcommand (an unknown subcommand exits non-zero with "unrecognized subcommand").
`--help` is the safe probe — it never runs side effects yet proves the command is
wired.

## The banned-jargon vocabulary

Two sources define the ban:

1. **user-global CLAUDE.md** (brand voice): no "case study", "build log",
   "research release", "orchestration", "deployment architecture", "leverage",
   "solutions".
2. **E-036 / S-036-01**: no "DAG-driven", "orchestration", "concurrent task
   scheduling", or category framing; the about-line and operator help must read
   as plain action.

The union, as substrings/phrases to forbid in the about-line + operator help:
`dag`, `orchestrat`, `scheduling`, `concurrent task scheduling`, `leverage`,
`solutions`, `deployment`, `case study`, `build log`, `research release`.

Scope caveat (from the AC and predecessor tickets): the jargon gate applies to
**the about-line and operator help only** — NOT the hook commands. The hook help
legitimately mentions `codex exec`, "pane signals", "provenance ledger", none of
which are operator-facing. `capture-usage`'s help contains "provenance"; the test
must not scan hook help for jargon or it would false-flag domain vocabulary the
AC deliberately left alone.

A subtlety for the matcher: the short token `dag` must match `DAG-driven`
(bounded by `-`) without matching a hypothetical `dagger`. Word/phrase-boundary
matching (non-alphanumeric on each side), case-insensitive, is the correct
primitive — a naive `contains("dag")` risks future false positives, a naive
whole-`split_whitespace` token check would miss `DAG-driven` (hyphen-joined).

## Existing test conventions — `crates/lisa-cli/tests/`

Two integration tests already exist and establish the house style:

- `atomic_provider_contract.rs` and `real_zellij_delivery_boundary.rs` both
  invoke the built binary via `env!("CARGO_BIN_EXE_lisa")` and
  `std::process::Command`, then assert on `output.status.success()` and on
  substrings of `String::from_utf8_lossy(&output.stdout)`.

So the established pattern is **black-box: spawn the real `lisa` binary, parse
stdout**. Cargo sets `CARGO_BIN_EXE_lisa` for integration tests automatically and
builds the binary as a prerequisite. There is no in-process `CommandFactory`
route available: main.rs is a `[[bin]]`, not a lib, so `Cli`/`Commands` cannot be
imported by a test crate. Black-box is the only route and it matches convention.

`[dev-dependencies]` currently has `tempfile` only. The help test needs no new
dependency — it runs `lisa --help` / `lisa <cmd> --help`, which touch no
filesystem and need no temp project.

## Constraints and assumptions

- **Behavior-free / test-only.** Story scope confines this ticket to a new file
  under `crates/lisa-cli/tests/`; main.rs must not change. The test observes; it
  does not alter the surface.
- **No new dependency.** Reuse `std::process::Command` +
  `env!("CARGO_BIN_EXE_lisa")`.
- **Own only the new test file.** `lisa commit-ticket --include` must name only
  the single new test path. No other file is ticket-owned.
- **The test must pin, not merely smoke.** It has to fail if a command is
  removed, if a hook command is promoted into the operator block, or if a banned
  term is reintroduced into operator-facing copy — the three failure modes the AC
  enumerates.
- **Assumption:** kebab-case names (`agent-exec`, etc.) are the stable public
  identifiers; clap derives them from the PascalCase variant names. A rename
  would be a deliberate breaking change and should trip the test — so hard-coding
  the expected name list is a feature, not brittleness.
- **Assumption:** `help` (clap's synthetic entry) and the top-level `-h/--help`,
  `-V/--version` options are clap-provided and out of scope for the command-set
  assertion.
