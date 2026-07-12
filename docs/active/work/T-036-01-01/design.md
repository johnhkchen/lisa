# Design — T-036-01-01: about-line and operator/internal grouping

## The two decisions this ticket makes

1. **The about-line** — one plain sentence replacing
   "Lisa - DAG-driven concurrent task scheduling".
2. **The grouping mechanism** — how the 12 commands are ordered/hidden so
   operators lead and plumbing is set apart, using only
   `help_heading`/`display_order`/`hide`.

Everything grounds in the Research findings, especially the clap 4.5 capability
probe (headings for subcommands are unreachable without nesting; `display_order`
and `hide` both work and are behavior-free).

---

## Decision 1 — the about-line

### Options

- **A. "Runs your coding agents through a project's tickets."** Verb-forward,
  operator-first, kitchen-table plain. Mirrors John's b28.dev label for Lisa
  ("Runs my coding agents"). Says what a person gets, not the category.
- **B. "Schedules coding agents across your tickets by dependency."** Accurate
  but "Schedules … by dependency" reintroduces the scheduling-category framing
  the epic is trying to retire; "dependency" edges toward jargon.
- **C. "Runs your coding agents through a project's tickets — in order, and in
  parallel where they don't collide."** Truthful about concurrency, but a
  two-clause sentence for the very first line trades legibility for detail the
  operator learns from `status`/`loop` anyway.

### Choice: **A**

"Runs your coding agents through a project's tickets."

Rationale: the epic's `serves` line wants an operator to learn *what Lisa does*
in one screen. A single verb-forward sentence ("Runs … agents … through …
tickets") does that with zero category words. It drops every banned term
(`DAG-driven`, `orchestration`, "concurrent task scheduling"). It reads at a
kitchen table. The concurrency nuance (option C) is real but belongs to `loop`/
`status`, not the masthead; keeping the about-line to one clause is the higher
legibility win. It also harmonizes with the brand's existing one-liner for Lisa,
so the CLI and the personal site say the same thing.

Rejected B because it keeps the "scheduling/dependency" category frame the epic
explicitly calls rotted. Rejected C for length; the second clause is detail, not
orientation.

---

## Decision 2 — grouping mechanism

### Options

- **A. Distinct heading per group** (operators under one heading, plumbing under
  another). *Rejected — infeasible.* Research proved clap 4.5's derive API does
  not group subcommands under separate headings; `help_heading` is rejected on a
  variant and `next_help_heading` only affects args. The only way to get a
  literal heading is a nested subcommand enum, which the AC forbids.
- **B. `display_order` bands + `hide` for the leftovers.** Operators get a low
  band (0–4) so they lead; the four named hook commands get a high band (20–23)
  so they trail as a set; setup-guide/hooks-guide/version are hidden. Feasible,
  behavior-free, verified end-to-end in Research.
- **C. `display_order` for operators + `hide` for ALL non-operators** (hide the
  four hook commands too). Cleanest operator screen, but the AC says the four
  hook commands should "appear … ahead of[/listed]", so hiding them contradicts
  the ticket's own wording. Rejected as over-reach.

### Choice: **B**

Use `display_order` to place the five operator commands first as a block and the
four hook/plumbing commands last as a block; hide the three leftovers.

This is the only option that satisfies "operators lead, plumbing set apart"
within "no subcommand nesting" while keeping the four contract commands visible.

### The band assignment

| Command          | Treatment              | Why |
|------------------|------------------------|-----|
| init             | `display_order = 0`    | operator |
| validate         | `display_order = 1`    | operator |
| status           | `display_order = 2`    | operator |
| doctor           | `display_order = 3`    | operator |
| loop             | `display_order = 4`    | operator — the command that starts a run, kept last of the block so it reads as the payoff |
| agent-exec       | `display_order = 20`   | hook/plumbing, machinery-invoked |
| capture-usage    | `display_order = 21`   | hook/plumbing |
| commit-ticket    | `display_order = 22`   | hook/plumbing |
| complete-ticket  | `display_order = 23`   | hook/plumbing |
| setup-guide      | `hide = true`          | classified: agent-facing guide emitter, not an operator loop command |
| hooks-guide      | `hide = true`          | classified: agent-facing guide emitter |
| version          | `hide = true`          | classified: redundant with `-V`/`--version` |

Gap between the 4-band and 20-band leaves headroom for T-036-01-02 to slot a
command without renumbering, and makes the operator/plumbing split obvious in
source. Operator order matches the epic's own enumeration (init, validate,
status, doctor, loop) for a reader who already knows the list.

### Why hide those three specifically ("explicit classification")

The AC requires setup-guide/hooks-guide/version to be *explicitly classified*.
The classification is: **none of the three is an operator loop command, so none
belongs in the foregrounded listing.**
- `version` duplicates the built-in `-V`/`--version`; a separate listed command
  is pure clutter.
- `setup-guide` emits "LLM-friendly setup instructions" and `hooks-guide` emits
  a hooks-config guide "for agents" — both are consumed by agents/machinery, not
  read by an operator scanning for what to run.
Hiding (not removing) keeps them fully resolvable for the machinery and power
users while clearing the operator screen. This is consistent with the epic's
"moved out of the primary listing" as an acceptable treatment.

The four named hook commands are *not* hidden, per the AC's wording that they
"appear … ahead of[/listed after operators]"; they stay visible but trail.

---

## Scope discipline (what this ticket does NOT change)

- No `///` per-command copy is rewritten (that is T-036-01-02). The `loop` doc
  comment still contains "DAG-driven task scheduling"; the AC scopes the jargon
  ban here to the *about-line* only, so it is left for the next ticket. Only the
  top-level `about=` string changes.
- `main()` match arms, every subcommand's flags/logic, and dispatch are
  untouched. The enum's variant declaration order is left as-is; ordering is
  expressed purely through `display_order` attributes so the diff is
  attribute-only and the risk of disturbing a match arm is zero.
- No test is added here (T-036-01-03 owns the regression lock).

## Verification plan (previewing Plan)

Build the real binary and confirm `lisa --help` renders the about-line and the
operator-then-plumbing ordering with the three commands hidden; then confirm
each hidden command still resolves (`lisa version`, `lisa setup-guide`,
`lisa hooks-guide`) and a spot operator command still runs. `cargo test
--workspace` must stay green (no test touches help, so this is a no-regression
check).
