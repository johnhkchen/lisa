# Design — T-045-01-01 atomic assignment file writer

## Decision target

The design must make an assignment reference an explicit ticket/attempt/nonce value,
publish its bytes through sibling-temp rename, and preserve the exact reference until
the scheduler delivers it.
It must reuse the existing publication mechanism, preserve E-034 lease authority, and
avoid implementing later claim, launcher, ownership, or boundary stories.

## Option 1 — rename the constant deterministic file

The smallest textual edit would change `assignment.md` to a leaf containing the
attempt ID, such as `assignment-7.md`, while leaving the existing writer and delivery
reconstruction intact.

Advantages:

- minimal scheduler state change;
- still uses the atomic publication helper;
- attempt identity is visible in the path.

Disadvantages:

- there is no durable nonce;
- a claim cannot distinguish two publications within the same attempt;
- it does not satisfy the explicit ticket+attempt+nonce contract;
- delivery still guesses a filename instead of consuming the writer's result.

This option is rejected because it omits the main new identity boundary.

## Option 2 — put the nonce only in the temporary filename

The existing implementation already writes `.assignment.md.tmp.{nonce}` and renames
it to `assignment.md`.
One interpretation could treat the temporary nonce as sufficient.

Advantages:

- no production code change;
- generic atomicity is already tested.

Disadvantages:

- the nonce disappears after publication;
- the launcher cannot reference it;
- the later claim command cannot present or validate it;
- the ticket explicitly describes a nonce-bearing file, not only a transient temp.

This option is rejected as incompatible with the story's assignment/claim identity.

## Option 3 — nonce-bearing durable path discovered from the directory

The writer could publish `assignment-{attempt}-{nonce}.md` and delivery could scan the
attempt work directory for a matching prefix.

Advantages:

- no new state field;
- durable filenames contain attempt and nonce;
- the existing adapter can receive the discovered path.

Disadvantages:

- repeated preparation or recovery leaves multiple matches;
- filesystem enumeration order is not an authority rule;
- cleanup failure can change which assignment is selected;
- a stale file could be delivered after a newer writer result;
- the scheduler would infer identity from ambient files rather than durable state.

This option is rejected because it weakens the exact-reference contract.

## Option 4 — add the nonce to `AttemptLease`

The lease could gain a nonce field and the writer could derive the file path directly
from that expanded type.

Advantages:

- ticket, attempt, and nonce travel together everywhere;
- delivery could reconstruct the destination deterministically;
- claim validation would naturally compare one object.

Disadvantages:

- it changes E-034's established authority type;
- every serialized lease signal, completion value, provenance record, and fixture
  would change;
- nonce rotation semantics would become entangled with attempt generation;
- the ticket explicitly says to preserve lease fencing and isolate the smallest
  assignment boundary.

This option is rejected as a broad and semantically risky change.

## Option 5 — explicit assignment reference plus retained scheduler mapping

Create a focused plugin assignment module with an `AssignmentRef` value containing:

- the exact `AttemptLease`;
- an opaque numeric nonce;
- the durable path.

The writer accepts an attempt work directory, lease, nonce, and assignment bytes.
It builds an attempt-tagged, nonce-bearing leaf, then publishes through the existing
`RustPublication` helper.
It returns the complete reference only after rename succeeds.

`State` retains the current reference by ticket ID.
Each production preparation call stores the returned reference.
Delivery obtains the reference from the map and requires its lease to equal the exact
current pane lease before testing the file and passing the path to the adapter.

Advantages:

- makes the assignment identity explicit;
- preserves the lease type and authority checks;
- never guesses among files;
- reuses the existing atomic helper;
- creates the contract T-045-01-02 can consume;
- stays local to assignment preparation and delivery.

Costs:

- adds a small state collection;
- production call sites must mint a nonce and store the result;
- tests that call the old helper need focused updates.

This option is selected.

## Assignment filename

Use `assignment-{attempt_id}-{nonce}.md`.

The containing production path already includes the ticket ID and attempt ID:

