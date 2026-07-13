# Plan: level-triggered eligibility reconciliation

## 1. Establish the clean ownership baseline

Inspect `git status --short` before editing source.

Record that Lisa-managed provenance/ticket changes and unrelated ticket work
already exist.

Confirm `crates/lisa-plugin/src/lib.rs` has no uncommitted pre-existing change.

Verification: the source file is clean relative to HEAD and can be owned
exactly by T-042-01-03.

## 2. Extend completion-domain imports

Import core reconciliation, durable inputs, current-lease admission, and
Reconciliation result types into the plugin.

Keep reducer and reconciler names distinct.

Verification: `cargo check -p lisa-plugin` reaches type checking with no unused
import once later steps are complete.

## 3. Add the typed reconciliation source and input

Add `CompletionSource::Reconcile`.

Add `CompletionInput::Reconcile { ticket_id, source_lease }`.

Keep all existing variants intact.

Verification: exhaustive matches identify every location that needs the new
variant; no wildcard hides an unhandled completion input.

## 4. Implement honest aggregate-state derivation

Add the private `completion_state` helper.

Map pending to Requested first.

Map durable DAG Done/status Done to Confirmed second.

Map all remaining states to Eligible.

Verification: focused test setup can independently observe Eligible,
Requested, and Confirmed decisions without inventing correlation state.

## 5. Implement current-attempt durable-input derivation

Add the private Review input builder.

Admit private `review.md` through the exact current lease.

Construct CurrentLeaseArtifactAdmission only after successful admission.

Admit and parse `review-disposition.json` from the same lease.

Represent missing/invalid evidence as fail-closed Invalid disposition.

Keep Block reason text intact.

Log filesystem/admission failures with ticket context.

Verification: Pass produces admission plus Pass; Block produces admission plus
Block; missing Review produces no admission; stale lease never creates
admission.

## 6. Route Reconcile through the sole typed dispatcher

Refactor `dispatch_completion` so Reconcile uses core `reconcile` and existing
inputs continue using core `reduce`.

Normalize both branches to one optional EffectCommand.

Keep exactly one `execute_completion_effect` call in dispatch.

Store Attempt authority and Reconcile source when an effect launches.

Return false for Reconciliation::None.

Log an actionable correlation if CommandInFlightActionRequired is encountered.

Verification: `completion_has_one_typed_request_gateway` still passes and
source inspection finds one completion host-command launch boundary.

## 7. Add the level-triggered candidate collector

Snapshot non-completed current leased threads whose thread or DAG phase is
Review/Done.

Dispatch Reconcile for each candidate.

Do not launch or admit outside typed dispatch.

Verification: calling the collector twice after the first accepted effect
leaves `launched_completion_effects.len()` at one because pending derives
Requested.

## 8. Wire the poll boundary

Call the collector after artifact and idle phase advancement.

Ensure it runs before Review timeout evaluation.

Retain existing ObservedDone handling later in the poll.

Verification: artifact-before-phase sequence reaches pending during the same
poll ordering and repeated poll reconciliation emits no duplicate effect.

## 9. Wire the load boundary

Call the same collector after the initial DAG has been stored.

Do not reconstruct or fabricate current lease state.

Document that a default fresh load is an authority-safe no-op.

Verification: a state with no current attempt emits no effect; the shared
collector remains usable by restored/test state with honest lease evidence.

## 10. Suppress false Review finish-up delivery

Add a predicate that returns true for pending completion.

Otherwise re-admit the current attempt's `review.md`.

Suppress a deadline action when either fact exists.

Perform suppression before follow-up construction and pane I/O.

Do not mutate `finish_up_sent`, activity clocks, or phase clocks for suppressed
actions.

Retain existing behavior when Review is genuinely absent.

Verification: existing timeout-positive test still sends one prompt; the new
admitted-Review and pending cases send none.

## 11. Add the acceptance regression

Create a temporary Implement ticket and current leased thread.

Write private Review and Pass disposition before advancing phase.

Set old activity/phase clocks to make a prompt otherwise eligible.

Drive artifact advancement and the poll reconciliation collector.

Assert one exact LaunchCompletion effect and one pending transaction.

Drive Review timeout and assert no finish-up event or sent marker.

Invoke collector again as reload reconciliation and assert exact-one effect.

Create blocked disposition coverage and assert zero effect.

Create confirmed durable Done coverage and assert zero effect.

Verification: the new test fails if reconciliation is removed, if pending or
confirmed suppression is removed, if Block becomes eligible, or if timeout
suppression is removed.

## 12. Run focused verification

Run the new test by exact name.

Run `completion_has_one_typed_request_gateway`.

Run `test_check_artifact_advances_review_to_done`.

Run disposition gating tests.

Run Review timeout tests.

Verification: all focused tests pass with exact effect counts and no ignored
failures.

## 13. Run package and workspace verification

Run:

```text
cargo fmt --all -- --check
cargo test -p lisa-plugin --lib --no-fail-fast
cargo clippy -p lisa-plugin --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
```

Run `just check` or the release WASM build if not already covered.

Verification: no warning, test, formatting, or WASM target failure remains.

## 14. Record progress before committing

Create `progress.md` in the attempt-private directory.

Document completed steps, deviations, exact tests, and remaining work.

Do not place progress in the shared work directory.

Verification: progress accurately reflects implementation state before the
isolated source transaction.

## 15. Commit the meaningful source unit

Use only:

```text
lisa commit-ticket --ticket-id T-042-01-03 \
  --message "fix(plugin): reconcile Review completion eligibility" \
  --include crates/lisa-plugin/src/lib.rs
```

Do not run ordinary `git add`, `git add -A`, or `git commit`.

Verification: the new commit contains exactly `crates/lisa-plugin/src/lib.rs`.

## 16. Verify repository hygiene

Inspect `git status --short` and the ordinary index.

Confirm the ticket-owned source file is clean.

Confirm unrelated pre-existing modifications/untracked paths remain untouched.

Confirm no attempt-private artifact was included in the source commit.

Verification: no ticket-owned source remains staged, modified, or untracked.

## 17. Review the implementation

Inspect the final committed diff.

Re-evaluate exact-one effect, lease fencing, Block fail-closed behavior,
pending/confirmed suppression, timeout suppression, and the load-boundary
limitation.

Document test coverage and any open concern in `review.md`.

Write exactly one valid `review-disposition.json` shape.

Use Pass only if all required work is complete and verification is green.

## 18. Stop on this ticket

After Review artifacts exist, do not edit ticket phase/status.

Do not publish the artifacts manually.

Do not start another ticket.

Wait for Lisa to admit artifacts and prepare the completion commit.
