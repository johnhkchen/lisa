# T-034-01-03 Structure — revoke and fence before reschedule

## Change boundary

The ticket modifies one source file:

```text
crates/lisa-plugin/src/lib.rs
```

No public crate interface, serialized core type, configuration schema,
dependency, CLI command, hook, layout, or ticket frontmatter changes.

Workflow artifacts are created under:

```text
docs/active/work/T-034-01-03/
```

## `TransitionState`

Extend the private enum with one terminal variant:

```rust
Fenced
```

Meaning: Lisa has closed or otherwise permanently disqualified this terminal
pane after revoking its ticket attempt. The slot record remains for bounded
diagnostics but cannot be reused by the scheduler.

The variant has no deadline and no transition fallback.

`find_slot_for_client` already requires `TransitionState::Idle`, so no selection
predicate change is required for `Fenced` exclusion.

Any exhaustive transition match gains a `Fenced` no-op arm where needed.

## Fence result type

Add a private enum near `TransitionState`:

```rust
enum FenceOutcome {
    Fenced { pane_id: u32 },
    AlreadyFenced { pane_id: u32 },
    NoAssignedPane,
}
```

The result makes every invocation terminate in a named bounded state.

It is diagnostic/control-flow state only. It is not serialized and is not part
of `lisa-core`.

The normal acceptance path must produce `FenceOutcome::Fenced`.

## Test lifecycle trace

Under `#[cfg(test)]`, add a private enum:

```rust
enum AttemptLifecycleEvent {
    LeaseRevoked { ticket_id: TicketId },
    PaneFenced { ticket_id: TicketId, pane_id: u32 },
    SlotReleased { ticket_id: TicketId },
}
```

Derive `Debug`, `Clone`, `PartialEq`, and `Eq` for exact assertions.

Add a test-only field to `State`:

```rust
#[cfg(test)]
attempt_lifecycle: Vec<AttemptLifecycleEvent>,
```

`State::default` initializes the trace automatically.

Production WASM state contains no trace allocation or event retention.

## `State` lease registries

Retain:

```rust
current_leases: HashMap<TicketId, AttemptLease>
```

Refine its documentation and semantics to mean only currently authorized
ticket attempts.

Add:

```rust
lease_high_water: HashMap<TicketId, AttemptLease>
```

This map records the latest successfully minted lease for monotonic successor
generation. Entries survive revocation, release, and thread removal.

The invariant while an attempt is assigned is:

```text
current_leases[ticket]
    == lease_high_water[ticket]
    == slot.attempt_lease
    == thread.attempt_lease
```

The invariant after release is:

```text
current_leases[ticket] is absent
lease_high_water[ticket] is retained
slot.attempt_lease is absent
thread is absent (once the caller completes teardown)
```

## Dispatch mutation

In `schedule_ready_tickets`, replace the mint predecessor:

```rust
self.current_leases.get(&ticket_id)
```

with:

```rust
self.lease_high_water.get(&ticket_id)
```

After successful minting, insert the same clone into both registries:

```rust
self.lease_high_water
    .insert(ticket_id.clone(), attempt_lease.clone());
self.current_leases
    .insert(ticket_id.clone(), attempt_lease.clone());
```

Both insertions remain before pane lifecycle side effects.

No provider branch changes. The physical slot and logical thread continue to
receive the same minted value.

## Current-lease revocation helper

Add a small private method:

```rust
fn revoke_current_lease(&mut self, ticket_id: &TicketId) -> Option<AttemptLease>
```

The method removes and returns `current_leases[ticket_id]`.

Under tests, it records `LeaseRevoked` only when an entry was actually removed.

It never mutates `lease_high_water`.

This method is the single mutation spelling used by the timeout fence and
shared release boundary.

## Terminal-pane close wrapper

Add a narrow private method or free function:

```rust
fn close_fenced_pane(pane_id: u32)
```

In production/WASM builds it calls:

```rust
close_terminal_pane(pane_id)
```

In unit tests it performs no host call. Ordering and state are observed through
the test lifecycle trace and slot transition state.

Keeping the conditional at this wrapper prevents native tests from invoking a
Zellij host function while keeping production behavior direct.

## `revoke_and_fence_attempt`

Add a private `State` method:

```rust
fn revoke_and_fence_attempt(
    &mut self,
    ticket_id: &TicketId,
) -> FenceOutcome
```

### Step 1 — revoke

Call `revoke_current_lease(ticket_id)` before inspecting or mutating the pane.

The high-water entry remains unchanged.

### Step 2 — locate slot

Find the slot whose `ticket_id` equals the requested ticket.

If no slot exists, log a warning/error and return `NoAssignedPane`. Authority
stays revoked.

If the matching slot is already `Fenced`, return `AlreadyFenced { pane_id }`
without issuing another close request.

### Step 3 — disqualify state

Before releasing the ticket reservation, mutate the slot to:

```rust
transition_state = TransitionState::Fenced;
transition_started_at = None;
has_session = false;
last_client = None;
cooldown_until = None;
```

Keep `ticket_id` and `attempt_lease` temporarily intact so release remains the
operation that clears assignment stamps.

### Step 4 — clear pane-scoped queues

Remove the pane from:

- `seat_assignments`;
- `awaiting_human`;
- `notified_attention`.

