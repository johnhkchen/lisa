# Structure — T-052-02-01 say-it-once

Blueprint for the two decisions in `design.md`. Two files touched, both in
`crates/lisa-plugin/src/`. **`lisa-core` is not touched** — no `ActivityEvent`
variant is added, removed, or reshaped, so the serialized wire form, the `Eq`
derives, and `diagnostics::startup_diagnostics`'s clock-free construction are
all unaffected.

| File | Change |
|---|---|
| `crates/lisa-plugin/src/lib.rs` | modified — new types, new state fields, new method, 5 emit-site edits, 3 projection edits, 1 dump section, tests |
| `crates/lisa-plugin/src/ui.rs` | **untouched** |

`ui.rs` staying untouched is a deliberate outcome of keeping the feed sentence
as-is (design Part B). If a later step finds itself editing `ui.rs`, the
sentence decision has drifted and should be revisited, not patched.

---

## 1. New types in `lib.rs`

Placed next to `LoggedActivity` (lib.rs:751), which is the existing home for
plugin-local activity metadata.

```rust
/// Why a ready ticket did not receive a thread on a scheduling pass.
///
/// Recorded rather than logged: a scheduling decline is state, not news. The
/// activity feed carries facts an operator did not already know; a healthy
/// in-flight thread declining to be double-spawned is neither new nor
/// actionable, so it lives here and surfaces in the Shift+D dump.
#[derive(Debug, Clone, PartialEq, Eq)]
enum DeclineReason {
    /// A durable Done record masks this ticket's ready state.
    DurableDoneMasked,
    /// A live thread already owns this ticket. (Demoted from the feed by
    /// T-052-02-01; this was "Skipping {}: thread already exists".)
    ThreadAlreadyRunning,
    /// The global concurrency ceiling is saturated.
    GlobalCapReached { running: usize, max: usize },
    /// The resolved provider's sub-cap is saturated.
    ProviderCapReached { agent: AgentClient },
    /// No compatible or recyclable pane slot is free.
    NoSlotAvailable,
    /// The only candidate pane is blocked on an agent question.
    PaneAwaitingQuestion { pane_id: u32 },
    /// An admitted ticket failed to launch. The detail is also logged as an
    /// `Error` — failures stay in the feed.
    SpawnFailed { detail: String },
}

impl DeclineReason {
    /// One-line dump rendering.
    fn describe(&self) -> String { /* match → String */ }
}

/// The most recent `schedule_ready_tickets` pass that actually ran.
///
/// Last-write-wins: "why didn't X spawn?" is a question about the current
/// pass, not about history, so this is overwritten rather than appended.
/// Bounded by construction — at most one entry per ready ticket.
#[derive(Debug, Clone, Default)]
struct SchedulingPass {
    /// Wall clock at the end of the pass, seconds since the Unix epoch.
    at: std::time::Duration,
    /// Ready candidates considered.
    ready: usize,
    /// Tickets that received a thread on this pass.
    spawned: Vec<TicketId>,
    /// Tickets that did not, and why — in candidate order.
    declined: Vec<(TicketId, DeclineReason)>,
}
```

`DeclineReason::describe` renderings (exact strings, so tests can assert them):

| Variant | String |
|---|---|
| `DurableDoneMasked` | `durable Done record masks ready state` |
| `ThreadAlreadyRunning` | `thread already running` |
| `GlobalCapReached{running, max}` | `global thread cap reached ({running}/{max})` |
| `ProviderCapReached{agent}` | `{agent} provider cap reached` |
| `NoSlotAvailable` | `no compatible pane slot free` |
| `PaneAwaitingQuestion{pane_id}` | `pane #{pane_id} is awaiting an answer` |
| `SpawnFailed{detail}` | `spawn failed: {detail}` |

## 2. New `State` fields

Both added to `State` (lib.rs:776). `State` derives `Default`; `Option` and
`HashMap` both default correctly, so no `impl Default` edit is needed.

