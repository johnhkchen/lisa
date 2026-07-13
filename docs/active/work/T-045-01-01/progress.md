# Progress — T-045-01-01 atomic assignment file writer

## Status

Implementation is complete, verified, and committed.
Review remains after commit confirmation.

## Completed work

### Focused assignment contract

Created `crates/lisa-plugin/src/assignment.rs`.

The module defines `AssignmentRef`, which retains:

- the exact ticket/attempt `AttemptLease`;
- the durable assignment nonce as `u128`;
- the exact durable assignment path.

The module defines `write_assignment` with explicit inputs for attempt work directory,
lease, nonce, and assignment bytes.
The durable leaf is `assignment-{attempt_id}-{nonce}.md`.
The writer creates the directory and delegates publication to the existing
`RustPublication` sibling-temp rename helper.
The temporary family is
`.assignment-{attempt_id}-{nonce}.md.tmp.{publication_nonce}`.
The reference is returned only after rename succeeds.

### Existing atomic helper reuse

Modified `crates/lisa-plugin/src/publication.rs` only to make the established
wall-clock nanosecond `publication_nonce` generator crate-visible.
The nonce algorithm and generic publication behavior are unchanged.
No dependency or manifest change was required.

### Scheduler reference retention

Modified `crates/lisa-plugin/src/lib.rs`.

Registered the assignment module and added `State::assignment_refs`, keyed by ticket
ID.
The scheduler's preparation method now:

1. mints a nonce;
2. publishes the exact assignment;
3. stores the returned reference only on success.

Normal dispatch, same-pane startup recovery, and post-exit relaunch all pass an exact
`AttemptLease` to the writer.
The post-exit path now fails closed when its ticket reservation has no attempt lease,
instead of constructing assignment context with attempt `0`.

`deliver_assignment_to_pane` no longer reconstructs `assignment.md`.
It obtains the retained reference, verifies the reference lease equals the exact
current pane lease, requires the exact path to be a file, and passes that path to the
provider adapter.
No directory scan or ambiguous latest-file selection was introduced.

### Test coverage

Added the acceptance-focused success test:

`assignment::tests::writes_ticket_attempt_nonce_assignment_and_reads_it_back_intact`

It writes a large hostile payload for a known ticket, attempt, and nonce and asserts:

- exact returned lease;
- exact returned nonce;
- attempt-tagged, nonce-bearing durable filename;
- byte-for-byte readback;
- no publication temporary residue.

Added the interruption test:

`assignment::tests::interrupted_partial_temporary_never_becomes_the_published_assignment`

It places partial bytes only in a hidden sibling temporary, proves the durable path is
absent, invokes the real writer, and proves the durable reference contains only the
complete assignment.

Updated existing plugin publication tests for the new leaf and temporary families.
Updated dispatch and startup-recovery tests to inspect the exact stored reference.
Corrected the recycle exit-grace fixture to include the attempt lease that production
scheduling stamps before the transition.

## Verification performed

### Focused assignment tests

Command:

`cargo test -p lisa-plugin assignment::tests`

Result:

- 2 passed;
- 0 failed.

### Plugin suite

Command:

`cargo test -p lisa-plugin`

Initial result:

- 386 passed;
- 1 failed.

The failure was `test_recycle_exit_grace_launches_fresh_incoming_client`.
Its manually constructed reservation omitted `AttemptLease`, while the production
scheduler always stamps the incoming lease before waiting for exit.
The fixture was updated to include the exact current lease.

Focused rerun:

`cargo test -p lisa-plugin test_recycle_exit_grace_launches_fresh_incoming_client`

Result:

- 1 passed;
- 0 failed.

Final plugin result through `just check`:

- 387 passed;
- 0 failed.

### Formatting and diff checks

Commands:

- `cargo fmt --all`;
- `cargo fmt --all -- --check`;
- `git diff --check`.

Result: all passed.

### Workspace suite

Command:

`cargo test --workspace`

Result: all workspace unit, integration, state-machine, and doc tests passed.
The repository's explicitly ignored real-Zellij test remained ignored by its existing
environment gate.

### Repository check

Command:

`just check`

Result:

- `cargo check -p lisa-plugin --target wasm32-wasip1` passed;
- `cargo test --workspace` passed;
- plugin total 387 passed, 0 failed;
- no ticket-attributable warning was emitted.

## Plan deviations

The design anticipated that post-exit relaunch might need an explicit missing-lease
guard.
Implementation confirmed this and added the guard.
One old native fixture used a lease-free reservation; it was corrected rather than
weakening the production contract or fabricating attempt `0`.

The interrupted-write test deliberately leaves its simulated partial temporary in
place.
This models a process interruption, where cleanup cannot run, and demonstrates that
the partial file is never the published durable reference.
The generic publication failure test separately verifies cleanup when rename returns
an ordinary error to the running process.

No other deviation from Design or Plan occurred.

## Source ownership

Ticket-owned source paths are exactly:

- `crates/lisa-plugin/src/assignment.rs`;
- `crates/lisa-plugin/src/publication.rs`;
- `crates/lisa-plugin/src/lib.rs`.

Unrelated `.lisa` protocol files and active epic/story/ticket files remain outside the
ticket include set.
Lisa automatically admitted phase artifacts into a shared work path while observing
the private attempt directory; those scheduler-owned/untracked publications are not
part of the source transaction.

## Remaining

The isolated transaction completed successfully:

`a0ad89c9379a224c2b3965f04ef8081ab046e3ba feat(plugin): publish nonce-bound assignments`

Post-commit inspection confirmed:

- all three ticket-owned paths are included in that commit;
- no ticket-owned path remains modified or untracked;
- the ordinary Git index contains no staged path;
- unrelated working-tree state remains outside the transaction.

Remaining work is Review artifact publication and waiting for Lisa's completion
handling.
