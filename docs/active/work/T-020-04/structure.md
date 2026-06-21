# T-020-04 Structure — timeout-exemption-surfacing

File-level blueprint. Two files modified, no files created or deleted. All changes
additive. Ordering matters only for compilation (the `ActiveThread` field must land
with all its construction sites in the same edit).

## Files

| File | Change |
|------|--------|
| `crates/lisa-plugin/src/lib.rs` | reclaim exemptions (×2) + `to_ui_state` field set + tests |
| `crates/lisa-plugin/src/ui.rs` | `ActiveThread.awaiting` field + render branch + fixture/test updates |

Nothing in `lisa-core` or `lisa-cli` changes. No public plugin interface changes
(all touched items are private to the crate or internal struct fields).

## `crates/lisa-plugin/src/lib.rs`

### S1. `check_session_timeouts` — exempt the kill (~line 1540)
Current silence split:
```rust
if silent_for >= hard_silence {
    timed_out.push((tid.clone(), elapsed_secs, phase));
} else {
    over_budget_active.push((tid.clone(), elapsed_secs, phase));
}
```
New: gate the kill branch on not-awaiting so an awaiting over-budget pane falls into
the warn branch instead:
```rust
if silent_for >= hard_silence && !self.awaiting_human.contains(&t.pane_id) {
    timed_out.push((tid.clone(), elapsed_secs, phase));
} else {
    over_budget_active.push((tid.clone(), elapsed_secs, phase));
}
```
Disjoint shared borrow of `self.awaiting_human` alongside the `&self.threads` loop —
compiles. No other lines in this fn change.

### S2. `detect_stale_threads` — exempt the kill (~line 1590)
Bind the set before the chain and add a filter:
```rust
let awaiting = &self.awaiting_human;
let stale: Vec<TicketId> = self
    .threads
    .iter()
    .filter(|(_, t)| t.status == ThreadStatus::Running)
    .filter(|(_, t)| t.health(now, hard_timeout) == HealthStatus::Stuck)
    .filter(|(_, t)| !awaiting.contains(&t.pane_id))
    .map(|(tid, _)| tid.clone())
    .collect();
```
Rest of fn (removal loop, logging) unchanged.

### S3. `to_ui_state` — populate the new UI field (~line 2712)
In the `active_threads` map closure, add `awaiting: self.is_pane_awaiting(t.pane_id)`
to the `ui::ActiveThread { … }` literal. `self` is borrowed immutably across the
`.map`, and `is_pane_awaiting` is `&self` — fine.

### S4. Tests (in `mod tests`, after the T-020-03 awaiting tests ~line 5580)
New native tests:
- `test_session_timeout_skips_kill_when_awaiting` — Running thread, global timeout
  configured, `last_activity` past hard silence, flag set → after
  `check_session_timeouts`, thread **still present** (not reclaimed).
- `test_session_timeout_kills_after_flag_clears` — same fixture, flag **not** set →
  thread removed (proves the exemption is the only thing keeping it alive, and that
  normal reclaim resumes once the flag clears).
- `test_detect_stale_skips_when_awaiting` — Running thread silent past hard timeout,
  flag set → after `detect_stale_threads`, thread still present.
- `test_detect_stale_kills_after_flag_clears` — same fixture, flag cleared → removed.
- `test_to_ui_state_marks_awaiting_thread` — build State with a Running thread + slot
  whose pane is in `awaiting_human` → `to_ui_state().active_threads[0].awaiting` true;
  a non-awaiting thread → false.

Fixtures use `Thread::new(id, pane)` then backdate `last_activity` to `now -
(stuck_threshold_secs*2 + slack)`; for session-timeout tests also set
`config.session_timeout_secs` small or rely on default 3600 with `started_at`
backdated past it. Keep slack generous (e.g. +100s) to avoid clock flakiness.

## `crates/lisa-plugin/src/ui.rs`

### S5. `ActiveThread` struct (~line 134)
Add field:
```rust
pub struct ActiveThread {
    pub ticket_id: String,
    pub phase: Phase,
    pub started_at: Duration,
    pub slot_number: usize,
    pub awaiting: bool,
}
```

### S6. `render_threads` active branch (~line 718)
Replace the single active-row push with awaiting-aware rendering:
```rust
if let Some(active) = active_by_slot.get(&slot.slot_number) {
    let elapsed = format_time_since(active.started_at, state.current_time);
    let phase_color = active.phase.color_code();
    let (ticket_cell, status_color, status_text) = if active.awaiting {
        (format!("{} [AWAITING]", active.ticket_id), CYAN, "Awaiting")
    } else {
        (active.ticket_id.clone(), GREEN, "Running")
    };
    output.push(format!(
        "{:<6} {:<12} {}{:<10}{} {}{:<14}{} {}",
        slot_label, ticket_cell, phase_color, active.phase.short_name(),
        RESET, status_color, status_text, RESET, elapsed,
    ));
}
```
`CYAN` is already in scope (used elsewhere in the file). The `[AWAITING]` token may
push the ticket cell past its 12-wide pad on long ids — acceptable for a rare state.

### S7. Construction-site / fixture updates
Every `ActiveThread { … }` literal must gain `awaiting: …`:
- `lib.rs` `to_ui_state` (set in S3).
- `ui.rs` test fixtures that build `ActiveThread` (e.g. ~lines 1298, 1824, plus any
  in `render_threads` tests ~1375+). Set `awaiting: false` unless the test exercises
  the marker.

### S8. UI test for the marker (in `ui.rs mod tests`)
- `test_render_threads_marks_awaiting` — build a `PluginState` with one slot + one
  `ActiveThread { awaiting: true, .. }`; assert the joined `render_threads` output
  contains `"[AWAITING]"` and `"Awaiting"`.

## Ordering / compile gate

1. S5 (add field) + S7 (update all literals) together — must be one compiling step.
2. S6 render branch.
3. S3 `to_ui_state` set (depends on S5).
4. S1, S2 reclaimer guards (independent of UI).
5. S4, S8 tests.
6. `just check`.
