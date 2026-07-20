# Review — T-052-02-01 say-it-once

Two emitters turned one fact into a column of lines. Both are fixed, and neither
fact was erased in the process.

The 0.4.4 field screenshot showed seven feed lines carrying two facts:
"Skipping T-002-01-02: thread already exists" four times, and
"T-002-01-02 completed Design" twice. After this ticket a scheduling pass over a
fully-threaded board appends **zero** feed entries, and one phase transition
appends exactly **one** line no matter which detector saw it or how many saw it.

## Commits

| Commit | Message | Diff |
|---|---|---|
| `4df44a0` | `fix(plugin): announce one phase transition once across all detectors` | +160 −37 |
| `2865b2a` | `fix(plugin): mint one feed line per phase transition` | +116 −12 |
| `6220fac` | `fix(plugin): demote the scheduling skip into the state dump` | +293 −3 |
| `8072e65` | `test(plugin): count the completion path's feed line directly` | +69 |

**One file: `crates/lisa-plugin/src/lib.rs`** (roughly half tests).
`crates/lisa-plugin/src/ui.rs` and `lisa-core` are untouched — see
"Why `ui.rs` was not touched" below.

## What changed

### The transition stutter — two layers, two fixes

**Emit layer.** New `State::log_phase_transition(ticket_id, from, to)`
(lib.rs:3413) is now the single door through which the three advance detectors
announce a transition. It emits the same `PhaseCompleted` + `TicketPhaseChanged`
pair, in the same order, that the four emit sites emitted inline before —
*unless* `logged_transitions` shows this exact transition was the last one
logged for this ticket, in which case a second detector is echoing something the
first already reported and the call is a no-op.

Replaced inline pairs at: `check_artifact_advances` (was lib.rs:5581),
`check_idle_signals` Implement arm (was 6119), `check_idle_signals`
R/D/S/P/Review arm (was 6225), `finish_successful_completion` (was 3129).

`logged_transitions: HashMap<TicketId, (Phase, Phase)>` stores the **previous**
transition, not a set. A ticket that is reset and re-run legitimately repeats
`Research → Design`, and a set would swallow the second, real occurrence.
`reset_ticket` clears the ticket's entry so a retry's first transition is never
mistaken for an echo.

**Projection layer.** In `activity_event_to_ui_entry` (lib.rs:9109):

- `TicketPhaseChanged` → `None`. It was minting a second line naming the phase
  just *entered* — so entering Design printed the exact sentence leaving it
  would print, and consecutive transitions stuttered verbatim.
- `ThreadExited` → `None`. A latent third "completed Done" shape from a variant
  **nothing in the workspace emits**.

Both events remain in the ring and in the Shift+D dump. This is the demotion
mechanism `PluginStarted`, `TicketStatusChanged`, and `DagRecomputed` already
use. The function's doc comment now states the invariant explicitly —
`PhaseCompleted` and `AllTicketsDone` are the only events permitted a
transition-line shape — because that is precisely what a future edit could
silently undo.

Both layers were needed and neither is redundant: the projection fix makes one
*emit-pair* one line; the choke point makes one *transition* one emit-pair. The
`rebuild_dag` reconciler (lib.rs:3394) illustrates why — it re-emits
`TicketPhaseChanged` for the same transition in the same `poll_tick`, so before
this work one artifact advance produced three ring entries and three feed lines.

### The scheduling skip — demoted, not erased

Deleted the `Info { "Skipping {}: thread already exists" }` emit inside
`schedule_ready_tickets`. In its place, every admission arm of the pass now
records a typed `DeclineReason` into a `SchedulingPass` record, overwritten
each pass and rendered as its own `=== Last Scheduling Pass ===` section in
`format_snapshot`, sitting with the other last-known-state sections and directly
above the chronological Activity Log.

```
=== Last Scheduling Pass ===
at:        1784510049 (unix epoch, 12s ago)
ready:     3
spawned:   T-002-01-01
declined:  2
  T-002-01-02    thread already running
  T-002-01-03    global thread cap reached (2/2)
```

