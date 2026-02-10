# T-005-01 Design: Scheduling Decision Logging

## Decision: Add `ActivityEvent::Info` Variant

**Chosen approach:** Add a new `Info { message: String }` variant to `ActivityEvent` and a corresponding `Info` variant to `ui::ActivityType`.

### Alternatives Considered

1. **Reuse `DagRecomputed` with extra fields** — Rejected. DagRecomputed has specific semantics (DAG rebuild). Adding scheduling messages to it conflates two concerns. It's also currently filtered from the UI, so we'd need to change that filtering logic.

2. **Reuse `Error` variant** — Rejected. This is the current bug. Slot discovery logged as Error is misleading. More Error-level messages would make the problem worse.

3. **Reuse `Warning` (ui only)** — Rejected. Warnings imply something is wrong. Scheduling decisions are informational.

4. **Add `Info { message: String }` variant** — Chosen. Matches the shape of `Error { message: String }`. Clean, extensible, no semantic overloading.

### UI Rendering for Info

Info events will render with:
- Icon: `ℹ` (info symbol)
- Color: `CYAN` (neutral, informational)
- Format: `ℹ {time_ago} {message}`

This provides visual distinction from errors (✗ red), warnings (⚠ yellow), and completions (✓ green).

## Logging Specifications

### 1. `schedule_ready_tickets()` — three new log points

**a. Scheduling success** — already exists as `ThreadSpawned`. No change needed.

**b. Skip: thread already exists (line 251)**
```
ActivityEvent::Info { message: "Skipping {ticket_id}: thread already exists" }
```
Logged when `self.threads.contains_key(&ticket_id)` is true.

**c. Slots exhausted (after the loop)**
```
ActivityEvent::Info { message: "No idle slots available, {N} ready tickets waiting" }
```
Logged when `find_idle_slot()` returns None and there are still unscheduled ready tickets. Count computed from remaining unprocessed tickets in the ready list.

### 2. `release_slot_for_ticket()` — two log points

**a. Success:**
```
ActivityEvent::Info { message: "Released slot #{pane_id} for {ticket_id}" }
```

**b. Not found:**
```
ActivityEvent::Info { message: "No slot found for {ticket_id}" }
```

### 3. `discover_slots()` — change Error to Info

Replace:
```rust
ActivityEvent::Error { message: format!("Discovered {} agent pane slots", ...) }
```
With:
```rust
ActivityEvent::Info { message: format!("Discovered {} agent pane slots", ...) }
```

### 4. `poll_tick()` — cycle summary

```
ActivityEvent::Info { message: "Poll: {N} ready, {M} running, {K} idle slots" }
```

Logged every poll cycle. The ticket says "at debug level — use DagRecomputed or similar to avoid log spam." Since we're adding Info, we'll use Info but filter it from the UI the same way DagRecomputed is filtered: `activity_event_to_ui_entry()` will return `None` for poll summaries.

**Decision: poll summary filtering.** Rather than filtering all Info events, we'll check the message prefix. Poll summaries start with "Poll:" — the mapping function will skip these. All other Info messages will render in the activity log.

Actually, a cleaner approach: add a dedicated `PollSummary` variant instead of overloading Info for filtered messages. This avoids string-matching logic.

**Revised decision:** Add both `Info { message: String }` and `PollSummary { ready: usize, running: usize, idle_slots: usize }` variants. PollSummary is filtered from the UI (returns None in mapping), just like DagRecomputed.

## Changes by File

### `crates/lisa-core/src/types.rs`
- Add `Info { message: String }` variant to `ActivityEvent`
- Add `PollSummary { ready: usize, running: usize, idle_slots: usize }` variant

### `crates/lisa-plugin/src/lib.rs`
- `discover_slots()`: Change `Error` → `Info`
- `release_slot_for_ticket()`: Add success/not-found Info logs. Needs to return whether a slot was found (currently void). Change to track `found` flag.
- `schedule_ready_tickets()`: Add skip and exhausted Info logs
- `poll_tick()`: Add PollSummary log after scheduling
- `activity_event_to_ui_entry()`: Add `Info` → `ui::ActivityType::Info` mapping; `PollSummary` → `None`

### `crates/lisa-plugin/src/ui.rs`
- Add `Info { ticket_id: String, message: String }` variant to `ActivityType`
- Add rendering case in `render_activity_log`: `ℹ` icon, CYAN color

### Test updates
- New test: `test_schedule_ready_tickets_skip_existing_thread` — verify Info log when thread exists
- New test: `test_schedule_ready_tickets_no_slots` — verify Info log when slots exhausted
- New test: `test_release_slot_logs_success` — verify Info log on release
- New test: `test_release_slot_logs_not_found` — verify Info log when ticket not in any slot
- New test: `test_discover_slots_logs_info_not_error` — verify Info (not Error) on discovery
- New test: `test_poll_tick_logs_summary` — verify PollSummary event
- Update: `test_activity_event_to_ui_entry` — add Info and PollSummary cases
- Update: `test_detect_stale_threads` — if slot discovery log changes affect assertions

## Risks

- **Log volume**: `schedule_ready_tickets()` is called frequently (every poll tick + on events). The "skip: thread already exists" message will repeat for every running ticket every cycle. Mitigation: these are Info-level and the activity log is capped at 100 entries (MAX_ACTIVITY_LOG). The skip logs provide value for debugging but could be noisy. We'll proceed as-is since the ticket explicitly requests them.

- **release_slot_for_ticket logging needs &self → &mut self**: Already takes `&mut self`. No issue — `log_activity` also needs `&mut self`. But release_slot is called in contexts where we also need mutable access to other fields (threads, etc). Need to ensure no borrow conflicts. Looking at call sites: all call `release_slot_for_ticket` then do other work, no overlap.
