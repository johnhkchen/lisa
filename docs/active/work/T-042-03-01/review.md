# Review: operator-requested authority emission

## Disposition

Ready to complete.
The acceptance criterion is satisfied by committed source and native regressions.
No critical issue remains within T-042-03-01 scope.

## Commit

Ticket-owned source was committed through Lisa's isolated transaction.

Commit:

`e8924986d1d123f86ad3ee68f6f7b10bc62b60fb`

Message:

`fix(plugin): emit explicit operator completion requests`

The commit contains exactly one repository path:

`crates/lisa-plugin/src/lib.rs`

No ordinary-index `git add` or `git commit` was used.
The ticket-owned source file has no remaining worktree diff.
Unrelated concurrent changes remain outside this ticket's commit.

## Summary of behavior

Dashboard `[d]one` is now an explicit operator command.
Its adapter event is `CompletionInput::OperatorRequested`.
Its authority is always `CompletionAuthority::Operator`.
Its reducer identity is the stable string `operator`.
The command no longer examines a thread's attempt lease.

The request carries `OperatorRequestSource::MarkDoneKey`.
That source flows into `CompletionSource::OperatorRequested`.
Pending completion state retains the exact source.
Tests can audit the source without parsing logs.

An active thread does not change the emitted authority.
An orphaned ticket does not prevent request emission.
Both cases use the same operator identity and source.

## Type-safety assessment

The previous Manual input accepted optional completion authority.
That shape permitted Attempt, Operator, or missing authority.
The same UI action therefore changed principal based on thread presence.

The new OperatorRequested input has only ticket ID and operator source.
It has no CompletionAuthority field.
It has no AttemptLease field.
An operator adapter event cannot represent attempt borrowing.

The executor accepts operator authority only for OperatorRequested sources.
The completion-result handler applies the same condition.
Scheduler sources remain restricted to current attempt authority.
Stale-attempt fencing remains unchanged.

## E-040 disposition assessment

Operator recovery now consumes the canonical Review disposition.
It does not admit or copy a private attempt artifact.
This is important when an active attempt still exists.
Operator authority remains distinct from artifact-attempt authority.

Canonical Pass allows the request to reach the reducer.
Canonical Block returns the existing typed DispositionBlocked rejection.
Invalid or missing disposition fails closed as DispositionBlocked.
The authored Block reason is retained in correlated activity.

Automatic attempt-driven completion still calls `admit_passing_review`.
That method retains exact-lease artifact admission.
It now shares canonical verdict evaluation with the operator path.
No E-040 parser or schema changed.

## Dependency assessment

The sole effect executor still calls `Dag::all_dependencies_done`.
Operator authority does not bypass this check.
A passing Review with an unfinished dependency is refused.
The refusal is the existing typed DependencyBlocked variant.
No pending completion is stored.
No command effect is recorded or launched.
Ticket frontmatter remains Review.

## Adapter-boundary assessment

All requests still enter `dispatch_completion`.
All returned effects still enter `execute_completion_effect`.
No second command launch boundary was added.
The pure lisa-core reducer contract remains unchanged.
No scheduler-wide rewrite was introduced.

The new disposition helper is plugin-local.
It separates canonical verdict evaluation from attempt artifact admission.
This permits operator evaluation without fabricating a lease.
The separation is narrow and named around the authority distinction.

## Files changed

### `crates/lisa-plugin/src/lib.rs`

Added `OperatorRequestSource::MarkDoneKey`.
Changed CompletionSource Manual to OperatorRequested with source payload.
Changed CompletionInput Manual to explicit OperatorRequested.
Extracted canonical `passing_review_disposition` evaluation.
Mapped OperatorRequested unconditionally to Operator authority.
Applied canonical disposition validation before reducer request dispatch.
Updated executor and result-handler authority/source guards.
Simplified `mark_ticket_done` to remove thread/lease lookup.
Updated active, orphaned, and retry regressions.
Added blocked-disposition and unmet-dependency regression coverage.

No source file was created or deleted.
No other crate source belongs to this ticket.

## Acceptance-criterion trace

### Pressing `[d]one` emits OperatorRequested

