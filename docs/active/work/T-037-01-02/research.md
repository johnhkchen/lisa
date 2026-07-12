# Research — T-037-01-02 codex-startup-grace-pacing

Descriptive map of the bootstrap-readiness machine this ticket must extend. No
solutions here; those are Design.

## Where this lives

All behaviour is in `crates/lisa-plugin/src/lib.rs`, in the `SeatAssignmentState`
machine and the `check_assignment_ack_timeouts_at` deadline evaluator. The
provider-readiness classification it keys on already exists in
`crates/lisa-plugin/src/adapter.rs` (landed by T-037-01-01) and is already
imported and recorded in `lib.rs`.

## The seat-assignment state machine

`SeatAssignmentState` (lib.rs:284) is the per-pane assignment truth, keyed by
physical pane id in `State::seat_assignments` (lib.rs:436). Relevant variants:

- `Starting { generation, start_deadline: Option<SystemTime>, relaunches: u8 }`
  — a fresh provider process was launched; its exact process-start signal has not
  been observed. `start_deadline` is `None` until the launcher is actually
  submitted, then `Some(bound)`.
- `ReadyForAssignment { generation }` — the exact process-start signal arrived;
  the bounded chat reference will be submitted on the next poll.
- `Delivering { generation, ack_deadline, retries }` — the chat reference was
  submitted; awaiting exact `UserPromptSubmit` evidence.
- `Owned` — the provider positively acknowledged the assignment.
- `ResettingStartup`, `StartupFailed`, `AssignedPendingAck`, `Recovering`,
  `RecoveryFailed`, `DeliveryFailed` — recovery/terminal states.

## The current fresh-launch lifecycle (provider-uniform today)

1. **Dispatch** (`schedule_ready_tickets`, ~lib.rs:2593–2793): writes the
   assignment file (`prepare_assignment`, lib.rs:2609), submits a bare launcher,
   inserts `Starting { start_deadline: None, relaunches: 0 }` (lib.rs:2767), and
   records the provider readiness mode for the pane
   (`seat_readiness.insert(pane_id, adapter.readiness_mode())`, lib.rs:2787).
2. **Arm the wait** (`start_assignment_ack_wait`, lib.rs:1559): when the pane is
   `Idle`, rewrites `Starting.start_deadline` from `None` to
   `Some(assignment_ack_deadline(now))` (lib.rs:1567–1576). Today this same
   deadline is used for both providers.
3. **Process-start proof** (`acknowledge_process_start`, lib.rs:1405): a
   provider-neutral `pane-<id>.started` signal file (consumed by
   `check_process_start_signals`, lib.rs:3095) carrying the exact `AttemptLease`
   promotes `Starting → ReadyForAssignment`.
4. **Deliver** (`deliver_ready_assignments` → `deliver_assignment_to_pane`,
   lib.rs:1447/1517): submits the bounded, attempt-tagged assignment reference
   and inserts `Delivering { retries: 0, ack_deadline }`.
5. **Own** (`acknowledge_codex_assignment`, lib.rs:1600): an exact-generation
   `UserPromptSubmit` payload promotes `Delivering → Owned`. This is the only
   edge that publishes `Owned` for a fresh launch.

## The deadline evaluator (the transition seam this ticket changes)

`check_assignment_ack_timeouts_at(now)` (lib.rs:2144) collects every seat whose
armed deadline is `<= now`, re-checks the snapshot is still current (lib.rs:2179),
then dispatches by state:

- `Starting { relaunches: 0, .. }` → `begin_startup_recovery` (lib.rs:2183) —
  revoke and start the one same-pane shell reset.
- `Starting { .. }` (relaunches > 0) → `fail_startup_recovery` (lib.rs:2186).
- `Delivering { retries < MAX, .. }` → re-`deliver_assignment_to_pane` with
  `retries + 1` (lib.rs:2198). `MAX_ASSIGNMENT_DELIVERY_RETRIES = 1` (lib.rs:175).
- `Delivering { .. }` (exhausted) → `fail_assignment_delivery` → `DeliveryFailed`
  (lib.rs:2209).
- `AssignedPendingAck` / `Recovering` → recovery / recovery-fail.

