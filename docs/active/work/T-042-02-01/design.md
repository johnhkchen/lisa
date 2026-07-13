# Design: Durable completion identity and commit discovery

## Goal

Make a completion transaction replayable by stable identity.

The same ticket, attempt, and completion generation must converge on the first
commit created for that identity. A different identity must not be mistaken for
the first, even if the ticket is already Done.

## Identity options

### Reuse `CompletionId`

`CompletionId` currently contains the ticket id in production. It could be
reinterpreted as the entire idempotency identity.

That would erase the distinction between the aggregate identity already used
by the reducer and a generation of an external completion transaction. It
would also require callers to encode attempt and generation into an opaque
string without a type-enforced component boundary.

Rejected.

### Use the existing pair only

The existing `CompletionId` plus `AttemptId` pair could act as the key.

This is stable across duplicate events for one attempt, but it cannot represent
successive completion command generations for the same attempt. The next two
story tickets explicitly need that dimension for journal and reconciliation
records.

Rejected.

### Add an untyped CLI token

The plugin could generate an arbitrary string and pass `--idempotency-key`.
The transaction would treat it as opaque.

This would make request construction easy, but lisa-core would not express the
ticket/attempt/generation invariant. Tests and future journal code could pair a
token with the wrong ticket or attempt without a type-level signal.

Rejected.

### Add a typed three-component identity

Add `CompletionGenerationId` to lisa-core. It owns a `CompletionId`, an
`AttemptId`, and a numeric generation behind private fields, with a constructor
and read-only accessors.

The type has value equality, ordering, hashing, cloning, and a stable Display
representation. Display uses a versioned, unambiguous ASCII encoding so the
same value can be embedded in commit metadata and later journal entries.

Chosen.

This is a new strong identity type while preserving the established meanings
of `CompletionId`, `AttemptId`, and `CorrelationId`.

## Stable encoding

Ticket and attempt values are opaque strings and could contain punctuation.
Joining them with a raw delimiter can create collisions.

The Display form will encode each UTF-8 byte as lowercase hexadecimal and use:

`v1:<ticket-hex>:<attempt-hex>:<generation>`

Hex components contain only `[0-9a-f]`, so separators cannot collide with
component content. The `v1` prefix reserves room for an intentional future
format migration.

The numeric generation remains decimal for readability. Construction does not
parse user-supplied encoded strings; trusted typed components create the value.

The CLI receives component options and constructs the core type. This avoids a
second parser and lets Clap validate the numeric generation.

## Completion generation source

The existing effect contains ticket and attempt identity but no journal-backed
generation counter yet. The first generation for every effect identity is `1`.

The plugin effect executor will construct `CompletionGenerationId` from:

- the effect's `CompletionId`;
- the effect's `AttemptId`; and
- generation `1`.

Every retry of the same current adapter effect therefore reuses the same key.
A new attempt automatically produces a different key. T-042-02-02 can persist
this value, and T-042-02-03 can explicitly advance or replay generation without
changing the CLI contract introduced here.

Operator completion retains its existing `operator` attempt identity and uses
generation `1`. The operator-authority story remains responsible for changing
that authority model.

## CLI transport options

### One encoded `--completion-key`

This is compact but requires parsing and validating the stable encoding in the
CLI, including proving it matches `--ticket-id`.

The encoded form is storage metadata, not the most ergonomic command surface.

Rejected.

### Separate attempt and generation options

Retain `--ticket-id` and add:

- `--attempt-id <opaque-string>`; and
- `--completion-generation <u64>`.

`main.rs` builds `CompletionGenerationId` using the ticket id as the
`CompletionId`. `CompleteTicketRequest` carries the resulting typed key.

Chosen.

This makes the full binding visible in command diagnostics, avoids redundant
ticket input, and keeps the library request strongly typed.

## Commit identity storage options

### Sidecar idempotency file

The transaction could write a key-to-commit map under `.lisa`.

That introduces a second atomic publication and recovery problem. If the map
and commit diverge, neither alone proves the outcome. It also adds a new exact
path to every completion commit and overlaps the journal work assigned to the
next ticket.

