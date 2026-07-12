# T-034-02-01 Progress — bind Codex ack to lease

## Status

Implementation, verification, and isolated source commit complete.

## Completed phases

- [x] Research written to `research.md`.
- [x] Design written to `design.md`.
- [x] Structure written to `structure.md`.
- [x] Plan written to `plan.md`.

## Implementation checklist

- [x] Remove the independent acknowledgement generation counter.
- [x] Source reused-Codex markers from the minted attempt lease.
- [x] Require exact current lease authority before ownership promotion.
- [x] Remove the obsolete dead-code allowance.
- [x] Mint and stamp a successor lease for fresh recovery.
- [x] Update scanner coverage with real lease state.
- [x] Add current-versus-prior replacement acknowledgement coverage.
- [x] Reconcile bounded recovery and consecutive-reuse tests.
- [x] Run focused verification.
- [x] Run complete verification.
- [x] Commit ticket-owned source through `lisa commit-ticket`.
- [x] Verify source cleanliness and isolated commit contents.

## Deviations

No scope deviation.

The implementation followed the selected design's recovery treatment: the
fresh fallback now receives a successor attempt lease so its marker remains
distinct from the abandoned reused-session marker without retaining a second
generation namespace.

The existing replacement ownership test was strengthened in place rather than
adding another similarly named test.

The first commit attempt used `/opt/homebrew/bin/lisa`, which is older than the
repository CLI and does not expose `commit-ticket`. No Git state changed. The
same command was then run through the repository-built `target/debug/lisa`,
which completed the required isolated transaction.

## Implementation completed

### Lease-sourced acknowledgement markers

Removed `State::next_assignment_generation` and
`allocate_assignment_generation`.

`schedule_ready_tickets` now derives the pending reused-Codex marker generation
from the successfully minted `AttemptLease::attempt_id`.

Fresh Codex and all Claude assignments retain immediate `Owned` behavior.

### Exact promotion gate

`acknowledge_codex_assignment` now requires:

- a pending or recovering assignment;
- one slot carrying both the ticket reservation and attempt lease;
- equality between reservation ticket and lease ticket;
- equality between pending generation and lease attempt ID;
- exact validation against `current_leases`;
- an exact detector match for the validated ticket and attempt ID.

It rejects already-owned seats explicitly through `seat_is_owned`, making that
helper production-used and allowing removal of `#[allow(dead_code)]`.

### Recovery authority

`begin_assignment_recovery` validates the original lease against current and
high-water state, mints a strict successor, and installs the same successor on:

- `lease_high_water`;
- `current_leases`;
- the assigned physical slot;
- the logical thread;
- the `Recovering` generation.

Missing or stale authority and mint errors use the existing actionable
`RecoveryFailed` path without sending replacement provider input.

### Regression coverage

`test_codex_ack_signal_promotes_matching_pending_seat` now installs a real
lease in the scanner fixture.

`test_recycled_codex_ownership_requires_matching_ack_exactly_once` now performs
two real scheduler dispatches of the same ticket:

- attempt 1 is released;
- attempt 2 becomes the current pending replacement;
- an attempt-1 acknowledgement is rejected;
- an attempt-2 acknowledgement is rejected while current authority is absent;
- restoring attempt 2 permits one promotion;
- a duplicate acknowledgement remains inert.

`test_bounded_ack_wait_recovers_once_then_fails_actionably` now asserts exact
successor agreement across high-water, current, slot, thread, and recovery
state, plus old-lease invalidation.

## Focused verification

Passed:

```text
cargo test -p lisa-plugin test_recycled_codex_ownership_requires_matching_ack_exactly_once
1 passed; 0 failed

cargo test -p lisa-plugin test_bounded_ack_wait_recovers_once_then_fails_actionably
1 passed; 0 failed

cargo test -p lisa-plugin test_recovery_ack_promotes_only_the_fresh_generation
1 passed; 0 failed
```

The focused `codex_ack` run initially exposed the scanner fixture's missing
lease stamp. The fixture was updated to match production state, after which the
full plugin suite passed.

## Complete verification

Passed:

```text
cargo test -p lisa-plugin
268 passed; 0 failed

cargo test --workspace
Lisa CLI: 270 passed
atomic provider contract integration: 1 passed
Lisa core: 155 passed
Lisa plugin: 268 passed
total: 694 passed; 0 failed

cargo check -p lisa-plugin --target wasm32-wasip1
passed

cargo clippy -p lisa-plugin --lib -- -D warnings
passed

cargo fmt --all -- --check
passed

just check
passed

git diff --check
passed
```

`rg` confirms no remaining `next_assignment_generation`,
`allocate_assignment_generation`, or acknowledgement-path
`#[allow(dead_code)]` occurrence.

## Isolated source commit

Created through the repository-built Lisa CLI:

```text
target/debug/lisa commit-ticket \
  --ticket-id T-034-02-01 \
  --message "fix: bind Codex acknowledgements to attempt leases" \
  --include crates/lisa-plugin/src/lib.rs
```

Result:

```text
77e524f70eb2356904edd8fd15a8b4ecd4f308e3
```

Commit inspection shows exactly one path:

```text
crates/lisa-plugin/src/lib.rs
```

The ticket-owned source path is clean after the commit.

`git diff --cached --name-only` is empty, so the ordinary index remains
untouched.

The ticket and RDSPI artifacts remain untracked for Lisa's final completion
transaction, as required by the workflow.

## Working-tree boundary

The only planned ticket-owned source path is:

```text
crates/lisa-plugin/src/lib.rs
```

Pre-existing unrelated modifications and untracked files are present in the
shared worktree. They will not be edited, staged, or included.
