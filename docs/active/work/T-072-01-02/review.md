# T-072-01-02 — running low is a decision, not a crash

## The design question, answered

`docs/knowledge/spend-autonomy.md` states the chosen position and why: **it
stops, but never starts or changes.** Of the three positions in the ticket's
own Context (only says / stops / downshifts), stopping is the reversible
direction — a stopped loop restarts with `lisa loop`, nothing is guessed at —
and "only says" fails at exactly the moment the story opens on: an operator
asleep while four panes crash. An automatic downshift gear was rejected;
the *manual* one (`[agent].model`/`[agent].effort`, `T-071-01-01`) already
exists and is the right tool for an operator who wants to spend deliberately.

## What changed

- **`crates/lisa-cli/src/spend.rs`** — `lisa spend --guard`. Refactored the
  existing host-reading code into a shared `compute_report()` used by both the
  unchanged bare `lisa spend` and the new guard path. Added:
  - `decide_guard_action()` — pure function: `(SpendReport, allowance,
    priority) -> GuardAction`. Five variants (`NoAllowanceConfigured`,
    `UnreadableSpend`, `BelowThreshold`, `NotEligible`, `Stop`), each carrying
    what it needs to render its own sentence.
  - `render_guard_action()` — pure, renders each variant.
  - `run_guard()` — reads the desk-wide report (same code path as bare
    `lisa spend`) plus this project's own `[scheduling]` config, decides, and
    returns the rendered text alongside the decision.
  - `stop_for_guard()` — the only impure half. On `GuardAction::Stop`, walks
    every scheduler this board has recorded and stops each one through
    `crate::schedulers::run_schedulers` — the *identical* path
    `lisa schedulers --stop` already uses, so a guard-stopped loop is exactly
    as restartable and exactly as describable by `lisa schedulers` as one an
    operator stopped by hand. Then calls `rail tell --kind loop-degraded`
    (best-effort; falls back to a stdout line if `rail` is absent or fails,
    so a stop that happened is never silently unreported).
- **`crates/lisa-cli/src/config.rs`** — two new `[scheduling]` keys:
  - `priority` (`low`/`medium`/`high`/`critical`, default `medium`) — reuses
    `lisa_core::types::Priority`'s vocabulary. A board that never configures
    this defaults to `medium`, never `low`, so it is never a stop target —
    literal to the AC: "a board with no priority must not become the one that
    gets stopped."
  - `weekly_token_allowance` (`Option<u64>`, no default) — the number the
    guard compares this week's spend against. Absent means the guard is
    inert.
  - Both wired through `resolve_config`, validated (`priority` refuses an
    unknown word; `weekly_token_allowance` refuses `0`), cataloged in
    `CONFIG_KEYS` with brand-voice-passing descriptions, and rendered in
    `lisa init`'s default `.lisa.toml` as commented stubs.
- **`crates/lisa-cli/src/main.rs`** — `lisa spend --guard` flag and dispatch:
  prints the guard's report, and on `GuardAction::Stop` calls
  `stop_for_guard`.
