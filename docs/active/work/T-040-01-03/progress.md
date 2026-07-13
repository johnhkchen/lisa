# Progress: Gate completion on explicit pass

## Status

Implementation is complete.
The Review disposition gate is active on every automated Review-to-Done edge,
the required block/pass/invalid regression coverage passes, and the final
ticket-owned plugin source state is committed through Lisa's isolated
transaction.

No ticket-owned source path remains staged, modified, or untracked.

## Implemented authorization boundary

`crates/lisa-plugin/src/lib.rs` now imports the typed Review model from
`lisa-core`:

```rust
parse_review_disposition
ReviewDisposition
```

`State::request_review_completion` is the shared automated Review authority
boundary.

It first admits `review-disposition.json` through `State::admit_artifact` using
the caller's current attempt lease. This preserves the established lease fence
and atomically publishes the exact attempt bytes into canonical ticket work.

Admission failure is fail-closed:

- a missing disposition logs a visible error and returns false;
- a stale/mismatched lease or publication error logs its reason and returns
  false;
- neither path calls `request_completion`.

After successful admission, the helper parses the canonical artifact through
the predecessor core parser and matches all three outcomes exhaustively.

`ReviewDisposition::Pass` is the only arm that delegates to
`request_completion`.
It preserves the existing ticket ID, completion source, and current attempt
authority, so all dependency, lease, pending-state, isolated command, and
result-publication behavior is unchanged after approval.

`ReviewDisposition::Block { reason }` logs:

```text
Completion blocked for {ticket}: {reason}
```

as an operator-visible warning and returns false.

`ReviewDisposition::Invalid { reason }` logs an operator-visible refusal error
and returns false.

Neither non-pass branch changes thread ownership, slot assignment, ticket
frontmatter, current lease, pending completion state, or the DAG.

## Routed completion sites

The following automated Review-to-Done edges now call
`request_review_completion`:

1. `check_artifact_advances` when a Review artifact observes Done as the next
   phase;
2. `check_idle_signals` when Implement reaches Review and an already-written
   review artifact is caught up in the same cycle;
3. `check_idle_signals` when an existing Review thread receives its artifact
   plus idle signal;
4. `auto_complete_review` when a stopped Review session is eligible to finish.

The ticket explicitly names the artifact polling and stopped-session sites.
The two idle compatibility edges were also routed because they represent the
same automated Review-to-Done transition and otherwise could bypass the stated
safety invariant.

Manual operator completion remains direct.
`mark_ticket_done` still uses `CompletionSource::Manual` and operator authority
without requiring an agent-authored pass.

Observed-Done reconciliation also remains direct because it reconciles durable
external ticket state rather than interpreting Review evidence.

## Regression coverage added

Added test helpers to write arbitrary and canonical passing disposition files
into the current attempt directory.

Added `review_disposition_gates_artifact_completion_and_dependents`.
It drives the real artifact polling consumer with a fresh two-ticket DAG for
each of three documents.

The block case writes an actionable reason and proves:

- no `pending_completions` entry is created;
- the Review thread remains running;
- the pane slot remains assigned;
- the attempt lease remains current;
- ticket frontmatter remains Review;
- the dependent's prerequisites remain incomplete;
- the exact block reason appears in operator activity;
- the disposition evidence is admitted to canonical work.

The pass case proves:

- a pending completion is created;
- the transaction remains atomic: thread, slot, lease, and ticket all remain in
  Review before successful native command publication;
- the dependent remains blocked until committed Done, not merely requested
  completion;
- admitted evidence exactly matches the current attempt document.

The invalid case uses a contradictory pass reason and proves:

- no completion request is created;
- all Review ownership remains intact;
- the dependent stays blocked;
- the invalid disposition reason is operator-visible;
- no Done state is prepared or published.

Added
`test_auto_complete_review_block_retains_assignment_with_visible_reason`.
It directly drives the second named completion site and verifies that a stopped
Review with a valid block remains assigned and visible without entering the
pending transaction.

## Existing fixture updates

Existing positive automated-completion tests previously represented implicit
approval by writing only `review.md`.
They now write the exact explicit passing JSON before retaining their original
pending/atomicity assertions.

Updated coverage includes:

- direct Review artifact completion;
- Implement-to-Review artifact catch-up;
- full RDSPI artifact catch-up;
- idle Implement catch-up;
- idle Review completion;
- stopped Review completion;
- Codex artifact-only phase progression;
- Codex stopped-session dependency behavior;
- split-brain replacement completion;
- verified commit-result publication.

Lower-level direct `request_completion` tests were intentionally unchanged.
They characterize transaction authority and publication after Review approval,
not the agent disposition consumer.

## Verification performed

Focused artifact outcome test:

```text
cargo test -p lisa-plugin review_disposition
```

Result: 1 passed, 0 failed.

Focused stopped Review tests:

```text
cargo test -p lisa-plugin auto_complete_review
```

Result: 7 passed, 0 failed.

Complete plugin native suite:

```text
cargo test -p lisa-plugin --lib
```

Result after the dedicated regressions: 336 passed, 0 failed.

Complete workspace suite:

```text
cargo test --workspace
```

Result: passed across `lisa-cli`, `lisa-core`, and `lisa-plugin`, including 276
CLI tests and 336 plugin tests; doc tests passed.

WASM compile check:

```text
cargo check -p lisa-plugin --target wasm32-wasip1
```

Result: passed.

Formatting and diff hygiene:

```text
cargo fmt --all --check
git diff --check -- crates/lisa-plugin/src/lib.rs
```

Both passed.

## Commit transaction

Executed the required isolated transaction:

```text
lisa commit-ticket \
  --ticket-id T-040-01-03 \
  --message "Gate review completion on explicit pass" \
  --include crates/lisa-plugin/src/lib.rs
```

Created commit:

```text
7caacaeb4c3541d61b895ab6e68d8c1dc567f698
```

`git show` confirms the commit contains exactly
`crates/lisa-plugin/src/lib.rs`.
The commit records the dedicated regression suite, updates all positive
fixtures to explicit approval, and carries the final formatting delta.

Post-commit scoped ordinary-index and worktree diffs for the plugin source are
empty.

## Concurrent same-file serialization

While implementation was in progress, Lisa serialized concurrent ticket
T-040-02-02's exact include of the same large plugin source file.
Its source commit `a7e4a003` advanced HEAD and captured the then-current
worktree bytes, including the newly added production Review helper and routed
call sites, alongside that ticket's changes.

No ordinary Git staging or commit command was used by this attempt.
Both source transactions used Lisa's isolated command with the exact
repository-relative plugin path, and the final repository state is verified by
this ticket's complete source commit and test suite.

This overlap demonstrates the workflow document's warning that isolated Git
indexes do not substitute for dependency edges when concurrent tickets modify
the same file. It did not leave a code or test defect, but the commit history
places part of this ticket's production hunk in the immediately preceding
same-file source commit.

## Deviations from plan

The implementation structure matched the selected design.

One planned detail changed operationally: the production helper hunk was
serialized into the concurrent same-file commit described above before this
ticket's own isolated source transaction ran. This ticket's transaction then
committed the remaining complete regression and fixture unit from the same
exact plugin path.

The plan anticipated updating positive fixtures based on failures. The initial
plugin run produced ten expected failures for implicit-pass fixtures; each was
updated with explicit approval, after which the full suite passed.

## Remaining work

Implementation work is complete.
Only the Review handoff artifacts remain for this attempt.
