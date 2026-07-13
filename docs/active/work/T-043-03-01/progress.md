# Progress: attribute captures by pane time

## Status

Implementation is complete.

The ticket-owned source change is confined to:

- `crates/lisa-plugin/src/lib.rs`.

Research, Design, Structure, and Plan artifacts were written to this attempt's
private work directory before source implementation.

The ticket frontmatter was not edited by this agent.

## Baseline verification

Before source edits:

```text
git diff -- crates/lisa-plugin/src/lib.rs
```

reported no existing diff for the ticket-owned source file.

The predecessor ownership contract was verified with:

```text
cargo test -p lisa-plugin owner_at
```

Result:

```text
2 passed; 0 failed
```

This established a clean source baseline and a green pane-time lookup before
the consumer changed.

## Completed: capture-ledger path documentation

Updated the `State` comments for `codex_dir` and `claude_dir`.

They now describe the provider-specific append-only `captures.jsonl` ledgers.

The obsolete comments about per-ticket usage artifacts and shared nested usage
shapes were removed.

No path field type or initialization changed.

Production still resolves:

- Codex to `.lisa/codex`;
- Claude to `.lisa/claude`.

## Completed: current ownership interval wiring

`emit_provenance` still constructs the full current terminal
`ProvenanceRecord` before usage attribution.

The record already carries:

- the active ticket ID;
- the exact attempt lease;
- current outcome and authority;
- route;
- epoch-second start and end;
- concurrency-at-spawn;
- physical pane ID.

The usage call changed from a ticket ID input to a borrow of that current
record.

The current interval is therefore available to `owner_at` even though it has
not yet been appended.

The method still performs exactly one terminal append after token fields are
filled.

Existing append-error logging and teardown behavior remain unchanged.

## Completed: mixed provenance history load

The plugin imports `ProvenanceLedgerRecord` from `lisa_core::provenance`.

`read_usage` reads the existing provenance ledger best-effort.

Each line is deserialized independently through the shared mixed-row enum.

Execution records are retained as durable ownership intervals.

Assignment-transition rows are discarded from the ownership set because they
represent pre-provider failures.

Malformed provenance lines are skipped rather than blocking later valid
append-only rows.

An absent or unreadable ledger becomes an empty prior history.

This permits the first ticket on a pane to attribute through the in-memory
current interval alone.

## Completed: capture consumer

`read_usage` now selects the existing client directory and reads its fixed:

```text
captures.jsonl
```

It no longer derives a filename from `ticket_id`.

It no longer reads `<ticket>.usage.json`.

It no longer expects a nested `{ usage: ... }` object.

It no longer calls `provenance::extract_usage` from the plugin path.

Each valid line is deserialized as the shared
`lisa_core::capture::CaptureRecord`.

Malformed capture rows are skipped.

Missing or unreadable capture storage returns null token and cost values.

## Completed: pane-time attribution

The consumer first rejects captures from any physical pane other than the
current record's pane.

For each current-pane capture it calls `ownership::owner_at` over:

```text
all prior durable execution records
+ the current in-memory execution record
```

It passes the capture's own pane ID and `captured_at` epoch second.

The capture contributes only when the unique returned owner equals the current
record's ticket ID.

Captures owned by another ticket are left out of the current total.

Captures with no unique owner are also left out.

No shared, last, environment-derived, or fallback owner was introduced.

The later `T-043-03-02` ticket remains responsible for routing those
unattributable valid captures into session-keyed quarantine and raising a
visible activity event.

## Completed: token summation

All capture rows attributed to the current ticket contribute their full input
and output token counts.

The accumulator starts as `None`, preserving the distinction between no
measurement and measured zero.

The first matching capture creates concrete totals.

Subsequent captures use `checked_add` for both `u64` values.

An overflow fails closed to null usage rather than wrapping.

At least one successfully summed capture produces:

```text
(Some(input_tokens), Some(output_tokens), None)
```

No matching capture produces:

```text
(None, None, None)
```

Cost remains null because `CaptureRecord` has no observed dollar-cost field.

## Completed: provider fixture migration

The existing Codex and Claude usage-flow plugin tests were migrated away from
legacy `<ticket>.usage.json` fixtures.

Both now write real `CaptureRecord` rows through
`append_capture_record` into the correct provider's `captures.jsonl`.

The Codex test still proves input/output flow and null cost.

The Claude test still proves input/output flow and null cost.

The missing-capture Claude test remains unchanged and continues to prove all
three usage fields stay null.

No `usage.json` reference remains in the edited plugin source file.

## Completed: recycled-pane acceptance regression

Added:

```text
provenance_recycled_pane_attributes_capture_sums_to_each_ticket
```

The fixture uses one physical pane, ID 7.

It creates four append-only captures:

- two for A's earlier ownership window;
- two for B's later ownership window.

A's expected totals are:

```text
input  = 10 + 20 = 30
output =  3 +  7 = 10
```

B's expected totals are:

```text
input  = 100 + 200 = 300
output =  40 +  60 = 100
```

The intervals are deterministic epoch-second windows with a gap between A and
B.

A is attributed using the same private consumer and appended through the shared
core provenance writer.

B is emitted through the real `State::emit_provenance` teardown fill and append
path.

After A, the test checks one row with A's sums.

After B, the test checks exactly two rows.

