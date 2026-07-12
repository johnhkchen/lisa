# T-035-01-04 Design — bounded startup recovery

## Decision summary

Extend the provider-neutral `Starting` assignment with an optional absolute deadline,
arm it only when a fresh launcher is actually submitted, and transition an expired
start wait into a new terminal `StartupFailed` assignment state.

Reuse E-033's positive `assignment_ack_timeout_secs`, deadline arithmetic, injected-time
evaluator, alert/thread retention pattern, and signal-before-timeout poll ordering. Do
not reinterpret E-033's Codex prompt-ack recovery generation or automatically submit a
second provider launch.

Render the terminal state as `startup-failed` in red. Retain the ticket, thread, and
physical seat reservation until an operator uses the existing reset path.

## Goals

- Bound every actual fresh provider launch's wait for `.started`.
- Preserve exact lease-based `Starting -> Owned` admission.
- Let a matching start signal win on the deadline poll.
- Produce a stable, visible, actionable failure name.
- Prevent generic scheduler release/retry from creating an infinite launch loop.
- Cover the missing-signal path with deterministic native time injection.
- Leave E-033 recycled-Codex semantics and E-034 lease fencing intact.

## Non-goals

- Changing the SessionStart/process-start hook producer.
- Changing the atomic launcher transport.
- Adding a second startup timeout configuration value.
- Automatically restarting a fresh process whose positive start signal is absent.
- Generalizing all provider errors or session health behavior.
- Running a real Zellij PTY or installed provider.
- Changing ticket phase/status or completion publication.

## Option 1 — rely on session health or hard timeout

The scheduler already detects silent or overlong threads and can eventually fail them.

### Advantages

- No assignment-state change.
- No new tests or UI variant.

### Disadvantages

- `Starting` remains semantically unresolved during a much broader health window.
- Health is based on session activity and phase progress, not positive process start.
- Generic failure may release the seat and make the ready ticket schedulable again.
- The acceptance criterion requires a bounded startup-specific deadline and named state.

### Decision

Reject. The existing health system does not establish the requested state contract.

## Option 2 — treat timeout as implicit ownership

After a fixed wait, promote `Starting` to `Owned` on the assumption the signal hook was
lost even though the provider probably launched.

### Advantages

- Avoids blocking a healthy process when only signal delivery failed.
- Minimal scheduler machinery.

### Disadvantages

- Recreates the phantom ownership condition the story exists to remove.
- Breaks P2: state would claim provider acceptance without evidence.
- Makes a broken startup hook indistinguishable from a working agent.
- Directly violates “never reaching Owned” in the no-signal acceptance test.

### Decision

Reject.

## Option 3 — release and let normal scheduling retry

On expiry, fail/remove the thread and release the slot. The ready ticket can then be
dispatched again by ordinary scheduling.

### Advantages

- Reuses generic session error handling.
- May self-heal a transient provider launch failure.

### Disadvantages

- No bounded number of attempts exists at this boundary.
- A persistent missing hook can produce an endless relaunch loop.
- The actionable state disappears when the thread is removed.
- Operators see repeated starts rather than a retained diagnosis.

### Decision

Reject under N2.

## Option 4 — feed `Starting` into E-033's Codex recovery state

On startup timeout, call the existing `begin_assignment_recovery`, mint a successor
lease, exit the provider, launch one fresh session, and await E-033 acknowledgment.

### Advantages

- Reuses the one-fallback state machine literally.
- Provides one automatic recovery attempt and a terminal failure boundary.

### Disadvantages

- `begin_assignment_recovery` accepts only `AssignedPendingAck` and is intentionally
  specific to a recycled Codex prompt.
- It resolves the Codex adapter, sends Codex `/exit`, and expects `.ack`, while fresh
  start observation is provider-neutral and arrives as `.started`.
- Claude fresh launches would be forced through a Codex-specific contract.
- A newly launched process is already the fresh-session attempt at this boundary;
  launching another one does not repair a consistently missing start-signal producer.
- T-035-01-03 explicitly left startup recovery as an extension of `Starting`, separate
  from Codex acknowledgment recovery fields.

### Decision

Reject. Reuse the bounded pattern and policy, not the provider-specific state meaning.

## Option 5 — add startup deadline and terminal `StartupFailed`

Change `Starting` to carry `start_deadline: Option<SystemTime>`. Extend the existing
deadline arming helper to arm a fresh start wait after actual launcher submission. Add
startup states to the same injected-time evaluation pass. On expiry, retain the
reservation and enter `StartupFailed` with a failed thread, deduplicated alert, and
operator reset instruction.

### Advantages

- Preserves provider-neutral start semantics.
- Makes the wait finite using an already configured positive policy.
- Provides the clearest operator-facing diagnosis.
- Does not create any automatic launch edge, so retry count is statically zero.
- Reuses E-033's testable deadline and retained-failure architecture.
- Keeps start signal acceptance independent of Codex prompt acknowledgment.

