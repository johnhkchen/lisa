# Review: named failure transition outcomes

## Summary

T-039-03-02 is implemented. Every failure/reclaim path characterized by
T-039-03-01 now returns an explicit path-specific typed outcome without changing
the scheduler state that authorizes retention, fencing, release, or retry.

The source was committed through Lisa's isolated transaction as:

```text
f63bab2317037d15bb2cd54166f6b2bbc0ceca27
refactor: name failure transition outcomes
```

## What changed

`crates/lisa-plugin/src/lib.rs` adds the private
`FailureTransitionOutcome` enum with seven variants:

1. `AssignmentDeliveryFailed`;
2. `AssignmentRecoveryFailed`;
3. `StartupFailed`;
4. `StartupRecoveryFailed`;
5. `ErrorReclaimed`;
6. `SessionTimedOut`;
7. `StaleThreadReclaimed`.

The four retained failure helpers now return optional typed outcomes. Their
existing guards return `None`; completed edges return `Some` only after state,
thread, alert, pane, lease, and log effects have run.

The three batch scanners now return outcome vectors. This preserves one result
per transition when a single poll consumes multiple errors or reclaims multiple
threads.

`crates/lisa-plugin/src/tests/signal_consumer_characterization.rs` wraps two
error-consumer match arms in unit-valued blocks. This is required because the
error scanner now returns a vector while the characterization intentionally
dispatches all consumers through one match. Signal handling behavior is
unchanged.

## Outcome payloads

Every variant carries the physical `pane_id`.

Completed transitions with resolved scheduler ownership carry `ticket_id`.
Delivery, assignment recovery, and initial startup retain `Option<TicketId>`
because their existing malformed-reservation branches still commit a terminal
seat state before discovering that the reservation has no ticket identity.

Timeout and stale reclaim carry the actual `fenced` result. This agrees with the
boolean already sent to provenance and keeps `AlreadyFenced` behavior truthful.

Ordinary error reclaim is explicitly non-fenced and therefore does not carry a
redundant fencing flag.

## Authority and semantic review

The new type is descriptive and private. It is neither serialized nor used as a
new scheduling decision source.

The T-039-03-01 invariant matrix remains intact:

- assignment delivery failure retains its current lease, failed thread,
  reservation, and reusable pane;
- assignment recovery retains exactly one successor lease and failed
  reservation for operator reset;
- initial startup failure retains lease and pane;
- exhausted startup recovery revokes the successor, fences the pane, and
  retains the failed reservation;
- ordinary error emits failed/non-fenced provenance, releases, and removes;
- session timeout revokes, fences, emits timed-out provenance, releases, and
  removes;
- stale reclaim uses the same hard-silence teardown with failed provenance;
- no retained terminal state becomes automatically retryable;
- no retry bound, deadline, readiness gate, or awaiting-human/pending-completion
  exemption changed.

This preserves E-034 lease semantics and E-035 recovery semantics.

## Test coverage

Added `retained_failure_helpers_return_path_specific_outcomes`, covering all
four retained variants directly.

Extended existing invariant tests to assert exact variants for ordinary error,
timeout, and stale reclaim. The timeout awaiting-human test also asserts an
empty outcome vector, confirming that a guarded non-transition is not reported
as a failure.

Existing tests continue to cover:

- bounded delivery retry and late-ack rejection;
- one-successor assignment recovery and operator retention;
- startup successor revocation and pane fencing;
- provenance kind and ordering;
- lease revoke/fence/release lifecycle order;
- monotonic redispatch above lease high-water;
- pending-completion and awaiting-human exclusions;
- signal-consumer ordering and one-shot ingestion.

## Verification record

```text
cargo test -p lisa-plugin --lib
314 passed; 0 failed; 0 ignored
```

```text
cargo test --workspace
pass; one real-Zellij integration intentionally ignored by its declared
environment/wasm-target gate
```

```text
just check
WASM check: pass
workspace tests: pass
```

```text
cargo fmt --all -- --check
pass

git diff --check
pass
```

## Files and commit hygiene

Modified and committed:

- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/tests/signal_consumer_characterization.rs`.

No source file was staged through the ordinary index. The exact two paths were
committed with `lisa commit-ticket`. After the transaction, no ticket-owned
source file is staged, modified, or untracked.

Remaining working-tree changes are Lisa-managed provenance, ticket phase, and
published workflow artifacts. They are not part of the source transaction and
were not edited as ticket-owned source.

## Open concerns and limitations

The outcomes are currently internal return values; the scheduler poll discards
them after existing logs/state updates. This satisfies the typed-boundary goal
without adding a growing production history or a new dashboard contract. A
future ticket may consume them for consolidated observability.

The optional ticket payload on malformed reservation failures is deliberate.
Making it mandatory would require fabricating identity or rejecting a terminal
seat transition that currently remains operator-visible.

No critical issue, TODO, semantic gap, or follow-up fix is required for this
ticket. The real-Zellij test was not run because its test definition marks it
ignored unless the full external environment is present; native and WASM gates
cover the changed code.
