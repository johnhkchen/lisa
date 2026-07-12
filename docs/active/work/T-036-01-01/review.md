# Review — T-036-01-01: about-line and operator/internal grouping

## What changed

One source file, one commit (`5d8b8663`), through `lisa commit-ticket`.

**`crates/lisa-cli/src/main.rs`** — two behavior-free edits:

1. **About-line** (struct `Cli`): `about = "Lisa - DAG-driven concurrent task
   scheduling"` → `about = "Runs your coding agents through a project's
   tickets."` Plain, verb-forward, operator-first; drops all banned jargon
   (`DAG-driven`, `orchestration`, `concurrent task scheduling`).

2. **Twelve grouping attributes** on `enum Commands` variants (one `#[command(…)]`
   each), leaving every `///` doc comment and field list untouched:
   - `display_order = 0..=4` → init, validate, status, doctor, loop (operators).
   - `display_order = 20..=23` → agent-exec, capture-usage, commit-ticket,
     complete-ticket (hook/plumbing).
   - `hide = true` → setup-guide, hooks-guide, version.

No files created or deleted. `main()`, dispatch, flags, module list, and every
subcommand's logic are unchanged.

## Rendered result (`lisa --help`, built release binary)

```
Runs your coding agents through a project's tickets.

Usage: lisa <COMMAND>

Commands:
  init             Initialize a project for lisa-loop completion
  validate         Validate ticket DAG and project setup
  status           Show DAG status: tickets, dependencies, execution waves, scheduling readiness
  doctor           Check that all runtime dependencies are installed
  loop             Launch zellij with the Lisa plugin for DAG-driven task scheduling
  agent-exec       Run Codex under Lisa's legacy JSON signal/rendering wrapper
  capture-usage    Capture Claude session token usage from a Stop-hook payload on stdin, …
  commit-ticket    Commit ticket-owned paths without using the repository's ordinary index
  complete-ticket  Mark a ticket done and commit its loop-owned files atomically
  help             Print this message or the help of the given subcommand(s)
```

Operators lead as a block; the four hook commands trail as a block;
setup-guide/hooks-guide/version are absent from the listing.

## Acceptance-criteria trace

- **about-line, no jargon** — ✅ new sentence carries none of
  `DAG-driven`/`orchestration`/category terms.
- **init, validate, status, doctor, loop appear together ahead of the four hook
  commands** — ✅ via `display_order` bands 0–4 vs 20–23. (The "distinct
  heading" alternative was proven infeasible in the derive API without
  subcommand nesting — see below — so the "ahead of" branch was taken.)
- **setup-guide/hooks-guide/version explicitly classified** — ✅ all three
  `hide = true`: version is redundant with `-V`/`--version`; setup-guide and
  hooks-guide are agent-facing guide emitters, not operator loop commands.
- **all 12 subcommands still resolve and run** — ✅ verified: `lisa version`
  prints `lisa 0.4.0-rc.6` (exit 0); `lisa setup-guide --help` / `lisa
  hooks-guide --help` exit 0; hook commands (`lisa commit-ticket --help`)
  resolve; visible operators unaffected. The compiler's exhaustive `match` in
  `main()` guarantees no variant was dropped or renamed.
- **achieved with help_heading/display_order/hide only, no subcommand nesting** —
  ✅ only `display_order` and `hide` used. `help_heading` was evaluated and
  deliberately not used (see below).

## Test coverage

- `cargo build -p lisa-cli --release` — clean.
- `cargo test --workspace` — **285 passed, 0 failed**.
- Manual `--help` render + per-command resolve checks (above).

No automated help-surface test is added here **by design**: T-036-01-03 owns the
regression lock in its own test file (disjoint from main.rs), per the story's
serial-wave rationale. Until that lands, the operator-first layout is guarded
only by manual verification — that is the intended, documented gap, not an
oversight.

## Notable findings / decisions a reviewer should know

1. **A per-group heading for subcommands is not reachable via clap 4.5's derive
   API without nesting.** `help_heading` is rejected on a subcommand variant
   (clap suggests `next_help_heading`), and `next_help_heading` governs *argument*
   grouping, not the `Commands:` listing — it compiles but produces no visual
   split. Confirmed empirically against clap 4.5.57 in a scratch crate. So the
   AC's "or under a distinct heading" branch is infeasible under "no subcommand
   nesting"; the "ahead of" branch (display_order) is the only feasible path and
   is what shipped.

2. **`display_order` gap (4 → 20) is intentional headroom** so T-036-01-02 can
   slot copy/commands without renumbering, and to make the operator/plumbing
   split legible in source.

## Open concerns / deferrals (intentional)

- **`loop`'s `///` still says "DAG-driven task scheduling".** The per-command
  copy rewrite is T-036-01-02; this ticket's jargon ban is scoped to the
  about-line only (per the AC and the story's explicit seam). Flagged so the
  next ticket picks it up — not a defect of this ticket.
- The four hook commands remain **visible** (ordered last) rather than hidden,
  matching the AC's "appear … ahead of" wording. If a reviewer prefers them
  fully out of the primary listing, that is a one-line swap (`display_order` →
  `hide`) but would contradict the current AC phrasing.

## Handoff

Nothing requires human intervention. The change is attribute-only, compile- and
test-gated, and leaves the CLI's parse/dispatch behavior identical. Ready for
Lisa to publish Done.
