# Review: Gate completion on explicit pass

## Disposition

Pass.

The final repository state satisfies T-040-01-03's acceptance criterion and is
ready for Lisa's completion transaction.

## Summary

Automated Review completion now requires a current-attempt,
machine-readable disposition that parses to the exact typed `Pass` outcome.

Valid blocks, invalid documents, missing files, stale attempt artifacts, and
publication failures cannot enter `request_completion`.
They leave the ticket in Review with its thread, pane assignment, and current
lease intact.

The change preserves the existing atomic pass path: approval creates a pending
completion request, but Done is not published and dependents are not unblocked
until the isolated completion transaction succeeds and its result is verified.

## Files changed

Modified `crates/lisa-plugin/src/lib.rs`.

Production changes in the final file include:

- imports of `parse_review_disposition` and `ReviewDisposition`;
- the private `State::request_review_completion` authorization boundary;
- disposition admission through the current attempt lease;
- exhaustive Pass, Block, and Invalid handling;
- operator-visible block and invalid diagnostics;
- routing of artifact polling, idle compatibility, and stopped-session Review
  completion through the gate.

Test changes include:

- helpers for arbitrary and passing Review documents;
- a table-driven block/pass/invalid scheduler regression with a real dependent;
- a direct stopped-session block regression;
- explicit passing dispositions in all positive automated-completion fixtures.

No files were created or deleted in production source.
No manifest or dependency change was required.

## Acceptance criteria trace

“`check_artifact_advances` ... call `request_completion` only on a Pass” is met
by routing its Review-to-Done branch through `request_review_completion`.
The helper's only transaction call is inside the exact
`ReviewDisposition::Pass` match arm.

“`auto_complete_review` ... only on a Pass” is met by routing the pane-derived
current attempt lease through the same helper.
The direct block regression proves this caller cannot prepare completion for a
valid refusal.

“a test drives block/pass/invalid dispositions” is met by
`review_disposition_gates_artifact_completion_and_dependents`, which constructs
fresh real scheduler state for all three outcomes and calls the production
artifact consumer.

“block -> ticket stays assigned” is asserted through both thread presence and
pane slot ticket ownership, with the current attempt lease unchanged.

“no request_completion” is asserted by absence of the ticket from
`pending_completions`, the observable state created by the lower-level request.

“dependents still blocked” is asserted using the real DAG's
`all_dependencies_done` query for a ticket that depends on the blocked Review.

“operator-visible reason” is asserted against the dashboard activity source,
including the exact actionable agent reason.

“pass -> atomic completion unchanged” is asserted by presence of one pending
request while thread, slot, lease, ticket frontmatter, and dependent readiness
remain unchanged until successful transaction publication.
Existing commit-result tests continue to prove the later Done publication and
cleanup behavior.

“invalid -> safe refusal, not Done” is asserted with a contradictory pass
document: no pending request, Review state retained, dependent blocked, and an
invalid diagnostic visible in activity.

## Design assessment

The Review-specific helper is the correct layer for this policy.
It centralizes agent approval without burdening the lower-level transaction
primitive, manual operator completion, or external Done reconciliation with an
artifact contract they do not own.

Admission before parsing is important.
It proves current lease authority, prevents fallback to stale canonical bytes,
and ensures the exact disposition evidence is part of canonical work before a
passing transaction commits that directory.

The exhaustive enum match makes approval auditable.
There is no truthy conversion, default result, error fallback, or wildcard arm
that can manufacture pass.

Routing the two idle compatibility calls in addition to the ticket's named
sites closes real bypasses in the current code shape. This broadens no external
authority; it applies the same invariant to every automated occurrence of the
same Review-to-Done edge.

Manual completion remains intentionally available to an operator.
An explicit operator command is distinct authority and can resolve a blocked or
legacy Review without forging agent approval.

## State safety review

On block or invalid input, the only state changes are:

- canonical publication of the current attempt's evidence;
- a bounded activity-log entry.

The code does not:

- insert a pending completion;
- write Done frontmatter;
- mark or remove the thread;
- release or retarget the slot;
- revoke the current attempt lease;
- emit Done provenance;
- schedule dependents.

On pass, all pre-existing `request_completion` checks still apply:

- duplicate pending request rejection;
- current attempt authority;
- dependency completion;
- ticket file availability;
- isolated command construction;
- deferred result publication.

No transaction or cleanup behavior was duplicated in the new helper.

## Test evidence

Focused disposition regression:

```text
cargo test -p lisa-plugin review_disposition
```

Passed: 1 test, 0 failed.

Focused stopped-session regressions:

```text
cargo test -p lisa-plugin auto_complete_review
```

Passed: 7 tests, 0 failed.

Complete plugin suite:

```text
cargo test -p lisa-plugin --lib
```

Passed after final additions: 336 tests, 0 failed.

Complete workspace suite:

```text
cargo test --workspace
```

Passed across all crates and doc tests.

WASM check:

```text
cargo check -p lisa-plugin --target wasm32-wasip1
```

Passed.

Formatting and scoped diff checks passed.

## Commit and ownership review

This ticket executed:

```text
lisa commit-ticket \
  --ticket-id T-040-01-03 \
  --message "Gate review completion on explicit pass" \
  --include crates/lisa-plugin/src/lib.rs
```

Commit:

```text
7caacaeb4c3541d61b895ab6e68d8c1dc567f698
```

The commit contains exactly `crates/lisa-plugin/src/lib.rs`.
No ordinary `git add` or `git commit` was used.
The source path has no remaining staged, modified, or untracked state.

Unrelated Lisa-managed provenance/ticket changes and other ticket work were
preserved and excluded.

## Concurrent history note

While this attempt was editing `crates/lisa-plugin/src/lib.rs`, concurrent
ticket T-040-02-02 committed the same exact file through Lisa and advanced
HEAD. Because both tickets modified one shared monolithic source path, that
transaction captured the production helper/routing bytes along with its own
same-file work in commit `a7e4a003`.

This ticket's subsequent exact-path transaction committed the remaining
regression, fixture, and formatting unit as `7caacae`.
The combined HEAD state is complete and all verification was run after both
serialized commits.

This is not a functional blocker, but it is a history/graph concern: concurrent
tickets that own the same source file should carry a dependency edge or use
separate module boundaries. Isolated indexes serialize commits but cannot
separate simultaneously edited bytes within one include path.

## Coverage gaps and limitations

The plugin integration test uses a contradictory pass reason as the Invalid
representative. Missing files and malformed JSON are already covered by the
core parser tests, and the helper's missing/admission branches are direct
non-pass returns, but there is no separate scheduler-level test for every
parser invalid subtype.

Operator visibility is provided through the existing bounded activity log.
Repeated artifact polling can repeat a block warning. There is no persistent
deduplicated Review-block alert model. The block remains actionable because the
ticket and seat stay assigned and the reason is repeatedly visible; adding
deduplication would require separate lifecycle state beyond this ticket.

The tests do not run a live Zellij host or actual native completion subprocess.
They exercise the real scheduler state boundary up to pending command state,
while existing command-result tests cover successful publication and cleanup.
The WASM target compilation confirms the added core integration builds for the
runtime target.

## Open concerns

No blocking code or test concern remains.

The concurrent same-file history note should be considered when scheduling
future tickets against `crates/lisa-plugin/src/lib.rs`; it does not prevent this
ticket from completing because the final source state is durable, exact-path
committed, and fully verified.
