# T-034-02-01 Review — bind Codex ack to lease

## Review outcome

The implementation satisfies the ticket acceptance criterion.

Codex acknowledgement generations now come from the ticket's current
`AttemptLease`.

Ownership promotion requires the pane's stamped lease to validate exactly
against scheduler current authority.

A regression test proves an acknowledgement carrying the prior lease leaves a
replacement pending, while the current lease acknowledgement promotes it to
`Owned` exactly once.

The obsolete acknowledgement/ownership-path `#[allow(dead_code)]` is removed.

No critical issue requires human intervention before Lisa completes the
ticket.

## Source commit

The ticket-owned source change was committed through Lisa's isolated
transaction.

```text
77e524f70eb2356904edd8fd15a8b4ecd4f308e3
fix: bind Codex acknowledgements to attempt leases
```

The commit contains exactly:

```text
crates/lisa-plugin/src/lib.rs
```

The ordinary Git index remained empty.

The ticket-owned source path is clean after the commit.

Unrelated pre-existing working-tree changes were preserved and excluded.

## Files modified

### `crates/lisa-plugin/src/lib.rs`

Changed acknowledgement identity allocation, promotion authority validation,
recovery lease handling, and scheduler tests.

## Files created

Created the six RDSPI artifacts under:

```text
docs/active/work/T-034-02-01/
```

- `research.md`;
- `design.md`;
- `structure.md`;
- `plan.md`;
- `progress.md`;
- `review.md`.

These artifacts are intentionally outside the ticket-owned source commit.

Lisa owns their final isolated completion transaction.

## Files deleted

None.

## Marker generation change

The prior plugin maintained two independent identity sequences:

- per-ticket scheduler attempt leases;
- a process-global Codex acknowledgement generation counter.

The second sequence has been removed.

`State::next_assignment_generation` no longer exists.

`allocate_assignment_generation` no longer exists.

After dispatch successfully mints and installs an `AttemptLease`, a reused
Codex seat now receives:

```text
SpawnContext.assignment_generation = AttemptLease.attempt_id
```

The existing adapter and `LISA_ASSIGNMENT` marker carry that value without a
schema change.

The ticket ID remains part of the marker, so independent tickets may each use
attempt 1 without ambiguity.

Fresh Codex launches remain immediately `Owned` under the established contract
and therefore do not need a pending marker.

All Claude paths remain immediately `Owned` and are unchanged.

## Promotion authority gate

`acknowledge_codex_assignment` now fails closed unless all relevant scheduler
facts agree.

It requires:

1. the seat is not already `Owned`;
2. assignment state is pending or recovering;
3. the addressed slot has a ticket reservation;
4. the same slot has an attempt lease stamp;
5. the lease ticket equals the reservation ticket;
6. the pending generation equals the lease attempt ID;
7. the stamped lease exactly validates against `current_leases`;
8. the payload detector matches that validated ticket and attempt ID.

Only after every check succeeds does assignment state become `Owned`.

Absence from `current_leases` rejects even a correctly formed payload for the
slot's stamped lease.

This matters during revocation: a stale reservation or delayed hook file cannot
recreate authority after the current lease has been removed.

A stale slot stamp also cannot borrow a replacement's current ticket identity.

Duplicate acknowledgement remains inert because the explicit production
`seat_is_owned` guard rejects it before detector evaluation.

## Dead-code path retirement

`seat_is_owned` previously had:

```text
#[allow(dead_code)]
```

with an obsolete comment referring to future UI work.

The helper is now a production guard in the acknowledgement promotion path.

The attribute and comment are removed.

Search confirms there is no remaining independent acknowledgement allocator or
allowance on this path.

## Bounded recovery behavior

E-033's one-shot fresh-session fallback remains intact.

Reusing the original lease generation for the fresh fallback would allow a
delayed acknowledgement from the abandoned reused session to masquerade as
acceptance by the fresh process.

To preserve that fence while giving marker generation one meaning, recovery
now mints a successor `AttemptLease`.

Before sending `/exit`, `begin_assignment_recovery` validates that the original
lease agrees across:

- pending assignment generation;
- slot stamp;
- `current_leases`;
- `lease_high_water`.

It then uses `AttemptLease::mint` and installs the successor consistently in:

- `lease_high_water`;
- `current_leases`;
- `AgentSlot::attempt_lease`;
- `Thread::attempt_lease`;
- `SeatAssignmentState::Recovering.generation`.

The prior acknowledgement becomes stale before provider recovery input.

The fresh fallback prompt carries the successor attempt ID.

The existing finite recovery deadline, exactly-one launch rule, actionable
`RecoveryFailed` terminal state, and operator-reset requirement remain.

If the original lease is absent or inconsistent, recovery fails without
sending replacement input.

If successor minting fails, recovery also fails without fabricating or wrapping
a generation.

## Acceptance-test coverage

The principal regression is:

```text
test_recycled_codex_ownership_requires_matching_ack_exactly_once
```

It uses `schedule_ready_tickets` for two real dispatches of the same ticket on
a resident Codex seat.

The first dispatch establishes attempt 1 and pending generation 1.

