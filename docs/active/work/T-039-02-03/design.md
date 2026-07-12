# Design: T-039-02-03

## Decision summary

Add a separate post-refactor regression module beside the retained characterization
suite. The new module will test four seams: the exact request-to-record contract,
strict versus broad deletion behavior, the separation between typed lease parsing
and current-attempt admission, and the complete interleaved signal order in
`poll_tick`.

No production code will change unless the new tests reveal a defect. The existing
characterization file will remain byte-for-byte unchanged.

## Design goals

- Make the typed ingestion contract explicit in executable assertions.
- Make every supported signal family visible in one mapping test.
- Preserve the distinction between lease JSON and provider-native raw payload.
- Preserve the distinction between payload-bearing and presence-only records.
- Preserve strict recognition versus broad one-shot deletion.
- Prove ingestion accepts syntactically valid stale leases.
- Prove scheduler admission rejects those stale leases.
- Lock the meaningful scheduler interleaving around the eight consumers.
- Avoid depending on filesystem iteration order.
- Avoid expanding a crate-private implementation into a public API.
- Keep the pre-refactor characterization baseline unchanged.
- Produce failures whose names identify the violated contract.

## Option 1: rely on existing tests only

The characterization suite and `signal.rs` unit tests already cover much of the
required runtime behavior. One option is to run them and add no code.

Advantages:

- No duplication.
- No maintenance cost.
- Existing tests are green after the refactor.

Disadvantages:

- It does not satisfy the explicit requirement for new regression tests.
- The characterization suite predates and intentionally ignores boundary shape.
- Existing unit tests do not enumerate the full request/record matrix.
- Existing poll coverage checks only relative consumer order.
- The ingestion/admission separation is distributed across separate tests.
- A future edit could retain behavior accidentally while eroding typed distinctions.

Decision: reject. Passing predecessor tests is necessary but not this ticket's
deliverable.

## Option 2: expand tests inside `signal.rs`

Add more unit tests to the private module that owns ingestion.

Advantages:

- Direct access to private helpers and enum variants.
- Tests remain close to the implementation.
- Minimal module wiring.
- Natural location for request/record and deletion assertions.

Disadvantages:

- `signal.rs` cannot naturally exercise scheduler attempt admission in context.
- A source-level poll-order assertion does not belong in the filesystem module.
- The ticket's cross-boundary regression story becomes split across files.
- Unit tests can mirror implementation too closely and miss integration drift.
- The predecessor already added focused implementation unit tests there.

Decision: reject as the primary approach. The new suite should cross the ingestion
and scheduler boundary deliberately.

## Option 3: extend the characterization file

Append post-refactor assertions to `signal_consumer_characterization.rs`.

Advantages:

- Existing fixtures and helper functions are available.
- All signal behavioral tests would be in one file.
- No additional test module declaration is needed.

Disadvantages:

- The story requires the before/after characterization suite to be retained.
- Editing it blurs which assertions existed before the structural refactor.
- Reviewers lose a byte-for-byte baseline for the bracketing sequence.
- Future history cannot distinguish characterization from typed-contract tests.
- The acceptance criterion calls out retaining the characterization suite.

Decision: reject. Preservation is clearer when the file is untouched.

## Option 4: add a sibling regression module

Create `src/tests/signal_ingestion_regression.rs` and declare it next to the
characterization module under `lib.rs`'s test module.

Advantages:

- Cleanly labels tests as post-refactor regression locks.
- Preserves the characterization file byte-for-byte.
- Has access to private ingestion types and scheduler state.
- Can test direct record production and downstream effects together.
- Can inspect the scheduler source for structural order.
- Produces one ticket-owned source unit with a clear responsibility.
- Fits the crate's established nested-test-module convention.

Disadvantages:

- Some behavioral overlap with characterization is unavoidable.
- Source inspection is sensitive to method spelling and formatting.
- Private enum changes will require coordinated test changes.
- The main test module needs one production-file declaration line.

Decision: choose this option. It best expresses the ticket's role as the AFTER
half of the story's regression bracket.

## Regression 1: exact typed-record matrix

