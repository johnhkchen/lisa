# Review: T-039-02-03

## Outcome

The ticket is complete. A dedicated post-refactor regression suite now locks the
typed signal-record contract, scheduler poll interleaving, and the separation
between filesystem lease parsing and current-attempt admission. Provider payload,
deletion, lease, and legacy naming distinctions are directly asserted.

The pre-refactor characterization suite is retained byte-for-byte. All focused
tests, the full workspace suite, warnings-denied Clippy, formatting, the repository
WASM/native check, and whitespace validation are green.

## Commit

- Commit: `01da05e55fc6cc90df4acd85c3ce996cd04642ea`.
- Message: `T-039-02-03: lock signal ingestion regressions`.
- Created with `lisa commit-ticket`.
- Exact include: `crates/lisa-plugin/src/lib.rs`.
- Exact include: `crates/lisa-plugin/src/tests/signal_ingestion_regression.rs`.
- The ordinary Git index was not used.
- Both ticket-owned source paths are clean after the transaction.
- The ordinary index is empty.

## Files changed

### `crates/lisa-plugin/src/tests/signal_ingestion_regression.rs`

- Added a 259-line post-refactor regression module.
- Added local temporary-directory and running-attempt fixtures.
- Added a normalization helper for unordered multi-record scans.
- Added four focused contract tests.
- The tests access the crate-private boundary without widening visibility.
- No persistent fixture files or new dependencies were required.

### `crates/lisa-plugin/src/lib.rs`

- Added the nested test-module declaration.
- No production item changed.
- No signal consumer changed.
- No poll implementation changed.
- No public API changed.

## Files deliberately unchanged

### Characterization baseline

`crates/lisa-plugin/src/tests/signal_consumer_characterization.rs` remains
byte-for-byte unchanged.

Its SHA-256 before and after implementation is:

`49aed08994ba9a5b4578b22b49632f921e7b735070a088322613b2e138909e9b`

This preserves the story's intended BEFORE/AFTER test bracket:

- T-039-02-01 captured behavior before the refactor.
- T-039-02-02 introduced the typed boundary while retaining that behavior.
- This ticket adds explicit AFTER assertions for the boundary contract.

### Production boundary

`crates/lisa-plugin/src/signal.rs` is unchanged. The new tests confirmed its
current contract without requiring a correction.

### Other areas

- No `lisa-core` source changed.
- No `lisa-cli` source changed.
- No adapter changed.
- No Codex acknowledgement parser changed.
- No hook or template changed.
- No Cargo manifest changed.
- No timeout, failure, reclaim, or publication behavior changed.

## Typed-record contract coverage

The regression matrix asserts all eight requests:

1. `Heartbeats`;
2. `ProcessStarts`;
3. `ShellReady`;
4. `CodexAcknowledgements`;
5. `Awaiting`;
6. `Idle`;
7. `Transitions`;
8. `Errors`.

It asserts all nine returned record variants:

1. `Heartbeat`;
2. `ProcessStarted`;
3. `ShellReady`;
4. `CodexAcknowledgement`;
5. `Awaiting`;
6. `Idle`;
7. `Stopped`;
8. `Cleared`;
9. `Error`.

Both idle identity variants are also covered:

- pane-scoped `IdleTarget::Pane`;
- historical `IdleTarget::LegacyTicket`.

This test will fail if a request maps to a different record, a record loses its
typed lease, presence records acquire payload semantics, stopped and cleared are
collapsed, or legacy idle identity is removed.

## Provider payload distinctions

The three lease-bearing families receive serialized `AttemptLease` JSON and are
asserted as typed lease records.

The acknowledgement fixture is provider-shaped raw JSON that is not an attempt
lease. Exact string equality proves that ingestion preserves the raw native
payload rather than normalizing it into the provider-neutral lease contract.

Awaiting, stopped, cleared, and error fixtures use arbitrary nonempty bodies but
produce pane-only records. Their contents cannot leak into the typed record.

## Deletion and recognition distinctions

The deletion regression asserts strict pane-first behavior:

- `pane-seven.heartbeat` remains because the pane grammar is invalid;
- `pane-7.heartbeat` with invalid lease JSON is consumed but emits no record;
- a valid pane acknowledgement with arbitrary raw text is consumed and emitted;
- `T-LEGACY.ack` remains because legacy naming is not accepted for ack.

It separately asserts broad suffix-first behavior:

- `pane-seven.idle` is consumed without a record;
- `pane-seven.stopped` is consumed without a record;
- a transition scan leaves an unrelated valid idle record untouched.

These assertions fail on changes to recognition timing, one-shot deletion,
legacy-name spread, or request selectivity.

## Attempt admission coverage

The authority regression constructs a valid stale lease with the correct ticket
and a non-current attempt ID.

Direct ingestion must return that stale lease as a typed heartbeat and consume
its file. This proves the filesystem boundary validates serialized shape but does
not claim current-attempt authority.

The scheduler consumer then receives the same stale lease and must reject it.
Rejection is observed through unchanged thread activity, unchanged slot activity,
and retained awaiting/attention gates.

The exact current lease is then admitted. It updates thread and slot activity and
clears both gates. This test fails if admission moves into ingestion, if stale
attempts acquire effects, or if current attempts stop producing their effects.

The retained characterization suite continues to cover exact process-start,
shell-ready successor, and Codex tag admission independently.

## Poll-order coverage

The new source-structure test locks more than relative consumer order. It asserts:

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
12. assignment acknowledgement timeouts.

This prevents a future edit from preserving the eight consumers' relative order
while moving readiness delivery, artifact advancement, or timeout evaluation
across the signal admission boundary.

The assertion uses call strings rather than comments or line positions, so
documentation and whitespace changes remain harmless.

## Test results

### New regression suite

Command:

`cargo test -p lisa-plugin signal_ingestion_regression`

Result: 4 passed, 0 failed.

### Retained characterization suite

Command:

`cargo test -p lisa-plugin signal_consumer_characterization`

Result: 11 passed, 0 failed.

### Existing boundary unit tests

Command:

`cargo test -p lisa-plugin signal::tests`

Result: 7 passed, 0 failed.

### Full workspace

Command:

`cargo test --workspace`

Result: passed.

- CLI unit tests: 274 passed.
- Atomic provider integration: 1 passed.
- Help integration: 3 passed.
- Real Zellij integration: 1 environment-gated test remained intentionally ignored.
- Core unit tests: 155 passed.
- Plugin unit tests: 312 passed.
- Doc tests passed.

### Static and platform checks

- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- `just check`: passed, including WASM/native checks.
- `git diff --check`: passed.

## Acceptance criteria assessment

- New regression tests: satisfied by four new tests.
- Poll order drift fails: satisfied by the full interleaving assertion.
- Attempt admission drift fails: satisfied by stale/current heartbeat effects.
- Provider payload distinction drift fails: satisfied by exact raw ack preservation.
- Deletion distinction drift fails: satisfied by strict/broad edge cases.
- Lease distinction drift fails: satisfied by typed lease records plus downstream currency.
- Full suite green: satisfied.
- Clippy green: satisfied with warnings denied.
- Characterization retained: satisfied byte-for-byte and hash-verified.

## Open concerns and limitations

- The poll-order test intentionally inspects source call spelling. Renaming a
  method without semantic change will require updating the regression assertion.
- Directory iteration order is deliberately excluded from the contract; tests
  normalize multi-record output before comparison.
- Heartbeat represents the direct current-attempt boundary in the new suite.
  Process-start, shell-ready, and Codex acknowledgement admission remain covered
  by the unchanged characterization suite rather than duplicated here.
- Unreadable-file behavior is not recreated portably in the new suite. Existing
  focused boundary tests and code structure retain read-failure handling.
- The real Zellij delivery test remains environment-gated as before; this ticket
  is deterministic fixture coverage, consistent with the story boundary.
- No critical issue or human-blocking concern remains.

## Handoff

All requested work is complete. Ticket-owned source changes are durable through
Lisa's isolated transaction, focused and broad verification are green, and phase
artifacts are present in the attempt-private work directory. Lisa should now
perform the final completion publication and commit before releasing the seat.
