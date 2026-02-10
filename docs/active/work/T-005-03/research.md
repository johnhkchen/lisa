# Research: T-005-03 fix-phase-change-detection

## Overview

This ticket addresses two related bugs in `crates/lisa-plugin/src/lib.rs` where phase change detection and slot release logic prevent the scheduler from making progress under certain conditions.

## Bug 1: `rebuild_dag()` misses new/first-seen tickets

**Location**: `lib.rs:148-192`, specifically the phase change detection loop at lines 164-176.

**Current logic**:
```rust
for ticket in dag.tickets() {
    if let Some(&old_phase) = self.last_phases.get(&ticket.id) {
        if old_phase != ticket.phase {
            // ... log change, set changed = true
        }
    }
}
```

**Problem**: The `if let Some(...)` guard means tickets with NO prior entry in `last_phases` are silently skipped. This happens in two scenarios:
1. **First rebuild** — `last_phases` is empty (initialized as `HashMap::default()`), so no ticket is detected as changed. The `changed` variable stays `false`.
2. **New ticket appears** — If a ticket file is added between polls, it won't have an entry in `last_phases`, so it's also missed.

**Impact**: `changed` remains `false`, which gates the done-ticket detection in `poll_tick()`.

**Note**: After the comparison loop, `last_phases` IS correctly updated (line 179): `self.last_phases = dag.tickets().map(|t| (t.id.clone(), t.phase)).collect();`. So on the NEXT rebuild, all tickets will have entries. The bug is a one-cycle delay — but that delay is critical on first load and when tickets appear dynamically.

## Bug 2: `poll_tick()` gates slot release behind `if changed`

**Location**: `lib.rs:481-545`, specifically the `if changed { ... }` block at lines 493-530.

**Current logic**:
```rust
let changed = self.rebuild_dag();

if changed {
    // Find done tickets with running threads → complete thread, release slot
    // Detect phase advances → update thread phase
}

// Always try to schedule
self.schedule_ready_tickets();
```

**Problem**: The entire "find done tickets and release their slots" logic is inside `if changed`. When `rebuild_dag()` returns `false` (which it does whenever `last_phases` misses the change — see Bug 1), slots are never freed even if the ticket's phase IS Done in the DAG.

**Impact**: Agent slots stay occupied by completed tickets. Since `schedule_ready_tickets()` runs unconditionally, it TRIES to schedule, but all slots are full, so no new work starts. The loop stalls.

**Combined effect of Bug 1 + Bug 2**: On first load with tickets already at `phase: done`, the system never detects them as done, never releases their slots, and never schedules downstream work.

## Existing Mechanisms

### `check_artifact_advances()` (lines 306-379)
Runs BEFORE `rebuild_dag()` in `poll_tick()`. Detects phase artifacts and advances ticket frontmatter. But it:
- Skips `Implement` phase (progress.md is not a completion signal)
- Only advances within Running threads
- Cannot handle the Done phase (no artifact for Done)

### `detect_stale_threads()` (lines 436-466)
Handles threads stuck beyond 2x the threshold. Marks them failed, releases slots, removes threads. This is a hard-timeout safety net, not a phase-detection mechanism.

### `evaluate_health()` (lines 386-429)
Computes health transitions (Healthy→Stuck, etc.) and logs them. Read-only diagnostics, doesn't release slots.

### `release_slot_for_ticket()` (lines 231-239)
Iterates `agent_slots`, finds the slot with matching `ticket_id`, clears it (sets `ticket_id = None`, `has_session = false`). This is the function that needs to be called — the bug is that it's never reached.

### `schedule_ready_tickets()` (lines 242-298)
Already runs unconditionally (outside `if changed`). Finds idle slots, builds Claude commands, queues pane writes. The problem is upstream — no slots are idle.

## Data Flow Summary

```
poll_tick()
  → check_artifact_advances()   // advance non-implement phases
  → evaluate_health()            // log health transitions
  → detect_stale_threads()       // hard timeout cleanup
  → rebuild_dag()                // scan tickets, detect phase changes
      → returns `changed: bool`
  → if changed:                  // BUG: gates critical logic
      → find done threads → complete() + release_slot
      → sync thread phases
  → schedule_ready_tickets()     // always runs, but slots may be full
```

## Relevant State Fields

- `self.last_phases: HashMap<TicketId, Phase>` — snapshot from previous rebuild
- `self.threads: HashMap<TicketId, Thread>` — active thread records
- `self.agent_slots: Vec<AgentSlot>` — pane slots with `ticket_id: Option<TicketId>`
- `self.dag: Dag` — current dependency graph with ticket phase data

## Test Infrastructure

Tests in `lib.rs` (lines 1011-1793) build State manually with:
- `tempfile::tempdir()` for ticket/work directories
- `lisa_core::ticket::scan_tickets()` + `Dag::from_tickets()` for DAG construction
- Direct `state.threads.insert()` for thread setup
- Direct `state.agent_slots.push(AgentSlot { ... })` for slot setup
- Cannot call `schedule_ready_tickets()` in tests (calls zellij host functions)
- Can test `rebuild_dag()`, `check_artifact_advances()`, `detect_stale_threads()`, `evaluate_health()`, `release_slot_for_ticket()`, `find_idle_slot()`, `check_all_done()`

## Files to Modify

Only `crates/lisa-plugin/src/lib.rs`:
- `rebuild_dag()` — fix phase change detection for missing entries
- `poll_tick()` — move slot release logic outside `if changed`
- New `sweep_stale_slots()` method — safety sweep after scheduling

No changes needed to `types.rs`, `dag.rs`, `ticket.rs`, `scheduler.rs`, or `ui.rs`.
