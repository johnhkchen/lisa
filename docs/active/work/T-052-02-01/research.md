# Research — T-052-02-01 say-it-once

Descriptive map of the two emitters that turn one fact into a column of lines,
the paths that reach them, and every consumer that would feel a change. No
solutions proposed here.

## 1. The activity ring

`State.activity_log: Vec<LoggedActivity>` (lib.rs:806). `LoggedActivity`
(lib.rs:751) is a plugin-local envelope added by the predecessor T-052-01-01:

```rust
struct LoggedActivity { at: std::time::Duration, event: ActivityEvent }
```

- `log_activity(event)` (lib.rs:3341) → `log_activity_at(event, now)`
  (lib.rs:3349), which pushes and trims to `MAX_ACTIVITY_LOG = 100`
  (lib.rs:974). This is the **only** append path; every emitter goes through it.
- `activity_events()` (lib.rs:3363) yields `&ActivityEvent` oldest-first,
  `DoubleEndedIterator`. Used by the state dump and by ~40 unit tests.
- `ActivityEvent` itself lives in `lisa-core` (types.rs:976). It is
  `Serialize + Deserialize + PartialEq + Eq`, and `lisa_core::diagnostics::
  startup_diagnostics` constructs it as a pure clock-free function. T-052-01-01
  documented and re-verified this: **time (and by extension any other
  plugin-only metadata) does not belong inside the enum.**

The ring has exactly two readers:

1. **The feed.** `activity_event_to_ui_entry(&LoggedActivity) -> Option<
   ui::ActivityEntry>` (lib.rs:9102), applied via `filter_map` when building
   `PluginState` (lib.rs:8849–8853). Returning `None` drops an event from the
   feed while leaving it in the ring — the established demotion mechanism.
   `PluginStarted`, `TicketStatusChanged`, and `DagRecomputed` already use it.
2. **The Shift+D dump.** `format_snapshot()` (lib.rs:7926) ends with an
   `=== Activity Log (last 50) ===` section (lib.rs:8130–8139) that renders
   `self.activity_events().rev().take(50)` through
   `format_activity_event(&ActivityEvent) -> String` (lib.rs:7810). This is the
   dump's **only** event record — the epic and ticket both flag that demoting a
   fact out of the feed alone would still leave it here, but demoting it out of
   the ring would erase it.

The feed renders through two functions in `ui.rs`, both switching on
`ui::ActivityType` (ui.rs:266): `render_activity_log` (ui.rs:1022, full feed) and
`render_filtered_activity_log` (ui.rs:1119, alerts-only view). Both map
`ActivityType::PhaseCompleted { ticket_id, phase }` to the literal string
`"{ticket_id} completed {phase.full_name()}"` (ui.rs:1041 and ui.rs:1154 — the
same sentence, duplicated). `PhaseCompleted` is one of the four variants that
survive the alerts-only filter (ui.rs:1130–1138).

## 2. Noise source A — the scheduling skip

`schedule_ready_tickets()` (lib.rs:4929). After the health/permission/pause
early-return and per-ticket completion-mask check, it reaches:

```rust
if self.threads.contains_key(&ticket_id) {
    if is_completed { self.threads.remove(&ticket_id); }
    else {
        self.log_activity(ActivityEvent::Info {
            message: format!("Skipping {}: thread already exists", ticket_id),
        });                                              // lib.rs:4968
        continue;
    }
}
```

`ActivityEvent::Info` projects to `ui::ActivityType::Info` and reaches the full
feed (it is filtered out of alerts-only). The message is the sole occurrence of
this string in the workspace — **no test asserts on it** (grep for `Skipping`
across `crates/` hits only this line plus unrelated workflow prose in
`lisa-cli/data/`).

**Call frequency.** `schedule_ready_tickets()` is invoked from:

| Site | Trigger |
|---|---|
| lib.rs:8498 | permission grant |
| lib.rs:8638 | `Event::Timer` — the 5s poll |
| lib.rs:8656 | `RunCommandResult` arm |
| lib.rs:8691 | `RunCommandResult` arm |
| lib.rs:3147 | after every successful completion |
| (several more) | pane/transition consequences |

So a board with one healthy in-flight thread appends one Info per pass,
indefinitely — at minimum every 5 seconds. Note the two other `continue` arms
below it (global cap, lib.rs:4998; per-provider cap, lib.rs:5010) increment a
local `unscheduled` counter and log **nothing**. That is the precedent: a
recurring capacity no-op is already silent. The skip arm is the outlier.

The decision itself is derivable from state the dump already prints —
`=== Threads ===` and `=== Agent Slots ===` — but only indirectly. There is
currently no record anywhere of "the last scheduling pass considered N ready
tickets and declined M of them, for these reasons."

## 3. Noise source B — the phase-transition pair

Two `ActivityEvent` variants describe one transition:

- `PhaseCompleted { ticket_id, phase }` — `phase` is the phase being **left**.
- `TicketPhaseChanged { ticket_id, old_phase, new_phase }`.

`activity_event_to_ui_entry` maps **both** to `ui::ActivityType::PhaseCompleted`
(lib.rs:9113 and lib.rs:9117–9124), the second using `new_phase`. So one
transition Research→Design renders:

```
✓ T-002 completed Research
✓ T-002 completed Design      ← the phase it just ENTERED
```

Consecutive transitions therefore mint verbatim-identical lines: entering Design
prints the exact sentence leaving it will print one artifact later.

### Emit sites

