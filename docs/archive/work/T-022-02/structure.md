# T-022-02 Structure — Error Signal Consumer

## Files touched

| File | Change | Why |
|------|--------|-----|
| `crates/lisa-plugin/src/lib.rs` | modify | New `error_alerts` field, `check_error_signals` method, `poll_tick` call, reschedule-clear, `to_ui_state` alert, tests |
| `crates/lisa-cli/data/hooks-guide.md` | modify | Document `.error` in the signal contract |

No new files, no deletions, no cross-crate (`lisa-core`, `ui.rs` enums) additions —
`AlertType::Failed` and `ActivityEvent::Error` already exist.

## lib.rs changes (ordered)

### 1. New state field — `error_alerts`

Next to `timeout_alerts` (`lib.rs:233`):

```rust
/// Recent `.error`-signal reclaims for dashboard display.
/// Entries: (ticket_id, pane_id). Cleared when the ticket is rescheduled.
error_alerts: Vec<(TicketId, u32)>,
```

`State` derives `Default` (used throughout tests as `State::default()`), so a `Vec`
field needs no manual init.

### 2. New method — `check_error_signals`

Placed adjacent to `check_transition_signals` (after `handle_cleared_signal`, before
`check_session_timeouts`). Signature and shape:

```rust
/// Scan for `pane-<id>.error` signal files and fail the owning thread promptly.
///
/// Emitted by adapters (Codex `turn.failed` / non-zero exit, T-023-01) — never by
/// Claude Code hooks. On `.error` for a running thread: fail it, release its slot,
/// remove it (re-schedulable for retry), and surface a Failed alert — the same
/// reclaim `check_session_timeouts` performs on silence, but immediately. For an
/// idle/unknown pane the file is consumed harmlessly (logged, no state change).
///
/// Runs before `check_transition_timeouts` so an errored pane is failed, not
/// force-advanced. Presence is the signal; body (if any) is ignored.
fn check_error_signals(&mut self) {
    // read_dir(self.signal_dir) → for each pane-<id>.error:
    //   remove_file immediately
    //   parse pane_id (strip_prefix "pane-", strip_suffix ".error", parse u32)
    //   resolve running thread: threads.iter().find(pane_id match && Running)
    //   Some(tid): thread.fail(); release_slot_for_ticket(&tid);
    //              threads.remove(&tid); error_alerts.push((tid, pane_id));
    //              log_activity(Error { ... "marked failed for retry" })
    //   None: log_activity(Info { ... "no running thread — ignored" })
}
```

Uses only existing primitives: `release_slot_for_ticket`, `Thread::fail`,
`ThreadStatus::Running`, `ActivityEvent::{Error,Info}`.

### 3. `poll_tick` wiring

Insert one line between `check_transition_signals()` (`lib.rs:1734`) and
`check_transition_timeouts()` (`lib.rs:1737`):

```rust
self.check_transition_signals();

// Fail panes that reported an error before the transition-timeout fallback can
// force-advance them (adapter-emitted; inert for Claude panes).
self.check_error_signals();

self.check_transition_timeouts();
```

### 4. Clear on reschedule

Alongside the `timeout_alerts` retain at `lib.rs:645`:

```rust
self.timeout_alerts.retain(|(tid, _, _)| tid != &ticket_id);
self.error_alerts.retain(|(tid, _)| tid != &ticket_id);
```

### 5. `to_ui_state` alert surfacing

After the `timeout_alerts` loop (`lib.rs:2874-2888`):

```rust
for (ticket_id, pane_id) in &self.error_alerts {
    alerts.push(ui::HealthAlert {
        ticket_id: ticket_id.clone(),
        alert_type: ui::AlertType::Failed,
        detail: format!("Session reported an error (pane {})", pane_id),
        suggested_actions: vec!["Check pane output".to_string(), "Retry".to_string()],
    });
}
```

## Public interface / boundary notes

- `check_error_signals` is a private method — same visibility as its siblings.
- No change to `AgentAdapter` / `SignalCapabilities`: `.error` is core, not optional,
  so it is not gated by a capability flag. The consumer fires regardless of adapter and
  is inert for Claude panes because they never write the file.
- `error_alerts` is not part of `UiState`'s own struct — it is folded into the existing
  `alerts: Vec<HealthAlert>`, so `ui.rs` and `UiState` are unchanged.

## hooks-guide.md change

Add a row to the signal table (`data/hooks-guide.md:26-31`) and a following note:

```
| (adapter)          | Codex `turn.failed` / exit≠0 | `.lisa/signals/pane-<id>.error`     | session failed          |
```

Plus a sentence: `.error` is written by non-Claude adapters (the Codex wrapper), not by
the Claude Code hook scripts; on it the plugin fails the thread and releases the slot
immediately rather than waiting for the silence clock.

## Test additions (native, `#[cfg(test)]` in lib.rs)

1. `test_check_error_signals_fails_running_thread` — write `pane-1.error`, thread on
   pane 1 running + slot bound → thread removed, slot released, `error_alerts` has 1,
   file deleted, `Error` logged.
2. `test_check_error_signals_idle_pane_noop` — write `pane-9.error`, no running thread
   → no state change, `error_alerts` empty, file deleted, `Info` logged.
3. `test_check_error_signals_deletes_file` — asserted within (1) and (2); a focused
   assertion that the file is gone after consumption on both paths.
4. `test_to_ui_state_includes_error_alerts` — push an `error_alerts` entry, render,
   assert a `Failed` alert with the ticket id is present (mirrors
   `test_to_ui_state_includes_timeout_alerts`).

## Ordering of changes

Field → method → poll wiring → reschedule-clear → UI → doc → tests. Each compiles
independently except the UI/reschedule which reference the field. Single atomic commit
is appropriate (the feature is one cohesive consumer), but the field must land before
its readers.
