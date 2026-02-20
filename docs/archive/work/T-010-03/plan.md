# T-010-03 Plan: Auto-complete Review tickets on Stop signal

## Step 1: Add `check_stopped_signals()` method

File: `crates/lisa-plugin/src/lib.rs`
Location: After `check_idle_signals()` method (after line 685).

```rust
fn check_stopped_signals(&mut self) {
    let entries = match std::fs::read_dir(&self.signal_dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) if name.ends_with(".stopped") => name.to_string(),
            _ => continue,
        };

        // Delete signal file immediately
        let _ = std::fs::remove_file(&path);

        // Parse pane-{id}.stopped
        let pane_id: u32 = match filename
            .strip_prefix("pane-")
            .and_then(|s| s.strip_suffix(".stopped"))
            .and_then(|s| s.parse().ok())
        {
            Some(id) => id,
            None => continue,
        };

        // Resolve pane → slot → ticket
        let ticket_id = match self
            .agent_slots
            .iter()
            .find(|s| s.pane_id == pane_id)
            .and_then(|s| s.ticket_id.clone())
        {
            Some(tid) => tid,
            None => continue,
        };

        // Only auto-complete if ticket is in Review phase
        let is_review = self
            .dag
            .get_ticket(&ticket_id)
            .map(|t| t.phase == Phase::Review)
            .unwrap_or(false);

        if !is_review {
            continue;
        }

        // Skip if thread is already Completed
        let dominated = self
            .threads
            .get(&ticket_id)
            .map(|t| t.status == lisa_core::types::ThreadStatus::Completed)
            .unwrap_or(true); // No thread = nothing to complete

        if dominated {
            continue;
        }

        self.auto_complete_review(ticket_id, pane_id);
    }
}
```

Verification: `cargo check -p lisa-plugin --target wasm32-wasip1` compiles.

## Step 2: Add `auto_complete_review()` method

File: `crates/lisa-plugin/src/lib.rs`
Location: Immediately after `check_stopped_signals()`.

```rust
fn auto_complete_review(&mut self, ticket_id: TicketId, pane_id: u32) {
    let file_path = match self.dag.get_ticket(&ticket_id).map(|t| t.file_path.clone()) {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => {
            self.log_activity(ActivityEvent::Error {
                message: format!("Cannot find file for {} during auto-complete", ticket_id),
            });
            return;
        }
    };

    // Update phase to Done
    if let Err(e) = ticket::update_ticket_phase(&file_path, Phase::Done) {
        self.log_activity(ActivityEvent::Error {
            message: format!("Failed to auto-complete {} phase: {}", ticket_id, e),
        });
        return;
    }

    // Update status to Done
    if let Err(e) = ticket::update_ticket_status(&file_path, lisa_core::types::TicketStatus::Done) {
        self.log_activity(ActivityEvent::Error {
            message: format!("Failed to auto-complete {} status: {}", ticket_id, e),
        });
        // Phase already changed, continue anyway
    }

    self.log_activity(ActivityEvent::TicketPhaseChanged {
        ticket_id: ticket_id.clone(),
        old_phase: Phase::Review,
        new_phase: Phase::Done,
    });
    self.log_activity(ActivityEvent::Info {
        message: format!("Auto-completed {} (Review → Done) on pane #{}", ticket_id, pane_id),
    });

    // Complete thread, release slot, remove thread
    if let Some(thread) = self.threads.get_mut(&ticket_id) {
        thread.complete();
    }
    self.release_slot_for_ticket(&ticket_id);
    self.threads.remove(&ticket_id);

    // Rebuild DAG so dependents become ready, then schedule
    self.rebuild_dag();
    self.schedule_ready_tickets();
}
```

Verification: `cargo check -p lisa-plugin --target wasm32-wasip1` compiles.

## Step 3: Wire `check_stopped_signals()` into `poll_tick()`

File: `crates/lisa-plugin/src/lib.rs`
Location: `poll_tick()` method, after `self.check_idle_signals();` (around line 819).

Add one line:
```rust
self.check_stopped_signals();
```

Verification: `cargo check -p lisa-plugin --target wasm32-wasip1` compiles.

## Step 4: Add tests

File: `crates/lisa-plugin/src/lib.rs`, in `mod tests` section.

### Test 1: `test_check_stopped_signals_review_auto_complete`

Setup:
- Create temp dir with tickets/ and signals/ subdirs
- Write ticket file with `phase: review`
- Build DAG from ticket
- Create State with signal_dir, config, dag
- Add parked thread for ticket
- Add agent slot with ticket assigned
- Write `pane-{id}.stopped` signal file

Run `state.check_stopped_signals()`.

Assert:
- Thread removed from `state.threads`
- Slot released (ticket_id = None)
- Activity log contains `TicketPhaseChanged { Review → Done }`
- Activity log contains `Info` with "Auto-completed"
- Ticket file on disk has `phase: done`
- Signal file deleted

### Test 2: `test_check_stopped_signals_non_review_ignored`

Setup:
- Ticket in `phase: implement`, running thread, `.stopped` signal file

Run `state.check_stopped_signals()`.

Assert:
- Thread still exists, unchanged
- Slot still assigned
- Signal file deleted
- No `TicketPhaseChanged` in activity log

### Test 3: `test_check_stopped_signals_no_ticket_ignored`

Setup:
- Agent slot with no ticket assigned, `.stopped` signal file

Run `state.check_stopped_signals()`.

Assert:
- Signal file deleted
- No errors in activity log

### Test 4: `test_check_stopped_signals_completed_thread_ignored`

Setup:
- Ticket in Review, thread already Completed, `.stopped` signal file

Run `state.check_stopped_signals()`.

Assert:
- Thread still Completed (not re-processed)
- Signal file deleted

## Step 5: Run full test suite

```bash
cargo test --workspace
```

Verify all existing tests pass plus the 4 new tests.
