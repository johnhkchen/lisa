# Structure: recorded Review livelock regression

## File inventory

Create one file:

- `crates/lisa-core/tests/recorded_livelock_regression.rs`

Modify no production source, manifest, lockfile, plugin, CLI, ticket, or shared
work artifact. The attempt-private RDSPI documents are orchestration artifacts,
not part of the source commit.

## Integration test boundary

The new file compiles as a standalone lisa-core integration test target named
`recorded_livelock_regression`. It imports only public values from:

- `lisa_core::completion`;
- `lisa_core::disposition`.

It cannot access private completion helpers. This verifies the settled public
aggregate is sufficient for a downstream adapter to drive.

## Top-level organization

The file is organized in this order:

1. public-contract imports;
2. recorded constants;
3. local event and observation types;
4. aggregate-backed fixture driver;
5. naive edge-triggered comparison driver;
6. deterministic regression test.

No reusable production API is introduced.

## Recorded constants

Define stable string constants for attempt, completion, and correlation
identities. Define the ten-minute timeout as fixture metadata if useful for
making the historical timing explicit.

The constants are test-local and provider-neutral.

## `RecordedEvent`

Define a private enum with variants for:

- Review artifact written;
- phase advanced to Review;
- stop observed;
- Review timeout elapsed;
- plugin reloaded;
- manual completion result confirmed.

Derive Copy/Clone/Debug as needed. A `recorded_trace()` helper returns the exact
ordered array so chronology is visible in one place.

## `Observation`

Define a private result struct containing:

- `requests: usize`;
- `confirmations: usize`;
- `finish_up_prompts: usize`;
- `re_requests: usize`;
- final aggregate state or an equivalent confirmed flag.

Derive Debug and equality so assertions report complete divergence.

## Aggregate driver state

Define a private driver with:

- `phase_is_review: bool`;
- `review_artifact_present: bool`;
- `state: CompletionState`;
- observation counters.

The Pass disposition is constant for this incident and can be constructed in
the reconciliation helper instead of stored.

## Durable input construction

The driver's reconciliation helper constructs `DurableCompletionInputs` from
current fixture facts. Artifact admission is Some only when the artifact fact
is present. It uses stable AttemptId and CompletionId values.

The driver calls reconciliation only once phase Review is observed. This local
phase gate represents the future adapter boundary, because phase is not part of
the pure durable input type.

## Effect application

When `reconcile` yields `Reconciliation::Effect`, the helper checks whether a
request was already counted. A later effect increments `re_requests` and fails
the contract assertion rather than silently overwriting history.

For the first effect, build a matching `CompletionEvent::Request`, apply it via
`reduce`, and require:

- next state Requested;
- returned effect exactly equals the reconciliation effect.

Then apply `CommandLaunched` with the recorded correlation and require:

- next state CommandInFlight with that correlation;
- no effect.

## Non-effect reconciliation

`Reconciliation::None` leaves state unchanged.

`CommandInFlightActionRequired` must carry the exact recorded correlation. It
also leaves state unchanged and increments no request counter. The assertion
proves reload/timeout observations retain attribution without launching again.

## Event application

Artifact-written changes only the durable artifact fact.

Phase-advanced changes the phase fact and invokes reconciliation.

Stop invokes reconciliation.

Timeout first checks artifact presence. It increments finish-up only when the
artifact is missing, then invokes reconciliation.

Reload invokes reconciliation from retained durable and aggregate facts.

Manual confirmation applies matching CommandSucceeded, requires Confirmed,
increments confirmation exactly on that transition, and reconciles once more.

## Naive stub structure

Keep the naive implementation separate from the aggregate driver. It retains
only booleans/counters needed to reproduce the historical failure:

- artifact edge requests only if phase is already Review;
- phase edge never inspects existing artifact;
- stop and reload do nothing;
- timeout prompts when request was missed;
- manual result records one external authoritative confirmation.

The stub does not call `reduce` or `reconcile`, ensuring it remains a clear
negative-control model rather than another aggregate implementation.

## Test structure

One test named for T-009-01-01 obtains the fixed trace and expected observation.
It first runs the naive stub and asserts the exact counterexample. It then runs
the aggregate driver and asserts exact expected counts plus Confirmed final
state.

Keeping both checks in one test guarantees they consume the identical event
array and prevents fixture drift between negative and positive evidence.

## Compiler and ownership boundaries

All types are private to the integration test except imported production types.
No feature flag or dev dependency is needed. The only ticket-owned source path
is the new integration test and it is the only exact include passed to Lisa's
isolated commit transaction.

