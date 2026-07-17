# Plan — T-049-05-02 plain-ask-floor

## Preconditions

- [x] Read `AGENTS.md` and the canonical `CLAUDE.md` project context.
- [x] Read the complete assignment and RDSPI workflow.
- [x] Confirm ticket `T-049-05-02` starts in Research.
- [x] Inspect current worktree changes before claiming ownership.
- [x] Confirm all seven planned source paths have no pre-existing diff.
- [x] Identify unrelated `.lisa`, ticket, and `T-049-06-01` changes to preserve.
- [x] Complete Research, Design, and Structure artifacts privately.

## Step 1 — establish the core plain-ask projection

File:

- `crates/lisa-core/src/parking.rs`

Actions:

1. Add the exact public `LEGACY_BLOCK_ASK` constant.
2. Add `reason: String` to `ParkedRemedy`.
3. Destructure `reason` and `unstructured` from block dispositions.
4. Select `LEGACY_BLOCK_ASK` when `unstructured` is true.
5. Preserve the structured `ask` when `unstructured` is false.
6. Preserve raw `reason` for both paths.
7. Leave ticket filtering, pass/invalid exclusion, sorting, and checks unchanged.

Tests:

1. Add `reason` to structured expected remedies.
2. Change the legacy expectation to the standard fallback ask.
3. Pin preservation of whitespace/raw reason.
4. Keep operator ownership and absent check assertions.

Verification:

```text
cargo test -p lisa-core parking
```

Pass condition:

- all parking tests pass;
- parser tests remain unchanged and passing;
- the fallback is defined in only one production location.

## Step 2 — render lead then detail in `lisa status`

File:

- `crates/lisa-cli/src/status.rs`

Actions:

1. Refactor `waiting_on_you_lines` to emit zero or two lines per remedy.
2. Preserve operator/world filtering.
3. Preserve the world-owned automatic-check suffix.
4. Emit the ask as the first ticket line.
5. Emit `Reviewer's note: <reason>` as the immediately following line.
6. Keep the section heading and section ordering unchanged.

Tests:

1. Extend current structured fixtures with distinct reasons.
2. Pin exact four-line output for operator and world remedies.
3. Add the full T-046-06-03 field reason as a legacy regression.
4. Pin the standard sentence as the regression entry's first line.
5. Pin the raw reason as its second line.
6. Assert the raw reason is absent from the first line.

Verification:

```text
cargo test -p lisa-cli status::tests::waiting
```

Pass condition:

- exact string tests pass;
- no technical field prose can occupy the first rendered entry line.

## Step 3 — extend dashboard item semantics and rendering

File:

- `crates/lisa-plugin/src/ui.rs`

Actions:

1. Add required `reason: String` to `WaitingItem`.
2. Render the existing lead line first.
3. Render the labeled raw reason second.
4. Preserve the world suffix.
5. Preserve empty-state and section-order behavior.

Tests:

1. Add reason values to all UI fixtures.
2. Pin structured ask/reason ordering.
3. Add a complete field-regression fixture.
4. Assert exact output vector indices.
5. Assert the field reason does not appear in the lead element.
6. Keep internal-schema-word absence assertions.

Verification:

```text
cargo test -p lisa-plugin ui::tests::waiting
```

Pass condition:

- dashboard output has the same lead/detail invariant as CLI status;
- section ordering remains Waiting on you before other operations content.

## Step 4 — carry reason through the plugin adapter

File:

- `crates/lisa-plugin/src/lib.rs`

Actions:

1. Copy `remedy.reason` into operator `WaitingItem` values.
2. Copy `remedy.reason` into world `WaitingItem` values.
3. Keep `checks_on_own` derivation unchanged.
4. Keep agent-owned filtering unchanged.
5. Update exact `WaitingItem` literals throughout plugin tests.

Tests:

1. Structured projection expects authored ask plus engineering reason.
2. Orphan legacy field fixture expects standard ask plus raw field reason.
3. Other projection fixtures retain their actual reason values.

Verification:

```text
cargo test -p lisa-plugin dashboard_projection
cargo test -p lisa-plugin orphaned_legacy_block
```

Pass condition:

- production state supplies both semantic strings to the pure renderer;
- field fixture proves the raw reason is demoted rather than discarded.

## Step 5 — verify and commit rendering unit

Review exact diff:

```text
git diff -- crates/lisa-core/src/parking.rs crates/lisa-cli/src/status.rs crates/lisa-plugin/src/ui.rs crates/lisa-plugin/src/lib.rs
```

Run formatter check after applying normal formatting:

```text
cargo fmt --all -- --check
```

Run focused tests:

```text
cargo test -p lisa-core parking
cargo test -p lisa-cli status::tests
cargo test -p lisa-plugin waiting_section
cargo test -p lisa-plugin dashboard_projection
cargo test -p lisa-plugin orphaned_legacy_block
```

