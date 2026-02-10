# Plan: T-006-04 Runtime State Snapshot

## Step 1: Add `format_activity_event` helper

Add a private method or function in `lib.rs` that takes an `&ActivityEvent` and returns a single-line `String` representation. Handles all enum variants with a match.

**Verify:** `cargo check -p lisa-plugin --target wasm32-wasip1` passes.

## Step 2: Add `format_snapshot` method on State

Add `fn format_snapshot(&self) -> String` to the `impl State` block. Builds a multi-section text dump:

1. Header with timestamp
2. Config section
3. Plugin status flags
4. Tickets table (sorted by ID)
5. DAG edges (sorted)
6. DAG stats
7. Threads table (sorted by ticket ID)
8. Agent slots table (sorted by pane ID)
9. Health status table (sorted by ticket ID)
10. Activity log (last 50, newest first)

Each section starts with `=== SECTION NAME ===` and ends with a blank line.

**Verify:** `cargo check -p lisa-plugin --target wasm32-wasip1` passes.

## Step 3: Add 'D' key binding in handle_key

In `handle_key()`, after the existing `BareKey::Char('d')` check, add:

```rust
if key.bare_key == BareKey::Char('D') {
    let snapshot = self.format_snapshot();
    if let Err(e) = std::fs::write("/host/.lisa-state-dump.txt", &snapshot) {
        self.log_activity(ActivityEvent::Error {
            message: format!("Failed to write state snapshot: {}", e),
        });
    } else {
        self.log_activity(ActivityEvent::Info {
            message: "State snapshot written to .lisa-state-dump.txt".to_string(),
        });
    }
    return true;
}
```

**Verify:** `cargo check -p lisa-plugin --target wasm32-wasip1` passes.

## Step 4: Add tests

Add tests in the existing `mod tests` block at the bottom of `lib.rs`:

1. **`test_format_snapshot_contains_sections`** — construct State with a 3-ticket DAG, 2 threads (1 running, 1 parked), 2 agent slots (1 occupied, 1 idle), and some activity events. Call `format_snapshot()`, assert all section headers appear.

2. **`test_format_snapshot_ticket_data`** — assert specific ticket IDs, phases, and dependency edges appear in the snapshot output.

3. **`test_format_snapshot_thread_data`** — assert thread ticket IDs, statuses, and pane IDs appear.

4. **`test_format_snapshot_slot_data`** — assert slot pane IDs and assignments appear.

5. **`test_format_snapshot_activity_log_limit`** — create State with 100 activity events, verify only last 50 appear in snapshot.

**Verify:** `cargo test -p lisa-plugin` passes with all new tests.

## Step 5: Full verification

Run `cargo test --workspace` and `cargo check -p lisa-plugin --target wasm32-wasip1` to confirm everything compiles and all tests pass.