`.lisa/attempts/{ticket_id}/{attempt_id}/work/assignment-{attempt_id}-{nonce}.md`

The repeated attempt tag in the leaf makes the bounded reference self-describing even
when logged independently from its parent directory.
The ticket ID remains in the authoritative attempt directory and in `AssignmentRef`;
it is not copied into the leaf, avoiding a new filename sanitization boundary.

The sibling temporary uses
`.assignment-{attempt_id}-{nonce}.md.tmp.{publication_nonce}`.
The durable assignment nonce and write-operation nonce serve different purposes:
the first is claim identity; the second avoids temporary collisions.

## Nonce contract

Use the plugin's established wall-clock nanosecond nonce generator and expose it only
within the crate.
Represent the nonce as `u128`.
It is opaque equality identity, not a secret or authentication credential.
No cryptographic guarantee is asserted.
The writer accepts the nonce explicitly so unit tests are deterministic and later
callers can carry the same token into claim-related state.

Production preparation mints a nonce immediately before the write.
A failed write returns no `AssignmentRef` and is not stored.
A later retry mints a new nonce, so a reference is never reused after uncertain
publication.

## Atomicity contract

The writer creates the attempt work directory before publication.
It asks `RustPublication` to write the entire payload to a sibling temporary.
It renames that complete temporary to the unique durable destination.
The writer returns `AssignmentRef` only after successful rename.

On write failure:

- the durable destination is absent;
- no reference is returned.

On rename failure:

- the helper removes the temporary;
- no reference is returned;
- an existing destination remains complete.

An interrupted producer can at worst leave a partial hidden sibling temporary.
Because launchers receive only the returned durable path, that temporary is not a
published assignment reference and cannot be read as the assignment.

## State validation

The mapping is keyed by ticket ID because only one current lease per ticket is
authoritative.
The stored value also carries its full lease.
Delivery validates all of the following:

1. pane reservation exists;
2. reservation lease matches requested generation;
3. reservation lease is current;
4. an assignment reference exists for the ticket;
5. the reference lease exactly equals the reservation lease;
6. the referenced durable path is a file.

Only after those checks does the adapter receive the path.
This preserves and strengthens the current fail-closed behavior.

## Repeated preparation

Repeated preparation for the same ticket replaces the map entry with a freshly
published reference.
The previous immutable file can remain in the attempt-private directory.
It is unreachable from scheduler state and cannot pass reference equality.
Explicit deletion is deferred because later boundary work owns revocation/cleanup,
and deleting a file that a launched process may still address could introduce a race.

## Test design

Add focused module tests rather than only extending the large scheduler test module.

The success test supplies a hostile ticket ID in the lease, a known attempt ID, a
known nonce, and a large hostile body.
It asserts:

- returned lease and nonce are exact;
- returned path is attempt-tagged and nonce-bearing;
- reading the path returns the complete byte sequence;
- no temporary remains.

The interruption test creates a partial sibling temporary without publishing it.
It asserts the durable assignment path is absent and the partial bytes are visible
only under the hidden temporary name.
It then invokes the real writer and asserts the returned durable path contains only
the complete payload.
This directly demonstrates the temp-then-rename publication boundary.

The existing generic publication failure tests continue to cover rename failure and
cleanup.
Scheduler tests continue to cover bounded reference delivery.

## Rejected scope

This ticket will not:

- add a `lisa claim` command;
- persist claim records;
- change ownership state transitions;
- change Codex or Claude launch argv;
- change Zellij injection;
- revoke nonces at completion;
- add dashboard labels;
- add live Codex or Zellij tests;
- modify `AttemptLease` serialization.

Those behaviors have explicit dependent tickets in E-045.

## Expected source ownership

The meaningful source unit consists of:

- a new focused `crates/lisa-plugin/src/assignment.rs` module;
- `crates/lisa-plugin/src/publication.rs` exposing the established nonce generator;
- `crates/lisa-plugin/src/lib.rs` wiring exact references into scheduler state and
  delivery.

These three paths form one coherent contract and should be committed together through
Lisa's isolated transaction.
