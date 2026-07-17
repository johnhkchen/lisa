# Review — T-049-04-01

## Disposition

Pass.

The implementation satisfies the ticket acceptance criteria and is ready for Lisa's
completion transaction.

## Reviewed Commit

- Commit: `f63839706c040bc26c38786cdb07d6dfa5fc5ea1`
- Subject: `fix(plugin): bound and park completion failures`
- Ticket-owned paths:
  - `crates/lisa-plugin/src/completion_journal.rs`
  - `crates/lisa-plugin/src/lib.rs`

The commit was created with `lisa commit-ticket` and exact include paths.

The ordinary Git index is empty. Both ticket-owned source files are clean after the
commit. Existing unrelated modifications in the shared worktree were neither staged
nor included.

## Change Summary

The plugin now enforces a fixed two-attempt limit at the completion-commit boundary.

Failure handling is driven by a conservative classifier:

- explicit unborn-history or missing-identity failures retry and then park;
- explicit repository-write failures retry and then park;
- explicitly stale lock failures retry and then park;
- transient contention retries within the bound and then waits for the existing
  absolute reconciliation deadline without launching again;
- unrecognized failures park immediately with their raw reason;
- deadline expiry uses the same park machinery instead of dead-ending.

The completion journal records each command failure before its scheduler consequence.
Each observation contains the complete technical reason, class, failure number,
fixed bound, and chosen consequence. Reopening the journal reconstructs the counter
and exhausted state, so process restart cannot reset the retry budget.

Parking reuses the E-048 contracts:

- canonical block disposition publication;
- restoration of the prior phase;
- durable `blocked` status;
- operator remedy ownership for structured cases;
- Park provenance;
- ordinary Unpark provenance and recovery.

## Acceptance-Criteria Review

### Failure classes map to bounded behavior

Pass.

History and identity classification recognizes the known Git messages narrowly. The
operator-facing ask is exactly:

> Lisa needs a name for recording finished work. Run: `git config user.name "You"`
> and `git config user.email you@example.com` — or rerun `lisa init` and accept the
> history offer.

Known operator failures select retry while the durable failure count is below two,
then select park at two. Tests prove two failure observations and no unbounded launch.

Transient contention selects retry below two and `WaitForDeadline` at two. The test
proves exactly two launch effects, no immediate rejected terminal, no park provenance,
and an exhausted in-flight aggregate that cannot launch a third command.

Unrecognized errors select park on their first observation. Their canonical block
disposition omits structured `ask` and remedy-owner fields. The existing parser marks
that form unstructured, and the raw complete reason becomes the visible ask.

Park and unpark rows use E-048's existing `ParkingTransitionType` and provenance
schema. Tests assert both transitions and their retry-count metadata.

### Deadline expiry is recoverable through ordinary unpark

Pass.

The deadline handler now calls the common completion park helper. It journals the
action-required rejection, publishes a structured operator disposition, restores the
ticket's prior phase, and writes `status: blocked`.

The reconciliation state treats an action-required completion whose durable ticket
has returned to `open` as eligible. The ordinary unpark path is therefore sufficient:
no journal edits and no special completion-reset command are needed.

The fixture proves the complete sequence:

1. an in-flight completion reaches its absolute deadline;
2. the ticket returns to Review/Blocked;
3. a Park provenance row lands;
4. the expired lease cannot relaunch completion;
5. ordinary unpark writes Open and Unpark provenance;
6. reconciliation becomes eligible;
7. a replacement attempt installs a new lease;
8. the later attempt starts a fresh completion generation.

### Every attempt and bound are visible and enforced

Pass.

`FailureObserved` is appended for every failed command result before retry, wait, or
park is performed. The journal fold requires exact monotonic ordinals and a stable,
positive bound. It rejects over-bound counts and retry scheduling at exhaustion.

The scheduler consults the folded aggregate before reconciliation replay. Once a
failure consequence records exhaustion or park, replay is suppressed. Because the
state is reconstructed from disk, this enforcement holds across plugin restart.

The common park path follows a `FailureObserved { consequence: Park }` observation.
That observation itself marks retries exhausted, preventing churn even if a later
publication step is interrupted.

