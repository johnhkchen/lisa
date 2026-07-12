# Progress: named failure transition outcomes

## Result

Implemented a private `FailureTransitionOutcome` enum that gives every
characterized scheduler failure/reclaim path a distinct typed result:

- `AssignmentDeliveryFailed`;
- `AssignmentRecoveryFailed`;
- `StartupFailed`;
- `StartupRecoveryFailed`;
- `ErrorReclaimed`;
- `SessionTimedOut`;
- `StaleThreadReclaimed`.

The enum is descriptive only. Scheduler authority remains in current leases,
lease high-water history, seat assignments, threads, and agent slots.

## Retained transition helpers

`fail_assignment_delivery`, `fail_assignment_recovery`, `fail_startup`, and
`fail_startup_recovery` now return `Option<FailureTransitionOutcome>`.

`None` means the pre-existing source-state/reservation guard rejected the edge.
`Some` is constructed after the existing mutation and logging sequence.

Delivery, assignment recovery, and initial startup results carry an optional
ticket because their prior malformed-reservation branches still transition the
seat before discovering that no ticket identity is available.

No retained helper revokes, releases, removes, fences, emits provenance, or
adds retries beyond its previous behavior.

## Automatic reclaim scanners

`check_error_signals`, `check_session_timeouts`, and `detect_stale_threads` now
return ordered vectors because one poll can process more than one transition.

Ordinary error signals append `ErrorReclaimed` after failed provenance, release,
thread removal, alerting, and logging. Recovery errors remain routed to
`AssignmentRecoveryFailed`; unknown-pane errors remain outcome-free no-ops.

Session timeout and stale reclaim results include the ticket, pane, and actual
fence boolean. Each result is appended after the existing revoke/fence,
provenance, release, removal, alert/log sequence.

Disabled timeouts, active over-budget sessions, awaiting-human sessions,
pending completions, and healthy threads return no reclaim outcome.

## Tests

Added `retained_failure_helpers_return_path_specific_outcomes`, which directly
asserts all four retained variants.

Extended the existing matrix tests to assert:

- `ErrorReclaimed` for ordinary `.error`;
- `SessionTimedOut` with `fenced=true` for expired hard-silent sessions;
- `StaleThreadReclaimed` with `fenced=true` for stale hard-silent threads;
- no timeout outcome for the awaiting-human guard.

All previous state, lease, pane, provenance, alert, retry, lifecycle-order, and
redispatch assertions remain in place and passing.

## Source files

Modified:

- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/tests/signal_consumer_characterization.rs`.

The characterization module required two match arms to discard the new batch
return explicitly so all signal consumers still form a unit-valued match.

## Plan deviation

The structure anticipated only `lib.rs`. Native test compilation showed that
the separately included signal-consumer characterization module uses a match
whose arms must have identical types. The two error arms therefore needed
blocks with semicolons. This is a signature adaptation only; no test behavior or
signal-consumer ordering changed.

## Verification

Formatting and whitespace:

```text
cargo fmt --all -- --check
pass

git diff --check
pass
```

Plugin invariant suite:

```text
cargo test -p lisa-plugin --lib
314 passed; 0 failed; 0 ignored
```

Workspace suite:

```text
cargo test --workspace
all native tests passed; 1 real-Zellij integration intentionally ignored by
its environment/target gate
```

Project gate:

```text
just check
cargo check -p lisa-plugin --target wasm32-wasip1: pass
cargo test --workspace: pass
```

## Implementation status

- Complete: seven explicit named outcomes.
- Complete: retained versus automatic authority preserved.
- Complete: outcome assertions for all seven variants.
- Complete: T-039-03-01 matrix and E-034/E-035 regression coverage green.
- Complete: isolated Lisa source commit
  `f63bab2317037d15bb2cd54166f6b2bbc0ceca27`.
- Complete: no ticket-owned source remains staged, modified, or untracked.
- Pending: final Review artifact.
