# T-033-03-01 Research — deterministic stall reproduction

## Ticket boundary

This ticket adds standing regression evidence for the Codex reused-seat handoff
failure that motivated `S-033-01`. It does not change scheduler behavior. The
story explicitly places scheduler transitions in `S-033-01`, dashboard work in
`S-033-02`, and live consecutive-reuse evidence in follow-on ticket
`T-033-03-02`.

The acceptance criterion requires one committed, CI-runnable test that proves
three connected facts:

- a post-prompt Codex acceptance event can be deterministically dropped;
- the former fire-and-forget contract would then retain apparent ownership
  without acknowledgment and without a recovery boundary;
- the current acknowledgment-gated contract stays unowned and reaches bounded
  recovery instead of silently waiting.

No live Codex process may be required. The evidence must exercise native Rust
state and injected time.

## Historical open-loop behavior

The parent of commit `47e64b4` contains the original scheduler shape. On a
compatible resident Codex pane, `schedule_ready_tickets` sent `/clear`, retained
the pane's live-session flag, attached the new ticket to the slot, and inserted
a running thread. After `.cleared`, `handle_cleared_signal` sent the ticket
prompt and returned the transition to `Idle`.

That version had no `seat_assignments` map, assignment generation, native
acceptance scanner, or acknowledgment deadline. The scheduler's observable
facts after prompt delivery were therefore:

- `AgentSlot::ticket_id` named the new ticket;
- the thread existed with `Running` status;
- the pane had a resident session;
- the transport state was `Idle`;
- no provider acceptance fact existed;
- no timeout could distinguish accepted work from a lost prompt event.

Those facts made the seat appear successfully handed off even if the
`UserPromptSubmit` evidence never reached Lisa. Subsequent scheduling could not
claim the reserved pane, while the assigned ticket could remain quiet with no
handoff-specific recovery. This is the owned-without-ack silent stall the
ticket asks to preserve as regression evidence.

## Current assignment truth

`crates/lisa-plugin/src/lib.rs` now stores explicit seat truth in
`State::seat_assignments`, keyed by physical terminal pane ID.

The relevant states are:

```text
AssignedPendingAck { generation, ack_deadline }
Owned
Recovering { generation, ack_deadline }
RecoveryFailed
```

Only exact equality with `Owned` makes `seat_is_owned` true. Ticket reservation,
thread presence, a live session, an idle transport, and prompt injection are no
longer sufficient ownership evidence.

For an in-place reused Codex assignment, scheduling allocates a nonzero
generation and enters `AssignedPendingAck` with no deadline. `/clear` is only
transport. `handle_cleared_signal` builds the generation-tagged reuse prompt,
types it into the pane, arms the acknowledgment deadline, and returns pane
transport to `Idle`.

The deadline is absolute `SystemTime` and includes the deferred Enter delay.
Tests can read it from state and pass it directly to
`check_assignment_ack_timeouts_at`, so no sleep, timer, Zellij host, or Codex
process is involved.

## Acceptance-event transport

Codex prompt acceptance is represented by a raw `UserPromptSubmit` JSON
payload. Lisa's generated `on-ack.sh` hook writes that payload to
`.lisa/signals/pane-<id>.ack`.

`State::check_codex_ack_signals` scans the configured signal directory, parses
the pane ID from the filename, reads and deletes the file, and asks
`acknowledge_codex_assignment` to promote the seat. Promotion succeeds only if
the payload's ticket and generation match the seat's active identity.

The signal file is the clean deterministic loss seam. A test can construct the
exact matching payload, write the normal `pane-10.ack` file, and remove it
before the scanner runs. Calling the scanner then observes the same absence as
a post-hook event lost before scheduler consumption. This is more explicit
than merely omitting a call to the acknowledgment helper.

The existing fixture in
`crates/lisa-plugin/tests/fixtures/codex_ack/matching-prompt-submit.json` is tied
to ticket `T-033-01-02` and generation 42. Scheduler tests commonly construct
payloads with `codex_ack::tag_codex_assignment`, which permits the current
test's ticket and dynamically allocated generation without adding another
static fixture.

## Bounded recovery path

`check_assignment_ack_timeouts_at` inspects all armed pending and recovering
assignments. At the original deadline it calls `begin_assignment_recovery`.