No normal journal-backed path can reject and relaunch the same completion more than
the fixed bound.

## Correctness Review

### Classification is conservative

Pass.

The classifier only uses explicit lowercased phrases. It does not use broad matches
such as any occurrence of `fatal`, `lock`, or `git`. A lock is considered stale only
when stale/dead-process evidence is present; a conventional `index.lock` plus another
Git process is treated as transient. Everything else remains unrecognized and parks
without a guessed repair.

### Operator text and technical detail are separated

Pass.

The journal receives the full envelope: ticket, authority, completion source, exit
status, and stderr. For structured causes, the activity surface leads with the plain
ask and appends the technical envelope in brackets. For unrecognized causes the same
raw envelope is retained as the unstructured ask.

### Retry identity is stable

Pass.

Retry replay preserves the same completion key, correlation, source, authority,
generation, and absolute deadline. It does not manufacture a new attempt or extend
the reconciliation window. A subsequent operator recovery uses a legitimately newer
attempt and a new completion generation.

### Crash and restart behavior is bounded

Pass.

The durable observation precedes its side effect. A crash before observation leaves
the command in-flight for reconciliation. A crash after observation preserves the
consumed count. An exhausted or park observation prevents another launch. Journal
unit tests close and reopen the file to verify that state.

### Durable ticket state is not masked

Pass.

Completion masking now projects Done only when scanning the ticket bytes confirms
Done, and it does not overwrite Blocked. This is required for the parking state to be
visible to the scheduler and operator. Existing successful completion tests continue
to pass, providing regression coverage for the narrower mask.

### Compatibility surface

Pass.

A small compatibility branch remains for tests or legacy state with a pending native
completion but no aggregate. It preserves the previous retryable rejection behavior.
Production requests create their journal aggregate before command launch, so bounded
durable handling is the operative path.

## Test Review

### New journal tests

- bounded observations are durable and restart-safe;
- failure count and limit fold correctly;
- exhaustion survives reopen;
- skipped counts are rejected;
- changing a bound is rejected;
- exceeding a bound is rejected.

### New plugin tests

- conservative classification and plain asks;
- history/identity bounded retry then park;
- structured disposition parsing;
- Park and Unpark provenance;
- transient bounded launches without immediate park;
- unrecognized raw unstructured park;
- deadline park and ordinary-unpark recovery.

### Regression verification

The following commands passed:

- `cargo fmt --all -- --check`
- `git diff --check`
- `cargo check -p lisa-plugin --target wasm32-wasip1`
- `cargo test -p lisa-plugin --lib` — 415 passed
- `cargo test --workspace --quiet` — all executed tests passed
- `just check` — passed before warning-only cleanup

The final full workspace suite ran after the cleanup and passed. One real-Zellij test
remains ignored under its existing environment-dependent annotation.

## Scope Review

Pass.

The source changes are limited to the completion journal and plugin coordinator.
There are no CLI changes, new dependencies, public protocol changes, ticket
frontmatter edits, or shared-work-artifact edits in the ticket commit.

Schema version 3 remains backward compatible with both prior accepted journal schema
versions. No migration command is required.

## Open Concerns

No blocking concerns.

Two operational nuances are intentional and covered:

- At the transient limit, the aggregate stays in-flight until its original deadline
  rather than parking immediately. Durable exhaustion prevents a third launch.
- The exact history/identity ask required by the ticket does not append an unblock
  command. This preserves the mandated sentence; ordinary unpark remains the recovery
  mechanism after the operator repairs repository setup.

A rare filesystem failure after journaling `ActionRequired` but before canonical
disposition or ticket publication will be logged and will not relaunch the exhausted
generation. The operator-request reconciliation path treats that state as eligible,
so it is not an unrecoverable dead end. The normal atomic-publication path is covered
by the integration fixtures.

## Final Assessment

The incident's unbounded retry loop is closed at the durable completion boundary.
Known operator problems converge on a forwardable ask, transient problems remain
bounded without premature parking, unknown problems are never guessed, and deadline
expiry is recoverable through the ordinary E-048 park/unpark workflow.

Disposition: pass.
