# T-020-03 Progress — awaiting-human suppression

All changes in `crates/lisa-plugin/src/lib.rs` (single file, per structure.md).

## Completed

- [x] **S1** `awaiting_human: HashSet<u32>` field added beside `notified_attention`.
- [x] **S2** `check_awaiting_signals()` (mirror of `check_heartbeat_signals`) +
  `is_pane_awaiting()` accessor added beside `send_line_to_pane`.
- [x] **S3** Clear-on-heartbeat: `self.awaiting_human.remove(&pane_id)` added next to
  the existing `notified_attention.remove` in `check_heartbeat_signals`.
- [x] **S4** In-method guard in `send_line_to_pane`: drops the write + skips the
  deferred Enter when the target `PaneId::Terminal(id)` is awaiting (logs it).
- [x] **S5** Per-caller guards (all before any state mutation):
  - `schedule_ready_tickets` → `unscheduled += 1; continue` (defensive).
  - `handle_stopped_signal` (WaitingForStop arm) → early `return`.
  - `handle_cleared_signal` → early `return`.
  - `check_transition_timeouts` → `continue` in both the stop and clear drain loops.
  - `check_review_timeouts` → `continue` before `finish_up_sent.insert`.
- [x] **S6** Wired `check_awaiting_signals()` into `poll_tick` immediately after
  `check_heartbeat_signals` and before `check_idle_signals`.
- [x] **S7** Seven native tests added after `test_attention_debounce_*`.
- [x] **S8** Verification gate — see below.

## Deviations from plan

- None of substance. The in-method guard test is implicit: `test_stopped_signal_*`,
  `test_cleared_signal_*`, `test_transition_timeouts_*` all drive callers that would
  reach `send_line_to_pane` (a zellij host call that aborts natively) if their guard
  were missing — so a green run is itself evidence the suppression path holds. This
  matches the plan's "missing guard surfaces as a test failure" property.

## Liveness invariant (held)

Grep confirms no new code touches `bump_pane_activity` or `last_activity_at`. The
flag only gates writes; the silence clock is untouched, so a blocked-then-abandoned
pane still trips stale detection normally. Reclaim exemption remains T-020-04.

## Verification

See the command output captured during the run; `just check` (WASM check + full
workspace test suite) passed with the 7 new tests green.
