# T-022-02 Plan — Error Signal Consumer

## Strategy

One cohesive feature (a signal consumer) → one atomic commit. Steps are ordered so the
tree compiles after the code steps and the test step proves behaviour. Verification is
`cargo test --workspace` plus the WASM build check (`just check`).

## Steps

### Step 1 — Add `error_alerts` state field
- Add `error_alerts: Vec<(TicketId, u32)>` after `timeout_alerts` (`lib.rs:233`) with a
  doc comment mirroring `timeout_alerts`.
- Verify: `cargo build -p lisa-plugin` (native) still compiles (field unused warning is
  acceptable until Step 2/4 read it).

### Step 2 — Implement `check_error_signals`
- Add the method next to the transition consumers. Read-and-delete loop, parse
  `pane-<id>.error`, resolve running thread, fail+release+remove+alert or log-noop.
- Verify: compiles; field now read.

### Step 3 — Wire into `poll_tick`
- Insert `self.check_error_signals();` between `check_transition_signals()` and
  `check_transition_timeouts()` (`lib.rs:1734`), with an explanatory comment.
- Verify: compiles.

### Step 4 — Clear on reschedule
- Add `self.error_alerts.retain(|(tid, _)| tid != &ticket_id);` beside the
  `timeout_alerts` retain (`lib.rs:645`).
- Verify: compiles.

### Step 5 — Surface in `to_ui_state`
- Append the `error_alerts → HealthAlert { Failed }` loop after the `timeout_alerts`
  loop (`lib.rs:2888`).
- Verify: compiles; `error_alerts` fully consumed.

### Step 6 — Document `.error` in hooks-guide
- Add the `.error` row to the signal table and a clarifying sentence
  (`data/hooks-guide.md:26-37`).
- Verify: table renders (visual check).

### Step 7 — Tests
- `test_check_error_signals_fails_running_thread`
- `test_check_error_signals_idle_pane_noop`
- `test_to_ui_state_includes_error_alerts`
- (file-deletion assertions folded into the first two)
- Verify: `cargo test --workspace` green.

### Step 8 — Full check
- `cargo build -p lisa-plugin --target wasm32-wasip1 --release` (WASM builds).
- `cargo test --workspace` (all native tests pass).
- `cargo clippy` if available — no new warnings on touched code.

## Testing strategy

**Unit (native target)** — the consumer is pure file-read + in-memory mutation, so unit
tests fully cover it using the established `tempfile::tempdir()` + `State::default()`
scaffolding (`lib.rs:7227`). No integration/WASM test is needed; the plugin is not
driven end-to-end in CI, and the signal contract is identical across consumers already
covered this way.

Coverage matrix (maps to acceptance criteria):

| Acceptance criterion | Test |
|----------------------|------|
| `.error` on running thread → failed + slot released | `test_check_error_signals_fails_running_thread` |
| `.error` on idle/unknown pane → harmless no-op | `test_check_error_signals_idle_pane_noop` |
| `.error` file deleted after consumption | asserted in both tests above |
| Alert surfaced in UI | `test_to_ui_state_includes_error_alerts` |
| Consumed before transition timeouts | ordering enforced by Step 3 placement; documented, and the reclaim removes the thread so the fallback is a no-op |
| Contract documented in one place | Step 6 (hooks-guide table) |

## Verification criteria (done = all true)

1. `cargo test --workspace` passes, including the three new tests.
2. WASM release build succeeds.
3. `.error` for a running thread removes the thread, releases the slot, logs an `Error`
   event, and produces a `Failed` UI alert.
4. `.error` for an idle/unknown pane changes no state and logs an `Info` event; file is
   deleted in both cases.
5. hooks-guide signal table lists `.error`.
6. No behavioural change to Claude panes (they never emit `.error`; existing tests
   unaffected).

## Risks / mitigations

- **Risk:** resolving via `agent_slots` instead of `threads` could act on a stale slot.
  **Mitigation:** resolve via the running thread's `pane_id` (design Decision 4).
- **Risk:** double-reclaim if a `.stopped` and `.error` arrive same tick.
  **Mitigation:** `.error` removes the thread first; subsequent consumers find nothing.
- **Risk:** field-unused warnings between steps. **Mitigation:** land as one commit; the
  final tree reads the field in UI + tests.