```rust
/// Record of the most recent scheduling pass that ran to completion.
/// Rendered in the Shift+D dump; never projected to the activity feed.
last_scheduling_pass: Option<SchedulingPass>,

/// The last phase transition logged for each ticket, keyed by ticket ID.
///
/// Guards against two advance detectors observing the same transition in one
/// tick and each minting its own feed line. Holds only the *previous*
/// transition, not a set: a ticket that is reset and re-run legitimately
/// repeats `Research -> Design`, and that repeat is real news.
logged_transitions: HashMap<TicketId, (Phase, Phase)>,
```

## 3. New method: the transition choke point

Placed immediately after `log_activity_at` (lib.rs:3357), with
`activity_events` following as today.

```rust
/// Log one phase transition, exactly once.
///
/// Emits the `PhaseCompleted` + `TicketPhaseChanged` pair the advance
/// detectors have always emitted, unless this exact transition was the last
/// one logged for this ticket — in which case a second detector is observing
/// a transition the first already reported, and this is a no-op.
///
/// Feed cost is one line: `TicketPhaseChanged` no longer projects (see
/// `activity_event_to_ui_entry`), so the pair renders as the single
/// "{ticket} completed {from}" headline while both events remain in the ring
/// and in the state dump.
fn log_phase_transition(&mut self, ticket_id: &str, from: Phase, to: Phase) {
    if self.logged_transitions.get(ticket_id) == Some(&(from, to)) {
        return;
    }
    self.logged_transitions
        .insert(ticket_id.to_string(), (from, to));
    self.log_activity(ActivityEvent::PhaseCompleted { .. });
    self.log_activity(ActivityEvent::TicketPhaseChanged { .. });
}
```

Ordering note: the pair is emitted in the same order as today
(`PhaseCompleted` then `TicketPhaseChanged`), so the dump's chronological tail
is unchanged for anyone reading it.

## 4. Emit-site edits

Each of these replaces two adjacent `self.log_activity(...)` calls with one
`self.log_phase_transition(...)` call. No surrounding control flow moves.

| Site | Detector | Call becomes |
|---|---|---|
| lib.rs:5581–5589 | `check_artifact_advances` | `self.log_phase_transition(&ticket_id, current_phase, next_phase)` |
| lib.rs:6119–6127 | `check_idle_signals`, Implement arm | `self.log_phase_transition(&ticket_id, Phase::Implement, Phase::Review)` |
| lib.rs:6225–6233 | `check_idle_signals`, R/D/S/P/Review arm | `self.log_phase_transition(&ticket_id, current_phase, next_phase)` |
| lib.rs:3129–3137 | `finish_successful_completion` | `self.log_phase_transition(ticket_id, pending.prior_phase, Phase::Done)` |

**Unchanged emit sites:**

- lib.rs:3394 `rebuild_dag` reconciler — keeps its bare `TicketPhaseChanged`.
  Design Part B records why it is not routed through the choke point: two of
  its callers rebuild the DAG precisely *because* a completion failed to
  verify, and a "completed" claim from those branches would be a lie. After the
  projection edit it costs zero feed lines, which is the whole problem it
  contributed.
- lib.rs:8425 `reset_ticket` — keeps its bare `TicketPhaseChanged`, and gains
  two lines:
  ```rust
  self.logged_transitions.remove(&tid);       // a retry's transitions are fresh news
  self.log_activity(ActivityEvent::Info {
      message: format!("Reset {} to Ready for retry", tid),
  });
  ```
  The `Info` replaces the feed line the projection edit removes. Net feed lines
  for the reset path: one before, one after — but `"T-001 completed Ready"`
  becomes `"T-001 reset to Ready for retry"`.

## 5. Projection edits — `activity_event_to_ui_entry` (lib.rs:9102)

Three arms change; everything else is untouched.

```rust
ActivityEvent::ThreadExited { .. } => return None,        // was PhaseCompleted{Done}
ActivityEvent::PhaseCompleted { .. } => /* unchanged */,  // the one feed line
ActivityEvent::TicketPhaseChanged { .. } => return None,  // was PhaseCompleted{new}
```

`ThreadExited` has no emitter anywhere in the workspace (research §3); its arm
was a latent third "completed Done" shape. It is demoted, not deleted — the
variant, its `format_activity_event` rendering, and its dump line all remain.

After this edit, `ActivityEvent::PhaseCompleted` and
`ActivityEvent::AllTicketsDone` are the only two events that can produce a
`ui::ActivityType::PhaseCompleted`. The doc comment on
`activity_event_to_ui_entry` gains a sentence stating that invariant, because
it is the thing a future edit could silently break.

