# Review — T-045-01-01 atomic assignment file writer

## Disposition

Pass.

The acceptance criterion is met by a focused writer test that round-trips an exact
ticket/attempt/nonce assignment and a second test that demonstrates partial temporary
bytes never become the durable assignment reference.
The implementation is committed through Lisa's isolated ticket transaction and the
ticket-owned source paths are clean.

## Commit

`a0ad89c9379a224c2b3965f04ef8081ab046e3ba`

Message:

`feat(plugin): publish nonce-bound assignments`

The commit contains exactly three source paths:

- `crates/lisa-plugin/src/assignment.rs`;
- `crates/lisa-plugin/src/publication.rs`;
- `crates/lisa-plugin/src/lib.rs`.

No ordinary-index commit was used.
No ticket-owned source path remains staged, modified, or untracked.

## What changed

### New assignment boundary module

`crates/lisa-plugin/src/assignment.rs` now owns the plugin's exact assignment-file
identity and publication entry point.

`AssignmentRef` carries three values:

- the existing `AttemptLease`, containing ticket ID and attempt ID;
- the assignment nonce;
- the exact durable file path.

Keeping `AttemptLease` intact preserves E-034 lease semantics.
The nonce is assignment identity layered beside lease authority, not a change to the
meaning or serialization of the lease.

`write_assignment` accepts the attempt work directory, exact lease, explicit nonce,
and assignment bytes.
It publishes the bytes to:

`assignment-{attempt_id}-{nonce}.md`

under the ticket/attempt-private work directory.
The production full path therefore contains ticket, attempt, and nonce identity.

### Atomic publication

The writer reuses `RustPublication` from `publication.rs`.
It writes the complete byte slice to a hidden sibling temporary named from:

`.assignment-{attempt_id}-{nonce}.md.tmp.{publication_nonce}`

and then renames that sibling to the durable destination.
The returned `AssignmentRef` is constructed only after rename succeeds.
Callers cannot receive a path for a merely partial temporary.

Write failures remain attached to the temporary path.
Rename failures continue to remove the temporary and return the existing operator
error label.
No direct write targets the durable assignment path.

`publication.rs` changed only by widening the existing nonce generator to
crate-visible scope.
The established wall-clock nanosecond implementation is unchanged.

### Exact scheduler reference retention

`State` now retains the successfully published reference in `assignment_refs`, keyed
by ticket ID.
This is intentionally separate from:

- `current_leases`, which controls authority;
- `seat_assignments`, which controls lifecycle/ownership state.

Normal dispatch, same-pane startup recovery, and post-exit relaunch pass an exact
lease to the writer.
The scheduler stores a new reference only after successful publication.
On write failure, it publishes no new mapping.

Delivery no longer reconstructs a constant `assignment.md` path.
It loads the exact retained reference, requires its lease to equal the current pane
lease, requires the referenced durable path to be a file, and then gives that exact
path to the provider adapter.
This prevents guessing among nonce-bearing files left by repeated preparation.

### Fail-closed relaunch context

The post-exit relaunch path previously used attempt ID `0` if a manually constructed
reservation lacked a lease.
The new writer cannot truthfully publish a ticket/attempt/nonce reference without an
attempt.
That path now reports a missing current lease and does not write or launch.

The production scheduler already stamps the incoming lease before entering the exit
wait.
One native fixture omitted that production invariant; it was updated to carry the
exact current lease.

## Acceptance coverage

### Complete readback

`writes_ticket_attempt_nonce_assignment_and_reads_it_back_intact` supplies:

- an explicit ticket-bearing lease;
- attempt `7`;
- nonce `8675309`;
- a large payload containing quotes, dollar syntax, backticks, escapes, control
  characters, tabs, carriage returns, and newlines.

It asserts the returned lease and nonce, exact durable filename, byte-for-byte
readback, and absence of successful-publication temporary residue.

This directly satisfies the ticket requirement to write an assignment for a
ticket+attempt+nonce and read it back intact.

### Interrupted/partial write