Retain only pending Enter records for other panes.

### Step 5 — terminate and record

Call the close wrapper exactly once.

Under tests, append `PaneFenced` after the slot has entered `Fenced` and before
release can run.

Log an informational/error activity entry naming ticket and pane.

Return `Fenced { pane_id }`.

The helper never schedules, sleeps, arms a timer, or retries.

## `release_slot_for_ticket`

At method entry, call `revoke_current_lease(ticket_id)`.

This is idempotent after `revoke_and_fence_attempt` and protects every other
release caller.

Split slot cleanup by transition state.

### Ordinary slot

Preserve existing semantics:

- clear ticket and lease stamps;
- retain resident `has_session` state;
- apply wind-down cooldown;
- derive resident idle name;
- remove seat assignment;
- rename pane;
- log ordinary release.

### Fenced slot

Perform:

- clear ticket and lease stamps;
- preserve `TransitionState::Fenced`;
- keep `has_session = false` and `last_client = None`;
- leave cooldown absent;
- remove seat assignment;
- skip pane rename because the terminal is closed;
- log a fenced release.

Under tests, append `SlotReleased` after assignment stamps are clear.

The lifecycle trace for a normal hard timeout is therefore fixed as:

```text
LeaseRevoked -> PaneFenced -> SlotReleased
```

## `check_session_timeouts`

Keep budget calculation, hard-silence calculation, awaiting-human guard,
warning debounce, provenance, alerts, and activity output unchanged.

For each `timed_out` ticket, mutate in this order:

1. mark thread failed;
2. emit timed-out provenance;
3. call `revoke_and_fence_attempt`;
4. call `release_slot_for_ticket`;
5. remove thread;
6. publish timeout alert;
7. publish `SessionTimedOut` activity.

The fence outcome is logged by the helper. A missing pane does not prevent
logical cleanup or leave authority current.

Update the method documentation: the provider pane is closed at hard silence,
not preserved.

## `detect_stale_threads`

Keep stale detection and awaiting-human filtering unchanged.

For each stale ticket, insert `revoke_and_fence_attempt` before shared release.

This path uses `RunOutcome::Failed` rather than `TimedOut` and retains its
existing activity message.

Both hard-silence paths therefore share identical lease/pane teardown.

## Tests in `crates/lisa-plugin/src/lib.rs`

### Update dispatch lease test

Change `dispatch_mints_and_stamps_strictly_new_attempt_lease` to assert:

- first dispatch populates both maps with the same lease;
- release removes the current-authority entry;
- release retains `lease_high_water` as attempt 1;
- second dispatch mints attempt 2 from high-water;
- both maps and both stamps equal attempt 2;
- attempt 1 is not current after release or redispatch.

### Replace/extend timeout acceptance test

Use a real ticket/DAG fixture with two slots.

Dispatch attempt 1 or explicitly install a minted lease consistently across:

- high-water map;
- current map;
- timed-out thread;
- slot 1.

Make the thread exceed both configured budget and hard-silence threshold.

Call `check_session_timeouts`.

Assert:

- exact lifecycle ordering;
- `FenceOutcome` is represented by slot 1 `Fenced` state;
- current authority is absent;
- old lease fails `is_current`;
- high-water retains attempt 1;
- slot 1 is unassigned and has no lease/session;
- thread removal and named timeout alert remain intact.

Make slot 2 schedulable, invoke `schedule_ready_tickets`, and assert attempt 2
is assigned there and is strictly greater/current.

### Stale-path regression

Extend an existing stale-thread test or add a focused test proving pure stale
detection also leaves its slot `Fenced` and current authority absent.

### Ordinary release regression

Existing release and session-reuse tests continue to assert that non-timeout
release retains a healthy resident session and cooldown.

Add current-authority removal where a lease fixture is present.

## Build and verification surfaces

Run:

```text
cargo fmt --all -- --check
cargo test -p lisa-plugin revoke
cargo test -p lisa-plugin session_timeout
cargo test -p lisa-plugin stale
cargo test -p lisa-plugin dispatch_mints_and_stamps_strictly_new_attempt_lease
cargo test --workspace
just check
git diff --check
```

The WASM check is important because the production close wrapper compiles the
Zellij host call only outside native unit-test configuration.

## Commit boundary

The meaningful ticket-owned source unit is the scheduler lease/fence change and
its colocated tests in:

```text
crates/lisa-plugin/src/lib.rs
```

Commit it through:

```text
lisa commit-ticket --ticket-id T-034-01-03 \
  --message "fix: revoke and fence timed-out attempts" \
  --include crates/lisa-plugin/src/lib.rs
```

The workflow artifacts remain for Lisa's completion transaction and are not
included in the source commit.

## Final architecture

```text
dispatch
  mint from lease_high_water
  -> lease_high_water[ticket] = N
  -> current_leases[ticket] = N
  -> slot/thread = N

hard-silence timeout
  revoke current_leases[ticket]
  -> close pane + TransitionState::Fenced
  -> release slot stamps
  -> remove thread

redispatch on another idle pane
  mint from lease_high_water[ticket] = N
  -> current/high-water/slot/thread = N + 1
```

This structure makes release unable to expose a valid prior lease and makes a
timed-out pane unable to return to scheduler ownership.