Commit only the rendering unit:

```text
lisa commit-ticket \
  --ticket-id T-049-05-02 \
  --message "Lead parked blocks with a plain ask" \
  --include crates/lisa-core/src/parking.rs \
  --include crates/lisa-cli/src/status.rs \
  --include crates/lisa-plugin/src/ui.rs \
  --include crates/lisa-plugin/src/lib.rs
```

After transaction:

- confirm those four paths no longer appear in `git status --short`;
- do not disturb unrelated ordinary-index or working-tree entries.

## Step 6 — strengthen the Review authoring rule

Files:

- `docs/knowledge/rdspi-workflow.md`
- `crates/lisa-cli/data/rdspi-workflow.md`

Actions:

1. Extend the existing `ask` guidance in the canonical document.
2. Say explicitly that the reader may be a bystander.
3. Require the action in plain language.
4. Assign technical jargon to `reason` or `steps`, not `ask`.
5. Quote the complete T-046-06-03 field text as the counter-example.
6. Apply byte-identical workflow-body wording to the embedded copy.
7. Preserve all existing Review schema and release examples.

Manual copy check:

- read the new sentences aloud as an operator instruction;
- reject internal shorthand or unexplained subsystem language in new copy;
- retain the field quote as deliberately bad copy, clearly labeled as such.

## Step 7 — pin bundled contract wording

File:

- `crates/lisa-cli/src/templates.rs`

Actions:

1. Extend `test_review_disposition_contract_is_injected`.
2. Assert the bystander/plain-action/jargon rule.
3. Assert the field counter-example is present.
4. Leave byte-sync test logic unchanged.

Verification:

```text
cargo test -p lisa-cli templates::tests::test_rdspi_workflow_embedded
cargo test -p lisa-cli templates::tests::test_review_disposition_contract_is_injected
```

Pass condition:

- canonical and rendered embedded workflow are byte-equal;
- generated agent context contains both the positive rule and bad field example.

## Step 8 — verify and commit authoring unit

Review exact diff:

```text
git diff -- docs/knowledge/rdspi-workflow.md crates/lisa-cli/data/rdspi-workflow.md crates/lisa-cli/src/templates.rs
```

Commit only the authoring unit:

```text
lisa commit-ticket \
  --ticket-id T-049-05-02 \
  --message "Teach reviewers to write bystander-ready asks" \
  --include docs/knowledge/rdspi-workflow.md \
  --include crates/lisa-cli/data/rdspi-workflow.md \
  --include crates/lisa-cli/src/templates.rs
```

After transaction:

- confirm those three paths no longer appear in `git status --short`;
- preserve all unrelated changes.

## Step 9 — whole-ticket verification

Run:

```text
cargo fmt --all -- --check
cargo test --workspace
just check
```

If `just check` repeats workspace tests, record both results rather than skipping
the command; repository guidance names it as the quick WASM-plus-tests check.

Inspect:

```text
git status --short
git log -2 --oneline
```

Pass conditions:

- formatting passes;
- all workspace tests pass;
- WASM check passes;
- both ticket-owned commits exist;
- no ticket-owned source path is staged, modified, or untracked;
- unrelated pre-existing state remains present and untouched.

## Step 10 — progress artifact

Create private `progress.md` with:

- each completed step;
- exact files changed;
- tests and outcomes;
- commit IDs returned by `lisa commit-ticket`;
- deviations and rationale, or an explicit `none`;
- final source cleanliness evidence.

Do not include private artifacts in source transactions.

## Step 11 — Review

Create private `review.md` covering:

- behavioral result;
- per-file changes;
- acceptance-criteria mapping;
- test coverage;
- copy/brand-voice assessment;
- open concerns or limitations;
- commit and cleanliness evidence.

Create private `review-disposition.json` with exactly:

```json
{"disposition":"pass","reason":null}
```

Use `pass` only if every required check succeeds and no ticket-owned source
change remains outside Lisa's isolated transaction.

## Failure handling

- If a test exposes a ticket-owned defect, document the deviation in progress,
  fix it, rerun focused tests, and use another exact-path ticket transaction.
- If a relevant file gains an overlapping unrelated diff, stop editing that
  file and identify the ownership conflict rather than overwriting it.
- If `lisa commit-ticket` fails, diagnose and retry it; do not use ordinary Git
  staging or committing as a fallback.
- If a required tool or external state makes completion impossible, write a
  structured block disposition with a plain bystander-ready ask.

## Completion boundary

After both Review artifacts exist, remain on `T-049-05-02` and stop.
Do not update phase/status, publish shared work artifacts, run completion, or
begin another ticket. Lisa owns final publication and seat release.