Create one temporary signal directory and exercise each `SignalRequest` separately.
Write one valid path for each supported record variant, invoke its request, and
assert the exact resulting `SignalRecord`.

The matrix will cover:

- `Heartbeats` to `Heartbeat { pane_id, lease }`.
- `ProcessStarts` to `ProcessStarted { pane_id, lease }`.
- `ShellReady` to `ShellReady { pane_id, lease }`.
- `CodexAcknowledgements` to raw `CodexAcknowledgement`.
- `Awaiting` to pane-only `Awaiting`.
- `Idle` to both pane and legacy ticket targets.
- `Transitions` to both `Stopped` and `Cleared`.
- `Errors` to pane-only `Error`.

This test locks the closed protocol vocabulary. The raw acknowledgement body will
be intentionally non-lease provider JSON so converting it to lease parsing fails.
Presence-only bodies will contain arbitrary text, proving bodies are ignored.

Where one request returns multiple records, sort by debug representation before
comparison. The contract does not include directory iteration order.

## Regression 2: deletion and recognition matrix

Create representative malformed or inapplicable records:

- Strict malformed pane name, which must remain.
- Strict valid pane with malformed lease, which must be deleted.
- Strict valid acknowledgement with raw non-JSON text, which must return a record.
- Broad malformed idle pane, which must be deleted without a record.
- Broad malformed stopped pane, which must be deleted without a record.
- Unrelated suffix, which must remain after another request scans.
- Legacy non-idle filename, which must remain.

This test makes deletion timing observable without reaching into private helpers.
It asserts only public-to-the-crate behavior of `ingest`.

## Regression 3: ingestion versus attempt admission

Construct a state with one running attempt, slot lease, thread lease, and current
lease. First call `signal::ingest` directly with a syntactically valid stale
heartbeat. Assert that it returns a typed `Heartbeat` containing the stale lease
and deletes the file. This proves ingestion does not decide currency.

Then recreate the same stale file and call `check_heartbeat_signals`. Assert no
activity clock or attention/awaiting gate changes. Finally write the exact current
lease and call the consumer. Assert activity is refreshed and gates are cleared.

This single test locks both halves of the authority boundary. It will fail if
stale leases are filtered too early or admitted too late.

Heartbeat is the strongest representative because its consumer performs all
three checks directly: ticket, slot lease, and current registry. Existing
characterization retains separate process-start, shell-ready, and Codex admission
coverage, so duplicating all state machines is unnecessary.

## Regression 4: complete poll sequence

Inspect the body of `poll_tick` and assert an ordered sequence containing:

1. heartbeat;
2. awaiting;
3. ready-assignment delivery;
4. process start;
5. shell ready;
6. Codex acknowledgement;
7. artifact advancement;
8. idle;
9. transition;
10. error;
11. transition timeouts;
12. acknowledgement timeouts.

The test will use sequential `split_once` checks, matching the established
characterization technique. Including the two key non-signal calls prevents a
refactor from preserving relative consumer order while changing readiness or
artifact timing. Including immediate timeout followers locks admission precedence.

The source boundary will be located using the next stable method declaration.
Failure messages will name the missing or reordered call.

## Scope control

- Do not change `SignalRequest`, `SignalRecord`, or ingestion behavior.
- Do not change consumer methods.
- Do not alter characterization assertions.
- Do not add public visibility.
- Do not introduce fixtures outside temporary directories.
- Do not test unrelated failure, timeout, or publication transitions.
- Do not promise record ordering from `read_dir`.
- Do not edit ticket phase or status.

## Verification design

Run the new module alone first for fast feedback. Then run the retained
characterization module explicitly. Run all plugin tests, then the workspace
suite. Run formatter checking, workspace Clippy with warnings denied, and the
repository's `just check` command if available. Confirm the characterization
file has no diff. Confirm only the test declaration and new regression module
are included in the ticket source commit.

## Expected maintenance signal

A deliberate protocol change must now update a focused post-refactor suite and
explain which distinction changed. An accidental change should fail with a test
name that identifies typed shape, deletion policy, attempt authority, or poll
ordering. That is the intended regression lock.
