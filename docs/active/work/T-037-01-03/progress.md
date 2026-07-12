# Progress — T-037-01-03 delayed-send-and-prompt-miss-regression

## Status: Implement complete, committed via lisa commit-ticket

## Steps executed (per plan.md)

### Step 1 — Test 1: delayed-send → Owned only on exact ack — DONE
Added `codex_delayed_send_reaches_owned_only_on_current_attempt_ack` to the
`#[cfg(test)]` module in `crates/lisa-plugin/src/lib.rs`, after the T-037-01-02
grace tests. Proves: grace-mode classification; the paced send is *delayed*
(a poll strictly before the grace deadline delivers nothing, UI stays `Starting`,
zero delivery logs, not owned — no synthetic `ReadyForAssignment`); grace elapse
→ `Delivering { retries: 0 }` directly; a stale generation and a foreign ticket
both fail to own; the exact current-attempt ack → `Owned`.

### Step 2 — Test 2: prompt-miss → bounded DeliveryFailed, never Owned — DONE
Added `codex_prompt_miss_retries_then_recycles_to_delivery_failed_never_owned`
below Test 1. Proves: grace elapse → `Delivering{0}`; one bounded retry →
`Delivering{1}` (exactly two delivery logs); a stale-attempt signal mid-miss is
rejected; final elapse → `DeliveryFailed` (named UI status, thread `Failed`,
reservation + current lease retained for operator reset); never owned; a late
exact-generation ack after failure cannot resurrect `Owned` (terminal state).

### Step 3 — Regression + wasm check — DONE
- `cargo test -p lisa-plugin` → **290 passed, 0 failed**. Both new tests green.
- Named guards confirmed green:
  `codex_startup_grace_paces_first_prompt_into_delivering`,
  `session_start_seat_never_paces_on_grace_and_still_requires_the_signal`,
  `test_missing_fresh_chat_ack_retries_once_then_fails_without_ownership`,
  E-035 (`same_pane_replacement_requires_start_and_chat_ack_for_claude`,
  `test_shell_quote_round_trips_long_control_and_quote_heavy_values`,
  dropped-post-prompt-ack recovery), E-034 fencing
  (`split_brain_timeline_fences_old_attempt_and_admits_one_winner`,
  `missing_replacement_start_fences_without_second_relaunch`,
  `test_missing_shell_readiness_fences_without_relaunch`,
  `fenced_attempt_and_replacement_publish_one_authoritative_done_record`).
- `cargo test --workspace` → all crates green.
- `cargo check -p lisa-plugin --target wasm32-wasip1` → builds clean.

## Deviations from plan
- Added one small private test helper `delivery_log_count(state, ticket_id)`
  instead of inlining the activity-log filter in both tests (Design option D
  branch: extract when duplicated verbatim). Keeps the delivery-count assertions
  readable and identical across the two tests. No production symbol added.
- Helper block "grace elapse → capture deadline" was left inline in each test
  (per structure.md default); the two shapes diverge enough (Test 2 tightens the
  ack clock and threads `d0`/`d1`) that a shared helper would not have read
  cleaner.

## Files changed
- `crates/lisa-plugin/src/lib.rs` (+201 lines, test module only) — committed via
  `lisa commit-ticket`.

## No production code changed
As anticipated in research/design: the grace transition, retry→DeliveryFailed
resolution, and acknowledgement gate all already existed (landed by
T-037-01-02). This ticket is regression evidence only.
