# Plan — T-048-01-01 structured-block-schema

## Operating constraints

Work only on ticket-owned source paths and this attempt's private artifact
directory.

Do not edit the ticket's phase or status. Do not publish artifacts directly to
`docs/active/work/T-048-01-01/`.

Do not use ordinary `git add`, `git add -A`, or `git commit`.

Commit each meaningful source unit with `lisa commit-ticket`, the exact ticket
ID, and exact repository-relative include paths.

Preserve existing ordinary-worktree changes in `.lisa`, other tickets, and
other work directories.

## Step 1: establish the baseline

Inspect `git status --short` and record all pre-existing modified/untracked
paths mentally before source edits.

Run focused baseline tests:

```sh
cargo test -p lisa-core disposition::tests --no-fail-fast
cargo test -p lisa-core provenance::tests --no-fail-fast
```

Verification:

- existing parser tests pass;
- existing ledger tests pass;
- failures, if any, are recorded before implementation rather than attributed
  to new code.

## Step 2: add the remedy owner type

Modify `crates/lisa-core/src/disposition.rs`.

Import serde derive traits.

Add public `RemedyOwner` with agent, operator, and world variants and lowercase
serde representation.

Verification:

- each wire string deserializes to the intended variant;
- enum serialization is suitable for provenance rows;
- invalid strings cannot construct the enum through serde.

## Step 3: extend the parsed block shape

Modify `ReviewDisposition::Block` to include:

- unchanged raw `reason`;
- typed `remedy_owner`;
- `ask`;
- optional `steps`;
- optional `check`;
- `unstructured` flag.

Keep `Pass` and `Invalid` unchanged.

Verification:

- the type retains clone and equality derives;
- every semantic block has a concrete owner and ask;
- only optional authoring fields remain optional in memory.

## Step 4: implement structured parsing and fallback

Preserve current outer JSON validation.

After validating a non-empty block reason, parse the remaining structural
fields.

Structured success requires:

- recognized owner string;
- non-whitespace ask;
- absent or valid string-array steps;
- absent or valid non-whitespace check.

On any structural failure, build one operator-owned fallback with exact raw
reason as ask, no steps/check, and `unstructured: true`.

On success, preserve all string bytes and set `unstructured: false`.

Verification:

- legacy block stays Block;
- malformed outer document stays Invalid;
- partial malformed structure never leaks into the fallback;
- parser contains no process execution API.

## Step 5: expand disposition tests

Add unit cases for agent, operator, and world owners.

Test absent optional fields and present steps/check.

Test exact legacy raw reason preservation.

Add a malformed matrix covering:

- missing owner;
- unknown/non-string owner;
- missing/blank/non-string ask;
- non-array steps;
- non-string or blank step;
- non-string or blank check.

For every malformed structure assert the complete fallback, including discarded
otherwise-valid optional values.

Add hostile-looking check content and sentinel assertions proving parsing did
not execute it.

Run:

```sh
cargo test -p lisa-core disposition::tests --no-fail-fast
```

## Step 6: update block consumers

Modify direct constructors in:

- `crates/lisa-core/src/completion.rs`;
- `crates/lisa-core/tests/completion_state_machine.rs`.

Use the exact semantic shape produced by a legacy block.

Modify direct reason patterns in `crates/lisa-plugin/src/lib.rs` to ignore new
fields with `..`.

Do not alter rendered reason text or completion behavior.

Run:

```sh
cargo test -p lisa-core completion::tests --no-fail-fast
cargo test -p lisa-core --test completion_state_machine --no-fail-fast
cargo check -p lisa-plugin
```

Verification:

- pass remains the only eligible completion disposition;
- blocks remain completion rejections;
- plugin diagnostics still use reason;
- no scheduler branch uses owner yet.

## Step 7: format and inspect disposition unit

Run `cargo fmt --all -- --check`; if it reports changes, run `cargo fmt --all`
and inspect only ticket-owned path diffs.

Inspect:

```sh
git diff -- crates/lisa-core/src/disposition.rs \
  crates/lisa-core/src/completion.rs \
  crates/lisa-core/tests/completion_state_machine.rs \
  crates/lisa-plugin/src/lib.rs
```

Confirm no unrelated lines were reformatted.

## Step 8: commit disposition unit through Lisa

Run:

```sh
lisa commit-ticket \
  --ticket-id T-048-01-01 \
  --message "feat(core): parse structured review blocks" \
  --include crates/lisa-core/src/disposition.rs \
  --include crates/lisa-core/src/completion.rs \
  --include crates/lisa-core/tests/completion_state_machine.rs \
  --include crates/lisa-plugin/src/lib.rs
```

Verification:

- Lisa reports a successful isolated commit;
- included paths have no ticket-owned modifications afterward;
- ordinary staged entries, if any, remain untouched;
- unrelated worktree changes remain present.

