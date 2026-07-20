# Plan — T-052-02-01 say-it-once

Five steps, five `lisa commit-ticket` units. Every step compiles and every step
leaves `cargo test --workspace` green — steps 2 and 3 are deliberately split so
"how many events are emitted" and "how many lines they render as" fail
separately if they fail at all.

Ticket-owned paths: `crates/lisa-plugin/src/lib.rs` only. `ui.rs` should not
appear in any `--include`; if it does, the sentence decision in `design.md`
drifted and the step stops for reconsideration rather than being patched.

---

## Step 1 — types and state fields

**Edit** `crates/lisa-plugin/src/lib.rs`:

- Add `DeclineReason` (with `describe()`) and `SchedulingPass` next to
  `LoggedActivity` (~lib.rs:751), per `structure.md` §1.
- Add `last_scheduling_pass: Option<SchedulingPass>` and
  `logged_transitions: HashMap<TicketId, (Phase, Phase)>` to `State` (~776).

**Verify:** `cargo build -p lisa-plugin` compiles. `cargo clippy` will flag the
new types as dead code — expected and resolved by step 4, so this step carries
`#[allow(dead_code)]` only if clippy is denied at build time. Check first:
`just check` runs clippy with the workspace's configured lint level; if it
denies warnings, fold step 1 into step 4 rather than leaving an `allow` behind.

**Commit:** `feat(plugin): add scheduling-pass and transition-memory state`

---

## Step 2 — the transition choke point

**Edit** `crates/lisa-plugin/src/lib.rs`:

- Add `log_phase_transition(&mut self, ticket_id: &str, from: Phase, to: Phase)`
  after `log_activity_at` (~3357).
- Replace the emit pairs at the four detector sites (5581, 6119, 6225, 3129)
  with a single call each, per `structure.md` §4.
- `reset_ticket` (~8425): add `self.logged_transitions.remove(&tid);`.

**Verify — this is the load-bearing step.** The choke point must emit the *same
pair, in the same order*, so every existing internal-stream assertion keeps
passing untouched:

```
cargo test -p lisa-plugin -- \
  test_check_artifact_advances \
  test_idle_signal \
  journal_seal \
  auto_complete
```

then the full `cargo test --workspace`. If any of lib.rs:11776, 12183, 14807,
14898, 15508, or 19527 breaks here, the choke point is not faithful and the fix
is in `log_phase_transition`, not in the test.

**New test** — `two_detectors_observing_one_transition_yield_one_feed_line`
(AC 2, the fixture the ticket calls out):

Build a state with a ticket at Research, a running thread, and `research.md`
admitted. Call `check_artifact_advances()` (advances Research→Design, logs the
pair), then drive `check_idle_signals()` against the *same* transition by
resetting `thread.current_phase` to `Research` before the second call — the
shape of a second detector observing a transition the first already reported.
Assert exactly one `PhaseCompleted{Research}` in `activity_events()`.

**Commit:** `fix(plugin): emit one phase transition once across all detectors`

---

## Step 3 — the projection

**Edit** `crates/lisa-plugin/src/lib.rs`, `activity_event_to_ui_entry` (~9102):

- `TicketPhaseChanged` → `return None`
- `ThreadExited` → `return None`
- Extend the doc comment with the invariant: `PhaseCompleted` and
  `AllTicketsDone` are the only events that produce a feed
  `ui::ActivityType::PhaseCompleted`.

**Edit** `reset_ticket` (~8425): add the `Info` line
`"Reset {tid} to Ready for retry"`, replacing the `"completed Ready"` line the
projection edit removes.

**New tests:**

| Test | Claim |
|---|---|
| `ticket_phase_changed_no_longer_projects_to_the_feed` | `activity_event_to_ui_entry` returns `None` for `TicketPhaseChanged` and `ThreadExited`, and `Some` for `PhaseCompleted` |
| `artifact_advance_yields_one_feed_line` | AC 2, artifact path |
| `idle_advance_yields_one_feed_line` | AC 2, idle path |
| `completion_advance_yields_one_feed_line` | AC 2, completion path |
| `two_tickets_completing_a_phase_yield_two_distinct_lines` | AC 4, negative fixture |

**How the feed-line tests count.** The unit of measure is *feed lines*, not ring
entries, so they count through the projection rather than over
`activity_events()`:

```rust
fn feed_lines(state: &State) -> Vec<String> {
    state.activity_log.iter()
        .filter_map(activity_event_to_ui_entry)
        .filter_map(|e| match e.activity {
            ui::ActivityType::PhaseCompleted { ticket_id, phase } =>
                Some(format!("{ticket_id} completed {}", phase.full_name())),
            _ => None,
        })
        .collect()
}
```

A test helper in `mod tests`. `artifact_advance_yields_one_feed_line` asserts
`feed_lines(&state) == ["T-001 completed Research"]` — one element, exact
string. That form catches both a second line *and* a wrong sentence, and it is
the same assertion shape the AC 4 negative fixture needs (two elements, both
distinct, one per ticket).

For the completion path, reuse the existing `finish_successful_completion`
fixture scaffolding (lib.rs:12183 / 19527 neighborhoods build the pending
completion and the seal receipt already).

**Verify:** `cargo test --workspace`. Existing render fixtures in `ui.rs`
construct `ui::ActivityType` directly and are unaffected.

**Commit:** `fix(plugin): mint one feed line per phase transition`

---

## Step 4 — skip demotion and the pass record

