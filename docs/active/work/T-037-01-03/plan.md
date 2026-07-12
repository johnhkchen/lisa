# Plan — T-037-01-03 delayed-send-and-prompt-miss-regression

## Testing strategy

Pure native unit tests in `crates/lisa-plugin/src/lib.rs`'s `#[cfg(test)]`
module. No integration test, no live provider, no PTY — the story's honest
boundary says everything here is deterministic, injected-time, and FREE. Clock
is injected via `check_assignment_ack_timeouts_at(now)`; every deadline is read
back out of the matched state so there are no sleeps and no wall-clock reads.

Verification criteria: `cargo test -p lisa-plugin` passes with the two new tests
plus the entire existing suite green (grace happy path, Claude SessionStart,
E-035, E-034). `cargo check -p lisa-plugin --target wasm32-wasip1` still builds.

## Step 1 — Add Test 1: delayed-send → Owned only on exact ack

Add `codex_delayed_send_reaches_owned_only_on_current_attempt_ack` after
lib.rs:9839 (per structure.md).

Assertions, in order:
1. `seat_readiness_mode(10) == Some(ReadinessMode::Grace)`.
2. Capture `grace_deadline` from `Starting { start_deadline: Some(d),
   relaunches: 0, generation == lease.attempt_id }`.
3. Poll `check_assignment_ack_timeouts_at(grace_deadline - 1s)` → still
   `Starting`; UI status `Starting` (never `ReadyForAssignment`); zero
   `"delivering assignment for T-NAME"` info logs; not owned. (delayed send)
4. Poll `check_assignment_ack_timeouts_at(grace_deadline)` → `Delivering {
   generation == lease, retries: 0 }`; UI status `Delivering`; not owned.
5. Stale generation `attempt_id + 1` ack → false, not owned.
6. Wrong ticket `"T-OTHER"` ack → false, not owned.
7. Exact `("T-NAME", attempt_id)` ack → true → `Owned`.

Verify: `cargo test -p lisa-plugin
codex_delayed_send_reaches_owned_only_on_current_attempt_ack`.

## Step 2 — Add Test 2: prompt-miss → bounded DeliveryFailed, never Owned

Add `codex_prompt_miss_retries_then_recycles_to_delivery_failed_never_owned`
below Test 1.

Assertions, in order:
1. `assignment_ack_timeout_secs = 1` before scheduling (tighten Delivering
   clock); `seat_readiness_mode(10) == Some(Grace)`.
2. Capture `grace_deadline`; elapse → `Delivering { retries: 0 }`, capture `d0`;
   not owned.
3. Elapse `d0` → `Delivering { retries: 1 }`, capture `d1`; exactly two
   `"delivering assignment for T-NAME"` info logs (initial + one retry); not
   owned. (bounded retry)
4. Stale generation ack mid-miss → false; not owned. (stale-attempt rejected)
5. Elapse `d1` → `DeliveryFailed`; UI status `DeliveryFailed`;
   `threads["T-NAME"].status == Failed`; `agent_slots[0].attempt_lease ==
   Some(lease)`; `current_leases["T-NAME"] == Some(lease)`; not owned. (named
   recycle, reservation retained for operator reset)
6. Late exact-generation ack after failure → false; state still
   `DeliveryFailed`. (never Owned — terminal)

Verify: `cargo test -p lisa-plugin
codex_prompt_miss_retries_then_recycles_to_delivery_failed_never_owned`.

## Step 3 — Full-suite regression + wasm check

- `cargo test -p lisa-plugin` — both new tests + existing suite green. Confirm
  the named existing guards specifically:
  - `codex_startup_grace_paces_first_prompt_into_delivering`
  - `session_start_seat_never_paces_on_grace_and_still_requires_the_signal`
  - `test_missing_fresh_chat_ack_retries_once_then_fails_without_ownership`
  - the E-035 no-inline-prompt / same-pane dquote-recovery tests
  - the E-034 fencing tests
- `cargo test --workspace` — nothing else regressed.
- `cargo check -p lisa-plugin --target wasm32-wasip1` — plugin still builds for
  its real target.

## Commit

One `lisa commit-ticket --ticket-id T-037-01-03 --message "..." --include
crates/lisa-plugin/src/lib.rs`. The lib.rs edit is the only ticket-owned source
change; both tests land together as one meaningful unit. Work artifacts under
`.lisa/attempts/...` are attempt-private and published by Lisa — not included in
the source commit.

## Risk / rollback

Additive tests only; if a test reveals a real production gap (unexpected),
document it in progress.md and review.md rather than weakening the assertion —
the story treats these as regressions that must reflect true behaviour. Rollback
is trivial (remove the two fns); no production code is at risk.

## Out of scope (restated from story)

Live metered rerun (S-037-02), E-034 fencing changes, reusing a hard-silent
owned Codex process, Claude SessionStart path changes, any scheduler rewrite.