## Step 9: extend provenance schema

Modify `crates/lisa-core/src/provenance.rs`.

Import `RemedyOwner`.

Bump `SCHEMA_VERSION` to 4.

Add `ParkingTransitionType` with park/unpark serde names.

Add `ParkingTransitionRecord` carrying schema version, transition type, ticket,
attempt lease, remedy owner, start/end epoch seconds, and duration.

Extend `ProvenanceLedgerRecord` with a parking-transition variant before the
historical execution variant.

Verification:

- old structs and JSON shapes are unchanged;
- new variant is distinguishable from assignment and execution rows;
- all fields required by the acceptance criteria are public and typed.

## Step 10: add the parking append entry point

Add `append_parking_transition_record` beside the existing typed append
functions.

Delegate to `append_serialized` without changing its implementation.

Verification:

- parent creation, compact JSON, newline framing, true append, and error
  semantics are shared with existing records;
- callers cannot append arbitrary untyped values through this API.

## Step 11: expand provenance tests

Add a stable sample parking transition helper.

Test compact serialization and round-trip for park and unpark.

Test appending park and unpark to one ledger and replaying both typed rows.

Expand mixed replay to prove:

- schema-v2 execution remains `Execution`;
- schema-v3 assignment remains `AssignmentTransition`;
- schema-v4 park/unpark become `ParkingTransition`;
- owner, lease, timestamps, and duration survive replay.

Update only current-version assertions that construct records through the
constant. Keep historical fixture version assertions explicit.

Run:

```sh
cargo test -p lisa-core provenance::tests --no-fail-fast
```

## Step 12: repair exhaustive ledger readers

Compile the workspace to find exhaustive matches on `ProvenanceLedgerRecord`.

In `crates/lisa-plugin/src/lib.rs`, update execution-only filter maps to return
`None` for parking transitions and update test matches where required.

In CLI code, change only exhaustive matches. Existing assignment-only `if let`
readers need no behavioral change.

Do not add scheduler emissions in this ticket.

Run:

```sh
cargo check --workspace
```

## Step 13: format and inspect provenance unit

Run formatting and inspect exact diffs:

```sh
cargo fmt --all -- --check
git diff -- crates/lisa-core/src/provenance.rs crates/lisa-plugin/src/lib.rs
```

Confirm the plugin changes are match exhaustiveness only.

## Step 14: commit provenance unit through Lisa

If `crates/lisa-plugin/src/lib.rs` was already committed in the first unit and
has new provenance-only changes, include it again exactly.

Run:

```sh
lisa commit-ticket \
  --ticket-id T-048-01-01 \
  --message "feat(core): add parking transition provenance" \
  --include crates/lisa-core/src/provenance.rs \
  --include crates/lisa-plugin/src/lib.rs
```

If compilation requires another exact consumer path, add only that named path
after inspecting its diff.

Verification:

- isolated commit succeeds;
- no ticket-owned source remains modified, staged, or untracked.

## Step 15: focused verification after commits

Run:

```sh
cargo test -p lisa-core disposition::tests --no-fail-fast
cargo test -p lisa-core provenance::tests --no-fail-fast
cargo test -p lisa-core completion::tests --no-fail-fast
cargo test -p lisa-core --test completion_state_machine --no-fail-fast
```

Then run plugin tests focused by relevant names if discoverable. At minimum,
compile the plugin test targets.

Record commands, pass counts, and any warnings in `progress.md`.

## Step 16: workspace verification

Run:

```sh
cargo test --workspace --no-fail-fast
```

If the workspace suite is too slow or an environment dependency fails, isolate
the failure, run the nearest relevant subset, and record the exact limitation.
Do not claim a pass for an unverified behavior.

Run final formatting check:

```sh
cargo fmt --all -- --check
```

## Step 17: source cleanliness audit

Inspect:

```sh
git status --short
git diff --check
git log --oneline -5
```

Compare status with the baseline.

Requirements:

- ticket-owned source has no staged, modified, or untracked residue;
- unrelated and Lisa-managed baseline changes are untouched;
- each ticket source unit appears in a Lisa-created commit;
- attempt artifacts remain in the private work directory for admission.

## Step 18: Review artifacts

Write `progress.md` throughout implementation, including deviations before
acting on them.

Write `review.md` with:

- source file inventory;
- contract behavior;
- fallback behavior;
- provenance version/row behavior;
- test commands and results;
- compatibility assessment;
- open concerns and explicit out-of-scope work;
- commit IDs.

If all acceptance criteria pass and source is clean, write exactly:

```json
{"disposition":"pass","reason":null}
```

to private `review-disposition.json`.

If blocked, write the exact block shape with a non-empty actionable reason.

After both Review artifacts exist, remain on T-048-01-01 and stop. Lisa owns
publication, Done transition, completion commit, and seat release.