`interrupted_partial_temporary_never_becomes_the_published_assignment` first writes
only partial bytes to a hidden sibling temporary.
It asserts that the durable assignment path does not exist.
It then calls the real writer and asserts the durable path contains only the complete
payload.
The simulated interrupted temporary remains distinct and is never returned as the
assignment reference.

This models the process-interruption case where cleanup cannot run.
The pre-existing generic failure test separately covers an ordinary rename error in a
live process and verifies temporary cleanup plus preservation of prior durable state.

### Existing regression coverage

Existing publication tests were updated to cover:

- replacement at an exact attempt/nonce destination;
- hostile path and payload bytes;
- nonce-bearing temporary names in write errors;
- rename failure cleanup;
- no successful-publication residue.

Existing scheduler tests now inspect stored exact references for fresh dispatch and
startup recovery.
All prior ownership, acknowledgement, completion, fencing, and provider tests remain
green.

## Verification results

`cargo fmt --all -- --check` passed.

`git diff --check` passed before commit.

`cargo test -p lisa-plugin assignment::tests` passed:

- 2 passed;
- 0 failed.

`cargo test --workspace` passed all enabled workspace tests.

`just check` passed:

- WASM check for `wasm32-wasip1` passed;
- workspace tests passed;
- plugin suite: 387 passed, 0 failed;
- CLI and core suites passed;
- the existing environment-gated real-Zellij integration test remained ignored.

No live Codex tokens or real Zellij run are required or authorized by this ticket.
Those are explicitly assigned to S-045-05.

## Compatibility assessment

Provider adapters already accept an arbitrary assignment path, so neither Claude nor
Codex adapter code required a change.
Assignment content construction is unchanged.
The existing bounded chat-reference path now points to the retained nonce-bearing
file.

Lease fencing is unchanged.
The complete `AttemptLease` must still match `current_leases` before delivery.
The assignment reference adds a second exact equality check against that lease.

No serialization format, configuration key, CLI surface, dependency, template, or
dashboard state changed.
Native and WASM compilation both pass.

## Scope assessment

The implementation stays at the assignment boundary.
It does not implement:

- the `lisa claim` command owned by T-045-01-02;
- Codex launcher argv owned by T-045-02-01;
- Zellij launcher-only injection owned by T-045-02-02;
- claim-based ownership transitions owned by S-045-03;
- completion-time nonce revocation owned by S-045-04;
- real Codex/Zellij field validation owned by S-045-05.

Claude behavior was not forced through a new handshake.
No broad scheduler rewrite was introduced.

## Open concerns and limitations

The nonce uses the plugin's existing wall-clock nanosecond convention.
It is an opaque uniqueness token, not a cryptographic secret.
The ticket and story do not require cryptographic entropy.
If later claim design treats the nonce as an authentication secret rather than an
equality/correlation value, that later ticket must explicitly strengthen generation.

Repeated preparation can leave older immutable nonce-bearing files in the private
attempt directory.
Only the reference retained in state is live and delivery validates its exact lease.
Deletion and nonce revocation are intentionally deferred to the explicit boundary
story; deleting eagerly here could race a launched consumer.

The assignment reference map is in-memory scheduler state.
This matches current delivery orchestration, which republishes assignments at dispatch
and recovery.
Durable cross-process claim validation is the dependent command ticket's responsibility.

The interruption test simulates a crash by leaving partial hidden temporary bytes.
It cannot terminate the Rust process in the middle of `std::fs::write`, but it verifies
the essential visibility property: only rename creates the durable path, and the
partial sibling is never returned or read as the assignment.

None of these limitations blocks this ticket's atomic writer contract.

## Human review focus

A reviewer should confirm:

1. `AssignmentRef` keeps nonce identity separate from `AttemptLease` authority.
2. The durable leaf and containing attempt directory jointly bind ticket, attempt, and
   nonce.
3. Every scheduler preparation boundary stores the writer's exact result.
4. Delivery validates the stored lease and never scans or guesses a file.
5. The focused partial-temporary test accurately captures complete-or-absent
   publication visibility.
6. Later claim/launcher/state-machine work remains outside this commit.

## Final assessment

The implementation satisfies the acceptance criterion, preserves the existing lease
and provider boundaries, passes native and WASM verification, and is ready for Lisa's
completion transaction.