**Edit** `crates/lisa-plugin/src/lib.rs`, `schedule_ready_tickets` (~4929), per
`structure.md` §6:

- Delete the `"Skipping {}: thread already exists"` Info emit (4968–4970).
- Build a `SchedulingPass`, push a `DeclineReason` at each of the six admission
  `continue`s and the two failure `continue`s, push to `spawned` on success,
  and store it at the end.

**New tests:**

| Test | Claim |
|---|---|
| `scheduling_pass_over_live_threads_appends_no_feed_entries` | AC 1 |
| `declined_spawn_survives_in_the_scheduling_pass_record` | AC 3 |
| `reset_ticket_clears_the_transition_memory` | step 2's `remove`, pinned |

`scheduling_pass_over_live_threads_appends_no_feed_entries` is the AC's
before/after count, using the file's established idiom (lib.rs:20791):

```rust
let before = state.activity_log.len();
state.schedule_ready_tickets();
assert_eq!(state.activity_log.len(), before);
state.schedule_ready_tickets();        // second pass — still silent
assert_eq!(state.activity_log.len(), before);
```

Two passes, not one: the defect is *recurrence*, and a one-pass assertion would
pass against an implementation that merely logged the skip once per ticket.

`declined_spawn_survives_in_the_scheduling_pass_record` asserts the same state
afterwards has `last_scheduling_pass` with the ticket present in `declined`
carrying `DeclineReason::ThreadAlreadyRunning` — the P2 half of the ticket:
the fact survives, only the feed stops carrying it.

**Verify:** `cargo test --workspace`. Watch specifically for tests that assert
on total `activity_log` length after a scheduling pass (lib.rs:12101, 13013,
13203, 13955, 19285, 21444, 21471 assert emptiness) — removing an emit can only
*help* those, but a test that asserted a specific non-zero count would need its
expectation corrected, and that correction must be justified in `progress.md`,
not made silently.

**Commit:** `fix(plugin): demote the scheduling skip out of the activity feed`

---

## Step 5 — the dump section

**Edit** `crates/lisa-plugin/src/lib.rs`, `format_snapshot` (~8128): insert
`=== Last Scheduling Pass ===` between `=== Last Known Health ===` and
`=== Activity Log (last 50) ===`, per `structure.md` §7.

**New test** — `state_dump_answers_why_a_ticket_did_not_spawn`: run a pass with
a live thread, call `format_snapshot()`, assert the output contains
`"=== Last Scheduling Pass ==="` and a line matching the declined ticket and
`"thread already running"`. This is the AC 3 evidence quoted in `progress.md`.

Also assert the empty state: a fresh `State::default()` snapshot contains
`"(no scheduling pass has run)"`.

**Verify:** full gate.

```
just check
```

fmt, clippy, WASM check, workspace tests. `just check` exit code is the gate —
per the standing lesson, judged by exit code, never by grepping its output.

**Commit:** `feat(plugin): surface the last scheduling pass in the state dump`

---

## Testing strategy summary

**Unit tests only.** Every acceptance criterion is reachable from `mod tests` in
`lib.rs` against a `State` built over a `tempfile::tempdir()` ticket directory —
the pattern used by ~50 existing tests that call `schedule_ready_tickets()`
directly. No integration harness is needed and none is added.

**Coverage map:**

| AC | Test(s) | Step |
|---|---|---|
| 1 — pass over live threads appends zero entries | `scheduling_pass_over_live_threads_appends_no_feed_entries` | 4 |
| 2 — one transition, one line, all three paths + double-observe | `artifact_/idle_/completion_advance_yields_one_feed_line`, `two_detectors_observing_one_transition_yield_one_feed_line` | 2, 3 |
| 3 — dump still answers "why didn't X spawn?" | `declined_spawn_survives_in_the_scheduling_pass_record`, `state_dump_answers_why_a_ticket_did_not_spawn` | 4, 5 |
| 4 — two tickets, two distinct lines | `two_tickets_completing_a_phase_yield_two_distinct_lines` | 3 |
| 5 — `just check` green | the gate itself | 5 |

**Known gaps to declare in `review.md` rather than paper over:**

- Nothing here exercises the *rendered* feed end-to-end through
  `ui::render_activity_log`; the tests stop at the projection. That boundary is
  deliberate (the sentence is unchanged, so `ui.rs` is untouched) but it means
  a regression in `render_activity_log`'s copy would not be caught by this
  ticket's tests. `ui.rs`'s own render fixtures still cover it.
- The `rebuild_dag` reconciler's `TicketPhaseChanged` is verified to produce
  zero feed lines only indirectly, via the one-line-per-transition tests that
  run a full `poll_tick`-shaped sequence. A direct test of the reconciler in
  isolation is not planned.

## Risks

1. **The choke point is not faithful** — some existing internal-stream test
   breaks in step 2. Mitigation: step 2 is committed separately and its
   verification is the full workspace suite before any projection change lands.
2. **A decline arm is missed** in step 4, so the dump's account of a pass is
   incomplete and quietly wrong. Mitigation: `structure.md` §1 enumerates the
   arms against exact line numbers; step 4 re-greps `continue` inside
   `schedule_ready_tickets` and confirms each one is either recorded or is a
   loop-carried non-decision.
3. **Clippy dead-code in step 1** blocks the commit. Mitigation stated in step
   1: check the workspace lint level first and fold steps 1 and 4 if needed,
   rather than leaving an `#[allow]` in the tree.