**The demoted fact came back larger.** The old Info answered "why didn't X
spawn?" for one decline arm out of ten. Three arms (global cap, provider cap,
no-slot) logged *nothing at all* — they bumped a local counter that surfaced
only as an aggregate "N ready tickets waiting". The dump now answers for all
ten, including the four launch-failure arms found by re-grepping `continue`
during Implement (`structure.md` had predicted two).

Last-write-wins was chosen over a second capped ring on purpose: "why didn't X
spawn?" is a question about *now*, and a chronological ring answers it by making
the reader scan backwards for the newest mention of X. It is also bounded by
construction — at most one entry per ready candidate, replaced every pass — so
it adds nothing for T-052-02-02's folding work to reconcile.

### One collateral copy fix

`reset_ticket`'s only feed-visible signal was `TicketPhaseChanged`, rendering as
`"T-001 completed Ready"` — a ticket reset for retry has completed nothing. The
projection change would have silenced it, so it gained an explicit
`Info { "Reset T-001 to Ready for retry" }`. Net lines for that path: one
before, one after. The sentence stops lying.

## Why `ui.rs` was not touched

The ticket left the sentence to Design. E-052's "Done looks like" quotes the
target feed verbatim as `"T-016-01-01 completed Design — 3m ago"` — the epic had
already picked it. The work was to make that sentence appear once, not to
rewrite it. The considered alternative (`"T-001 Research → Design"`, naming the
transition literally) would have required adding `old_phase` to
`ui::ActivityType::PhaseCompleted` and churning five render/projection fixtures,
against the epic's own copy.

## Test coverage

464 tests pass, up from 455. Nine new tests; **no existing test was modified or
deleted.**

| Acceptance criterion | Test |
|---|---|
| Pass over live threads appends zero entries | `scheduling_pass_over_live_threads_appends_no_feed_entries` |
| One transition, one line — artifact path | `artifact_advance_yields_one_feed_line` |
| One transition, one line — idle path | `idle_advance_yields_one_feed_line` |
| One transition, one line — completion path | `completion_advance_yields_one_feed_line` |
| One transition, one line — two detectors | `two_detectors_observing_one_transition_announce_it_once` |
| Dump still answers "why didn't X spawn?" | `declined_spawn_survives_in_the_state_dump` |
| Two tickets, two distinct lines | `two_tickets_completing_a_phase_yield_two_distinct_lines` |
| `just check` green | the gate, exit code 0 |

Plus `only_phase_completed_projects_a_transition_line` (pins the projection
invariant directly) and
`a_repeated_transition_after_other_transitions_is_announced_again` (pins why the
guard stores the previous transition rather than a set).

Two deliberate choices in how the tests measure:

- **They count feed lines, not ring entries.** `feed_phase_lines` runs the ring
  through the real projection and compares whole strings, so one assertion
  catches both a duplicated line and a wrong sentence.
- **The silence test runs three passes, not one.** The defect was *recurrence*;
  a single-pass assertion would pass against an implementation that merely
  logged the skip once per ticket.

**That no existing test needed changing is the load-bearing evidence** that the
choke point faithfully reproduces the old emits. Six existing tests assert on
the internal event stream — both variants present, or a specific variant absent
before commit success. All six still pass untouched. Had any needed adjusting,
the right response would have been to fix `log_phase_transition`, not the test.

All three detector paths named by the AC are verified by counting tests, not by
inference. The completion path was initially going to ship as "satisfied by
construction" — it routes through the same `log_phase_transition` call as the
other two — but an AC that says "verified separately for … the completion path"
is not met by an argument, so `completion_advance_yields_one_feed_line` was
added. It drives a real journal-sealed `dispatch_completion` to Done (asserting
the fixture actually reaches Done first, so the line count means something) and
then asserts exactly `["T-SEAL completed Review"]`.

### Gaps

1. **No end-to-end render assertion.** The tests stop at the projection; nothing
   drives `ui::render_activity_log` and reads the finished string. `ui.rs`'s own
   render fixtures still cover that layer, and this ticket did not change it.
