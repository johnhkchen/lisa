# Structure: Gate completion on explicit pass

## Change inventory

Modify:

- `crates/lisa-plugin/src/lib.rs`

Create no source files.
Delete no files.
Change no manifests or public APIs.

Private workflow artifacts are created under:

```text
.lisa/attempts/T-040-01-03/1/work/
```

They are not part of the ticket-owned source commit.

## Import boundary

Add imports for the predecessor core model near the existing `lisa_core`
imports:

```rust
use lisa_core::disposition::{parse_review_disposition, ReviewDisposition};
```

No re-export is introduced from the plugin.
The dependency direction remains plugin -> core.

## New internal method

Add one private method on `State` adjacent to `request_completion`:

```rust
fn request_review_completion(
    &mut self,
    ticket_id: TicketId,
    source: CompletionSource,
    source_lease: Option<AttemptLease>,
) -> bool
```

This method is the automated Review authorization boundary.
It owns disposition admission, parsing, outcome logging, and delegation.

Keeping it adjacent to `request_completion` makes the two layers explicit:

- `request_review_completion`: proves agent Review approval;
- `request_completion`: prepares and tracks the isolated completion transaction.

## Method organization

The method first names the artifact constant locally as
`review-disposition.json` and calls:

```rust
self.admit_artifact(
    &ticket_id,
    source_lease.as_ref(),
    "review-disposition.json",
)
```

An admission error logs `ActivityEvent::Error` with ticket and reason, then
returns false.

An absent artifact logs `ActivityEvent::Error` explaining that explicit Review
disposition is required, then returns false.

After successful admission, construct the canonical path through:

```rust
self.config
    .work_dir
    .join(&ticket_id)
    .join("review-disposition.json")
```

Parse it with the core function and exhaustively match the enum.

The `Pass` arm delegates:

```rust
self.request_completion(
    ticket_id,
    source,
    source_lease.map(CompletionAuthority::Attempt),
)
```

The `Block` arm logs `ActivityEvent::Warning` including the reason and returns
false.

The `Invalid` arm logs `ActivityEvent::Error` including the reason and returns
false.

No state mutation occurs on the non-pass arms except bounded activity logging
and canonical evidence publication.

## Caller changes

In `check_artifact_advances`, replace the direct Review-to-Done
`request_completion` invocation with `request_review_completion`.
Pass the existing `source_lease` and `CompletionSource::Artifact` unchanged.

In `check_idle_signals`, replace both direct automated Review completion calls:

- the same-cycle catch-up after Implement advances to Review;
- the `next_phase == Phase::Done` branch for a Review idle signal.

Both pass `CompletionSource::Idle` and their current attempt lease.

In `auto_complete_review`, resolve the pane slot's lease exactly as today, then
call `request_review_completion` with `CompletionSource::Stopped(pane_id)`.

Do not alter `mark_ticket_done` or observed-Done reconciliation.
They retain direct calls to `request_completion` because their authority is
operator/external state, not agent Review disposition.

## Test fixture structure

Add a small test-only disposition case enum or use a compact tuple table inside
one regression test. Avoid adding production types for test representation.

The primary test builds a temporary two-ticket filesystem fixture and scans it
into a real `Dag`. It configures:

- `ticket_dir`;
- `work_dir`;
- one slot assigned to `T-REVIEW`;
- one running Review thread;
- one current attempt lease.

For each table entry, write `review.md` plus the disposition bytes into the
attempt work directory and call the real `check_artifact_advances` method.

Because every case needs pristine state, construct a new temp directory and
state inside the loop rather than manually clearing pending state, canonical
files, activity, and DAG mutations.

Use the DAG's existing readiness/dependency query to assert the dependent is
not schedulable. If no direct public helper expresses a single dependent's
readiness, assert `all_dependencies_done("T-DEPENDENT") == false`.

## Existing test updates

Update every positive fixture whose action now crosses the automated Review
gate:

- `test_check_artifact_advances_review_to_done`;
- `test_auto_complete_review_updates_ticket_and_cleans_up`;
- Codex artifact catch-up and stopped-completion tests;
- lease fencing and completion publication tests that invoke
  `check_artifact_advances` from Review;
- any other test found by searching writes of `review.md` followed by an
  automated completion call.

Each fixture writes the exact canonical passing JSON into the current attempt
directory (or canonical directory for intentionally unleased fixtures).

Tests focused directly on lower-level `request_completion` do not need a
disposition because they are testing transaction authority, not the Review
consumer.

## Stopped-site regression shape

Extend the existing direct `auto_complete_review` test to set `work_dir`, keep
the installed lease, write a passing disposition in its attempt directory, and
retain all atomicity assertions.

Add a focused block test for the same method:

- assigned slot and current lease;
- Review thread and ticket;
- valid block document with a recognizable reason;
- direct call to `auto_complete_review`;
- no pending completion;
- assignment and thread retained;
- warning includes the reason.

This locks the second named completion site independently of the artifact poll
table.

## Source commit boundary

All production logic and unit tests reside in the same large source file, so
they form one meaningful ticket-owned source unit:

```text
crates/lisa-plugin/src/lib.rs
```

Commit it with one exact include via `lisa commit-ticket` after focused and
workspace verification. Do not include ticket frontmatter, provenance, other
work directories, or private phase artifacts.

## Verification boundaries

Formatting:

```text
cargo fmt --all --check
```

Focused plugin regression tests should be runnable by name filters for
disposition completion and auto-complete Review behavior.

Full native verification:

```text
cargo test -p lisa-plugin
cargo test --workspace
```

WASM compile verification:

```text
cargo check -p lisa-plugin --target wasm32-wasip1
```

Diff verification is scoped to `crates/lisa-plugin/src/lib.rs` before the
isolated transaction. Post-commit status must show no staged, modified, or
untracked entry for that source path.

## State invariants after non-pass

For Block and Invalid:

- `pending_completions` has no ticket entry;
- the thread remains in the map with `Phase::Review` and non-completed status;
- the matching slot retains its ticket and attempt lease;
- the current lease remains installed;
- the ticket file does not contain `phase: done` or `status: done`;
- the DAG still reports dependent prerequisites incomplete;
- activity contains the refusal reason.

For Pass before native command result:

- `pending_completions` contains the ticket;
- all the same thread, slot, ticket, and DAG state remains Review;
- canonical work contains both review and disposition evidence;
- existing result handling remains solely responsible for publishing Done and
  releasing ownership.
