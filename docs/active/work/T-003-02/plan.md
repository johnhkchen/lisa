# T-003-02 Plan: Artifact-Phase Advance

## Step 1: Add `check_artifact_advances()` to `State`

Add a new private method to the `impl State` block in `lib.rs`.

```rust
fn check_artifact_advances(&mut self) {
    // 1. Collect running threads: Vec<(TicketId, Phase)>
    // 2. For each, check if current_phase.artifact_filename() exists in work_dir
    // 3. If so: get ticket file_path from self.dag, call update_ticket_phase()
    // 4. Log PhaseCompleted and TicketPhaseChanged events
    // 5. Update thread.current_phase to next_phase
    // 6. If next_phase == Review, park the thread
}
```

Verification: `cargo check -p lisa-plugin --target wasm32-wasip1` compiles.

## Step 2: Wire into `poll_tick()`

Add `self.check_artifact_advances();` as the first line of `poll_tick()`, before `self.rebuild_dag()`.

Verification: `cargo check -p lisa-plugin --target wasm32-wasip1` compiles.

## Step 3: Add unit test

Add a test in `lib.rs` `mod tests` that:
1. Creates a tempdir with a ticket file (phase: research) and a work dir with `research.md`
2. Constructs a `State` with a populated DAG and a running thread
3. Calls `check_artifact_advances()`
4. Verifies: ticket file updated to `design`, thread phase updated, activity log has PhaseCompleted and TicketPhaseChanged events

Since `State` requires `Dag` and `Thread` but not zellij APIs, this test runs on native.

Verification: `cargo test --workspace` passes, including the new test.

## Step 4: Add review-parking test

Add a test that verifies: when the implement phase artifact (`progress.md`) is detected, the ticket advances to `review` and the thread is parked (`ThreadStatus::Parked`).

Verification: `cargo test --workspace` passes.

## Testing Strategy

- Unit tests for the advance logic using tempdir + constructed State
- No integration tests needed — the method uses standard filesystem ops
- Existing tests must continue to pass (88 tests)
