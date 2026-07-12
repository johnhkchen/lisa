# Design: T-034-02-04 one authoritative provenance record

## Decision summary

Evolve provenance to schema version 2. Every new record carries the exact
`AttemptLease`, an explicit `fenced` flag, and an `authoritative` flag. Timeout
and failure rows remain append-only attempt history. Only a current-lease Done
row is authoritative.

Keep an admitted completion attempt current until its isolated transaction
returns by excluding pending completions from timeout/stale reclamation. At the
result boundary, revalidate the pending attempt authority before publishing
Done. The provenance publisher independently rejects stale Done emission.

## Goals

- attribute every production provenance row to one execution attempt;
- preserve timed-out and failed predecessor rows across redispatch;
- record whether scheduler fencing occurred for that terminal attempt;
- stamp the winning Done row with its exact lease;
- distinguish ticket-authoritative Done from attempt-history outcomes;
- reject stale completion results at the last scheduler publication boundary;
- retain isolated completion transaction behavior;
- keep append-only ledger semantics and provider usage fields;
- provide deterministic regression coverage for timeout then replacement.

## Non-goals

- changing lease minting, acknowledgement, heartbeat, or artifact staging;
- making provider usage artifacts attempt-scoped;
- persisting scheduler lease authority across plugin restarts;
- changing `complete-ticket` CLI arguments or commit transaction internals;
- deleting or rewriting schema-v1 ledger lines;
- adding a general provenance query command;
- implementing the full S-034-03 split-brain harness.

## Option 1: stamp only `attempt_id`

Advantages:

- smallest JSON addition;
- easy grouping by retry generation.

Disadvantages:

- loses the complete authority value used everywhere else;
- invites consumers to join ticket ID and attempt ID manually;
- cannot directly deserialize to the shared lease type;
- does not expose fence or authoritative-Done semantics.

Decision: rejected. Provenance should carry the same identity the scheduler checks.

## Option 2: add only an optional lease

Advantages:

- old synthetic fixtures continue without changes;
- old rows can deserialize through a defaulted field;
- timeout and replacement become distinguishable.

Disadvantages:

- new records can still silently lack attribution;
- no explicit fence history;
- downstream readers must infer authoritative status from outcome;
- weakens a production invariant to accommodate tests.

Decision: rejected. Schema v2 should make required evidence structurally required.

## Option 3: emit a separate lifecycle ledger

Record revoke, fence, release, and completion events individually.

Advantages:

- maximum event-level reconstruction;
- exact ordering can be replayed;
- no overloading of terminal run records.

Disadvantages:

- introduces a second schema and query surface;
- changes one terminal record into an event-sourcing system;
- requires correlation and partial-event handling;
- exceeds the ticket's attempt-outcome scope.

Decision: rejected. A terminal row with explicit fence state is sufficient here.

## Option 4: schema-v2 terminal records

Add three required fields:

```text
attempt_lease: AttemptLease
fenced: bool
authoritative: bool
```

Advantages:

- reuses the exact scheduler authority type;
- timeout/fence history is directly queryable;
- one boolean identifies the ticket-authoritative outcome;
- predecessor and replacement rows remain independent and append-only;
- invalid unleased production records cannot be constructed.

Disadvantages:

- schema version must change;
- old rows do not deserialize as schema-v2 records without version-aware readers;
- direct-construction tests require real attempt fixtures.

Decision: selected. The schema version exists for precisely this shape change.

## Field semantics

`attempt_lease` is the thread's immutable dispatch stamp. Both its ticket and
attempt components are serialized.

`fenced` is true only when teardown called `revoke_and_fence_attempt` and its
result was `Fenced` or `AlreadyFenced`. It is false for normal completion and
non-fencing error release.

`authoritative` is true only for `RunOutcome::Done` accepted from the current
lease. Failed and timed-out rows are true historical facts about attempts, but
they are not the ticket's authoritative successful outcome.

The field name is intentionally general enough for queries while its invariant
is narrow: schema-v2 publishers never set it on a non-Done row.

## Publisher contract

Change the publisher to accept fence state beside the outcome.

It reads the thread's lease and fails closed if absent.

For Done, it checks that the lease is still current before append.

For Failed and TimedOut, it permits the stamped attempt row because teardown
history remains meaningful even as authority is being revoked.

The method builds the record only after these checks.

A rejected publication logs a warning and appends nothing.

Append I/O failures keep their existing non-fatal error behavior.

## Fence ordering

Timeout and hard-silence teardown will become:

1. mark the thread failed;
2. revoke and fence the attempt;
3. classify the returned fence result;
4. append the terminal record using the still-present stamped thread;
5. release the slot;
6. remove the thread.

Revocation before record append is safe for non-Done history because the
publisher does not require current authority for failure outcomes.

This ordering records the actual fence result rather than an intention to fence.

## Completion stability

Pending completion is a commit-critical section for the admitted lease.

Session-timeout and hard-silence candidate collection will exclude ticket IDs
present in `pending_completions`.

This prevents the scheduler from revoking and redispatching the ticket while
its native completion transaction can still commit Done.

The result handler will still revalidate `PendingCompletion::authority`:

- an Attempt must equal `current_leases[ticket]`;
- an Operator remains allowed only for the existing no-thread manual path.

A stale result is removed from pending state, logged, and does not complete a
thread, publish Done provenance, release a replacement, or schedule dependents.

The defensive result check protects future callers even though the pending
timeout exclusion makes the normal asynchronous race unreachable.

## Exactly-one behavior

The existing one-pending-entry invariant rejects concurrent completion requests.

The result handler removes pending state before Done publication.

Duplicate callbacks find no pending state and return.

The new current-lease check rejects a predecessor callback after replacement.

The provenance publisher repeats the current-lease check before append.

Together these boundaries permit one authoritative Done row for the accepted
completion transaction while retaining any number of non-authoritative attempt
history rows.

## Test design

Core schema tests will assert:

- schema version 2;
- serialized complete lease;
- serialized fence and authority flags;
- round-trip equality;
- append continues to preserve earlier rows.

Plugin tests will update direct provenance fixtures to install current leases.

A focused acceptance test will:

1. create attempt one;
2. record its fenced TimedOut history;
3. mint attempt two;
4. present a stale completion request/result path and observe rejection;
5. complete attempt two through the verified result publisher;
6. repeat the callback;
7. assert two records total;
8. assert attempt one is timed-out, fenced, non-authoritative;
9. assert attempt two is Done, unfenced, authoritative;
10. assert exactly one authoritative Done row.

An additional test will assert pending completion is excluded from timeout
reclamation so the commit-critical lease cannot be replaced mid-command.

## Documentation

Update the ledger example, version, field table, query guidance, and append
semantics. Explain that schema-v1 rows predate attempt attribution and that
readers must branch on `schema_version` when mixing historical data.

## Chosen tradeoff

Schema v2 intentionally favors a strong required-attribution contract over
transparent deserialization of schema-v1 rows into the latest struct. The
ledger itself stays append-only; version-aware analysis can normalize old rows,
but the runtime must never manufacture a lease for history that did not record one.
