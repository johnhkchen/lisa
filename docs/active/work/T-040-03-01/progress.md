# Progress: blocking Review regression

## Status

Implementation is complete, verified, and committed through the required
isolated Lisa transaction.

## Completed work

Added
`tests::test_t039_06_02_blocking_review_never_prepares_done` to
`crates/lisa-plugin/src/lib.rs`.

The test constructs a real two-ticket DAG:

- `T-REVIEW` is assigned and running in Review;
- `T-DEPENDENT` is ready but depends on `T-REVIEW`.

The current attempt writes both `review.md` and a valid
`review-disposition.json` whose disposition is `block` with an actionable
reason. The test then calls the production `check_artifact_advances` poll.

## Assertions implemented

The regression asserts that a blocking Review:

- does not insert `T-REVIEW` into `pending_completions`;
- leaves its thread present, running, and in Review;
- leaves its pane slot assigned;
- preserves the slot and current-map attempt lease;
- leaves ticket frontmatter at `status: review` and `phase: review`;
- creates no provenance ledger and therefore no authoritative Done row;
- leaves `Dag::all_dependencies_done(T-DEPENDENT)` false;
- does not create a dependent thread;
- logs the exact actionable block reason.

The pending-completion assertion names its historical significance: the
pre-T-040-01-03 unconditional Review path would have inserted that entry even
with the blocking disposition present. This makes the test a discriminator for
the field bug, rather than only an assertion about delayed publication.

## Focused verification

Formatting check:

```text
cargo fmt --all -- --check
```

Result: passed with no formatting changes required.

Historical regression:

```text
cargo test -p lisa-plugin test_t039_06_02_blocking_review_never_prepares_done
```

Result: 1 passed, 0 failed.

Neighboring disposition coverage:

```text
cargo test -p lisa-plugin review_disposition
```

Result: 1 passed, 0 failed. The filter selects the existing generic
block/pass/invalid scheduler regression; the specifically named historical
test was run separately above.

## Broad verification

Native workspace:

```text
cargo test --workspace
```

Result: passed.

Notable totals:

- `lisa-cli`: 279 unit tests passed;
- CLI integration tests passed, with the real-Zellij environment test ignored
  by its declared requirements;
- `lisa-core`: 169 tests passed;
- `lisa-plugin`: 337 tests passed;
- doc tests passed.

Deployed target compile:

```text
cargo check -p lisa-plugin --target wasm32-wasip1
```

Result: passed.

The workspace test and WASM check ran concurrently. Cargo briefly reported
normal package-cache/build-directory lock waits; both commands completed
successfully.

## Diff verification

`git diff --check -- crates/lisa-plugin/src/lib.rs` passed.

The scoped diff contains 98 added test lines and no production changes.
Before the source transaction, the ordinary index contains no staged
ticket-owned path.

Unrelated Lisa-managed provenance, ticket, generated docs, and canonical work
paths remain in the worktree and were not edited or included by this attempt.

## Plan deviations

No functional deviation.

The plan listed a separate `cargo test -p lisa-plugin --lib`; the complete
workspace command ran that exact library target and reported all 337 plugin
tests passing, so a redundant second full plugin invocation was unnecessary.

## Source transaction

Executed exact transaction:

```text
lisa commit-ticket \
  --ticket-id T-040-03-01 \
  --message "Pin blocking Review completion regression" \
  --include crates/lisa-plugin/src/lib.rs
```

This section will be updated with the resulting commit after the command.

Result:

```text
b6a574abd4471f8a361b005ddfbac306cf98dffe
```

Commit `b6a574a` contains exactly
`crates/lisa-plugin/src/lib.rs`, with 98 insertions. The source path has no
staged, modified, or untracked state after the transaction. No ordinary
`git add` or `git commit` was used.

## Remaining work

- Write Review artifacts and stop on this ticket for Lisa publication.
