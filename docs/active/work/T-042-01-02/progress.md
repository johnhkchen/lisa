# Progress: fold all completion sources

## Status

Implementation is complete, verified, and committed through Lisa's isolated
transaction as `b8fca3313419c9d4f105d3af7386d18382562fdc`.

## Completed: typed input vocabulary

Extended private `CompletionInput` with source-specific variants for:

- Idle with a required attempt lease;
- ObservedDone with the reconciled thread's optional attempt lease; and
- Manual with the UI-selected optional attempt/operator authority.

Artifact and Stopped remain required-lease variants. Together the enum now
covers every existing production completion request origin.

## Completed: unified reducer dispatch

Refactored `dispatch_completion` to normalize each variant into ticket ID,
diagnostic CompletionSource, authority, and optional Review lease.

Artifact, Stopped, and Idle retain passing Review disposition admission inside
the adapter. ObservedDone and Manual preserve their prior no-admission behavior.

Every variant now constructs `CompletionEvent::Request`, calls the E-041 pure
reducer, and passes only a reducer-returned effect to
`execute_completion_effect`.

Attempt identities remain exact lease generations. Operator uses the stable
adapter value `operator`; malformed missing-authority cases traverse the typed
event seam and fail closed in the existing executor authority gate.

## Completed: idle routing

Both idle completion branches now dispatch `CompletionInput::Idle`:

- Implement advances to Review and catches an already-written review in the
  same signal cycle;
- Review with an admitted artifact advances toward Done.

Both branches reject a missing lease with a visible Idle-specific warning.
Callers no longer choose CompletionSource or invoke a Review boolean wrapper.

The existing Implement/Review catch-up regression now asserts PendingCompletion
source Idle and the exact reducer-produced LaunchCompletion effect.

## Completed: timeout/reload and observed-Done reconciliation

The post-timeout, post-DAG-rebuild running-thread scan in `poll_tick` now
dispatches `CompletionInput::ObservedDone`.

Its comment names the timeout/reload reconciliation boundary explicitly. The
pending mask and optional lease behavior remain intact.

The stale/current lease regression now drives this typed input, proves the
stale lease is rejected, and asserts the current lease produces exactly one
effect with matching AttemptId and CompletionId.

Split-brain and authoritative provenance tests were migrated from the deleted
wrapper to ObservedDone without weakening fencing or exact-winner assertions.

## Completed: manual UI routing

`mark_ticket_done` retains its authority selection and dispatches
`CompletionInput::Manual`.

The active-attempt test now asserts the exact attempt-bound launch effect. The
unassigned operator test asserts Manual diagnostic source, Operator authority,
and exact operator-bound launch effect.

Existing failure/retry tests continue to prove no early release or duplicate
Done provenance.

## Completed: boolean path removal

Deleted `request_review_completion`.

Deleted `request_completion`.

Production source now contains exactly one
`self.execute_completion_effect(...)` call and it is inside
`dispatch_completion`. The effect executor still contains the only completion
host command launch.

## Completed: architectural regression

Added `completion_has_one_typed_request_gateway`.

It reads the production portion of `lib.rs` and fails if either deleted legacy
method declaration returns, if another production executor call appears
outside dispatch, or if the executor gains a second host command launch.

This makes `cargo test` red for the acceptance criterion's second boolean
completion path, even if that path is not exercised by an existing behavioral
fixture.

## Verification

Focused completion filter passed:

- 10 passed;
- 0 failed.

This included the one-gateway invariant, typed stale/current reconciliation,
artifact completion, nested command/transaction regression, and manual retry.

Focused remaining-source tests passed independently:

- `test_idle_signal_implement_with_review_artifact_advances_to_done`;
- `test_mark_done_keeps_thread_and_slot_until_commit_result`;
- `test_mark_done_without_active_attempt_uses_operator_authority`.

Full plugin suite passed:

- 345 passed;
- 0 failed.

Full workspace passed:

- lisa-cli library: 14 passed;
- lisa-cli binary: 267 passed;
- CLI integration targets passed, with the real-Zellij environment test
  remaining ignored under its declared requirement;
- lisa-core: 194 passed;
- core generated and recorded completion integrations: 2 passed;
- lisa-plugin: 345 passed;
- doc tests passed.

Quality gates passed:

- `cargo fmt --all -- --check`;
- `cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`;
- `git diff --check`.

## Deviation: concurrent missing dependency edge

T-042-02-01 is concurrently active and also plans to modify
`crates/lisa-plugin/src/lib.rs`, despite no dependency edge between the two
tickets. That ticket noticed the collision, reversed/isolated its plugin hunks,
and is waiting for this exact-path transaction before reapplying them.

Its uncommitted core and CLI changes made `CompleteTicketRequest` temporarily
require a completion key. To run the connected plugin and workspace suites, a
temporary test-only request field was added, all verification above was run,
and that temporary field was then removed before this ticket's diff inspection
and commit. It is not part of this ticket's source unit.

The initial `cargo test -p lisa-plugin --no-run` passed against the pre-change
CLI contract after the typed production routing and legacy test migrations.
New assertions added afterward do not change that request construction.

No concurrent core, CLI, harness, ticket, provenance, or work-artifact path is
included in this ticket's transaction.

## Repository ownership

Ticket-owned source path:

- `crates/lisa-plugin/src/lib.rs`.

The ordinary Git index is empty. Lisa-managed provenance/ticket files,
T-042-02-01's in-progress source and work files, the provider harness change,
and the pre-existing untracked `crates/lisa-plugin/docs/` tree remain outside
this ticket.

## Source transaction

Executed:

```text
target/debug/lisa commit-ticket \
  --ticket-id T-042-01-02 \
  --message "refactor(plugin): route all completion sources through typed adapter" \
  --include crates/lisa-plugin/src/lib.rs
```

Returned commit:

`b8fca3313419c9d4f105d3af7386d18382562fdc`

`git show` confirms the commit contains exactly
`crates/lisa-plugin/src/lib.rs`. Immediately after the transaction, that path
was neither staged, modified, nor untracked, and the ordinary index was empty.

## Remaining

Write Review artifacts and remain on this ticket for Lisa completion.