The active Review regression calls `handle_key` with bare `d`.
It confirms the mark-done modal opens.
It calls `handle_key` with Enter to select the Review.
The pending source is OperatorRequested(MarkDoneKey).

### Carries an auditable source

The event carries `OperatorRequestSource`.
The concrete source is `MarkDoneKey`.
The pending completion retains that exact enum value.

### Never CompletionAuthority::Attempt

The active fixture installs current attempt lease 1.
The test asserts pending authority is Operator.
It asserts completion identity differs from the installed attempt.
It asserts the emitted effect identity is `operator`.
The input type itself cannot carry Attempt authority.

### Fires with no live thread

The orphaned Review fixture has no thread or current lease.
Calling mark-done produces a pending operator completion.
It emits one operator-identity completion effect.

### Refused against blocked Review

The new refusal regression installs an active blocked Review.
Canonical disposition contains an actionable Block reason.
The request creates no pending completion and no effect.
Activity records correlated DispositionBlocked detail.
The installed attempt lease remains untouched.

### Refused against unmet dependencies

The same regression creates a passing Review with an unfinished dependency.
The request creates no pending completion and no effect.
Activity records correlated DependencyBlocked detail.
The Review frontmatter remains unchanged.

## Test coverage

Focused verification passed:

- `test_mark_done_keeps_thread_and_slot_until_commit_result`;
- `test_mark_done_without_active_attempt_uses_operator_authority`;
- `test_operator_requested_refuses_blocked_disposition_and_unmet_dependencies`;
- `failed_operator_completion_retries_without_early_release_or_duplicate_provenance`.

The focused filters reported 2, 1, and 1 passing tests respectively.
There were no focused failures.

The full lisa-plugin native suite passed:

- 359 passed;
- 0 failed;
- 0 ignored.

The full workspace suite passed.
It covered lisa-core, lisa-cli, lisa-plugin, integration tests, and doc tests.
No workspace failure was reported.

Formatting verification passed with:

`cargo fmt --all -- --check`

Production-target compilation passed with:

`cargo check -p lisa-plugin --target wasm32-wasip1`

## Regression-risk assessment

Risk is concentrated in manual completion gating and source matching.
The full plugin suite exercises automatic and operator completion paths.
Existing artifact, stopped, idle, observed-Done, and reconciliation tests pass.
Existing command failure/retry and provenance tests pass after source migration.

The stricter canonical disposition requirement is intentional.
Operator recovery cannot turn missing Review evidence into success.
Existing orphaned tests now provide explicit Pass evidence.
This aligns `[d]one` with E-040 rather than preserving the old bypass.

No new dependency affects WASM size.
No serialization format changed.
No CLI argument changed.
No public API changed.

## Open concerns and follow-on scope

The mark-done modal still closes immediately after confirmation.
Durable visible acceptance/rejection behavior belongs to T-042-03-02.
This ticket intentionally does not claim that story-level criterion.

The full seven-case operator matrix belongs to T-042-03-03.
This ticket covers active, orphaned, blocked, dependency, and retry essentials.
Already-pending, stale, and launch-failure presentation remain follow-on work.

Durable operator-source serialization was not added.
Story B owns completion journal durability and idempotency semantics.
The pending in-memory source is sufficient for this authority-emission ticket.

No live Zellij/Codex seat was used.
Story S-042-03 explicitly defines native adapter/UI tests as this story's boundary.
Story D owns the disposable live-seat field run.

## Working-tree audit

`git diff -- crates/lisa-plugin/src/lib.rs` is empty after the Lisa commit.
The commit path list contains only the plugin source file.

The worktree contains unrelated concurrent changes in Lisa metadata,
lisa-core, other ticket files, and shared work publication.
Those paths were not staged, committed, reverted, or otherwise modified here.
They do not block this ticket because the exact owned source path is clean.

## Final assessment

The implementation makes operator authority explicit by construction.
The active attempt lease can no longer be silently borrowed by `[d]one`.
The command works without a live thread.
Canonical E-040 disposition remains fail-closed.
Dependency completion remains mandatory.
Named correlated refusals are covered.
All requested verification is green.
The ticket is ready for Lisa's completion publication.