- **`crates/lisa-cli/tests/spend_guard_cli.rs`** (new) — five black-box tests
  against the real compiled binary, fixture `rail` on `PATH`: the ticket's own
  reproduction recipe (low allowance, low priority, over threshold → stops
  and tells rail, with the `rail tell` call's argv captured and asserted), a
  board with no configured priority (never stops), frontloaded spend under
  the threshold (reports, does nothing — `S-072-01`'s frontloading-is-legal
  case), no allowance configured (inert however much was spent), and an
  unreachable machine (refuses to act even though the reachable part alone is
  over threshold).
- **`docs/knowledge/spend-autonomy.md`** (new) — the design decision, in full,
  including the two things the ticket asked to be said out loud: the
  downshift gear that was *not* built, and why (`S-072-01`'s two-gears note),
  and the second failure mode ("a day eats the week") that is explicitly out
  of scope here.
- **`docs/knowledge/flag-audit.md`**, **`README.md`** — audit rows and config
  table rows for the new flag and the two new config keys.

Two commits through `lisa commit-ticket`: `3ffc124` (the guard itself),
`f32405c` (the design doc + audit/README rows).

## Where "priority" and "the threshold" live

- **Board priority**: `[scheduling].priority` in each project's own
  `.lisa.toml` — a board's own file, read locally, same as every other
  per-board scheduling setting. This is distinct from a *ticket's*
  `priority:` frontmatter (unchanged, still ranks work within one board's
  queue); nothing on the desk previously said which whole *loop* was more
  expendable than another, and this is that missing word.
- **The threshold**: `LOW_SPEND_STOP_PCT = 90` (a constant in `spend.rs`, not
  a config key — matches this codebase's existing convention for fixed
  operational numbers like `RECEIPT_WINDOW_SECS`). 90% of the *configured*
  `weekly_token_allowance`, never of a number Lisa invented — consistent with
  `T-072-01-01`'s own refusal to guess a price or a ceiling.

## Reproduction

`cargo test -p lisa-cli --test spend_guard_cli` runs the ticket's own
recipe end to end: a temp project with a real `.lisa/claude/captures.jsonl`
totaling 950 of a configured 1000-token allowance, `[scheduling].priority =
"low"`, and a fixture `rail` on `PATH`. `lisa spend --guard --path <project>`
reports "950 of 1000 tokens spent this week (95%)... Stopping its loop",
attempts the stop (reports "nothing to stop" here, honestly, since the test
project runs no real Zellij scheduler — see Concerns), and the fixture
`rail`'s logged argv confirms `rail tell --kind loop-degraded --project
<project> --what "...low-priority board" --do "..."` was called with the
right words.

## Testing

- 8 new unit tests in `spend.rs` (`decide_guard_action`/`render_guard_action`
  coverage: no allowance, unreachable host, below threshold, default
  priority not eligible, over threshold stops, exact-threshold boundary,
  every variant renders without panicking).
- 6 new unit tests in `config.rs` (resolve defaults/configured for both new
  keys, validate rejects an unknown priority word and a zero allowance).
- 5 new integration tests in `tests/spend_guard_cli.rs`, against the real
  compiled binary (listed above under "What changed").
- Full gates: `cargo fmt --all -- --check`, `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo test --workspace` (788 + 32 in
  lisa-cli's two harnesses, 360 in lisa-core, 699 in lisa-plugin, all
  integration suites — 0 failed), `cargo check -p lisa-plugin --target
  wasm32-wasip1` clean.

## Concerns / open for a human reviewer

- **No test exercises a real live Zellij scheduler actually being killed by
  the guard.** `crate::presence::Machine::look()` does a genuine process/
  session probe against this machine's real Zellij, so a fixture scheduler
  record in a test temp directory reads as "gone" (no matching session is
  really running) rather than "live" — faking liveness convincingly would
  mean actually starting a Zellij session in the test, which felt like the
  wrong cost for this ticket. `stop_for_guard` does not pre-filter to "live"
  records itself; it iterates every recorded scheduler and calls
  `crate::schedulers::run_schedulers` for each, the exact function
  `lisa schedulers --stop` already uses and that already has its own
  passing coverage (including
  `stopping_a_live_scheduler_runs_kill_session_and_forgets_its_record`) for
  the real-kill path. What's *not* independently re-proven here is that a
  guard-triggered call into that function, specifically, reaches a genuinely
  live scheduler in a real `lisa loop` run. Worth a manual spot-check: run a
  real board with a tiny `weekly_token_allowance`, mark it `priority =
  "low"`, spend past 90%, run `lisa spend --guard`, and confirm the pane
  actually stops.
- **Multiple scheduler records on one board.** If a board somehow has more
  than one recorded scheduler and stopping the first errors (e.g. it's the
  session the guard itself is running in — the same refusal
  `lisa schedulers --stop` already has), `stop_for_guard` returns that `Err`
  and never attempts the rest. Ordinary case is one scheduler per board, so
  this wasn't treated as a gap worth partial-success handling for; flagging
  it as a known limitation rather than a silent one.
- **`rail tell`'s `loop-degraded` kind is a fit, not a perfect one** — it was
  written for "running on fewer panes than it was given," and this is a whole
  loop stopping. `spend-autonomy.md` says why a new fact wasn't invented here
  (that belongs to `rail`'s own repository) and why this is the closest
  existing word.
- **`weekly_token_allowance` is duplicated per-project**, same as
  `[scheduling].priority` and every other per-board `.lisa.toml` setting —
  an operator running several boards under one subscription sets the same
  number in each project's own file. Consistent with how the rest of this
  desk's per-project config already works (nothing here unifies config
  across projects), but worth naming since the allowance itself is
  conceptually a desk-wide, not a per-project, fact.
