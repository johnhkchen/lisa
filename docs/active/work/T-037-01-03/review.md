# Review — T-037-01-03 delayed-send-and-prompt-miss-regression

## Summary of changes

One file changed, test-only:

- **`crates/lisa-plugin/src/lib.rs`** (+201 lines, `#[cfg(test)] mod tests`):
  - `delivery_log_count(state, ticket_id) -> usize` — small private test helper
    counting "delivering assignment for <ticket>" info logs (one per actual chat
    send).
  - `codex_delayed_send_reaches_owned_only_on_current_attempt_ack` — the
    delayed-send regression.
  - `codex_prompt_miss_retries_then_recycles_to_delivery_failed_never_owned` —
    the prompt-miss regression.

No production code changed. The grace transition, the bounded
retry→`DeliveryFailed` path, and the acknowledgement gate were all landed by
T-037-01-02; this ticket only pins them as deterministic regressions (P5).

Committed through Lisa's isolated transaction as `ce0cab0`. Nothing left staged,
modified, or untracked in the ordinary index for ticket-owned source.

## Acceptance criteria — evidence

> Two new injected-time tests pass — a delayed-send test (Codex grace→Delivering
> directly, never synthetic ReadyForAssignment, then Owned only on the exact
> current-attempt UserPromptSubmit) and a prompt-miss test (grace elapses, no
> matching ack → bounded retry then named recycle/DeliveryFailed, never Owned,
> stale-attempt signals rejected) — while Claude's evidence-backed
> ReadyForAssignment path and the existing E-035 … and E-034 fencing tests
> remain green.

**Delayed-send test** — covers, in order: grace-mode classification; the send is
*delayed* (pre-deadline poll delivers nothing, UI `Starting`, zero delivery
logs); `Starting → Delivering` **directly** on grace elapse with no
`ReadyForAssignment` node; stale-generation and foreign-ticket acks rejected;
`Owned` only on the exact `(T-NAME, attempt_id)` `UserPromptSubmit`. ✓

**Prompt-miss test** — covers: grace elapse → `Delivering{0}`; one bounded retry
→ `Delivering{1}` (exactly two delivery logs); stale-attempt signal rejected
mid-miss; final elapse → named `DeliveryFailed` (RED UI status, thread `Failed`,
reservation + current lease retained for operator reset); never `Owned`,
including a late exact-generation ack that cannot resurrect a terminal
`DeliveryFailed`. ✓

**Regressions stay green** — confirmed by name:
- Claude SessionStart / ReadyForAssignment:
  `session_start_seat_never_paces_on_grace_and_still_requires_the_signal`,
  `test_fresh_dispatch_requires_start_then_chat_ack_for_both_providers`.
- E-035: `same_pane_replacement_requires_start_and_chat_ack_for_claude`,
  `test_shell_quote_round_trips_long_control_and_quote_heavy_values`,
  `test_dropped_post_prompt_ack_reproduces_open_loop_stall_and_recovers_boundedly`.
- E-034 fencing:
  `split_brain_timeline_fences_old_attempt_and_admits_one_winner`,
  `missing_replacement_start_fences_without_second_relaunch`,
  `test_missing_shell_readiness_fences_without_relaunch`,
  `fenced_attempt_and_replacement_publish_one_authoritative_done_record`.
- T-037-01-02 happy path:
  `codex_startup_grace_paces_first_prompt_into_delivering`.

## Test coverage assessment

- `cargo test -p lisa-plugin` → **290 passed, 0 failed**.
- `cargo test --workspace` → all crates green.
- `cargo check -p lisa-plugin --target wasm32-wasip1` → builds clean.

Determinism: both tests inject the clock via
`check_assignment_ack_timeouts_at(deadline)` with every deadline read out of the
matched state — no sleeps, no wall-clock reads, no provider tokens, no PTY
(honours the story's FREE, injected-time boundary).

Overlap with existing coverage is intentional and additive, not redundant:
- The delayed-send test adds the **pre-deadline quiescence** and **foreign-ticket
  rejection** the T-037-01-02 happy path did not assert.
- The prompt-miss test is the **only** grace-entered retry→`DeliveryFailed`
  case; the pre-existing `test_missing_fresh_chat_ack_...` enters the same
  resolution through the Claude `.started`-signal seam, exercising a different
  entry path to the same terminal state.

## Open concerns / limitations

- **Live validation remains deferred, by design.** These tests prove the state
  machine deterministically; they do not run a real Codex/Claude PTY. The single
  metered two-provider live rerun that actually closes E-037 is **S-037-02**,
  named explicitly in the story as the deferred step — not hidden here. Nothing
  in this ticket claims the live control is green.
- **No production change**, so no new runtime risk. If a future production edit
  alters the grace pacing or retry bound, these tests are the tripwire.
- **Helper scope:** `delivery_log_count` is `#[cfg(test)]`-only and private to
  the module; it asserts on an `Info` log-message substring, which couples the
  test to that message text. Acceptable — the same coupling already exists in
  `test_missing_fresh_chat_ack_...`; if the message changes, both update
  together.

## Nothing requires human escalation

The work is scoped to the lib.rs test module, matches the acceptance criteria
verbatim, and all suites are green. No TODOs, no known failures, no production
behaviour touched.
