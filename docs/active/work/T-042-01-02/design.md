# Design: one typed completion gateway

## Goals

Route every currently existing completion request origin through
`dispatch_completion`, ensure each origin constructs a typed core Request
event, and delete both temporary boolean request wrappers. Preserve Review
admission, lease fencing, operator restrictions, dependency gating, command
construction, pending-state masking, and result handling.

Add a regression that fails if the legacy boolean request path or a second
direct executor route appears again. Keep the change local to the plugin and
leave the E-041 core reducer unchanged.

## Option 1: one generic input struct

A struct containing ticket ID, source, optional authority, and an
`admit_review` flag would minimize matching code. Existing callers could fill
the fields and dispatch it.

This is rejected because it reproduces the old untyped function arguments in
a wrapper object. Invalid combinations such as Stopped plus operator authority
or Artifact without Review admission would remain constructible. It would not
make each source's evidence explicit, which is the point of folding sources
into typed events.

## Option 2: source-specific enum variants

Extend `CompletionInput` with Idle, ObservedDone, and Manual variants. Artifact,
Stopped, and Idle carry a required attempt lease. ObservedDone carries the
optional lease found on the reconciled running thread, preserving the current
missing-authority rejection. Manual carries the authority selected by the UI,
also preserving the distinction between a leased active thread and an
unassigned operator action.

The dispatcher exhaustively maps each variant to ticket, diagnostic source,
authority, and admission policy. It constructs the common core Request event,
reduces it, and alone invokes the effect executor. This is the chosen option.

The existing externally observed Done loop is the current poll reconciliation
entry point after timeout processing and DAG reload. Therefore ObservedDone is
the typed input for both named aspects of the existing path. T-042-01-03 can add
a distinct level-triggered passing-Review reconciliation input if its new
policy needs different evidence; this ticket does not pre-implement that
dependent behavior.

## Option 3: replace plugin pending state with the full aggregate

The plugin could persist `CompletionState` per ticket and route requests and
command callbacks through every core event variant. This would remove the
bridge between `pending_completions` and Eligible/Requested.

This is rejected for this ticket. It overlaps correlation rendering,
level-triggered reload behavior, and later durable journaling. It would expand
a caller-folding change into result-state architecture while the story's DAG
deliberately separates those concerns.

## Typed input shapes

Artifact remains `{ ticket_id, source_lease }`.

Stopped remains `{ ticket_id, pane_id, source_lease }`.

Idle becomes `{ ticket_id, source_lease }`. Both idle catch-up branches already
have the current thread lease available. Making it required moves missing-lease
rejection to the caller, matching artifact and stopped behavior.

ObservedDone becomes `{ ticket_id, source_lease: Option<AttemptLease> }`. The
poll snapshot currently contains an optional lease. Retaining optionality lets
typed dispatch construct the event and then lets the existing executor emit
the established authority rejection without changing scheduler cleanup.

Manual becomes `{ ticket_id, authority: Option<CompletionAuthority> }`. The UI
selection rule remains unchanged: a ticket with a thread uses its optional
lease; a ticket without a thread uses Operator. The optional form preserves the
fail-closed state for an inconsistent active thread lacking a lease.

These variants are private adapter vocabulary. They do not change public API
or serialized formats.

## Admission policy

Artifact, Stopped, and Idle are Review-evidence origins. Dispatch calls
`admit_passing_review` before reducer invocation. The required lease is passed
to artifact publication and disposition validation.

ObservedDone does not repeat Review admission because it represents durable
Done already read from the ticket file. Manual retains existing UI semantics
and does not acquire an implicit Review-disposition requirement in this
ticket. Broader manual authority policy remains Story C.

Encoding admission policy in the exhaustive dispatcher match keeps callers
from selecting it independently. No boolean `request_review_completion`
wrapper remains.

## Authority and event identity

For attempt authority, Request AttemptId is the decimal lease generation. For
operator authority, it is the stable adapter identity `operator`. For missing
authority, it is a diagnostic placeholder `missing-authority`; the event still
traverses the reducer, but the executor's existing authority check rejects the
effect before mutation.

