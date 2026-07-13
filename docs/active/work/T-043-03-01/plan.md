# Plan: attribute captures by pane time

## Objective

Replace legacy ticket-keyed usage ingestion with append-only capture
attribution through durable pane-time ownership and prove one recycled pane
files distinct summed usage on tickets A and B.

## Preconditions

Before editing source, confirm:

- `CaptureRecord` is public in `lisa_core::capture`;
- the CLI writes `.lisa/<client>/captures.jsonl`;
- `ownership::owner_at` is crate-visible;
- `ProvenanceLedgerRecord` separates execution and assignment rows;
- `emit_provenance` creates the current execution record before reading usage;
- the worktree's unrelated Lisa-managed changes are preserved.

These checks were completed during Research.

## Step 1: establish the source baseline

Inspect the exact diff for `crates/lisa-plugin/src/lib.rs` before editing.

Verify it has no pre-existing ordinary worktree change.

Run the focused existing ownership and provenance usage tests if the baseline is
uncertain.

Record any baseline failure before implementation rather than attributing it to
the ticket.

Verification:

- `git diff -- crates/lisa-plugin/src/lib.rs` is empty;
- `cargo test -p lisa-plugin owner_at` passes.

## Step 2: update production imports and path documentation

Add `ProvenanceLedgerRecord` to the core provenance import group.

Update `State::codex_dir` and `State::claude_dir` comments to name
`captures.jsonl` append logs.

Do not alter their types, initialization, or provider selection.

Verification:

- imports are formatted in the existing style;
- no unused import remains after the consumer is implemented;
- comments no longer describe ticket-keyed usage artifacts.

## Step 3: pass the current ownership interval into the consumer

Keep base `ProvenanceRecord` construction unchanged.

Change `emit_provenance` to call `read_usage(client, &record)`.

Keep usage application before `append_record`.

Do not append the current record early.

Verification:

- the current record contains complete start/end/pane/ticket facts at the call;
- only one terminal `append_record` call remains;
- existing write-error behavior remains intact.

## Step 4: replace the legacy reader

Change `read_usage` to accept the current `ProvenanceRecord`.

Choose the provider directory from `AgentClient`.

Read `captures.jsonl` instead of `<ticket>.usage.json`.

Delete parsing of nested `usage` JSON and the call to
`provenance::extract_usage` from this plugin path.

Update the method documentation to describe the new join.

Verification:

- no `usage.json` reference remains in plugin production code;
- no provider-specific parsing branch is introduced;
- the output tuple remains compatible with record construction.

## Step 5: load durable execution ownership

Read `self.ledger_path` best-effort.

Parse each non-empty line as `ProvenanceLedgerRecord`.

Collect only execution variants.

Ignore assignment-transition variants as non-ownership evidence.

Skip malformed lines.

Treat an absent ledger as an empty prior history.

Verification:

- a first run can attribute through the current record alone;
- mixed assignment rows do not become ownership intervals;
- legacy-compatible execution rows deserialize through the shared enum.

## Step 6: parse, attribute, and sum captures

Parse capture JSONL one row at a time as `CaptureRecord`.

Skip malformed rows.

Filter to the current physical pane.

For each row, call `ownership::owner_at` over prior execution rows chained with
the current record.

Keep only captures whose unique owner equals the current ticket.

Aggregate input and output totals with checked arithmetic.

Return `Some` totals when at least one row contributes.

Return null tokens on no contributions or overflow.

Always return null cost.

Verification:

- captures on another pane cannot contribute;
- captures in an earlier ticket's window cannot contribute to the current one;
- unattributable captures are not assigned to a fallback;
- one-sided measured zero remains a concrete zero when a capture exists;
- overflow cannot wrap.

## Step 7: add the recycled-pane regression

Add a plugin test beside provenance usage tests.

Use one temporary state and one fixed physical pane.

Create two disjoint historical intervals without sleeping.

Append at least two captures for A and two for B through
`append_capture_record`.

Emit A before installing B.

Read A's row and assert its sum.

Remove A's active thread.

Install B on the same pane with a later start.

Emit B.

Read the complete ledger.

Verification:

