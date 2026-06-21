# T-020-03 Structure — awaiting-human suppression

File-level blueprint. One file changes: `crates/lisa-plugin/src/lib.rs`. No new
files, no deletions, no public-interface changes (all additions are private methods
/ fields on `State`).

## Change set (ordered)

### C1 — New field on `State` (`lib.rs:241`, beside `notified_attention`)

```rust
/// Panes blocked on an `AskUserQuestion` (a `pane-<id>.awaiting` signal was
/// seen). While set, all injection into the pane is suppressed so lisa never
/// types over the question UI. Cleared on the pane's next heartbeat (the agent
/// resumed real work). Never touches the liveness clock — a blocked pane still
/// trips stale detection on the normal silence clock (reclaim is T-020-04).
awaiting_human: HashSet<u32>,
```

`#[derive(Default)]` already covers `State`, so no `load()` initialization.

### C2 — New scanner `check_awaiting_signals()` (place beside `check_heartbeat_signals`, ~`lib.rs:785`)

Mirror of `check_heartbeat_signals`:

```rust
/// Consume `pane-<id>.awaiting` signals (written by the PreToolUse
/// AskUserQuestion hook) and flag those panes as blocked on human input.
/// Must run before `check_idle_signals` so the flag gates this tick's consumers.
/// Deliberately does NOT bump activity clocks — gating writes only (see T-020-04
/// for reclaim exemption).
fn check_awaiting_signals(&mut self) {
    let entries = match std::fs::read_dir(&self.signal_dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let pane_id = match path.file_name().and_then(|n| n.to_str())
            .and_then(|n| n.strip_prefix("pane-"))
            .and_then(|n| n.strip_suffix(".awaiting"))
            .and_then(|id| id.parse::<u32>().ok())
        { Some(id) => id, None => continue };
        let _ = std::fs::remove_file(&path);
        if self.awaiting_human.insert(pane_id) {
            self.log_activity(ActivityEvent::Info {
                message: format!("Pane {} awaiting human (AskUserQuestion) — suppressing injection", pane_id),
            });
        }
    }
}
```

### C3 — Clear on heartbeat (`lib.rs:783`, inside `check_heartbeat_signals`)

Add one line next to the existing `notified_attention.remove`:

```rust
self.notified_attention.remove(&pane_id);
self.awaiting_human.remove(&pane_id);   // a real tool call ⇒ agent resumed, no longer blocked
```

### C4 — Accessor `is_pane_awaiting` (near the field's helpers / send_line_to_pane)

```rust
/// True if `pane_id` is currently blocked on an AskUserQuestion.
fn is_pane_awaiting(&self, pane_id: u32) -> bool {
    self.awaiting_human.contains(&pane_id)
}
```

### C5 — In-method guard inside `send_line_to_pane` (`lib.rs:268`)

Before the write, extract the terminal id and bail if awaiting:

```rust
fn send_line_to_pane(&mut self, text: &str, pane_id: PaneId) {
    if let PaneId::Terminal(id) = pane_id {
        if self.is_pane_awaiting(id) {
            self.log_activity(ActivityEvent::Info {
                message: format!("Suppressed injection into pane {} (awaiting human)", id),
            });
            return;   // drop write + do NOT queue a deferred Enter
        }
    }
    write_chars_to_pane_id(text, pane_id);
    self.pending_enters.push_back(pane_id);
    set_timeout(ENTER_DELAY_SECS);
    self.pending_timer_count += 1;
}
```

Returning before `pending_enters.push_back` is important: a dropped line must not
leave a stray Enter queued for the next `flush_pending_enters`.

### C6 — Per-caller early-returns / skips

Insert an awaiting check at the top of each caller's action, *before* any state
mutation:

- **`schedule_ready_tickets` (`lib.rs:540`-ish):** after resolving `pane_id` for the
  chosen slot, `if self.is_pane_awaiting(pane_id) { unscheduled += 1; continue; }`
  — leaves the slot unassigned and re-tried next poll. (Defensive: idle slots
  rarely host an awaiting agent.)
- **`handle_stopped_signal` (`lib.rs:1070`):** in the `WaitingForStop` arm, before
  sending `/clear`, `if self.is_pane_awaiting(pane_id) { return; }`.
- **`handle_cleared_signal` (`lib.rs:1183`):** inside the `if let Some(ticket_id)`,
  before `send_line_to_pane`, `if self.is_pane_awaiting(pane_id) { return; }`.
- **`check_transition_timeouts` (`lib.rs:1238`,`:1252`):** in both drain loops, skip
  the pane: `if self.is_pane_awaiting(pane_id) { continue; }` before the send and
  before flipping `transition_state`.
- **`check_review_timeouts` (`lib.rs:1302`):** in the candidate loop, `if
  self.is_pane_awaiting(pane_id) { continue; }` before the finish-up send and before
  `finish_up_sent.insert`.

For loops, the skip must precede `finish_up_sent.insert` / state flips so a pane is
re-evaluated cleanly on a later tick once unblocked.

### C7 — Wire into `poll_tick` (`lib.rs:1551`)

```rust
self.check_heartbeat_signals();   // clears flags on real activity
self.check_awaiting_signals();    // NEW — set awaiting before any consumer runs
self.check_artifact_advances();
self.check_idle_signals();
...
```

Order rationale: heartbeat clear first (a resumed agent shouldn't be re-flagged by a
stale file — though the writer deletes nothing, the plugin deletes on read, so each
`.awaiting` is consumed once), then set awaiting, then all consumers. Placing the
set immediately after the clear, both before `check_idle_signals`, satisfies the AC.

## Tests (append to `mod tests`, mirroring `test_attention_debounce_*` and `test_check_heartbeat_signals_*`)

1. `test_check_awaiting_signals_inserts_and_deletes` — write `pane-7.awaiting` to a
   temp `signal_dir`, call `check_awaiting_signals`, assert flag set + file gone.
2. `test_heartbeat_clears_awaiting` — insert pane into `awaiting_human`, write
   `pane-7.heartbeat`, call `check_heartbeat_signals`, assert flag cleared.
3. `test_is_pane_awaiting` — insert/remove, assert accessor.
4. `test_stopped_signal_skips_when_awaiting` — `WaitingForStop` slot, flag set,
   call `handle_stopped_signal`, assert `transition_state` unchanged (no advance).
5. `test_cleared_signal_skips_when_awaiting` — `WaitingForClear` slot with ticket,
   flag set, call `handle_cleared_signal`, assert state stays `WaitingForClear`.
6. `test_transition_timeouts_skip_when_awaiting` — slot past timeout + quiet, flag
   set, assert `transition_state` unchanged after `check_transition_timeouts`.
7. `test_review_timeout_skips_when_awaiting` — Review thread past timeout + quiet,
   flag set, assert `finish_up_sent` does NOT contain the ticket after
   `check_review_timeouts`.

All tests use only `std::fs` + direct state assertions — none reach an unguarded
`send_line_to_pane`, so no zellij host call occurs (see `research.md` test
constraints).

## Risk / ordering notes

- The guards must be inserted *before* any state mutation in each caller, else the
  FSM advances even though the write was dropped (the exact desync D4 warns about).
- No change to function signatures or module boundaries → no ripple to `ui.rs`,
  `scheduler.rs`, or `lisa-core`.