The test releases that assignment, removes its logical thread, and redispatches
the still-ready ticket.

The replacement establishes attempt 2 consistently in current authority,
slot stamp, and pending assignment state.

It then proves:

- attempt 1 no longer validates against attempt 2;
- an attempt-1 payload returns false;
- the replacement remains `AssignedPendingAck` for attempt 2;
- the replacement remains not owned;
- removing current attempt-2 authority makes an attempt-2 payload return false;
- restoring exact attempt-2 authority lets that payload promote to `Owned`;
- submitting the same payload again performs no second transition.

This directly covers both clauses of the ticket acceptance criterion.

## Additional regression coverage

`test_codex_ack_signal_promotes_matching_pending_seat` now installs a real
attempt lease on the slot and in current/high-water scheduler state.

It continues to cover the production `.ack` file scanner, file consumption,
activity bump, transition logging, and duplicate inertness.

`test_bounded_ack_wait_recovers_once_then_fails_actionably` now proves recovery
creates a strictly newer lease and stamps the same value across current,
high-water, slot, thread, and recovery state.

It continues to prove:

- the old acknowledgement is rejected;
- exactly one fresh fallback launches;
- repeated transition polls do not relaunch;
- a missing recovery acknowledgement ends in `RecoveryFailed`;
- the reservation remains inspectable for operator reset.

Existing tests continue to cover:

- isolated Codex marker parsing and stale fixtures;
- dashboard pending, owned, and recovering projection;
- dropped acknowledgement reproduction;
- fresh recovery acknowledgement promotion;
- ten consecutive recycled Codex assignments with one recovery;
- equivalent consecutive Claude behavior;
- dispatch lease monotonicity;
- release revocation;
- hard-timeout revocation and pane fencing;
- completion, artifact, liveness, and provenance behavior pending their
  dedicated S-034-02 tickets.

## Verification performed

Focused acceptance test passed before and after the isolated commit:

```text
cargo test -p lisa-plugin test_recycled_codex_ownership_requires_matching_ack_exactly_once
1 passed; 0 failed
```

Focused recovery tests passed:

```text
cargo test -p lisa-plugin test_bounded_ack_wait_recovers_once_then_fails_actionably
1 passed; 0 failed

cargo test -p lisa-plugin test_recovery_ack_promotes_only_the_fresh_generation
1 passed; 0 failed
```

The complete plugin suite passed:

```text
cargo test -p lisa-plugin
268 passed; 0 failed
```

The complete workspace suite passed:

```text
cargo test --workspace
```

Results:

- Lisa CLI: 270 passed;
- atomic provider contract integration: 1 passed;
- Lisa core: 155 passed;
- Lisa plugin: 268 passed;
- doc tests: 0 failures.

Total: 694 passed, 0 failed.

The deployed plugin target passed:

```text
cargo check -p lisa-plugin --target wasm32-wasip1
```

Warnings-denied plugin Clippy passed:

```text
cargo clippy -p lisa-plugin --lib -- -D warnings
```

Repository quick check passed:

```text
just check
```

Formatting and whitespace checks passed:

```text
cargo fmt --all -- --check
git diff --check
```

## Test coverage assessment

Coverage is sufficient for the acceptance criterion and primary regression
risks.

The current-versus-prior behavior uses real dispatch, release, high-water
retention, redispatch, and promotion rather than hand-constructed generation
counters.

The test separates marker-generation rejection from current-authority
rejection by temporarily removing the exact current lease.

Recovery coverage ensures the removal of the standalone counter does not weaken
E-033's delayed-ack fence or bounded terminal behavior.

The full suite protects provider parity and unrelated scheduler behavior.

## Coverage gaps

There is no dedicated plugin integration test forcing recovery lease mint at
`u64::MAX`.

The core `AttemptLease` tests cover exhaustion, while the recovery branch uses
the same checked helper and has explicit failure handling.

There is no live Zellij/Codex hook-delivery run in this ticket.

The epic assigns the deterministic split-brain harness and live proof to
S-034-03.

This ticket does not prove stale heartbeat, artifact, completion, or provenance
rejection. Those surfaces are intentionally owned by T-034-02-02 through
T-034-02-04.

## Open concerns and follow-up boundaries

Recovery now represents its fresh provider process as a new scheduler attempt.

That is required to keep acknowledgement generation equal to lease generation
while preserving the stale original-delivery fence. Later provenance work
should report this successor attempt rather than treating recovery as an
unidentified sub-delivery.

Attempt high-water remains process-local, as established by S-034-01. Scheduler
restart durability is not introduced here.

The Homebrew-installed `/opt/homebrew/bin/lisa` did not yet expose
`commit-ticket`. The repository-built `target/debug/lisa` was used for the
required isolated transaction. This did not affect source behavior or Git
isolation, but local packaging should eventually be refreshed separately.

## Critical issues

None found.

## Final assessment

Codex acknowledgement evidence now names and validates the exact scheduler
attempt it claims to accept. Prior-generation, revoked, inconsistent, malformed,
and duplicate evidence cannot promote ownership. Current evidence performs one
pending-to-owned transition, and the bounded fresh fallback retains its stale-
delivery fence through a truthful successor lease.
