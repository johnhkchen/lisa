# T-022-02 Research — Error Signal Consumer

## Ticket in one line

The scheduler consumes `.heartbeat` / `.stopped` / `.cleared` / `.idle` / `.awaiting`
signal files today, but has **no `.error` consumer**. The Codex wrapper (T-023-01)
will emit `pane-<id>.error` on `turn.failed` / non-zero exit; the scheduler must
fail the thread and release the slot promptly instead of waiting ~40 minutes for the
silence clock.

## The signal-consumption pattern (the thing to copy)

All signal consumers live in `crates/lisa-plugin/src/lib.rs` and share one shape:
read `self.signal_dir`, match a filename suffix, **delete the file immediately**,
parse `pane-<id>`, act. `self.signal_dir` is `.lisa/signals/` under `/host/`
(`lib.rs:208-209`). The flow is one-directional — hooks write, the plugin reads and
deletes (`data/hooks-guide.md:12-17`). Body content is currently ignored; presence
is the signal.

Existing consumers, all methods on `State`:

- `check_heartbeat_signals` (`lib.rs:812`): `pane-<id>.heartbeat` → `bump_pane_activity`,
  clears attention/awaiting debounce.
- `check_awaiting_signals` (`lib.rs:855`): `pane-<id>.awaiting` → inserts into
  `awaiting_human`, suppresses injection.
- `check_idle_signals` (`lib.rs:897`): `pane-<id>.idle` → phase advance / idle alert.
  Note the pattern of resolving `pane_id → ticket` via `agent_slots`.
- `check_transition_signals` (`lib.rs:1112`): `pane-<id>.stopped` / `.cleared` →
  drive the `TransitionState` handshake.

Each parses via `strip_prefix("pane-")` then `strip_suffix(".<ext>")` then
`parse::<u32>()`. This is the exact idiom the `.error` consumer must follow.

## poll_tick ordering (where `.error` must slot in)

`poll_tick` (`lib.rs:1717`) runs consumers in a deliberate order:

1. `check_heartbeat_signals` — refresh clocks first.
2. `check_awaiting_signals` — flag question-blocked panes before any injector.
3. `check_artifact_advances`
4. `check_idle_signals`
5. `check_transition_signals` — `.stopped` / `.cleared`.
6. `check_transition_timeouts` — **force-advance** stalled transitions.
7. `check_review_timeouts`, `evaluate_health`.
8. `check_session_timeouts`, `detect_stale_threads` — silence-based reclaim.
9. `rebuild_dag`, done-ticket sweep, `schedule_ready_tickets`.

The ticket requires `.error` handling to precede transition timeouts (step 6) so a
failed pane is not force-advanced by the fallback. The natural insertion point is
between step 5 (`check_transition_signals`) and step 6 (`check_transition_timeouts`),
at `lib.rs:1734-1737`.

## What "fail + release + alert" already looks like (the reclaim template)

Two existing reclaim paths do exactly the state mutation the `.error` consumer needs,
just triggered by silence instead of an explicit signal:

`check_session_timeouts` reclaim (`lib.rs:1616-1629`):

```rust
if let Some(thread) = self.threads.get_mut(&ticket_id) { thread.fail(); }
self.release_slot_for_ticket(&ticket_id);
self.threads.remove(&ticket_id);
self.timeout_alerts.push((ticket_id.clone(), elapsed_secs, phase));
self.log_activity(ActivityEvent::SessionTimedOut { ... });
```

`detect_stale_threads` reclaim (`lib.rs:1661-1674`): identical shape, logs
`ActivityEvent::Error { message }` instead of a typed event.

`release_slot_for_ticket` (`lib.rs:486-508`): finds the slot owning the ticket, clears
`slot.ticket_id`, keeps `has_session = true`, and arms a `cooldown_until` wind-down.
This is the single slot-release primitive both reclaimers call.

`Thread::fail()` (`lisa-core/types.rs:413`) sets `ThreadStatus::Failed`. Removing the
thread from `self.threads` makes the ticket re-schedulable (it re-enters
`get_ready_tickets`), matching how both reclaimers behave.

## Alert surfacing (UI)

Alerts are assembled in `to_ui_state` (`lib.rs:~2860-2888`). Two vectors are appended
to the health-alert list each render: `idle_alerts` (`lib.rs:213`) and `timeout_alerts`
(`lib.rs:233`). `timeout_alerts` entries clear on reschedule (`lib.rs:645`,
`retain(... tid != ...)`).

`ui::AlertType` (`ui.rs:159-168`) already has a `Failed` variant rendered as
`"✗ FAILED"` in RED (`ui.rs:441`) — semantically exact for an error reclaim. No new
variant is needed. A new `error_alerts` vector mirrors `timeout_alerts`: populated by
the consumer, drained into a `HealthAlert { alert_type: Failed }`, cleared on reschedule.

## Adapter contract (T-022-01, already landed)

`crates/lisa-plugin/src/adapter.rs` defines `SignalCapabilities { idle, awaiting, cleared }`
and documents (`adapter.rs:97-108`) that `.heartbeat`/`.stopped`(/`.error`) are the
normalized core every adapter emits, while `.idle`/`.awaiting`/`.cleared` are optional.
The module doc explicitly names T-022-02 as the `.error` consumer. `.error` is *not*
in `SignalCapabilities` because it is core, not optional — the consumer fires for any
adapter and is simply never written by Claude panes today (ticket Notes).

## Test scaffolding available

Signal tests build a `tempfile::tempdir()`, create `signals/`, write a signal file,
construct `State { signal_dir, ..State::default() }`, push an `AgentSlot`, insert a
`Thread::new(id, pane)`, call the consumer, assert file deleted + state changed
(`lib.rs:7227-7263` heartbeat; `lib.rs:7060-7104` timeout). `AgentSlot` and `Thread`
literal shapes are established there. This is a ready template for the three required
tests.

## Constraints / assumptions

- WASM plugin: no subprocess I/O; the consumer only reads/deletes files and mutates
  in-memory state — fully within existing capability.
- Presence-is-signal: body ignored (wrapper may write the error text for humans).
- Adapter-agnostic: Claude panes never emit `.error`, so the consumer is inert for
  them — no regression risk to the existing Claude path.
- An `.error` for an idle/unknown pane (no running thread) must be a harmless logged
  no-op (still delete the file so it does not accumulate).
