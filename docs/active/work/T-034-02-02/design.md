# Design: T-034-02-02 gate completion on current lease

## Decision summary

Make completion authority an explicit input to `request_completion` and reject
attempt-originated requests unless their lease is exactly current. Preserve the
operator-only manual path for tickets with no active attempt.

Callers will pass the attempt lease attached to the event context they already
hold: active-thread identity for artifact, idle, manual, and observed-Done
paths, and physical-slot identity for a pane-scoped stopped event.

The validated `CompletionAuthority` is retained in `PendingCompletion` so the
pending state identifies either the admitted attempt or operator action.

An admitted request follows the existing T-031 command construction, isolated
transaction, result verification, publication, teardown, and scheduling path
without behavioral changes.

## Goals

- Reject stale completion before the native commit command is launched.
- Use `AttemptLease::is_current` as the single authority predicate.
- Reject missing attempt identity and missing current authority.
- Keep diagnostic completion origin distinct from attempt identity.
- Preserve the exact T-031 isolated transaction for current attempts.
- Give later artifact-attribution work an explicit completion identity seam.
- Leave rejection observable without mutating ticket or scheduler lifecycle.
- Keep the change provider-neutral.

## Non-goals

- Attributing shared artifact files to their actual writer.
- Adding attempt identity to heartbeat or idle hook payloads.
- Changing Codex acknowledgement behavior.
- Persisting leases across plugin restarts.
- Passing lease identity into the native Git transaction.
- Changing commit serialization, staging, verification, or cleanup.
- Redesigning manual completion UX.
- Publishing attempt-aware provenance.

## Option 1 — infer authority from ticket identity only

Retain the existing request signature and assume an active ticket's completion
belongs to whichever attempt is current.

Advantages:

- no code change;
- existing tests and callers remain untouched.

Disadvantages:

- this is the current vulnerability;
- a prior attempt can complete the replacement's ticket;
- ticket identity does not identify an execution attempt;
- no stale/current regression can exercise a meaningful distinction.

Decision: rejected.

## Option 2 — derive the lease inside `request_completion`

Look up `threads[ticket].attempt_lease` at the start of the method and compare
it with `current_leases[ticket]`.

Advantages:

- small signature change, or none at all;
- production threads already carry leases;
- missing/stale thread stamps can fail closed.

Disadvantages:

- it identifies the replacement thread, not necessarily the event source;
- a stale pane event can be silently credited to the current thread;
- a stale artifact can be silently credited to the replacement;
- later artifact attribution would have to undo this implicit substitution;
- the request API would not express the authority fact it validates.

Decision: rejected. The boundary must validate source identity, not manufacture
it from current state.

## Option 3 — embed the lease in every `CompletionSource` variant

Change variants to carry both their diagnostic data and an `AttemptLease`.

Advantages:

- each event is self-contained;
- call sites cannot omit identity accidentally;
- pattern matching exposes the source lease.

Disadvantages:

- duplicates the lease field across all variants;
- makes a Copy diagnostic enum own a non-Copy string value;
- conflates "why completion was requested" with "who is authorized";
- callers and tests become noisier without improving the check.

Decision: viable but rejected in favor of orthogonal parameters.

## Option 4 — pass source lease explicitly

Extend the request boundary to accept `Option<CompletionAuthority>` beside
`CompletionSource`, where authority is `Attempt(AttemptLease)` or `Operator`.

Advantages:

- source attribution is explicit at every caller;
- missing identity reaches one centralized fail-closed check;
- the existing diagnostic enum remains compact and Copy;
- the method compares evidence with authority without substituting either;
- T-034-02-03 can later replace provisional artifact identity at its caller;
- tests can directly present stale and current leases.

Disadvantages:

- every request caller must resolve and clone a lease;
- legacy unleased completion fixtures must be corrected;
- `Option` permits a caller to compile while providing no evidence.

Decision: selected. The outer `Option` lets the boundary log and reject missing
evidence consistently. `Operator` is constructed only by the manual UI when no
thread exists; attempt-originated callers cannot select it.