- row count is two;
- row zero remains A;
- row zero retains A's exact sum;
- row one is B;
- row one has B's exact sum;
- the chosen totals make blending detectable.

## Step 8: update obsolete usage tests

Existing plugin tests that write `<ticket>.usage.json` will no longer represent
production input.

Replace or remove those fixtures while preserving useful coverage:

- missing capture data yields null tokens/cost;
- capture data flows for provider selection;
- cost remains null.

Prefer the new recycled-pane test as the main cross-ticket acceptance proof.

If a focused single-provider test remains useful, write a `CaptureRecord` in the
shared schema rather than raw legacy JSON.

Verification:

- no test in the edited file relies on `<ticket>.usage.json`;
- both Claude and Codex directory selection remain covered either directly or
  through existing/new tests;
- null-usage behavior remains covered.

## Step 9: format and inspect

Run `cargo fmt --all -- --check` first to identify formatting needs.

Run `cargo fmt --all` if required; inspect every resulting changed path.

Only `crates/lisa-plugin/src/lib.rs` may be ticket-owned.

Review the diff for:

- accidental broad formatting;
- stale docs;
- legacy path references;
- ownership lookup correctness;
- checked aggregation;
- deterministic timestamps;
- test clarity.

Verification:

- `cargo fmt --all -- --check` exits zero;
- exact source diff is scoped to the designed file.

## Step 10: run focused tests

Run the new regression by its exact test name.

Run the existing ownership suite.

Run provenance-focused plugin tests to catch adjacent behavior changes.

Suggested commands:

```text
cargo test -p lisa-plugin provenance_recycled_pane_attributes_capture_sums_to_each_ticket
cargo test -p lisa-plugin owner_at
cargo test -p lisa-plugin provenance_
```

Verification:

- the acceptance test passes;
- ownership boundary tests remain green;
- terminal emission, retry, fencing, and missing-usage coverage remain green.

## Step 11: run project gates

Run the repository's standard native workspace test gate:

```text
cargo test --workspace
```

Run the quick project check when available:

```text
just check
```

`just check` includes the WASM check and native tests per project guidance.

If environment limitations prevent a gate, capture the exact command and error
in `progress.md` and determine whether the issue is ticket-owned.

Verification:

- workspace tests pass;
- WASM compilation remains valid;
- no ticket-owned warning or failure remains.

## Step 12: write implementation progress

Create attempt-private `progress.md`.

Record:

- completed source changes;
- acceptance mapping;
- test commands and outcomes;
- any deviation from this plan;
- exact source ownership;
- unrelated worktree changes intentionally preserved.

Do not write to `docs/active/work/T-043-03-01` directly.

## Step 13: commit the meaningful source unit

Confirm no ordinary index entries are owned by this ticket.

Commit only the exact source path through Lisa:

```text
lisa commit-ticket \
  --ticket-id T-043-03-01 \
  --message "fix(plugin): attribute captures by pane time" \
  --include crates/lisa-plugin/src/lib.rs
```

Do not use `git add`, `git add -A`, or ordinary `git commit`.

Verification:

- command succeeds and reports a commit;
- `git diff -- crates/lisa-plugin/src/lib.rs` is empty afterward;
- the ordinary index has no ticket-owned entry;
- unrelated managed files remain untouched.

## Step 14: review committed result

Inspect the committed diff with `git show` scoped to the source path.

Confirm the current `HEAD` contains the ticket source unit.

Re-run a focused test after commit if the transaction changed or rebased the
worktree unexpectedly.

Review for:

- wrong-ticket attribution;
- loss of current interval;
- assignment-transition misuse;
- accidental cost fabrication;
- old row overwrite;
- hidden fallback ownership;
- source files left dirty.

## Step 15: produce Review artifacts

Write attempt-private `review.md` with:

- source change summary;
- files modified/created/deleted;
- architecture and behavior;
- acceptance evidence;
- test coverage and exact results;
- open concerns and intentional staged boundaries;
- commit and worktree status.

Write `review-disposition.json` with exactly one valid shape.

Use pass only if implementation, tests, commit, and source cleanliness are all
complete.

Use block with a non-empty actionable reason if a correctness or ownership
problem remains.

Do not update ticket phase or status.

After both artifacts exist, remain on this ticket and stop.
