# Implementation Progress — T-049-04-01

## Outcome

Implementation is complete.

Completion-commit failures now follow a conservative, durable, bounded policy:

- known operator-owned Git failures retry up to a fixed limit and then park;
- transient contention retries up to the same fixed limit without immediately parking;
- unrecognized failures park immediately with their raw reason marked unstructured;
- reconciliation deadline expiry parks instead of creating an unrecoverable completion state;
- an ordinary unpark returns the ticket to reconciliation eligibility;
- every failed attempt, its class, its ordinal, its bound, and its consequence are journaled.

The implementation remains scoped to the plugin completion boundary and its journal.

## Completed Work

### 1. Durable failure accounting

Updated `crates/lisa-plugin/src/completion_journal.rs`.

- Advanced the completion journal schema from version 2 to version 3.
- Preserved compatibility with schema versions 1 and 2.
- Added `CompletionFailureClass` with explicit variants for:
  - history or identity setup;
  - repository unwritability;
  - stale lock state;
  - transient contention;
  - unrecognized failure;
  - reconciliation deadline expiry.
- Added `FailureConsequence` with explicit variants for:
  - retry scheduled;
  - retry exhausted;
  - park.
- Added the `FailureObserved` journal transition.
- Persisted the complete technical failure reason.
- Persisted the classifier result.
- Persisted the failure ordinal.
- Persisted the fixed failure limit.
- Persisted the selected consequence.
- Folded failure count and exhaustion state into each aggregate.
- Reset failure accounting when a new completion generation is requested.

The journal fold rejects malformed or unsafe sequences:

- failure observations without a matching in-flight completion;
- observations carrying the wrong correlation identifier;
- empty failure reasons;
- a zero retry bound;
- a bound that changes within one completion generation;
- skipped or repeated failure ordinals;
- an ordinal greater than the bound;
- a scheduled retry at the bound;
- an exhausted consequence before the bound.

Marking a failure consequence as `Park` also marks the aggregate exhausted. This is
intentional crash safety: if publication is interrupted after the observation, the
same completion cannot be launched repeatedly.

### 2. Conservative classification and bounded policy

Updated `crates/lisa-plugin/src/lib.rs`.

- Defined one fixed completion-failure limit: two attempts.
- Added a pure, narrow classifier for known Git failure text.
- Classified only explicit, recognized phrases.
- Kept all unmatched text in the unrecognized class.
- Added a pure action selector driven by class and durable failure ordinal.
- Added plain operator asks for the known operator-owned classes.
- Used the ticket-required history/identity sentence verbatim.
- Kept the full technical envelope in the completion journal.
- Led operator-visible output with the plain ask before bracketed diagnostics.

The behavior matrix implemented is:

| Failure class | Before bound | At bound |
| --- | --- | --- |
| History or identity | Retry | Park with required ask |
| Repository unwritable | Retry | Park with repair ask |
| Stale lock | Retry | Park with lock ask |
| Transient contention | Retry | Wait for absolute deadline |
| Unrecognized | Park immediately | Not applicable |
| Deadline expired | Park immediately | Not applicable |

The transient exhausted state deliberately retains its existing in-flight journal
record and waits for the absolute reconciliation deadline. It launches no third
command, does not immediately park, and eventually reaches the common deadline
parking path if no success evidence arrives.

### 3. Common park path

Added a completion-specific park helper that reuses E-048 contracts.

The helper:

- identifies the pre-completion ticket phase;
- records `Rejected { ActionRequired }` with the complete technical reason;
- atomically publishes the canonical block disposition;
- publishes structured asks for recognized operator causes and deadlines;
- publishes raw unrecognized reasons without an `ask:` prefix;
- restores the ticket to its pre-completion phase;
- changes durable ticket status to `blocked`;
- records E-048 `Park` provenance;
- includes failure ordinal and bound in the provenance reason when available;
- removes pending host state;
- releases the scheduler slot;
- removes the running thread;
- rebuilds the dependency graph;
- emits the forwardable ask first on operator surfaces.

The raw unrecognized publication is parsed by the existing disposition parser as
`unstructured: true`; the implementation does not invent an explanation or repair.

### 4. Replay and restart behavior

Generalized completion replay so bounded retries preserve the original request.

- The ticket, attempt, source, authority, generation, and deadline are retained.
- A retry reuses the same completion generation and absolute deadline.
- A retry cannot start after the durable aggregate is exhausted.
- Reconciliation suppresses replay when exhaustion has already been journaled.
- Restart therefore cannot reset the failure counter or exceed the bound.
- The initial request and reconciliation replay remain distinguishable in memory.

Compatibility behavior for older tests that intentionally omit the completion
journal remains isolated: those fixtures retain their historic manual-retry surface.
Production completion requests always create a journal aggregate before launch.

### 5. Deadline recovery

Replaced the former deadline dead end with the common park path.

- An expired in-flight completion no longer ends solely in an unrecoverable
  `Rejected { ActionRequired }` aggregate.
- Deadline expiry publishes a structured, plain operator ask.
- The ticket is restored to its prior phase with `status: blocked`.
- E-048 park provenance is recorded.
- The existing ordinary unpark transition can change status back to `open`.
- Reconciliation regards the action-required aggregate plus open ticket as eligible.
- A replacement lease can install a later attempt and start a new generation.
- No manual journal edit or special reset command is required.

The completion masking logic was narrowed accordingly: it masks aggregate state only
when the durable ticket bytes are actually Done, and never overwrites a durable
Blocked status.

### 6. Test coverage

Added journal unit coverage for:

- durable bounded failure observations;
- aggregate restoration after reopening the journal;
- exact count and limit retention;
- exhaustion retention across restart;
- skipped ordinal rejection;
- changed limit rejection;
- over-bound rejection.

Added plugin fixture coverage for:

- conservative classification;
- the exact history/identity ask;
- unborn/history failure bounded retry then park;
- identity failure mapping;
- structured block disposition parsing;
- park provenance count and limit;
- ordinary unpark provenance;
- transient contention launching exactly two commands;
- transient exhaustion without immediate park;
- unrecognized immediate park;
- raw reason preservation;
- unstructured disposition parsing;
- deadline parking;
- status restoration to Review/Blocked;
- lease fencing after deadline;
- ordinary unpark restoring reconciliation eligibility;
- a later attempt starting a fresh completion generation.

Existing completion, reconciliation, scheduling, unpark, and hostile-order regression
tests continue to pass.

## Plan Deviations

Two small implementation refinements were made after exercising the initial design.

First, exhausted transient contention does not immediately create a rejected journal
terminal. It remains in-flight until the already-recorded absolute deadline, while
durable exhaustion prevents another launch. This satisfies both requirements: there
is no immediate park for transient contention, and there is no unbounded retry loop.

Second, completion masking now consults the durable ticket state before projecting
Done. The previous unconditional projection would have hidden the Blocked state that
the new common park path intentionally publishes.

Neither refinement changes ticket scope or introduces a new public interface.

## Verification

Completed successfully:

- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo check -p lisa-plugin --target wasm32-wasip1`
- `cargo test -p lisa-plugin --lib`
  - 415 passed
- `cargo test --workspace --quiet`
  - all test binaries passed
  - one existing real-Zellij integration remained ignored by design
- `just check`
  - passed before the final warning-only cleanup

The final workspace test run followed the warning-only cleanup and passed.

## Remaining Work

- Commit the two ticket-owned source files through `lisa commit-ticket` with exact
  repository-relative include paths.
- Perform the Review phase and write `review.md`.
- Write `review-disposition.json`.
- Remain on this ticket and wait for Lisa's completion commit confirmation.

No implementation work remains.
