# Structure: operator-requested authority emission

## Files modified

### `crates/lisa-plugin/src/lib.rs`

This is the only ticket-owned source file.
It contains the adapter input, authority, source, keyboard path, and native tests.
No file is created or deleted in the source tree.

### Attempt-private phase artifacts

`research.md`, `design.md`, `structure.md`, `plan.md`, `progress.md`,
`review.md`, and `review-disposition.json` live under the assigned attempt work path.
Lisa publishes admitted artifacts later.
They are not included in the source commit.

## Type-level changes

Add a private enum adjacent to `CompletionSource`:

```rust
enum OperatorRequestSource {
    MarkDoneKey,
}
```

Derive Debug, Clone, Copy, PartialEq, and Eq.
The type identifies the human-facing surface that requested completion.

Change `CompletionSource::Manual` to:

```rust
CompletionSource::OperatorRequested(OperatorRequestSource)
```

This source is stored in `PendingCompletion` unchanged.
Other completion source variants remain unchanged.

Change `CompletionInput::Manual` from ticket plus optional authority to:

```rust
CompletionInput::OperatorRequested {
    ticket_id: TicketId,
    source: OperatorRequestSource,
}
```

The input cannot contain an AttemptLease or CompletionAuthority.
The impossible combination is removed from the adapter vocabulary.

`CompletionAuthority` itself remains unchanged.
Attempt-driven paths continue to use `Attempt(AttemptLease)`.
The operator path continues to use `Operator` internally and in pending state.

## Disposition helper boundary

Add a helper on `State` near `admit_passing_review`:

```rust
fn passing_review_disposition(
    &self,
    ticket_id: &str,
) -> Result<(), CompletionRejection>
```

The helper reads the canonical work directory only.
It delegates parsing to `parse_review_disposition`.
Pass succeeds.
Block maps to `DispositionBlocked` with its reason.
Invalid maps to `DispositionBlocked` with invalid context.

Refactor `admit_passing_review` to retain admission logic first.
After successful admission it calls `passing_review_disposition`.
No attempt semantics or artifact publication rules change.

## Adapter dispatch organization

In `dispatch_completion`, handle OperatorRequested in the non-reconcile match.
Construct:

- `CompletionSource::OperatorRequested(source)`;
- `Some(CompletionAuthority::Operator)`;
- no review lease.

Before calling the reducer, branch on the normalized source.
For OperatorRequested, call `passing_review_disposition`.
On error, emit the existing correlated rejection and return false.
For all other sources, preserve existing admission behavior.

The reducer event remains `CompletionEvent::Request`.
Its AttemptId remains the stable string `operator` for operator authority.
Its CompletionId remains the ticket ID.
No lisa-core interface changes.

## Effect-executor organization

Change the operator authority admission guard from Manual to OperatorRequested.
The source pattern must accept any `OperatorRequestSource` variant.
Attempt authority current-lease validation stays unchanged.

Change completion-result authority validation similarly.
An Operator pending result is valid only when its pending source is OperatorRequested.
Attempt result validation remains current-lease based.

Dependency validation remains after authority and duplicate checks.
The existing `Dag::all_dependencies_done` call is not moved or bypassed.
No pending record is created before this validation succeeds.

## Keyboard command organization

Simplify `mark_ticket_done`.
Remove all thread lookup and lease extraction.
Dispatch exactly:

```rust
CompletionInput::OperatorRequested {
    ticket_id: ticket_id.to_string(),
    source: OperatorRequestSource::MarkDoneKey,
}
```

`handle_key` and `open_mark_done_modal` remain structurally unchanged.
This ensures active and orphaned selections invoke identical adapter inputs.

## Test changes

### Active Review test

Update `test_mark_done_keeps_thread_and_slot_until_commit_result`.
Use Review phase because `[d]one` active-thread recovery is a Review operation.
Create canonical passing disposition evidence.
Keep an installed active attempt to prove it is ignored as authority.
Assert Operator pending authority and MarkDoneKey source.
Assert the only effect uses AttemptId `operator`.
Retain thread and slot assertions.

### Orphaned Review test

Update `test_mark_done_without_active_attempt_uses_operator_authority`.
Create canonical passing disposition evidence.
Assert MarkDoneKey source in addition to Operator authority.
Retain no-thread behavior and stable effect identity checks.

### Gate regression

Add `test_operator_requested_refuses_blocked_disposition_and_unmet_dependencies`.
Create a blocked Review with a canonical block disposition.
Include an active thread and current attempt to exercise lease independence.
Assert no pending completion and no launched effect.
Assert a DispositionBlocked activity event includes the authored reason.

Create an unfinished dependency and a second Review depending on it.
Give the second Review a canonical pass disposition.
Invoke mark-done without a live thread.
Assert no pending completion and no launched effect.
Assert a DependencyBlocked activity event is correlated to the second ticket.

## Interface invariants

`CompletionInput::OperatorRequested` always means operator authority.
`OperatorRequestSource::MarkDoneKey` always means the dashboard `[d]one` surface.
No operator input can borrow or carry an AttemptLease.
No attempt-driven input can claim an operator source.
Every completion effect still crosses `execute_completion_effect`.
Every operator request must pass canonical disposition validation.
Every accepted reducer effect must pass dependency validation.

## Implementation ordering

First add and replace the private enums.
Second extract canonical disposition evaluation.
Third update adapter normalization and executor/result guards.
Fourth simplify the mark-done constructor.
Fifth update focused tests and add refusal coverage.
Sixth format and run targeted tests.
Seventh run workspace verification as time permits.
Eighth commit only `crates/lisa-plugin/src/lib.rs` with Lisa's transaction.

## Non-changes

No changes to `crates/lisa-core/src/completion.rs`.
No changes to `crates/lisa-plugin/src/ui.rs`.
No changes to ticket frontmatter.
No changes to completion journal schema.
No changes to complete-ticket CLI arguments.
No changes to modal close timing.
No changes to thread or slot release timing.
No changes to dependency graph semantics.
