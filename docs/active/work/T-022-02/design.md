# T-022-02 Design — Error Signal Consumer

## Problem restated

Add a scheduler consumer for `pane-<id>.error` that promptly fails the owning thread,
releases its slot, and surfaces an alert — mirroring `check_session_timeouts`' reclaim
but triggered by an explicit signal instead of silence. Idle/unknown panes: harmless
logged no-op. Document the signal in one contract location. Cover with native tests.

The design space is small; the decisions are about *where* the logic lives, *how* the
alert is surfaced, and *how* the pane→ticket resolution is done. Each is decided against
the research.

## Decision 1 — A dedicated `check_error_signals` method

**Chosen:** a new `fn check_error_signals(&mut self)` on `State`, one per-signal
consumer, matching `check_heartbeat_signals` / `check_awaiting_signals` exactly.

Rejected — *fold `.error` into `check_transition_signals`:* that method is scoped to the
`.stopped`/`.cleared` handshake and its doc comment says so. `.error` is a reclaim, not
a transition; co-locating them would blur two concerns and force the transition method's
per-file loop to branch on an unrelated action. A separate method keeps each consumer's
doc comment and ordering contract legible, which is the established convention.

## Decision 2 — Reclaim by copying the `check_session_timeouts` mutation, logging `Error`

**Chosen:** on `.error` for a running thread:

```rust
thread.fail();
self.release_slot_for_ticket(&ticket_id);
self.threads.remove(&ticket_id);
self.error_alerts.push((ticket_id.clone(), pane_id));
self.log_activity(ActivityEvent::Error { message: ... });
```

This is byte-for-byte the reclaim template both `check_session_timeouts` (`lib.rs:1616`)
and `detect_stale_threads` (`lib.rs:1661`) use. Removing the thread lets the ticket
re-enter `get_ready_tickets` for retry — the same recovery behaviour as a silence
reclaim, which is what the acceptance criteria ("thread is failed, the slot released")
imply.

**Log event — `ActivityEvent::Error` vs a new typed variant.** Rejected adding a new
`ActivityEvent::SessionErrored` variant to `lisa-core`. `detect_stale_threads` already
precedents using `ActivityEvent::Error { message }` for a reclaim, and the ticket keeps
lisa-core free of adapter concerns. A descriptive `Error` message
(`"{ticket} reported an error on pane {pane} — marked failed for retry"`) is enough for
the activity log; the structured surfacing is the UI alert (Decision 3). This keeps the
change contained to `lib.rs` + `ui.rs` with no cross-crate churn.

## Decision 3 — Surface via a new `error_alerts` vec + existing `AlertType::Failed`

**Chosen:** add `error_alerts: Vec<(TicketId, u32)>` to `State`, mirroring
`timeout_alerts`. `to_ui_state` drains it into `HealthAlert { alert_type: Failed, ... }`.
Clear it on reschedule alongside `timeout_alerts` (`lib.rs:645`).

Rejected — *reuse `timeout_alerts`:* semantically wrong (a timeout is a silence budget
overrun; an error is an explicit failure) and would mislabel the UI as `TimedOut` /
"Increase session_timeout_secs", which is unhelpful advice for a crash.

Rejected — *a new `AlertType` variant:* `AlertType::Failed` already exists, renders as
`"✗ FAILED"` in RED (`ui.rs:441`), and is exactly this semantic (the doc says "Session
exited with a non-zero exit code"). Adding a variant would duplicate it.

Alert detail + actions: `detail = "Session reported an error (pane {id})"`,
`suggested_actions = ["Check pane output", "Retry"]` — matching the tone of the existing
`Failed` health alert (`lib.rs:2849-2854`).

## Decision 4 — Resolve pane→ticket via the running thread, not the slot

**Chosen:** find the ticket by scanning `self.threads` for the running thread whose
`pane_id` matches, i.e. `threads.iter().find(|(_, t)| t.pane_id == pane_id && running)`.

Rationale: the reclaim must act on a **running thread**; `threads` is the authority on
what is running and its `pane_id` field is the direct key. `agent_slots` can hold a
`ticket_id` for a slot that is mid-transition or already released, so resolving through
the thread avoids acting on a stale slot binding. `release_slot_for_ticket` then finds
the slot from the ticket id — the same indirection both existing reclaimers use.

If no running thread owns the pane (idle/unknown), the file is still deleted and an
`Info` log is written — the required harmless no-op. This also covers Claude panes,
which never emit `.error` but, if one ever appeared, would be safely ignored.

## Decision 5 — Ordering: after `check_transition_signals`, before `check_transition_timeouts`

**Chosen:** insert the call at `lib.rs:1734`, between `.stopped`/`.cleared` processing
and the transition-timeout fallback.

Rationale (directly from acceptance criteria): a pane that errored mid-transition must
be failed, not force-advanced by `check_transition_timeouts`. Running the error consumer
first removes the thread and releases the slot, so the timeout fallback finds nothing to
advance. Placing it after `check_transition_signals` also means a `.stopped` that raced
ahead in the same tick is processed first, but since the error path removes the thread,
order between those two is immaterial for correctness; the hard constraint is
*before the force-advance*.

## Decision 6 — Document `.error` in the hooks-guide signal contract

**Chosen:** extend the signal table in `crates/lisa-cli/data/hooks-guide.md:26-31` with an
`.error` row and a sentence noting it is adapter-emitted (Codex `turn.failed`/non-zero
exit), not written by Claude Code hooks, and that the plugin fails+releases on it.

Rejected — a separate contract doc: the guide already *is* the single signal-contract
location the adapters reference; a second doc would fragment it. The `adapter.rs` module
doc already cross-references `.error` as core, so the guide table is the right home.

## Out of scope

- Emitting `.error` (that is the Codex wrapper, T-023-01/T-023-02).
- Reading the error body for display — presence is the signal; the body is for humans
  tailing the file. A future provenance ticket (T-027) may capture it.
- Retry backoff policy — removal-for-reschedule matches existing reclaim behaviour;
  changing retry semantics is not this ticket.
