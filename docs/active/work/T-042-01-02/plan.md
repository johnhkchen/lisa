# Plan: fold completion sources into one adapter

## 1. Expand typed source vocabulary

Add Idle, ObservedDone, and Manual variants to `CompletionInput` with
source-specific evidence. Preserve the existing Artifact and Stopped variants.

Compile the plugin tests to catch enum exhaustiveness and visibility issues.

Verification: `cargo test -p lisa-plugin --no-run`.

## 2. Normalize all inputs in dispatch

Refactor `dispatch_completion` to derive ticket ID, CompletionSource,
authority, and optional Review lease from one exhaustive match.

Keep passing Review admission for Artifact, Stopped, and Idle. Skip Review
admission for ObservedDone and Manual to preserve current behavior.

Derive AttemptId from attempt authority, operator authority, or a missing
authority placeholder. Construct one CompletionEvent::Request and invoke the
pure reducer for every variant.

Pass only the reducer-returned effect to `execute_completion_effect`.

Verification: focused existing artifact/stopped adapter test.

## 3. Route idle through typed dispatch

Replace both `request_review_completion` calls in `check_idle_signals` with
`CompletionInput::Idle` dispatches.

For an absent lease, log a source-specific warning and do not dispatch. Avoid
re-admitting Review at the call site; the dispatcher owns admission.

Verification: existing idle phase advancement and Review completion tests,
plus exact recorded-effect assertions.

## 4. Route observed Done reconciliation

Replace the `poll_tick` direct request wrapper call with
`CompletionInput::ObservedDone`. Preserve the current optional thread lease,
post-timeout/post-DAG-rebuild ordering, and pending mask.

Update the comment only where needed to describe the typed reconciliation
boundary.

Verification: existing externally Done and scheduler reconciliation tests,
plus a focused exact-effect assertion.

## 5. Route manual UI completion

Keep `mark_ticket_done` authority selection unchanged. Dispatch
`CompletionInput::Manual` with the selected optional authority.

Drive the real `mark_ticket_done` method in a focused test and assert the exact
effect, source, and authority in pending state.

Verification: existing mark-done modal and dependency tests.

## 6. Delete boolean bridges

Remove `request_review_completion`.

Remove `request_completion`.

Search production source for both names and for all direct
`execute_completion_effect` callers. The sole production call must be inside
`dispatch_completion`.

Verification:

```text
rg -n "request_review_completion|request_completion|execute_completion_effect"
  crates/lisa-plugin/src/lib.rs
```

## 7. Migrate direct legacy tests

For each direct test call to the deleted wrapper, identify what boundary the
test actually covers.

Use `CompletionInput::ObservedDone` when the test needs the full typed gateway
without Review admission. Use Artifact/Idle only when passing Review evidence
is present. Keep direct executor calls solely for tests of effect identity or
executor-specific validation.

Do not weaken stale-lease, split-brain, pending, or authoritative provenance
assertions.

Verification: run each affected test by exact name while editing.

## 8. Add remaining-source behavioral coverage

Add or enhance tests proving:

- Idle emits one LaunchCompletion with exact current AttemptId and ticket
  CompletionId;
- externally observed Done/reconciliation emits the same typed effect shape;
- manual UI emits one effect with operator or exact lease authority;
- duplicate requests remain one-effect-only.

Assert `PendingCompletion.source` so diagnostic origin mapping is pinned.

Verification: a shared test name filter such as `completion` plus exact new
test names.

## 9. Add the one-gateway source invariant

Add `completion_has_one_typed_request_gateway` using the production prefix of
`include_str!("lib.rs")`.

Assert legacy method declarations are absent. Assert the production prefix has
one executor call and it lies within dispatch. Assert the completion executor
has one command runner call.

Temporarily reason through the red condition: adding a second
`self.execute_completion_effect(` outside dispatch or restoring either legacy
method declaration must fail the test. No intentional source mutation is
needed to demonstrate this during the ticket run.

Verification: run the invariant by exact name.

## 10. Format and focused validation

Run `cargo fmt --all` as a mechanical formatting step, then
`cargo fmt --all -- --check`.

Run focused new and migrated tests. Run `cargo test -p lisa-plugin --no-fail-fast`.

If any failure reflects an intentional behavior change not described in the
design, stop and correct the implementation rather than updating expectations.

## 11. Workspace and target validation

Run:

```text
cargo test --workspace --no-fail-fast
cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
git diff --check
```

If workspace failures are unrelated, reproduce them against HEAD or document
clear evidence. Ticket-owned failures block disposition pass.

## 12. Record progress and inspect ownership

Write `progress.md` with completed steps, deviations, tests, and exact status.

Inspect:

```text
git diff -- crates/lisa-plugin/src/lib.rs
git diff --cached --name-only
git status --short
```

Confirm the ordinary index contains no ticket-owned entries and unrelated
dirty paths were not modified.

## 13. Commit the meaningful source unit

Use the exact isolated transaction:

```text
lisa commit-ticket \
  --ticket-id T-042-01-02 \
  --message "refactor(plugin): route all completion sources through typed adapter" \
  --include crates/lisa-plugin/src/lib.rs
```

If the installed command is unavailable, use `target/debug/lisa` without
changing flags or include paths.

Do not run `git add`, `git commit`, or any broad staging command.

## 14. Post-commit verification

Confirm the returned commit contains only
`crates/lisa-plugin/src/lib.rs`. Confirm that path is clean in ordinary Git
status and the ordinary index remains empty.

Run at least the focused invariant after the commit if the transaction changes
HEAD underneath the working tree. Preserve all unrelated dirty state.

## 15. Review handoff

Write `review.md` summarizing source changes, exact commit, acceptance mapping,
test coverage, repository preservation, and open concerns.

Write exactly one valid `review-disposition.json` shape. Use pass only if the
source is committed, all ticket-owned paths are clean, the gateway invariant
passes, and relevant tests pass.

Remain on T-042-01-02 after Review. Lisa owns artifact admission, final Done
publication, completion commit, and seat release.
