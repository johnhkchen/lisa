# Progress — T-045-03-01 claim is ownership proof

## Status

The typed claim consumer, full scheduler admission fence, cleanup integration, and
claim-only ownership acceptance test are implemented.
Focused tests pass.
The source unit is ready for its isolated Lisa commit.
Broad verification is in progress and encountered a documented shared-worktree race
from concurrent T-045-02-02 launcher integration.

## Baseline

Before source changes, the four planned ticket-owned paths were clean.

Ran:

```text
cargo test -p lisa-plugin signal::tests
cargo test -p lisa-plugin signal_consumer_characterization
cargo test -p lisa-plugin signal_ingestion_regression
```

Results:

- 7 signal module tests passed;
- 11 signal consumer characterization tests passed;
- 4 signal ingestion regression tests passed;
- no baseline failure occurred.

## Typed claim ingestion

Modified `crates/lisa-plugin/src/signal.rs`.

Added the shared `lisa_core::claim::AssignmentClaim` import.
Added `SignalRequest::Claims` and the typed record:

```text
SignalRecord::Claim { pane_id, claim }
```

The `.claim` consumer recognizes only exact `pane-<u32>.claim` names.
It reads and deserializes the shared JSON schema.
Every recognized claim path is removed after the acquisition attempt, including a
malformed body.
Invalid filenames remain outside the consumer's ownership.

No plugin-local claim schema was introduced.
Provider `UserPromptSubmit` payloads remain raw and separate.

## Claim ingestion tests

Added a signal module test using a nonce above `u64::MAX`.
It proves:

- exact typed parsing;
- pane routing from the filename;
- full `u128` nonce preservation;
- valid file deletion;
- malformed recognized file deletion.

Extended the ingestion regression's every-request contract with the typed claim
record and deletion assertion.

## Authoritative scheduler admission

Modified `crates/lisa-plugin/src/lib.rs`.

Added `admit_assignment_claim(pane_id, claim) -> bool`.

The method rejects an already-owned or ineligible seat and then requires all of:

- an active unowned assignment generation;
- an addressed physical slot;
- a slot ticket ID and complete attempt lease;
- claim ticket equality with the slot ticket;
- claim attempt equality with the state generation;
- slot lease ticket/attempt equality with the claim;
- exact lease currency in `current_leases`;
- a retained assignment reference for the ticket;
- retained assignment lease equality with the slot/current lease;
- retained assignment nonce equality with the claim nonce.

Only after every comparison succeeds does it insert
`SeatAssignmentState::Owned`.
Rejected claims make no scheduler state mutation.

The method deliberately does not reread the assignment path.
The retained `AssignmentRef` is the scheduler's exact successful-publication
identity; old filesystem residue is not authority.

## Claim signal consumer

Added `check_claim_signals` beside the other signal consumers.
It performs typed ingestion and calls the scheduler admission method.

On successful transition it:

- bumps the pane and thread activity clock;
- logs `Pane <id> claimed <ticket> attempt <n> assignment`.

Rejected claims are consumed without activity or log side effects.

The poll now runs claim admission after shell-readiness handling and before the
existing provider acknowledgement consumer.
This means delivery and lifecycle prerequisites have already run, while a claim
still wins before timeout evaluation.

Added `claim` to `clear_pane_lifecycle_signals` so reset cleanup removes unconsumed
predecessor evidence alongside the other pane-scoped lifecycle files.

## Consumer characterization

Modified `signal_consumer_characterization.rs`.

Updated the pinned poll order from eight to nine signal consumers.
Added claim cases to:

- malformed recognized one-shot records;
- legacy ticket-addressed names retained by every consumer except idle.

Added a focused consumer test with a delivered current attempt and retained exact
assignment reference.
It publishes a wrong nonce first and proves the signal is consumed, the seat remains
unowned, and activity is not bumped.
It then publishes the exact nonce and proves the seat becomes `Owned`, activity is
bumped, and the claim-specific event is logged.

## Ticket acceptance test

Added `delivered_assignment_becomes_owned_on_exact_claim_without_hook`.

The test uses the scheduler's real fresh Codex delivery fixture.
It schedules the ticket, advances the pane through the fresh lifecycle, and obtains
the actual current lease and nonce retained by assignment publication.

Before the valid claim it asserts:

- internal state is `Delivering`;
- the seat is not owned;
- the scheduler row visibly contains `delivering`;
- no `.ack` hook file exists.

A wrong nonce is consumed and leaves the same delivered/unowned output.
The exact claim then transitions to `Owned`, and the scheduler row visibly contains
`owned`.
No hook payload or hook consumer participates in the transition.

## Focused verification

Ran after formatting:

```text
cargo fmt --all
cargo fmt --all -- --check
cargo test -p lisa-plugin signal::tests
cargo test -p lisa-plugin signal_consumer_characterization
cargo test -p lisa-plugin signal_ingestion_regression
cargo test -p lisa-plugin delivered_assignment_becomes_owned_on_exact_claim_without_hook
```

Results:

- formatting check passed;
- 8 signal tests passed;
- 12 consumer characterization tests passed;
- 4 ingestion regression tests passed;
- the exact acceptance test passed;
- no focused failure or warning occurred.

## Diff audit

`git diff --check` passed.
The ticket-owned source diff is limited to:

- `crates/lisa-plugin/src/signal.rs`;
- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/tests/signal_consumer_characterization.rs`;
- `crates/lisa-plugin/src/tests/signal_ingestion_regression.rs`.

Existing `.lisa` runtime ledgers and materialized epic/story/ticket/work files remain
unrelated and excluded.

## Shared-worktree verification race

The first full `cargo test -p lisa-plugin` began while concurrent T-045-02-02 was
changing `AgentAdapter::launch_command` from one argument to two.
Its trait and implementations appeared before its test and scheduler call sites were
updated.
The compiler therefore reported 18 missing-argument errors in foreign adapter tests
and existing scheduler launch call sites.

Inspection showed `adapter.rs` as a new foreign worktree modification.
The errors do not involve claim types, signal ingestion, admission, or claim tests.
The focused claim suite had passed on the consistent snapshot immediately before
that concurrent edit appeared.

Because T-045-02-02 is beginning to edit the same `lib.rs`, this ticket will commit
its already-focused-tested exact source paths now through Lisa's isolated
transaction.
This prevents either ticket from accidentally capturing the other's same-file diff.
Broad plugin/workspace/WASM verification will rerun after the neighboring unit
settles.

## Scope retained

This ticket does not change:

- existing hook acknowledgement semantics;
- evidence ranking or artifact fallback;
- assignment timeout/retry policy;
- delivered-awaiting-claim states;
- launcher argv or adapter APIs;
- CLI claim production;
- dashboard labels;
- ticket-boundary exit or nonce revocation.

Those remain assigned to dependent E-045 tickets.

## Remaining work

1. Allow the neighboring launcher unit to commit its remaining `adapter.rs` and
   `lib.rs` test changes so the shared path is clean.
2. Complete Review artifacts.

## Isolated source commit

Committed through Lisa's isolated transaction with exact include paths:

```text
lisa commit-ticket \
  --ticket-id T-045-03-01 \
  --message "feat(plugin): own assignments from exact claims" \
  --include crates/lisa-plugin/src/signal.rs \
  --include crates/lisa-plugin/src/lib.rs \
  --include crates/lisa-plugin/src/tests/signal_consumer_characterization.rs \
  --include crates/lisa-plugin/src/tests/signal_ingestion_regression.rs
```

Commit:

`67a7f0eb976ff85d9b1d4b7ffe218412bb157c1a`

No ordinary Git staging or commit command was used.
The ordinary index remains empty.

## Same-file collision during commit

The transaction stat showed that concurrent T-045-02-02 inserted its launcher-path
call-site changes into `lib.rs` after this ticket's diff audit but before the exact
path transaction acquired the lock.
Consequently the shared-file commit contains both:

- this ticket's claim import, cleanup suffix, admission method, signal consumer,
  poll call, and acceptance test;
- T-045-02-02's changes passing the retained assignment path into fresh launcher
  call sites.

The collision is confined to the already-shared `lib.rs` path.
The other three included files contain only claim work.
No foreign path was included, and no foreign content was overwritten or removed.

Immediately after the commit, T-045-02-02 continued with its remaining uncommitted
`adapter.rs` implementation and an additional `lib.rs` launcher test.
The combined worktree then compiled and passed all tests.
Reverting the already-compatible call sites would have destroyed or destabilized
the neighboring active ticket, so they were preserved and the collision is made
explicit here for Review.

## Full plugin regression

After the neighboring adapter implementation reached a consistent snapshot, ran:

```text
cargo test -p lisa-plugin
```

Result:

- 391 plugin tests passed;
- 0 failed;
- 0 ignored;
- claim ingestion, claim admission, launcher integration, historical hook paths,
  lifecycle recovery, completion, provenance, and UI tests all passed.

This rerun resolved the earlier transient 18-error compile failure; no product
failure remained.

## Workspace regression

Ran:

```text
cargo test --workspace
```

Results included:

- 19 CLI library tests passed;
- 269 CLI binary unit tests passed;
- all CLI integration suites passed, including 3 claim producer tests;
- 200 core tests passed;
- both core integration tests passed;
- 391 plugin tests passed;
- doc tests passed;
- the declared real-Zellij boundary remained intentionally ignored by its
  environment gate.

No workspace test failed.

## Production-oriented verification

Ran:

```text
just check
```

Results:

- `cargo check -p lisa-plugin --target wasm32-wasip1` passed;
- the recipe's complete `cargo test --workspace` passed;
- no warning or target-specific error was attributable to the claim consumer.

## Final cleanliness state before Review

Clean committed claim-only paths:

- `crates/lisa-plugin/src/signal.rs`;
- `crates/lisa-plugin/src/tests/signal_consumer_characterization.rs`;
- `crates/lisa-plugin/src/tests/signal_ingestion_regression.rs`.

`crates/lisa-plugin/src/lib.rs` has no uncommitted claim hunk.
Its current worktree diff is a 97-line launcher integration test owned by active
T-045-02-02.
`crates/lisa-plugin/src/adapter.rs` is also modified only by T-045-02-02.

The ordinary index is empty.
Existing runtime ledgers and Lisa-materialized docs remain unrelated.

The neighboring transaction subsequently completed as:

`5f02b0f feat(plugin): launch Codex with exact assignment reference`

It committed its remaining `adapter.rs` and `lib.rs` changes through its own exact
transaction.
The final audit now shows every source path touched by either ticket clean and the
ordinary index empty.

The ticket's implementation is complete, committed, verified, and clean.
No claim implementation work remains; proceed to Review.