It reasserts row zero's A identity and totals, proving B did not overwrite or
mutate A.

It then checks row one's B identity and distinct totals, proving neither
ticket's captures blended into the other.

## Plan deviation: deterministic first interval

The Plan initially described emitting both A and B through live
`emit_provenance` calls.

Provenance timestamps have one-second resolution and `owner_at` treats both
endpoints as inclusive.

Two immediate live emissions can therefore make A's end second equal B's start
second, correctly producing ambiguous ownership at that boundary.

Sleeping across wall-clock seconds would make the test slow and timing-based.

The regression instead constructs A's closed record with deterministic past
epoch values, runs it through `State::read_usage`, and appends it with the same
core writer; B then uses the complete real emission path.

This preserves the acceptance behavior while keeping the fixture deterministic
and free of sleeps.

No production design changed because of this test-only adjustment.

## Formatting and diff checks

Commands:

```text
cargo fmt --all -- --check
git diff --check
git diff --stat -- crates/lisa-plugin/src/lib.rs
rg -n "usage\\.json|read_usage\\(" crates/lisa-plugin/src/lib.rs
```

Results:

- formatting check passed;
- whitespace/error diff check passed;
- the source diff contains only `crates/lisa-plugin/src/lib.rs`;
- no legacy `usage.json` occurrence remains;
- `read_usage` has one production call plus the deterministic acceptance
  fixture call.

The implementation diff before commit was:

```text
1 file changed, 196 insertions(+), 50 deletions(-)
```

Most added lines are the explicit recycled-pane regression.

## Focused verification

New acceptance test:

```text
cargo test -p lisa-plugin \
  provenance_recycled_pane_attributes_capture_sums_to_each_ticket
```

Result:

```text
1 passed; 0 failed
```

Adjacent provenance suite:

```text
cargo test -p lisa-plugin provenance_
```

Result:

```text
9 passed; 0 failed
```

Ownership suite after implementation:

```text
cargo test -p lisa-plugin owner_at
```

Result:

```text
2 passed; 0 failed
```

The adjacent suite includes:

- terminal failure emission;
- retry append preservation;
- append failure handling;
- Codex capture flow;
- Claude capture flow;
- missing capture behavior;
- ticket-frontmatter non-mutation;
- unset-ledger no-op;
- the new recycled-pane attribution case.

## Workspace verification

Command:

```text
cargo test --workspace
```

Result: passed with no failures.

Observed major suite totals included:

- 14 `lisa-cli` library tests;
- 266 CLI binary tests;
- 197 `lisa-core` unit tests;
- 378 `lisa-plugin` unit tests;
- CLI and core integration/regression targets;
- 0 doc-test failures.

The real-Zellij delivery test remained ignored by its declared environment gate;
it was not a failure.

## WASM-inclusive project gate

Command:

```text
just check
```

Result: passed.

The command completed:

```text
cargo check -p lisa-plugin --target wasm32-wasip1
cargo test --workspace
```

The WASM target compiled successfully.

The workspace suite then passed again, including all 378 plugin tests.

## Source transaction

The planned isolated command is:

```text
lisa commit-ticket \
  --ticket-id T-043-03-01 \
  --message "fix(plugin): attribute captures by pane time" \
  --include crates/lisa-plugin/src/lib.rs
```

Only the exact ticket-owned source path will be included.

No ordinary `git add`, `git add -A`, or `git commit` was used.

The final commit identifier and post-transaction cleanliness check will be
recorded below after the isolated transaction completes.

## Unrelated worktree state preserved

The repository contains Lisa-managed changes outside this ticket, including
active ticket/provenance/journal state.

Those paths were not edited, staged, reverted, or included by this
implementation.

The private RDSPI artifacts are intentionally untracked/ignored attempt data
until Lisa publishes admitted completion.

## Completed: isolated source transaction

Command:

```text
lisa commit-ticket \
  --ticket-id T-043-03-01 \
  --message "fix(plugin): attribute captures by pane time" \
  --include crates/lisa-plugin/src/lib.rs
```

Result:

```text
712de86ddd0997bb3208de49900c91666b407b87
```

Commit subject:

```text
fix(plugin): attribute captures by pane time
```

Commit stat:

```text
crates/lisa-plugin/src/lib.rs | 246 +++++++++++++++++++++++++++++++++---------
1 file changed, 196 insertions(+), 50 deletions(-)
```

The isolated transaction included exactly the planned source path.

No ordinary Git index command was used.

## Completed: post-commit audit

Commands:

```text
git status --short
git diff -- crates/lisa-plugin/src/lib.rs
git diff --cached --name-only
git show --stat --oneline 712de86ddd0997bb3208de49900c91666b407b87
git show -- crates/lisa-plugin/src/lib.rs \
  712de86ddd0997bb3208de49900c91666b407b87
```

Results:

- `crates/lisa-plugin/src/lib.rs` has no uncommitted diff;
- the ordinary index contains no staged path;
- the commit contains one file only;
- the committed diff matches the reviewed consumer, documentation, fixture
  migrations, and acceptance regression;
- unrelated Lisa-managed ticket/provenance/journal/work publication state
  remains outside the source commit.

The ticket-owned source unit is durable and clean.

## Remaining

- Write `review.md` and `review-disposition.json`.
- Remain on this ticket for Lisa's completion confirmation.