## Admission contract

`request_completion(ticket_id, source, authority)` will check, in order:

1. no completion is already pending for the ticket;
2. source authority exists;
3. Attempt authority exactly matches `current_leases[ticket_id]` through
   `is_current`, or Operator authority accompanies a Manual source;
4. all ticket dependencies are Done;
5. the ticket and its concrete file path exist;
6. the pending record can be installed;
7. the unchanged completion command can be built and launched.

The lease check occurs before dependency and file resolution because a stale
attempt has no authority to ask questions about the completion transition.

Exact equality also rejects a lease for a different ticket even if its numeric
attempt ID happens to match.

## Rejection behavior

Missing or stale source identity returns `false`.

The request does not:

- insert a pending completion;
- call `build_completion_command`;
- launch `lisa complete-ticket`;
- alter frontmatter;
- change thread status or phase;
- release or rename the slot;
- emit provenance;
- revoke or replace either lease map;
- schedule dependents.

The activity log receives a warning identifying the ticket, origin, and
rejected attempt when present. It does not claim that the transaction failed,
because no transaction was started.

## Caller identity rules

### Artifact

`check_artifact_advances` already snapshots active threads. Extend the snapshot
to include `thread.attempt_lease.clone()` and pass it when Review requests Done.

This is provisional attribution to the active thread. T-034-02-03 will replace
shared-path existence with attempt-scoped publication and can pass the
publication lease through the same API without changing the boundary.

### Idle

After resolving the ticket, clone its active thread lease before requesting
completion. This works for both pane-scoped and legacy ticket-scoped idle files.

Idle signal attempt transport itself remains outside this ticket; later stale
signal work can make the caller stricter without changing admission.

### Stopped

`auto_complete_review` receives the stopped pane ID. Resolve the slot matching
that pane and ticket, then pass its lease. This avoids substituting the current
thread's lease for an event from a different physical pane.

### Manual

The modal action passes the selected active thread's lease when a thread exists.
A ticket with no active thread uses explicit Operator authority, preserving the
existing recovery control. An existing unleased thread is rejected.

### Observed Done

The reconciliation loop already iterates active threads. Snapshot their leases
with ticket IDs and pass those values to the request boundary.

## Pending completion identity

Add `authority: CompletionAuthority` to `PendingCompletion`.

Only validated authority can populate this field. It does not alter the T-031
result path. Attempt completion retains the exact lease and gives later
provenance work an unambiguous handoff point.

The pending record no longer derives `Copy` because `AttemptLease` owns its
ticket string. Result handling will clone the pending record instead of copying
it.

## Transaction preservation

No change is made to:

- `build_completion_command` arguments or context;
- `lisa complete-ticket`;
- the isolated alternate-index transaction;
- commit-ID validation;
- durable Done verification;
- failure retry behavior;
- successful phase/activity logging;
- Done provenance timing;
- slot release and thread removal ordering;
- dependent scheduling.

This directly preserves the T-031-01 path after lease admission.

## Test strategy

Add a focused boundary test that creates two leases for one ticket:

1. mint prior attempt N;
2. mint successor N+1;
3. install N+1 in `current_leases`;
4. call `request_completion` with N;
5. assert false, no pending entry, and a stale warning;
6. call the same method with N+1;
7. assert true and one pending entry carrying N+1.

Update completion fixtures to install one matching current lease and stamp the
thread/slot when they intend to model a real active attempt.

Retain the existing verified-success test as evidence that current admission
still proceeds through the unchanged result publisher after the isolated
transaction reports a commit ID.

Run focused stale/current tests, the plugin suite, workspace tests, formatting,
Clippy where clean against repository baseline, and the WASM target check.

## Final rationale

Completion authority is an event property compared with scheduler state. An
explicit source lease lets the boundary reject a prior attempt without ever
relabeling its event as current. It also composes with the next ticket: better
artifact and liveness attribution can feed stronger evidence into the same
completion gate while T-031's durable transaction remains untouched.
