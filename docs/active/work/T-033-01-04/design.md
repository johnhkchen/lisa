# T-033-01-04 Design — bounded acknowledgment recovery

## Outcome

Add a configurable positive acknowledgment timeout to the existing recycled
Codex assignment state. Start the clock only when Lisa actually submits a
generation-tagged prompt. If the first deadline expires, fence that generation,
enter `Recovering`, exit the unacknowledged Codex TUI, and launch exactly one
fresh Codex session for the same ticket with a new generation. If the fresh
session acknowledges, promote it to `Owned`; if its deadline expires or it
reports an error, enter a terminal `RecoveryFailed` state and tell the operator
to reset the ticket.

This preserves the story's central invariant: a recycled physical seat is
never owned merely because Lisa typed into it. It also makes both the original
wait and the fallback finite.

## Decision 1 — configuration surface

### Option A: reuse `session_timeout_secs`

This value is an advisory whole-session budget measured from thread start. Its
default is one hour, it may be disabled with zero, and enforcement also depends
on prolonged silence. Those semantics do not describe a prompt acceptance
handshake and cannot satisfy a short finite contract deadline.

Rejected.

### Option B: reuse `CLEAR_SIGNAL_TIMEOUT_SECS`

The 90-second constant bounds a different provider event: confirmation that a
TUI reset completed. It is not project-configurable and begins before the
ticket prompt is sent. Coupling acknowledgment to clear transport would also
mis-handle the exit-grace launch path.

Rejected.

### Option C: add `assignment_ack_timeout_secs`

Add a scheduling setting carried through TOML, resolved CLI configuration, KDL,
and `PluginConfig`. Default it to 30 seconds. Require a positive value in CLI
validation and let the plugin retain the default for invalid direct KDL values.

Thirty seconds is comfortably above the two-second deferred Enter and the
five-second scheduler poll cadence while remaining operationally bounded. The
clock begins after prompt submission, so it does not need to absorb `/clear`
or `/exit` latency.

Selected.

Zero will not disable this deadline. An infinite pending assignment is the bug
this ticket removes, so a disabled form would contradict the acceptance
criterion. Projects that need more time can configure a larger positive value.

## Decision 2 — where the deadline lives

### Option A: pane-keyed side map

A `HashMap<pane_id, SystemTime>` would minimize enum changes, but it creates a
second lifecycle store that must be kept consistent across acknowledgment,
release, recovery, and terminal failure. Missing cleanup could apply an old
deadline to a later ticket.

Rejected.

### Option B: start time in `AgentSlot`

`AgentSlot::transition_started_at` already tracks `/clear` and `/exit` transport.
Reusing it would conflate pane mechanics with assignment acceptance, and the
value is cleared as soon as transition transport reaches `Idle`.

Rejected.

### Option C: optional absolute deadline in assignment variants

Extend generation-bearing states with `ack_deadline: Option<SystemTime>`.
`None` means the tagged prompt has not yet been submitted. `Some(deadline)`
means the provider acceptance clock is active. Ownership, identity, and timing
then change atomically in one map entry.

Selected.

An absolute deadline makes evaluation independent of which Zellij timer fired
and supports deterministic tests with an injected `now`.

## Decision 3 — recovery identity

### Option A: retain the original generation

This would let the fallback use the existing prompt marker, but a delayed
`UserPromptSubmit` payload from the abandoned reused session could arrive after
recovery begins and claim the replacement attempt.

Rejected.

### Option B: remove markers from the fresh fallback

Ordinary fresh Codex panes are immediately owned today. Applying that contract
would make recovery simple, but it would declare success solely on command
injection—the same category of false ownership this story is removing from a
known-bad handoff.

Rejected.

### Option C: allocate a new recovery generation

At initial timeout, allocate a second process-local generation and store it in
`Recovering`. The old generation becomes unreachable before `/exit` is sent.
The fresh launch carries the new marker and remains not-owned until the exact
recovery acknowledgment arrives.

Selected.

This turns generation into an attempt fence. Both a late original payload and
an unrelated ticket payload fail closed.

## Decision 4 — recovery state shape

Use these states:

```rust
enum SeatAssignmentState {
    AssignedPendingAck {
        generation: u64,
        ack_deadline: Option<SystemTime>,
    },
    Owned,
    Recovering {
        generation: u64,
        ack_deadline: Option<SystemTime>,
    },
    RecoveryFailed,
}
```

The meaning is explicit:

- pending/`None`: reserved, original prompt not submitted yet;
- pending/`Some`: original prompt submitted, awaiting exact acknowledgment;
- recovering/`None`: original attempt fenced, old TUI is being exited;
- recovering/`Some`: exactly one fresh fallback was submitted and is awaiting
  its exact acknowledgment;
- owned: one accepted assignment;
- recovery-failed: fallback reached a terminal actionable error.

`seat_is_owned` remains an equality check with `Owned`; every other state is
not-owned.

`RecoveryFailed` is preferable to leaving `Recovering` forever or deleting the
seat entry. Deletion would make the reservation look unassigned and enable a
second scheduler claim while the failed ticket still exists.

## Decision 5 — acknowledgment behavior

Generalize the current pending identity lookup to return the ticket-generation
identity for both `AssignedPendingAck` and `Recovering`. The exact detector
remains unchanged.

A matching payload performs either:

- `AssignedPendingAck -> Owned`, or
- `Recovering -> Owned`.

After either edge, the generation disappears and duplicates remain inert.

The prompt construction helper also reads the generation from either active
unowned state. This lets existing clear-timeout and exit-grace code build the
correct marker without provider-specific string rewriting.