CompletionId remains the ticket ID, matching the current one-pending-request
per-ticket bridge. The executor revalidates effect identity against ticket and
authority. Attempt leases must still be exact current leases. Operator remains
valid only for CompletionSource::Manual.

## Return type

`dispatch_completion` currently returns bool, primarily to support tests and
caller ignorance. The acceptance criterion targets the old
`request_completion` boolean path. Keeping a return value on the sole typed
gateway is not a second completion path; callers may discard it and tests can
assert acceptance.

Both `request_review_completion` and `request_completion` will be deleted.
There will be no function named `request_completion` and no wrapper that
fabricates LaunchCompletion outside the reducer.

`execute_completion_effect` retains bool because it is the single effect
execution boundary and existing focused tests exercise its fail-closed gates.
It is not a request entry point: only a reducer-returned effect may reach it.

## Production routing

The two idle branches replace `request_review_completion` with
`CompletionInput::Idle`. A missing lease logs a source-specific warning and
does not dispatch, consistent with Artifact and Stopped.

The post-timeout/post-rebuild poll reconciliation loop replaces the direct
request wrapper with `CompletionInput::ObservedDone`. It retains the pending
mask and optional lease behavior inside the executor.

`mark_ticket_done` preserves its authority selection and dispatches
`CompletionInput::Manual`.

Artifact and Stopped remain unchanged except for shared exhaustive matching.

## Structural invariant test

Behavioral source tests will assert that Idle, ObservedDone reconciliation, and
Manual each produce the expected recorded effect through the same dispatcher.
They will also retain duplicate suppression expectations.

A separate source-shape test will use `include_str!("lib.rs")` and inspect the
production prefix before the test module. It will assert:

- no `fn request_completion` exists;
- no `fn request_review_completion` exists;
- exactly one call expression to `self.execute_completion_effect(` exists;
- that call is inside the typed dispatcher's source range;
- exactly one completion command runner call remains in the executor.

Counting the production prefix avoids matching test-only direct executor calls
used to validate authority failures. Requiring the executor call to occur
inside the dispatcher makes a newly introduced boolean bypass fail cargo test
even if existing behavioral outcomes still pass.

Pure textual checks are intentionally narrow and anchored to stable function
names. A full Rust parser dependency would be disproportionate for a private
single-file invariant.

## Test fixture strategy

Reuse existing temporary State helpers, `install_current_attempt`, passing
Review disposition writers, and `launched_completion_effects`.

Idle is best covered by an existing idle signal phase test enhanced to assert
the exact effect and diagnostic source. ObservedDone needs a running thread
whose disk ticket is Done and whose in-memory DAG begins before reconciliation;
driving `poll_tick` may invoke unrelated scheduler behavior, so a focused
dispatcher test can establish adapter mapping while an existing poll behavior
test pins the caller.

Manual uses `mark_ticket_done`, because the acceptance is about the UI entry
point rather than direct enum construction. It should assert one recorded
effect and `PendingCompletion.source == Manual`.

Existing workspace tests cover commit results, lease fencing, disposition
blocking, dependencies, nested path construction, and the real transaction.

## Compatibility and risk

The largest risk is changing admission for ObservedDone or Manual. The chosen
mapping explicitly preserves their current lack of Review admission.

The next risk is accepting a missing or stale authority earlier than before.
Optional authority remains representable for the two legacy cases, and the
executor retains exact current-lease validation.

The final risk is a brittle invariant. Restricting it to production text,
legacy method names, a stable executor identifier, and call-site containment
keeps it aligned with the architectural requirement. Refactors that rename the
gateway will need to update the test deliberately, which is appropriate for a
one-gateway invariant.

## Commit boundary

All production and test changes are one meaningful source unit in
`crates/lisa-plugin/src/lib.rs`. Commit it through `lisa commit-ticket` with
that exact include path. Do not include Lisa-managed ticket/provenance files or
the pre-existing untracked plugin docs tree.
