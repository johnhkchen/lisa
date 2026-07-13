# Review: attribute captures by pane time

## Disposition

Pass.

The implementation satisfies the ticket acceptance criterion, all focused and
workspace verification passes, the WASM target checks cleanly, and the sole
ticket-owned source file is committed through Lisa's isolated transaction with
no residual source or index state.

## Change summary

The plugin no longer reads token usage from a ticket-derived
`<ticket>.usage.json` file.

At terminal provenance emission it now consumes the provider's append-only
`captures.jsonl` ledger.

Every capture is evaluated using its physical pane and capture timestamp.

Ownership comes from the existing `ownership::owner_at` contract over:

- durable prior execution provenance intervals;
- the current in-memory execution interval being closed.

Only captures uniquely owned by the current terminal record's ticket are
summed into that record.

The current record is then appended through the unchanged provenance writer.

Later pane reuse therefore adds a new terminal row without overwriting the
earlier owner's record.

## Files modified

### `crates/lisa-plugin/src/lib.rs`

Production changes:

- imports the mixed `ProvenanceLedgerRecord` reader;
- updates provider-directory comments to describe `captures.jsonl`;
- passes the current `ProvenanceRecord` into `read_usage`;
- replaces ticket-keyed JSON parsing with capture-ledger parsing;
- loads prior execution ownership from the mixed provenance ledger;
- excludes assignment-transition rows from ownership;
- filters captures to the current pane;
- attributes each through `owner_at` using prior plus current intervals;
- sums input and output tokens with checked arithmetic;
- preserves null usage when no record can be included;
- keeps cost null because captures do not carry observed cost.

Test changes:

- migrates the Codex usage-flow fixture to a real `CaptureRecord` append;
- migrates the Claude usage-flow fixture to a real `CaptureRecord` append;
- adds the deterministic A-then-B recycled-pane regression.

No source file was created or deleted.

No core or CLI source file changed.

## Workflow artifacts

The following artifacts were created only in the attempt-private work
directory:

- `research.md`;
- `design.md`;
- `structure.md`;
- `plan.md`;
- `progress.md`;
- `review.md`;
- `review-disposition.json`.

Lisa owns admission and publication to `docs/active/work/T-043-03-01`.

The agent did not directly update ticket phase or status.

## Architecture assessment

The change composes the three predecessor contracts rather than introducing a
parallel data model.

`CaptureRecord` remains the pre-attribution fact schema.

`ProvenanceRecord` remains the durable terminal ownership and usage schema.

`owner_at` remains the single pane-time ownership rule.

`emit_provenance` remains the only terminal execution append point.

The key integration detail is the not-yet-appended current interval.

Reading only the durable ledger would leave the current ticket with no owner at
its own capture times.

Appending the current row before usage fill would violate the established
append-only record shape.

Passing the completed null-usage record into the consumer solves that ordering
gap without an early write or mutable ledger patch.

## Ownership behavior

Prior provenance is parsed through `ProvenanceLedgerRecord`.

Only `Execution` variants participate in ownership.

`AssignmentTransition` variants correctly remain pre-provider evidence and
cannot own usage.

The current execution record is chained to prior execution records for every
lookup.

`owner_at` therefore retains its existing behavior:

- inclusive second-resolution endpoints;
- no result in gaps;
- no result for other panes;
- one answer for duplicate same-ticket evidence;
- no result for conflicting different-ticket overlap.

The consumer never assigns a no-owner result to the current, last, shared, or
environment-derived ticket.

## Aggregation behavior

The consumer processes all valid capture rows for the current physical pane.

Each row whose unique owner equals the current ticket contributes its full
input and output values.

Multiple owned rows are summed.

The accumulator remains absent until the first owned capture.

This preserves:

- `None` for no measured usage;
- `Some(0)` for a measured zero side of a successful capture;
- concrete totals for one or more owned captures.

Both additions use `checked_add`.

Overflow cannot wrap into a false smaller measurement; the method fails closed
to null usage.

Cost stays null because adding a dollar value would fabricate a fact not
present in `CaptureRecord`.

## I/O and malformed data

A missing or unreadable capture ledger yields null usage.

A missing or unreadable provenance ledger yields an empty prior history, while
the current record can still own first-run captures.

JSONL lines are parsed independently.

A malformed row is skipped without hiding later valid append-only rows.

Syntactically valid but unattributable rows are not blended into any usage
total.

Their durable quarantine and activity surfacing are intentionally owned by
`T-043-03-02`.

## Acceptance evidence

The new plugin test is:

```text
provenance_recycled_pane_attributes_capture_sums_to_each_ticket
```

It uses physical pane 7 for both tickets.

Ticket A owns the earlier deterministic interval.

Ticket B owns a later disjoint interval.

The append-only capture input contains two rows for each ticket window.

A is expected to receive:

```text
tokens_in  = 30
tokens_out = 10
```

B is expected to receive:

```text
tokens_in  = 300
tokens_out = 100
```

After filing A, the test requires exactly one provenance row with A's identity
and totals.

After emitting B through `State::emit_provenance`, it requires exactly two
rows.