## Decision 6 — starting the clock

Introduce one helper that changes `ack_deadline: None` to
`Some(now + configured_timeout)` for the addressed state. It does nothing for
owned, failed, already-armed, or missing assignments.

Call it immediately after each actual tagged delivery site:

- direct reuse-as-fresh-exec scheduling, if that adapter strategy is used;
- `.cleared` prompt submission;
- clear-timeout prompt fallback;
- exit-grace launch for the original assignment;
- exit-grace launch for the fresh recovery assignment.

Do not start it when `/clear` or `/exit` is submitted. Do not restart an
already-armed deadline on heartbeats or unrelated signals.

## Decision 7 — initial timeout transition

Add `check_assignment_ack_timeouts_at(now)` and a production wrapper using
`SystemTime::now()`. Run it in `poll_tick` after acknowledgment signals and
provider error signals, and after transition delivery so a prompt sent during
this poll is armed before evaluation.

For every expired original pending state:

1. verify the slot still retains the same ticket reservation;
2. allocate a new generation;
3. replace the pending state with recovering/`None` before any pane input;
4. send `/exit` using the resident Codex adapter contract;
5. set slot transport to `WaitingForExit`, with a fresh start time;
6. mark `has_session = false` so the next action is a shell launch;
7. clear stale attention/awaiting flags from the abandoned TUI;
8. log a warning naming pane, ticket, and recovery action.

Replacing the state before input makes a late old acknowledgment harmless even
if it is consumed on the next poll.

## Decision 8 — exactly one fresh launch

Reuse `WaitingForExit` and its eight-second grace. When that transition fires,
the state distinguishes normal cross-provider recycling from recovery.

For recovery:

1. construct the Codex launch command with the recovery generation;
2. submit it once;
3. set the slot to `Idle`, `has_session = true`, and Codex residency;
4. arm the recovery acknowledgment deadline;
5. retain `Recovering` rather than declaring ownership;
6. record one `SessionLaunch` event for observable test and operator history.

The transport transition is then cleared. Repeated timeout checks see neither
`WaitingForExit` nor a missing recovery deadline, so they cannot launch again.

The existing normal cross-provider branch keeps its behavior, aside from
arming an original pending deadline after its tagged launch.

## Decision 9 — fallback failure

Two conditions terminate recovery:

- the recovery acknowledgment deadline expires;
- the fresh Codex process writes `pane-<id>.error` while the seat is recovering.

Both call one `fail_assignment_recovery` helper. It:

- changes the seat to `RecoveryFailed`;
- marks the retained thread failed;
- records an error alert;
- logs an `ActivityEvent::Error` naming the ticket/pane and telling the operator
  to reset the ticket;
- retains the slot-to-ticket reservation and thread record.

The helper intentionally does not release the seat or remove the thread.
Automatic release would put the same ready ticket back through the handoff and
allow an infinite recovery loop. Retention makes the terminal state inspectable
and prevents another owner. The existing manual reset flow is the explicit
operator-authorized retry.

Generic `.error` behavior for non-recovery sessions remains unchanged.

## Decision 10 — missing or inconsistent state

If an expired assignment lacks its reserved slot/ticket, fail closed rather
than guessing:

- move the assignment to `RecoveryFailed` when the pane entry still exists;
- log an actionable state-consistency error;
- do not launch a process or release another ticket.

The normal release path continues to remove any assignment entry, so this is a
defensive condition expected only in malformed native tests or future bugs.

## Test design

### Core configuration

Test the 30-second default, positive map override, and rejection/fallback of
zero in the appropriate CLI/plugin layers.

### CLI transport

Test TOML parsing and resolution, known-key validation, default template
presence, init merge presence, and KDL layout emission.

### Scheduler acceptance test

Build a recycled Codex scheduling fixture with a one-second deadline. Deliver
the prompt through the clear handler, withhold acknowledgment, and evaluate
after the deadline. Assert pending-to-recovering, new generation, not-owned,
same ticket, and `/exit` transport.

Advance past exit grace and assert exactly one fresh Codex `SessionLaunch` with
the same ticket and recovery marker. Call transition timeout again and assert
the launch count remains one.

Advance past the recovery acknowledgment deadline with acknowledgment still
withheld. Assert `RecoveryFailed`, failed retained thread, retained ticket
reservation, one actionable error alert/log, no new launch, and not-owned.

Add a companion test that injects the matching recovery acknowledgment before
the second deadline and observes exactly one transition to `Owned`. Existing
stale/duplicate tests continue to prove fencing behavior.

### Regression coverage

Run focused plugin state-machine tests, core config tests, CLI config/layout/init
tests, package suites, workspace tests, formatting, WASM check, and production
Clippy. No live Codex tokens are required; the acceptance criterion is a
deterministic scheduler contract.

## Rejected broader changes

- No dashboard assignment-state rendering; that belongs to `S-033-02`.
- No change to ordinary fresh Codex immediate ownership.
- No Claude acknowledgment or recovery state.
- No terminal-screen scraping or synthetic heartbeat acknowledgment.
- No process spawning outside the existing pane command transport.
- No general scheduler retry-policy rewrite.

## Final design summary

The selected design gives each actual prompt delivery one absolute acceptance
deadline, uses a new generation to fence the abandoned attempt, reuses the
existing one-shot exit-grace transport for exactly one fresh fallback, and
turns a second failure into a durable operator-visible terminal state. Every
edge preserves the slot's ticket reservation, and only an exact current
generation acknowledgment can produce ownership.