## 6. Skip-emit removal and pass recording — `schedule_ready_tickets` (lib.rs:4929)

- **Delete** the `ActivityEvent::Info { "Skipping {}: thread already exists" }`
  emit (lib.rs:4968–4970). The `continue` stays.
- **Add** `let mut pass = SchedulingPass::default();` after
  `let ready = self.dag.get_ready_tickets();`, and `pass.ready = ready.len();`.
- **Add** one `pass.declined.push((ticket_id.clone(), reason))` before each of
  the six admission `continue`s, and the two failure `continue`s.
- **Add** `pass.spawned.push(ticket_id.clone())` at the end of a successful
  spawn iteration.
- **Add** at the end of the function:
  ```rust
  pass.at = /* epoch duration, via provenance::system_time_to_epoch */;
  self.last_scheduling_pass = Some(pass);
  ```

The existing `unscheduled` counter and everything it feeds are left alone —
`pass.declined.len()` is not a substitute for it and the two are not merged in
this ticket.

**Early returns are not recorded.** The guard at lib.rs:4935 (unhealthy
journal, permissions ungranted, slots undiscovered, paused) returns before a
pass exists, leaving the previous record in place. This is why `SchedulingPass`
carries `at` — the dump reader can see the record is stale. The dump section
labels it accordingly.

## 7. Dump section — `format_snapshot` (lib.rs:7926)

Inserted **between** `=== Last Known Health ===` (ends lib.rs:8128) and
`=== Activity Log (last 50) ===` (lib.rs:8130). It sits with the other
last-known-state sections and immediately above the chronological tail, so a
reader scanning for "why didn't X spawn?" meets it before the ring.

```
=== Last Scheduling Pass ===
at:        1784510049 (unix epoch, 12s ago)
ready:     3
spawned:   T-002-01-01
declined:  2
  T-002-01-02   thread already running
  T-002-01-03   global thread cap reached (2/2)
```

Empty-state line when `last_scheduling_pass` is `None`:
`(no scheduling pass has run)`. Empty `spawned` renders `(none)`; empty
`declined` renders `declined:  0` with no detail rows.

`format_snapshot` already computes `epoch_secs` at its top (lib.rs:7931) — the
"Ns ago" suffix reuses it rather than reading the clock again.

## 8. Test plan (detail in `plan.md`)

New tests all land in the existing `mod tests` in `lib.rs`. Named to state the
claim, matching the file's convention:

1. `scheduling_pass_over_live_threads_appends_no_feed_entries` — AC 1.
2. `artifact_advance_yields_one_feed_line` — AC 2.
3. `idle_advance_yields_one_feed_line` — AC 2.
4. `completion_advance_yields_one_feed_line` — AC 2.
5. `two_detectors_observing_one_transition_yield_one_feed_line` — AC 2.
6. `declined_spawn_survives_in_the_scheduling_pass_record` — AC 3.
7. `two_tickets_completing_a_phase_yield_two_distinct_lines` — AC 4.
8. `ticket_phase_changed_no_longer_projects_to_the_feed` — pins the invariant
   from §5 directly.
9. `reset_ticket_clears_the_transition_memory` — pins §4's `remove`.

**Existing tests expected to need no change**: all six internal-stream asserts
in research §4 keep passing, because `log_phase_transition` emits the same pair
in the same order. This is the load-bearing reason B4 was chosen over B1 and
should be verified early — if any of them break, the choke point is not
faithfully reproducing the old emits.

## 9. Ordering of changes

1. §1 + §2 (types and fields) — compiles standalone, unused-warning only.
2. §3 + §4 (choke point and emit sites) — internal-stream tests must stay green
   here, before any projection change.
3. §5 (projection) + test 8.
4. §6 + §7 (skip demotion, pass record, dump section) + tests 1 and 6.
5. Remaining tests, `just check`.

Steps 2 and 3 are separable and separately verifiable, which is why they are not
one commit: step 2 changes *how many events are emitted*, step 3 changes *how
many lines they render as*. Diagnosing a regression is much cheaper when those
two are not tangled.