### Disadvantages

- Adds one internal and one UI enum variant.
- The same config value governs two related acceptance waits.
- A provider that started successfully but lost its signal requires operator action.

### Decision

Choose this option. Positive ownership evidence is the story's safety invariant; a
false-negative signal should stop visibly rather than silently weaken that invariant.

## State design

Use:

```rust
Starting {
    generation: u64,
    start_deadline: Option<SystemTime>,
}
```

`None` means a fresh route is reserved but its launcher has not yet been submitted.
This is expected during cross-provider `WaitingForExit`.

`Some(deadline)` means the short prepared launcher was sent and its delayed Enter plus
provider-start acceptance window are bounded.

Add:

```rust
StartupFailed
```

The terminal state carries no retry counter or timestamp. Detailed reason and pane are
published through the activity error/alert path; the status label remains compact.

## Clock policy

Reuse `PluginConfig::assignment_ack_timeout_secs`. Both clocks bound positive provider
acceptance after submitting input to a native session boundary:

- E-033 bounds a recycled/recovery Codex prompt acknowledgment;
- this ticket bounds a fresh provider process-start acknowledgment.

The existing CLI guarantees a positive value and documents one finite fresh-session
fallback. A separate setting would increase configuration surface without a stated
product distinction.

Compute the deadline as configured seconds plus `ENTER_DELAY_SECS`, using the same
overflow fallback already present in `start_assignment_ack_wait`.

## Arming boundary

Arm only after actual fresh launcher submission.

For empty panes and `FreshExec`, scheduling has sent the launcher before it installs the
assignment. After inserting the state, the slot is `Idle`, so the existing post-dispatch
arming call can extend from generation-tagged acknowledgments to every unarmed timed
assignment in an idle slot.

For cross-provider recycling, scheduling installs `Starting { deadline: None }` while
the slot is `WaitingForExit`. The post-dispatch arming guard remains false. When
`check_transition_timeouts` sends the prepared incoming launch, it already calls the
arming helper; extending that helper arms startup at the correct point.

No deadline begins at lease reservation, `/exit`, or launcher file preparation.

## Timeout transition

Extend `check_assignment_ack_timeouts_at` to collect expired `Starting` values with a
present deadline. Preserve its collect-then-compare pattern so mutations are race-safe
within one evaluation pass.

For an unchanged expired `Starting`, call a new `fail_startup` helper. The helper:

1. verifies the current state is `Starting`;
2. writes `StartupFailed` before any logging;
3. resolves the retained ticket from the pane slot;
4. marks the retained thread failed when present;
5. adds one `(ticket, pane)` error alert if absent;
6. logs that provider startup was not observed and ticket reset is required.

It must not revoke the lease, release the slot, remove the thread, send `/exit`, or
submit any launch command.

## Poll ordering

Keep current order:

```text
check_process_start_signals
...
check_transition_timeouts
check_assignment_ack_timeouts
```

A `.started` file visible on the deadline tick promotes to `Owned` before expiry is
evaluated. Cross-provider launch delivery also arms the deadline before the evaluator,
but the newly computed future deadline cannot expire in that same tick.

## UI decision

Add `SeatAssignmentStatus::StartupFailed`:

- label: `startup-failed`;
- color: red;
- mapping: internal `StartupFailed` to UI `StartupFailed`.

Do not reuse `recovery-failed`: that label refers to E-033's failed fresh Codex recovery
after a reused-prompt timeout. The distinct label tells the operator which positive
signal boundary failed.

## Native regression design

Extend the existing fresh-dispatch test or add a sibling focused test. A separate test
keeps the positive exact-start assertions readable.

Test sequence:

1. create a native empty-pane scheduler fixture;
2. set `assignment_ack_timeout_secs` to one second;
3. schedule the ready ticket without creating `.started`;
4. assert `Starting` with `Some(deadline)` and not owned;
5. record the launch command/event count;
6. call `check_assignment_ack_timeouts_at(deadline)`;
7. assert `StartupFailed`, failed retained thread, alert, and `startup-failed` row;
8. assert never owned and launch count unchanged;
9. call the evaluator at multiple later times;
10. assert state and launch count remain unchanged.

The test proves a stronger finite property than merely counting one poll: no terminal
state branch contains a relaunch action.

## Compatibility and risks

Changing the shape of `Starting` requires updates to exhaustive test patterns and UI
mapping. Compiler errors provide a complete inventory.

E-033 states remain unchanged. Existing acknowledgment tests protect their behavior.
E-034 tests protect exact lease authority and fencing.

The main behavioral risk is arming before cross-provider launch. Retaining the existing
`transition_state == Idle` guard and arming in the exit-grace delivery path prevents it.

The main operational tradeoff is stopping on lost positive evidence. This is deliberate:
operators receive a named red state and explicit reset action rather than false ownership
or silent automatic retries.
