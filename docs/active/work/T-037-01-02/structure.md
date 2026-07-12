# Structure — T-037-01-02 codex-startup-grace-pacing

One file changes: `crates/lisa-plugin/src/lib.rs` (ticket-owned). No files
created or deleted. `adapter.rs` is untouched (its capability landed in
T-037-01-01). All edits are additive or narrowly branch-widening.

## 1. New constant (near lib.rs:175, beside `MAX_ASSIGNMENT_DELIVERY_RETRIES`)

```rust
/// Bounded startup grace for grace-mode providers (Codex). After a fresh launch
/// Lisa waits this long for the TUI to become input-ready, then paces the first
/// prompt directly from `Starting` into `Delivering`. The elapsed grace PACES
/// the send; it is never evidence of readiness or ownership (E-037, P2).
/// SessionStart-mode providers (Claude) ignore this and gate on their positive
/// process-start signal instead.
const STARTUP_GRACE_SECS: u64 = 8;
```

## 2. New helper `startup_grace_deadline` (beside `assignment_ack_deadline`, ~lib.rs:1436)

```rust
/// The absolute deadline at which a grace-mode seat's startup grace elapses and
/// its paced first prompt is attempted. Saturating on overflow.
fn startup_grace_deadline(&self, now: std::time::SystemTime) -> std::time::SystemTime {
    now.checked_add(std::time::Duration::from_secs(STARTUP_GRACE_SECS))
        .unwrap_or(now)
}
```

## 3. Grace-aware deadline arming (`start_assignment_ack_wait`, lib.rs:1567–1576)

Only the `Starting { start_deadline: None, .. }` arm changes; the
`AssignedPendingAck` and `Recovering` arms (Codex recycle/recovery, post-prompt)
keep `deadline = assignment_ack_deadline(now)`.

Replace the Starting arm:

```rust
SeatAssignmentState::Starting {
    generation,
    start_deadline: None,
    relaunches,
} => {
    // Grace-mode (Codex) paces its first prompt after a bounded startup grace;
    // SessionStart-mode (Claude) bounds the wait for its process-start signal.
    let start_deadline = Some(
        if self.seat_readiness_mode(pane_id) == Some(ReadinessMode::Grace) {
            self.startup_grace_deadline(now)
        } else {
            deadline
        },
    );
    SeatAssignmentState::Starting {
        generation,
        start_deadline,
        relaunches,
    }
}
```

`deadline` (the `assignment_ack_deadline(now)` bound at the top of the fn) stays
the value for SessionStart seats and the two non-Starting arms. Presence of
`Some(_)` is unchanged, so `test_pane_title_fresh_launch_uses_actual_fallback_
route` and `scheduler_records_provider_readiness_mode_at_dispatch` stay green.

## 4. Grace-aware expiry (`check_assignment_ack_timeouts_at`, lib.rs:2183–2185)

Replace the single `Starting { relaunches: 0, .. }` arm. Destructure
`generation`; the sibling `Starting { .. }` (relaunches > 0) arm is unchanged.

```rust
SeatAssignmentState::Starting {
    relaunches: 0,
    generation,
    ..
} => {
    if self.seat_readiness_mode(pane_id) == Some(ReadinessMode::Grace) {
        // The named startup grace elapsed. Pace the first prompt now: attempt
        // the bounded attempt-tagged assignment and enter Delivering directly.
        // Elapsed time paced the send — it is not readiness or ownership
        // (E-037). A missed acknowledgement is resolved by the existing bounded
        // Delivering retry → DeliveryFailed path; ownership stays gated on the
        // exact UserPromptSubmit. A send that cannot be submitted resolves in a
        // named DeliveryFailed via fail_assignment_delivery.
        if let Err(error) = self.deliver_assignment_to_pane(pane_id, generation, 0, now) {
            self.fail_assignment_delivery(pane_id, &error);
        }
    } else {
        self.begin_startup_recovery(pane_id, now);
    }
}
```

## 5. Widen `fail_assignment_delivery` origin guard (lib.rs:1637–1645)

Add `Starting` to the accepted-origin match so a grace send-failure resolves in a
named terminal state instead of being a silent no-op:

```rust
if !matches!(
    self.seat_assignment(pane_id),
    Some(
        SeatAssignmentState::Starting { .. }
            | SeatAssignmentState::ReadyForAssignment { .. }
            | SeatAssignmentState::Delivering { .. }
    )
) {
    return;
}
```

Safe: the only caller that can pass a `Starting` seat is the new grace-expiry
arm; existing callers pass `ReadyForAssignment`/`Delivering`.

## 6. Test: `codex_startup_grace_paces_first_prompt_into_delivering` (append to `#[cfg(test)] mod tests`)

Proves the AC transition without sleeping (injected time):

- Codex: `pane_name_schedule_state("codex", AgentClient::Codex, None)` →
  `schedule_ready_tickets()`. Assert `seat_readiness_mode(10) == Grace` and read
  the armed `Starting { start_deadline: Some(grace) }`.
- `check_assignment_ack_timeouts_at(grace)` → assert `Delivering { generation:
  1, retries: 0 }` (NOT `ReadyForAssignment`, `StartupFailed`, `ResettingStartup`,
  or `Owned`); assert `!seat_is_owned(10)`.
- Assert an exact-generation `UserPromptSubmit` (`acknowledge_assignment(&mut
  state, 10, "T-NAME", 1)`) → `Owned`; a wrong generation does not.
- Claude arm: `pane_name_schedule_state("claude", AgentClient::Claude, None)` →
  schedule → `check_assignment_ack_timeouts_at(deadline)` → assert the seat did
  NOT auto-deliver (it went to `ResettingStartup`, i.e. SessionStart recovery),
  and that only `acknowledge_process_start` reaches `ReadyForAssignment`.

## 7. Split the now-divergent shared test (`same_pane_replacement_requires_start_and_chat_ack_for_both_providers`, lib.rs:10000–10105)

Its Codex branch asserts the *old* primary-`Starting` expiry (`ResettingStartup`
via `begin_startup_recovery`), which grace pacing replaces. Restructure to
**Claude-only** — it validates the SessionStart same-pane replacement contract
that stays intact — and rename to
`same_pane_replacement_requires_start_and_chat_ack_for_claude`. Remove the
`for (requested_agent, default_agent) in [(claude…),(codex…)]` loop; the Codex
grace behaviour is covered by test §6 (and its recovery contract by the
untouched recovery tests). Update the doc comment to state the Codex divergence
and where it is covered.

## Ordering of changes

1. Constant + helper (§1, §2) — inert additions, compile clean.
2. Deadline arming (§3) — grace seats get a grace deadline.
3. Expiry fork + `fail_assignment_delivery` widening (§4, §5) — the behavioural
   change; the new lifecycle is now complete.
4. New grace test (§6) and the shared-test split (§7).

Commit as coherent ticket-owned units via `lisa commit-ticket --include
crates/lisa-plugin/src/lib.rs`.

## Non-goals / untouched (per story N4)

- `adapter.rs`, `types.rs` `PluginConfig`: unchanged (no config knob this ticket).
- `acknowledge_process_start`: unchanged (Design option D rejected) — E-034
  recovery-fresh tests keep passing.
- `SeatAssignmentState` variants/fields: unchanged (no new variant).
- Delivering retry, `DeliveryFailed`, `acknowledge_codex_assignment` Owned edge:
  reused unchanged.
- The two big delayed-send / prompt-miss regressions: T-037-01-03.
