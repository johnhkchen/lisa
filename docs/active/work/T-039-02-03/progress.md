# Progress: T-039-02-03

## Status

Implementation is complete and validated. The ticket adds a dedicated
post-refactor regression suite for the typed signal-ingestion boundary. No
production behavior changed, and the predecessor characterization suite remains
byte-for-byte unchanged.

## Baseline established

- Read `CLAUDE.md`, `AGENTS.md`, the ticket, and the RDSPI workflow.
- Read story `S-039-02` and predecessor artifacts.
- Inspected `signal.rs`, all eight signal consumers, and `poll_tick`.
- Inspected the complete characterization suite.
- Recorded the characterization SHA-256 before editing.
- Baseline hash:
  `49aed08994ba9a5b4578b22b49632f921e7b735070a088322613b2e138909e9b`.
- Ran the characterization suite before editing.
- Baseline characterization result: 11 passed, 0 failed.
- Ran the signal module unit tests before editing.
- Baseline signal-unit result: 7 passed, 0 failed.
- Confirmed no ticket-owned source file was dirty before implementation.

## Phase artifacts completed

- Wrote `research.md` in the attempt-private work directory.
- Wrote `design.md` in the attempt-private work directory.
- Wrote `structure.md` in the attempt-private work directory.
- Wrote `plan.md` in the attempt-private work directory.
- Did not write phase artifacts directly to the shared canonical path.
- Lisa independently admitted/published phase artifacts as it observed them.
- Did not manually edit ticket phase or status fields.

## Source implementation

### New regression module

Created:

`crates/lisa-plugin/src/tests/signal_ingestion_regression.rs`

The file contains four tests and local test fixtures.

### Module registration

Modified:

`crates/lisa-plugin/src/lib.rs`

Added only the nested test-module declaration:

`mod signal_ingestion_regression;`

No production function, type, import, or behavior changed.

## Regression test 1: typed record matrix

Test name:

`every_request_produces_its_exact_typed_record_contract`

Completed coverage:

- `Heartbeats` produces `Heartbeat` with a typed `AttemptLease`.
- `ProcessStarts` produces `ProcessStarted` with a typed lease.
- `ShellReady` produces `ShellReady` with a typed lease.
- `CodexAcknowledgements` preserves an exact raw provider payload.
- `Awaiting` produces a pane-only presence record.
- `Idle` produces a pane target.
- `Idle` also produces the isolated legacy ticket target.
- `Transitions` produces distinct `Stopped` and `Cleared` records.
- `Errors` produces a pane-only presence record.
- All eight `SignalRequest` variants are exercised.
- All nine `SignalRecord` variants are asserted.
- Both `IdleTarget` variants are asserted.
- Recognized files are asserted deleted.
- Multi-record results are normalized so filesystem order is not assumed.

## Regression test 2: deletion distinctions

Test name:

`recognition_keeps_strict_and_broad_deletion_policies_distinct`

Completed coverage:

- A strict malformed pane filename remains unconsumed.
- A strict recognized lease filename with malformed JSON is consumed.
- Malformed lease JSON produces no typed record.
- A raw acknowledgement body need not be JSON to produce a raw record.
- A legacy acknowledgement filename remains unconsumed.
- Legacy naming therefore remains isolated to idle.
- A broad malformed idle pane filename is consumed without a record.
- A broad malformed transition pane filename is consumed without a record.
- A transition scan leaves an unrelated idle record untouched.

## Regression test 3: authority boundary

Test name:

`typed_lease_ingestion_stays_separate_from_current_attempt_admission`

Completed coverage:

- A syntactically valid stale heartbeat is returned by direct ingestion.
- Direct ingestion preserves the complete stale lease value.
- Direct ingestion consumes the stale file.
- The heartbeat consumer rejects the same stale lease downstream.
- Rejection leaves thread activity unchanged.
- Rejection leaves slot activity unchanged.
- Rejection leaves awaiting-human state set.
- Rejection leaves attention debounce set.
- The exact current lease refreshes thread and slot activity.
- The exact current lease clears both gates.
- Both stale and current scheduler paths remain one-shot.

## Regression test 4: poll interleaving

Test name:

`poll_tick_preserves_signal_admission_and_timeout_interleaving`

Completed coverage locks this order:

1. heartbeat ingestion;
2. awaiting ingestion;
3. ready assignment delivery;
4. process-start ingestion;
5. shell-ready ingestion;
6. Codex acknowledgement ingestion;
7. artifact advancement;
8. idle ingestion;
9. transition ingestion;
10. error ingestion;
11. transition timeout evaluation;
12. assignment acknowledgement timeout evaluation.

This extends the retained characterization assertion by locking the important
non-signal work interleaved between consumers and the signal-before-timeout
precedence.

## Focused verification

Command:

`cargo test -p lisa-plugin signal_ingestion_regression`

Result:

- 4 passed.
- 0 failed.
- 308 filtered out.

Command:

`cargo test -p lisa-plugin signal_consumer_characterization`

Result:

- 11 passed.
- 0 failed.
- 301 filtered out.

Command:

`cargo test -p lisa-plugin signal::tests`

Result:

- 7 passed.
- 0 failed.
- 305 filtered out after the new module was registered.

## Broad verification

Command:

`cargo test --workspace`

Result:

- Passed.
- CLI unit tests: 274 passed.
- CLI atomic provider integration: 1 passed.
- CLI help integration: 3 passed.
- Real Zellij integration: 1 intentionally ignored by its environment gate.
- Core unit tests: 155 passed.
- Plugin unit tests: 312 passed, including all four new regressions.
- Doc tests passed.

Command:

`cargo clippy --workspace --all-targets -- -D warnings`

Result: passed with no warnings.

Command:

`cargo fmt --all -- --check`

Result: passed.

Command:

`just check`

Result: passed, including the repository's WASM check and native tests.

Command:

`git diff --check`

Result: passed.

## Preservation checks

- `signal_consumer_characterization.rs` has no diff.
- Its post-implementation SHA-256 is unchanged:
  `49aed08994ba9a5b4578b22b49632f921e7b735070a088322613b2e138909e9b`.
- `signal.rs` has no diff.
- No adapter or provider parser has a diff.
- No Cargo manifest has a diff.
- No production behavior has a diff.
- The ordinary Git index is empty.

## Deviations from plan

No functional deviations were required.

The planned local running-attempt helper successfully reused the parent test
fixture utilities. The planned record normalization was needed only for idle and
transition multi-record results. No production defect was exposed, so the
test-only scope remained intact.

## Commit plan

One meaningful source unit will be committed through Lisa with exact includes:

- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/tests/signal_ingestion_regression.rs`.

Planned message:

`T-039-02-03: lock signal ingestion regressions`

The ordinary Git index will not be used.

## Commit completed

- Commit: `01da05e55fc6cc90df4acd85c3ce996cd04642ea`.
- Message: `T-039-02-03: lock signal ingestion regressions`.
- Created with `lisa commit-ticket`.
- Exact include: `crates/lisa-plugin/src/lib.rs`.
- Exact include: `crates/lisa-plugin/src/tests/signal_ingestion_regression.rs`.
- Commit contains 260 insertions across exactly two paths.
- Both ticket-owned source paths are clean after the transaction.
- The ordinary Git index remains empty.
- Existing Lisa-managed ticket, provenance, and admitted-work changes remain
  outside the source commit.