It rechecks the first row's A identity and original totals.

It checks the second row's B identity and distinct totals.

Those assertions simultaneously prove:

- captures are attributed by pane and time;
- multiple captures are summed per owner;
- A's captures do not blend into B;
- B's captures do not blend into A;
- B appends rather than overwrites A.

## Determinism note

The A fixture uses an explicitly closed past `ProvenanceRecord`, calls the real
private consumer, and appends it through the shared provenance writer.

B uses the full real `emit_provenance` path.

This avoids a sleep between two live emissions.

A sleep-free approach matters because provenance has one-second resolution and
ownership intervals use inclusive endpoints; immediate live A and B emissions
can legitimately share a boundary second and become ambiguous.

The deterministic intervals test intended disjoint ownership without relying
on wall-clock scheduling.

## Test coverage

### Focused acceptance

```text
cargo test -p lisa-plugin \
  provenance_recycled_pane_attributes_capture_sums_to_each_ticket
```

Result:

```text
1 passed; 0 failed
```

### Adjacent provenance behavior

```text
cargo test -p lisa-plugin provenance_
```

Result:

```text
9 passed; 0 failed
```

This includes the migrated Claude and Codex capture paths, missing capture
behavior, retry append preservation, terminal failure emission, append failure
handling, ticket-frontmatter non-mutation, and unset-ledger behavior.

### Ownership contract

```text
cargo test -p lisa-plugin owner_at
```

Result:

```text
2 passed; 0 failed
```

The owner tests cover disjoint recycled windows, inclusive boundaries, gaps,
other panes, duplicate same-owner evidence, and conflicting overlap.

### Formatting and diff integrity

```text
cargo fmt --all -- --check
git diff --check
```

Both passed.

### Full workspace

```text
cargo test --workspace
```

Passed with no failures.

Observed suite totals included:

- 14 CLI library tests;
- 266 CLI binary tests;
- 197 core unit tests;
- 378 plugin unit tests;
- all enabled integration and regression targets;
- no doc-test failures.

The declared real-Zellij environment-gated test remained ignored, not failed.

### Project gate

```text
just check
```

Passed.

It successfully completed:

- `cargo check -p lisa-plugin --target wasm32-wasip1`;
- `cargo test --workspace`.

The complete workspace therefore passed twice after implementation, and the
plugin compiled for its production WASM target.

## Source transaction

The ticket-owned source unit was committed with:

```text
lisa commit-ticket \
  --ticket-id T-043-03-01 \
  --message "fix(plugin): attribute captures by pane time" \
  --include crates/lisa-plugin/src/lib.rs
```

Commit:

```text
712de86ddd0997bb3208de49900c91666b407b87
```

Commit subject:

```text
fix(plugin): attribute captures by pane time
```

The commit contains exactly one source file.

Post-commit checks show:

- no uncommitted diff for `crates/lisa-plugin/src/lib.rs`;
- no ticket-owned staged entry in the ordinary index;
- no ticket-owned untracked source file.

No ordinary `git add`, broad add, or ordinary `git commit` was used.

## Scope boundaries preserved

The implementation does not change:

- the `CaptureRecord` schema;
- capture append semantics;
- CLI transcript parsing;
- provenance schema version or row shape;
- `owner_at` interval semantics;
- scheduler ownership state;
- thread teardown order;
- attempt lease authority;
- assignment-transition records;
- activity event vocabulary;
- dashboard display;
- ticket frontmatter.

## Open concerns and staged follow-up

`T-043-03-02` must quarantine syntactically valid captures that `owner_at`
cannot uniquely map, keyed by `session_id`, and raise an operator-visible
activity event.

This ticket deliberately leaves those rows out of all ticket totals and does
not create a fallback bucket.

`T-043-03-03` still owns the complete recorded six-overwrite field regression,
including quarantine and no-capture surfacing.

The consumer scans the provider capture ledger and provenance ledger at each
terminal emission.

That is simple and correct for the current append-log scale; no cache or
incremental cursor was introduced because either would require durable
invalidation semantics outside this ticket.

Malformed JSONL rows are skipped and do not currently produce operator-visible
events.

They cannot be quarantined by session ID when the shared capture schema cannot
be parsed. The successor ticket's stated acceptance concerns valid captures
with no owner rather than malformed input.

There is no dedicated unit test for `u64` aggregation overflow, malformed-row
continuation, or assignment-transition exclusion in this ticket.

The production branches are fail-closed and use the already-tested mixed
ledger enum, while the required recycled-pane behavior and both provider paths
are directly covered.

No issue in these gaps blocks this ticket's acceptance or the next staged
consumer extension.

## Final assessment

The implementation replaces the dishonest ticket-keyed read boundary with the
intended capture-to-ownership join.

It uses the durable pane timeline for previous occupants and the current
in-memory interval for the occupant being torn down.

The regression demonstrates distinct summed A and B usage on one recycled pane
and proves the later append leaves A intact.

The source is formatted, fully tested, WASM-checked, exactly committed, and
ready for Lisa's completion transaction.
