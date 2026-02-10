# T-003-02 Structure: Artifact-Phase Advance

## Files Modified

### `crates/lisa-plugin/src/lib.rs`

**New method: `State::check_artifact_advances()`**

Added to the `impl State` block (near `poll_tick()`). No new public API surface.

```
fn check_artifact_advances(&mut self)
```

Iterates `self.threads`, checks artifact existence via `std::fs`, calls `ticket::update_ticket_phase()`, logs events, parks threads as needed.

**Modified method: `State::poll_tick()`**

Add `self.check_artifact_advances()` as the first line, before `self.rebuild_dag()`.

### No other files modified

- `lisa-core` types, ticket, dag — unchanged (all primitives exist)
- `scheduler.rs` — unchanged (plugin doesn't use Scheduler for this)
- `ui.rs` — unchanged (already renders parked threads)

## Module Boundaries

- `check_artifact_advances()` is a private method on `State`
- It uses `lisa_core::ticket::update_ticket_phase()` for file I/O
- It uses `Phase::artifact_filename()` and `Phase::next()` for phase logic
- It uses `ActivityEvent` variants for logging
- It directly mutates `self.threads` entries

## Internal Organization

The method is self-contained:

```rust
fn check_artifact_advances(&mut self) {
    // Collect ticket IDs with running threads to avoid borrow conflict
    let running: Vec<(TicketId, Phase)> = ...;

    for (ticket_id, current_phase) in running {
        // Check artifact existence
        // Update ticket file
        // Log events
        // Update thread phase
        // Park if review
    }
}
```

The collect-then-iterate pattern avoids borrowing `self` mutably while iterating `self.threads`.