| Site | Path | Emits |
|---|---|---|
| lib.rs:5581/5585 | `check_artifact_advances()` (lib.rs:5482) | pair |
| lib.rs:6119/6123 | `check_idle_signals()` Implement arm (lib.rs:6047) | pair |
| lib.rs:6225/6229 | `check_idle_signals()` R/D/S/P/Review arm | pair |
| lib.rs:3129/3133 | `finish_successful_completion()` (→ Done) | pair |
| lib.rs:3394 | `rebuild_dag()` phase-change reconciler | `TicketPhaseChanged` alone |
| lib.rs:8425 | operator ticket reset (→ Ready) | `TicketPhaseChanged` alone |

The ticket names three detectors — artifact, idle, completion — matching the
first four rows. The fifth is the piece that makes it worse.

### The reconciler doubles the pair

`poll_tick()` (lib.rs:7606) runs, in order: `check_artifact_advances()` (7638) →
`check_idle_signals()` (7651) → … → `rebuild_dag()` (7684).

`rebuild_dag()` (lib.rs:3369) compares each scanned ticket's phase against
`self.last_phases` (lib.rs:3391–3400) and emits `TicketPhaseChanged` on
mismatch, then refreshes `last_phases` (lib.rs:3413). But the detectors above
advance a ticket by **writing the file** (`ticket::update_ticket_phase`) —
they never touch `last_phases`. So within a single tick an artifact transition
produces:

```
PhaseCompleted{Research}          ← check_artifact_advances
TicketPhaseChanged{Research→Design}  ← check_artifact_advances
TicketPhaseChanged{Research→Design}  ← rebuild_dag, same tick
```

Three ring entries, three feed lines, for one fact. `last_phases` is also
initialized wholesale at load (lib.rs:8599), so the reconciler is quiet on
startup.

### A third shape

`ActivityEvent::ThreadExited` projects to
`ui::ActivityType::PhaseCompleted { phase: Done }` (lib.rs:9109) — "T-001
completed Done". Grep shows `ActivityEvent::ThreadExited` has **no emitter
anywhere in the workspace**; only `format_activity_event` (7819) and this
projection reference it. It is a latent shape, not a live one.
`ActivityEvent::AllTicketsDone` similarly projects to
`PhaseCompleted { ticket_id: "all", phase: Done }` (lib.rs:9140) — that one is
live and is a genuinely distinct fact.

## 4. Consumers that constrain a change

Tests asserting on the **internal event stream** (these see `ActivityEvent`, not
feed lines) — removing an emitter breaks them; changing only the projection does
not:

- lib.rs:11776–11788 — artifact advance asserts **both** `PhaseCompleted` and
  `TicketPhaseChanged` are present.
- lib.rs:14807, 14898–14906, 15508 — idle-path asserts on `PhaseCompleted`
  presence/absence by phase.
- lib.rs:12183, 19527 — assert `TicketPhaseChanged{new_phase: Done}` is
  **absent** before commit success.
- lib.rs:11037–11048, 13021 — assert `activity_event_to_ui_entry` maps
  `PhaseCompleted` and `AllTicketsDone` to `ui::ActivityType::PhaseCompleted`.
- lib.rs:14662 `test_format_snapshot_activity_log_limit`, lib.rs:14707 asserts
  `format_activity_event` renders `"TicketPhaseChanged: T-002 research -> design"`.

Non-test consumers of `TicketPhaseChanged`: none beyond the projection and the
snapshot formatter. It is not read back for scheduling decisions; `rebuild_dag`
returns a bare `changed: bool` computed alongside the emit, and callers use that
bool, not the log.

`ui.rs` fixtures at 1753, 1847–1856, 2732 construct `ActivityType::
PhaseCompleted` directly to drive render tests.

## 5. Constraints inherited from epic and story

- **N4 — no logging framework.** No severity system, no config surface, no
  filtering UI. The ring stays a capped `Vec`.
- **P2 — demotion is not erasure.** Skip decisions must remain reachable in the
  Shift+D dump through their own berth, not the activity ring.
- **Pure projection.** The story's honest boundary for the sibling ticket
  (T-052-02-02, folding) states the UI projection stays pure — no render-time
  rescan. Whatever this ticket does to the projection must not introduce
  cross-entry state at render time.
- **Same-file chain.** E-052/E-053/E-054 all edit `lib.rs` and `ui.rs`; the
  train runs as a strict chain. T-052-02-02 will edit the append path
  (`log_activity_at`) immediately after this ticket, so seams left in that
  function matter.
- **`lisa-core` should stay untouched** unless a consumer needs the change
  inside the enum — T-052-01-01 set that precedent and the wire-shape/`Eq`
  arguments still hold.

## 6. Open questions carried into Design

1. Which of the two variants should own the single feed line — and does the
   surviving sentence name the phase *left* or the phase *entered*? Today's
   `PhaseCompleted` copy says "completed {phase}", which reads correctly only
   for the `old_phase` reading.
2. Where does the duplicate suppression live — at the emit sites (stop emitting
   one variant), at the projection (`None` for one variant), or both? The
   reconciler duplicate (§3) is an *emit*-layer duplicate of the *same*
   variant, so a projection-only fix cannot reach it.
3. What is the vehicle for the demoted skip fact — a dedicated
   last-scheduling-pass struct rendered as its own dump section, or a second
   capped ring the feed never projects? The ticket explicitly leaves this to
   Design.
4. `ThreadExited`'s dead projection: leave, or retire with the third shape?

## 7. Verification surface that already exists

`just check` = fmt + clippy + WASM check + workspace tests. Tests live inline in
`crates/lisa-plugin/src/lib.rs` (`mod tests`, ~10k lines) and
`crates/lisa-plugin/src/ui.rs`. The established pattern for counting feed
entries is `state.activity_log.len()` before/after (lib.rs:20513, 20791) and
`state.activity_events().filter(...).count()`. Both are available for the
acceptance criteria's "count the log before and after the pass".