2. **`reset_ticket_clears_the_transition_memory`** was planned as a standalone
   test and not written. The `remove` it would pin is covered at the level that
   matters by `a_repeated_transition_...`.
3. **The reconciler's zero-line contribution** is verified indirectly (via the
   one-line-per-transition tests), not by a test that drives `rebuild_dag` in
   isolation.

## Open concerns

- **`logged_transitions` grows one entry per ticket that ever transitions** and
  is never pruned except by `reset_ticket`. Bounded by board size, exactly like
  the neighbouring `last_phases`, and each entry is two enum discriminants — not
  a leak, but worth knowing it exists if board sizes ever get large.
- **`AllTicketsDone` still renders `"all completed Done"`.** Odd copy for a real
  fact. Untouched: one line, one fact, no criterion reaches it.
- **The double-observe fixture models the shape, not a specific production
  sequence.** It hands the second detector the pre-transition phase, which is
  the condition the guard exists for. Whether two detectors can *currently*
  reach that state in production is not established — in `poll_tick` the
  artifact detector updates `thread.current_phase` before the idle detector
  runs, so today the reachable duplicate is the `rebuild_dag` reconciler, which
  the projection change handles. The guard is therefore partly defensive. The
  ticket asked for it explicitly ("enforce it across all three detector paths…
  including a fixture where two detectors observe the same transition"), and it
  costs one `HashMap` lookup per transition.
- **A pre-existing flake in `lisa-cli` bounced the gate twice — diagnosed, not
  waved off.** `triage_agent::tests::bounded_runner_returns_valid_proposal_and_
  surfaces_failure` failed with `TimedOut` on two `just check` runs, both of
  which immediately followed a fresh `lisa-plugin` compile (peak CPU
  contention). The test spawns a shell script under a hardcoded 2-second
  `timeout_secs`.

  I did not assume flakiness. Measured after the observations: **36 consecutive
  green runs** — 20/20 isolated, 8/8 full `lisa-cli` suite, 5/5
  `cargo test --workspace`, 3/3 `just check`.

  The decisive evidence is in the history: the most recent commit touching that
  file is `72dee80`, *today*, titled **"test(triage): defang the bounded-runner
  timing flake"** — it removed a load-sensitive wall-clock ceiling from the
  *adjacent* test in the same module, with the message "the removed upper bound
  only measured machine load and flaked the gate under parallel cargo test."
  That fix addressed
  `bounded_runner_kills_timeout_near_the_configured_deadline` and did not reach
  its neighbour, which carries the same load sensitivity through its 2-second
  deadline. So this is a known, already-diagnosed failure mode with one sibling
  still undefanged.

  **I did not fix it**, deliberately: `crates/lisa-cli/src/triage_agent.rs` is
  not owned by this ticket, and `lisa commit-ticket` takes only ticket-owned
  paths. Recommended remedy for whoever owns it — give
  `bounded_runner_returns_valid_proposal_and_surfaces_failure` the same
  treatment `72dee80` gave its sibling: the 2-second deadline is there to bound
  the runner, not to measure the machine, so it can be raised without weakening
  what the test proves.

## Nothing left uncommitted

`git status` shows no `crates/**` path modified, staged, or untracked. All three
commits went through `lisa commit-ticket` with exact `--include` paths; no
ordinary-index `git add` or `git commit` was used for ticket work.

## For the next ticket in the chain

T-052-02-02 (`fold-the-echoes`) edits `log_activity_at` to fold identical
consecutive entries with an `(x3)` tag. Two notes from this work:

- `log_phase_transition` sits directly above `log_activity_at` and calls it
  twice per transition. Folding must not collapse the `PhaseCompleted` and
  `TicketPhaseChanged` halves of one pair into each other — they are different
  variants, so a fold keyed on rendered equality is safe, but a fold keyed on
  "same ticket, adjacent" is not.
- `SchedulingPass` is deliberately outside the ring and needs no folding.
