# T-004-03 Design: Error & Health Alerts

## Problem

Failed and stuck sessions are invisible to the user. Stuck threads are silently failed and removed. Failed threads disappear from the UI. The user has no way to know something went wrong or take corrective action.

## Design Decisions

### 1. Alert Data Model

**Approach A: Separate AlertQueue in State**
Add a `Vec<HealthAlert>` to State that accumulates alerts. Alerts are created by health evaluation and cleared by user action or timeout. Passed to UI as a dedicated field.

**Approach B: Derive alerts from thread state at render time**
Keep failed/stuck threads in the threads map longer. At render time, compute alerts from current thread health. No separate alert storage needed.

**Decision: Approach A — Separate AlertQueue**

Rationale: Approach B requires keeping failed threads in the map, which complicates slot release logic and retry scheduling. A separate alert queue decouples alerting from thread lifecycle. Alerts persist until acknowledged even after the thread is cleaned up. It's also more extensible for future alert types.

### 2. Stuck Detection Strategy

**Current behavior:** Stuck threads are immediately failed and removed.

**Approach A: Two-stage (warn then fail)**
First time threshold exceeded → log warning alert, keep running. Second threshold (2x) → fail and remove. This gives the user a window to see the warning.

**Approach B: Warn at threshold, fail at separate hard timeout**
Use `stuck_threshold_secs` for the warning. Add a separate `stuck_hard_timeout_secs` (e.g., 2x threshold) for auto-failure. Cleaner separation.

**Approach C: Warn only, never auto-fail**
Remove auto-failure entirely. User must manually restart or mark blocked.

**Decision: Approach B — Warn at threshold, fail at hard timeout**

Rationale: Approach A is simpler but couples the two thresholds implicitly. Approach C removes a useful safety net. Approach B gives clear semantics: `stuck_threshold_secs` = warning, `stuck_threshold_secs * 2` = auto-fail. The hard timeout is derived (2x) to avoid another config knob.

### 3. Alert Presentation

**Approach A: Banner above DAG**
A prominent red/yellow banner at the top of the dashboard, below the title bar. Shows all alerts in a compact list.

**Approach B: Inline in thread tables**
Add a "health" column to active/parked thread tables. Mark rows red for failed, yellow for stuck.

**Approach C: Banner + inline markers**
Both: a summary banner with counts, plus inline markers in the thread table.

**Decision: Approach A — Banner above DAG**

Rationale: The primary goal is that users *notice* problems. A banner is impossible to miss. Inline markers in a table can be overlooked, especially if the table is empty (failed threads are removed). The banner persists independently of thread table state.

### 4. Suggested Actions

The ticket asks for: "restart session", "check logs", "mark as blocked".

These map to keyboard shortcuts in the plugin. Current keybindings: `d` = mark done modal, `j/k` = navigate, `Enter` = select, `Esc` = close.

**Design:**
- Show suggested actions as text in the alert banner (e.g., `[r] restart  [l] logs  [b] blocked`)
- When the attention banner is visible, these keys become active
- `r` on a failed/stuck ticket: release slot, remove thread, re-queue ticket for scheduling
- `b` on a ticket: mark ticket phase as blocked (not currently a phase — skip for now, use "mark as blocked" as descriptive text that the user can handle manually)
- `l` for logs: focus the pane associated with the ticket (if still available)

Simplification: For this ticket, implement the alert banner with descriptive suggested actions. Full keyboard-driven actions can be a follow-up. The banner text tells the user what to do; the user can press `d` to mark done or manually intervene in the agent pane.

### 5. Activity Log Entries for Health Changes

Add a new `ActivityEvent::HealthStateChanged` variant:
```rust
HealthStateChanged {
    ticket_id: TicketId,
    old_health: HealthStatus,
    new_health: HealthStatus,
}
```

This requires `HealthStatus` to derive `Serialize, Deserialize, Clone, Copy`. Track last-known health per thread to detect transitions.

Map to `ActivityType::Error` for Failed, a new `ActivityType::Warning` for Stuck.

### 6. Fix stuck_threshold_secs Usage

Replace the hardcoded 30-minute threshold in `detect_stale_threads()` with `config.stuck_threshold_secs * 2` (hard timeout). Add a new health evaluation method that uses `config.stuck_threshold_secs` for the warning threshold.

## Rejected Alternatives

- **Toast/popup notifications**: Too disruptive for a terminal plugin. Banners are visible but not blocking.
- **Sound alerts**: Not available in WASM/Zellij.
- **Separate "alerts" tab/mode**: Over-engineered for the current need. A banner is simpler.
- **New `Stuck` ThreadStatus**: Would complicate the existing status machine. Health is better as a derived property (which it already is).

## Summary

1. Add `HealthAlert` struct and `Vec<HealthAlert>` to State
2. Add `HealthStateChanged` activity event
3. Evaluate health each poll tick, create alerts for stuck/failed transitions
4. Fix `detect_stale_threads()` to use `config.stuck_threshold_secs * 2` as hard timeout
5. Add attention banner to UI above DAG section
6. Show suggested actions as text in banner
7. Track per-thread health state for transition detection
