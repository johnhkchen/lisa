# T-010-03 Implementation Progress

## What was done

### Production code (`crates/lisa-plugin/src/lib.rs`)

1. **Extended `handle_stopped_signal()`** with two cases:
   - **Case 1 (existing T-010-02):** Mid-transition (`WaitingForStop`) — sends `/clear` and advances to `WaitingForClear`
   - **Case 2 (new T-010-03):** Idle slot with Review-phase ticket — calls `auto_complete_review()`

2. **Added `auto_complete_review()` method:**
   - Updates ticket frontmatter: `phase: done`, `status: Done`
   - Logs `TicketPhaseChanged` and `Info` activity events
   - Completes thread, releases slot, removes thread tracking
   - DAG rebuild and scheduling deferred to normal `poll_tick()` cycle

3. **Added `host_run_plugin_command` stub** for native test target:
   - `#[cfg(not(target_arch = "wasm32"))]` no-op extern "C" function
   - Fixes linker error caused by T-010-02 methods calling `send_line_to_pane()`

### Tests (6 new tests)

| Test | What it verifies |
|------|-----------------|
| `test_auto_complete_review_updates_ticket_and_cleans_up` | Full flow: ticket phase/status updated, thread removed, slot released, activity logged |
| `test_auto_complete_review_condition_non_review_skipped` | Implement-phase ticket NOT auto-completed |
| `test_auto_complete_review_condition_completed_thread_skipped` | Already-completed thread NOT re-processed |
| `test_auto_complete_review_condition_missing_thread_skipped` | Missing thread NOT auto-completed |
| `test_auto_complete_review_condition_parked_thread_eligible` | Parked thread (typical Review state) IS auto-completed |
| `test_auto_complete_review_condition_running_thread_eligible` | Running thread in Review IS auto-completed |

## Test results

- `cargo test --workspace`: 306 tests pass (105 cli + 78 core + 123 plugin)
- `cargo check -p lisa-plugin --target wasm32-wasip1`: compiles clean

## Acceptance criteria status

- [x] `.stopped` signal in Review phase auto-marks ticket as done
- [x] Slot is released after auto-complete
- [x] Thread is removed from tracking
- [x] Activity log shows "Auto-completed {ticket_id} (Review -> Done)"
- [x] `.stopped` in non-Review phases does not auto-complete
- [x] `.stopped` during transitions (WaitingForStop) does not auto-complete
- [x] Manual `[d]` hotkey still works (unchanged code path)
- [x] All tests pass