Recovery performs these state changes:

1. verifies the seat still has a pending assignment;
2. allocates a new generation before sending terminal input;
3. replaces pending state with unowned `Recovering` and no deadline;
4. sends `/exit` to abandon the resident Codex TUI;
5. moves pane transport to `WaitingForExit`;
6. retains the same ticket reservation and running thread;
7. logs a warning naming timeout and recovery.

Replacing the generation before input fences any late original acceptance.
The first recovery boundary is finite and deterministic.

After exit grace, `check_transition_timeouts` launches a fresh Codex command
for the same ticket and the recovery generation, returns transport to `Idle`,
and arms the second acknowledgment deadline. Repeated transition checks cannot
launch again because `WaitingForExit` has been cleared.

If the recovery event is also absent, evaluating the second deadline enters
`RecoveryFailed`, marks the retained thread failed, records an error alert, and
logs reset-required guidance. The scheduler does not release the ticket into an
automatic retry loop. This is the terminal bounded outcome.

## Existing test surface

The plugin's native unit tests already provide a reusable state builder:
`pane_name_schedule_state(requested_agent, default_agent, resident_agent)`.
It creates a temporary ticket directory with `T-NAME`, builds a DAG, enables
scheduling, and installs pane 10 with the requested resident provider.

Related tests cover pieces of the contract:

- `test_codex_ack_signal_promotes_matching_pending_seat` proves a present raw
  signal promotes and is consumed;
- `test_recycled_codex_ownership_requires_matching_ack_exactly_once` proves
  exact identity and idempotence;
- `test_bounded_ack_wait_recovers_once_then_fails_actionably` covers withheld
  acknowledgment through recovery failure;
- `test_recovery_ack_promotes_only_the_fresh_generation` covers successful
  recovery acknowledgment;
- `test_reused_claude_assignment_remains_owned` protects the Claude contract.

The missing evidence is one named regression that explicitly creates and drops
the post-prompt event, records why the old open-loop interpretation was a false
owner, and follows the current state through a finite recovery boundary. The
existing bounded-recovery test withholds acknowledgment implicitly and focuses
on the implementation ticket's exhaustive transition criterion; it does not
materialize the lost event or contrast the historical contract.

## Best test location

The scheduler state and helpers are private to the plugin crate. Unit tests in
the `#[cfg(test)]` module at the bottom of `crates/lisa-plugin/src/lib.rs` can
exercise them directly. An external integration test would require widening
production visibility solely for test access or duplicating scheduler logic.

The new regression belongs beside the existing recycled-Codex ownership and
bounded-recovery tests. That location makes the historical comparison and the
current behavioral proof discoverable with the contract it protects.

No production file, configuration, hook, or fixture needs modification.

## CI and determinism constraints

`cargo test -p lisa-plugin` runs the plugin's native tests; it does not start
Zellij or Codex. Host-pane calls are replaced by test-side activity recording
under the plugin's existing test configuration.

The regression should not sleep or compare wall-clock durations. It can:

- set `assignment_ack_timeout_secs` to one second;
- extract the exact armed deadline from seat state;
- evaluate at that deadline;
- backdate `transition_started_at` past the fixed exit grace;
- invoke transition evaluation synchronously;
- extract and evaluate the recovery deadline.

Temporary directories isolate the signal file. Removing the matching `.ack`
before `check_codex_ack_signals` makes the loss deterministic on every run.

## Worktree and ownership constraints

The worktree contains unrelated modified and untracked files. The only planned
source path is `crates/lisa-plugin/src/lib.rs`. It is currently clean relative
to `HEAD` and was last changed by dependency commit `f907a76`.

Ticket-owned source must be committed with
`lisa commit-ticket --ticket-id T-033-03-01` and an exact include for that file.
The ordinary index must remain untouched. The ticket and RDSPI artifacts remain
for Lisa's completion transaction and must not be included in the source
commit.

## Research conclusion

The repository already exposes every required deterministic seam. A single
native plugin regression can use the real scheduling, prompt-delivery, signal
scanner, ownership state, deadline injection, and recovery launch path. The
test needs only to make event loss explicit and encode the historical
open-loop interpretation as a test-local observation of the same reserved,
running, transport-idle facts that previously stood in for acceptance.