So today a **grace-mode Codex primary launch deadlocks**: no pre-prompt
process-start signal ever arrives (Codex 0.144.1 emits SessionStart only *after*
the first prompt), so step 3 never fires, the `Starting` deadline elapses, and
the seat drops into `begin_startup_recovery` → eventually `StartupFailed` — the
E-037 root cause.

## The readiness classification already in place (T-037-01-01)

- `adapter::ReadinessMode { SessionStart, Grace }` (adapter.rs:125), `Copy`.
- `AgentAdapter::readiness_mode()` (adapter.rs:196): `ClaudeCodeAdapter →
  SessionStart` (adapter.rs:272), `CodexAdapter → Grace` (adapter.rs:397).
- `State::seat_readiness: HashMap<u32, ReadinessMode>` (lib.rs:443), written at
  **both** fresh-`Starting` dispatch sites: primary (lib.rs:2787) and the
  post-exit recovery relaunch (lib.rs:4032). Entries are overwritten per launch,
  never removed.
- Accessor `seat_readiness_mode(pane_id) -> Option<ReadinessMode>` (lib.rs:1398),
  currently `#[cfg_attr(not(test), allow(dead_code))]` — its first non-test
  consumer is this ticket.

## Delivery mechanics relevant to a paced send

`deliver_assignment_to_pane(pane_id, generation, retries, now)` (lib.rs:1447) is
state-agnostic on entry: it validates the slot lease matches `generation` and is
current, checks the assignment file exists, submits `adapter.assignment_reference`,
and inserts `Delivering { generation, ack_deadline: assignment_ack_deadline(now),
retries }`. It returns `Err` on: pane awaiting human (`is_pane_awaiting`), missing
reservation, stale lease, or missing assignment file. It does **not** require any
particular prior state, so it can be invoked directly from a `Starting` seat.

`fail_assignment_delivery` (lib.rs:1636) currently only acts when the seat is
`ReadyForAssignment | Delivering`; a `Starting` seat is a no-op for it today.

## Constants / config

- `POLL_INTERVAL_SECS = 5.0` (lib.rs:42) — deadline checks run per poll.
- `ENTER_DELAY_SECS = 2.0` (lib.rs:169) — deferred Enter after typed text.
- `MAX_ASSIGNMENT_DELIVERY_RETRIES = 1` (lib.rs:175).
- `assignment_ack_timeout_secs` (types.rs:665, default 30) — the acceptance
  clock; also currently doubles as the `Starting` wait bound.
- There is **no** dedicated startup-grace duration constant yet.

## Existing tests that touch the transition (breakage surface)

- `same_pane_replacement_requires_start_and_chat_ack_for_both_providers`
  (lib.rs:10000): loops Claude *and* Codex; for Codex it lets the primary
  `Starting` deadline elapse and asserts `ResettingStartup` (lib.rs:10019–10038)
  — this Codex branch is exactly what grace pacing replaces.
- `scheduler_records_provider_readiness_mode_at_dispatch` (lib.rs:9641) and
  `test_pane_title_fresh_launch_uses_actual_fallback_route` (lib.rs:9612): assert
  a fresh Codex `Starting` has `start_deadline: Some(_)`; grace changes the
  *value* not the *presence*, so these stay green.
- Four Codex **recovery** tests (`test_dropped_post_prompt_ack…`,
  `test_bounded_ack_wait_recovers_once…`, `test_recovery_ack_promotes_only…`, and
  the `T0330302` loop) use the `start_and_deliver_fresh_recovery` helper
  (lib.rs:9568), which drives the recovery-fresh `Starting` to `Owned` via a
  *synthetic* `acknowledge_process_start`. These acknowledge before any deadline
  elapses, so a grace-*expiry* change does not fire in them.

## Constraints / assumptions

- Claude's `SessionStart → ReadyForAssignment` evidence path must stay byte-for-
  byte unchanged (epic N1, story "Honest boundary").
- Elapsed grace PACES the send; it must never publish `ReadyForAssignment`,
  `StartupFailed`, or `Owned` (P2/N3). `Owned` stays gated solely on exact
  `UserPromptSubmit` via `acknowledge_codex_assignment`.
- Scope is the bootstrap-readiness gating and its Codex grace path only — no
  broad scheduler rewrite, no change to E-034 fencing (story N4). File ownership
  for this ticket is `lib.rs`; a config knob in `types.rs` is out of ownership.
- Injected-time, free, deterministic: no live PTY, no provider tokens.