Rejected.

### Git notes or a dedicated ref

Git notes or a Lisa-specific ref can map keys to commits.

They require extra ref update/rollback semantics and are less likely to travel
with ordinary branch history. The current isolated transaction is centered on
one HEAD compare-and-swap.

Rejected.

### Commit-message trailer

Append a machine-readable line to the completion commit message:

`Lisa-Completion-Key: <stable-id>`

The identity then lives in the same durable object whose id is returned. It
moves with branch history, requires no ticket-owned sidecar, and can be found
after unrelated later commits.

Chosen.

The caller's human message remains the subject/body prefix. The transaction
adds one blank line and the exact trailer.

## Discovery semantics

Search commits reachable from current HEAD for the marker corresponding to the
typed key.

Git's grep narrows candidates using fixed-string matching. Candidate commit
messages are then read and checked for an exact marker line. This final check
prevents a key from matching a longer value that merely has the same prefix.

Return the first reachable exact match, which is the most recent. At-most-one
creation means normal history contains only one.

The discovered result returns:

- the prior commit's id;
- an empty committed-path list, because replay committed nothing; and
- an internal previous id equal to the discovered id, since rollback is not
  applicable to a read-only discovery result.

## Serialization boundary

Discovery must occur while holding the same repository transaction lock used
for commit creation.

If lookup happened before the lock, two concurrent calls could both observe no
match and then serialize into two commits. The shared internal transaction
entry therefore accepts an optional completion key:

1. validate the commit request;
2. discover the repository;
3. acquire the transaction lock;
4. discover and return an existing keyed commit if present;
5. otherwise create exactly one marked commit;
6. clean up and unlock through the existing rollback rules.

Public `commit_ticket` calls this entry without a key and retains existing
behavior. `complete_ticket` calls it with the key.

## Already-Done behavior

The existing clean-Done shortcut returns current HEAD without proving which
request created it. It is incompatible with identity-specific results.

Remove the state-only shortcut from `complete_ticket`.

For a repeated key, commit discovery returns the actual prior completion commit
even if unrelated commits now follow it.

For a different key with no ticket/work changes, the normal transaction reports
that there are no changes. It does not return the first key's commit.

For a different key with new work changes, the transaction can create a new
completion commit carrying the different marker. This preserves ordinary
transaction behavior outside exact key equality.

## Ticket mutation and rollback

`complete_ticket` continues to read original bytes, prepare Done frontmatter,
and restore original bytes on a genuine transaction error.

On a same-key replay, `update_ticket_done` is idempotent and discovery succeeds
without creating a tree or advancing HEAD.

No broad rewrite of frontmatter preparation or alternate-index isolation is
needed for the acceptance scenario.

## Testing strategy

Add core tests proving:

- component accessors preserve ticket, attempt, and generation;
- equal components yield equal identity and Display output;
- component delimiter content cannot collide in Display output; and
- changing any of the three components changes the key.

Replace the state-only already-Done transaction regression with an identity
regression that:

1. creates a Review ticket and work artifact;
2. invokes `complete_ticket` with key A;
3. adds an unrelated commit after completion;
4. invokes `complete_ticket` again with key A;
5. asserts the returned id is the first completion commit, not current HEAD;
6. asserts commit count did not change on replay; and
7. invokes with key B and asserts it does not discover key A.

Use a changed work artifact for key B when proving it remains independently
committable. This demonstrates exact-match behavior rather than only an error.

Update every `CompleteTicketRequest` literal and plugin command assertion.
The connected nested-monorepo test continues to drive the real builder into the
real transaction with the new options.

## Compatibility and scope

`complete-ticket` is an internal Lisa command launched by the plugin. Making
the two new options required intentionally prevents identity-less new
completion commits.

`commit-ticket` remains unchanged. Existing isolated-index, exact-path,
rollback, nested-root, and foreign-stage behavior remains the transaction
safety boundary.

No journal file, reload logic, retry deadline, reducer transition, dashboard
variant, or provenance schema changes in this ticket.
