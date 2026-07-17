# Progress — T-049-05-02 plain-ask-floor

## Status

Implement is complete.
All ticket-owned source and documentation changes are committed through
`lisa commit-ticket` with exact repository-relative include paths.
All planned focused checks pass.
The full workspace suite passes.
The repository's WASM-inclusive `just check` passes.

## Completed phase inputs

- [x] Read repository and assignment guidance.
- [x] Read ticket `T-049-05-02`.
- [x] Read the canonical RDSPI workflow.
- [x] Wrote private `research.md`.
- [x] Wrote private `design.md`.
- [x] Wrote private `structure.md`.
- [x] Wrote private `plan.md`.
- [x] Preserved unrelated worktree state.

## Step 1 — core parked-remedy projection

Completed in `crates/lisa-core/src/parking.rs`.

Changes:

- Added public `LEGACY_BLOCK_ASK` with the ticket-pinned standard sentence.
- Added `reason: String` to `ParkedRemedy`.
- Retained parser behavior and raw legacy bytes.
- Used `ReviewDisposition::Block.unstructured` inside collection.
- Structured blocks preserve their authored ask.
- Legacy blocks project the shared standard ask.
- Both kinds preserve the raw reason for detail rendering.
- Existing blocked-ticket filters and ticket ordering are unchanged.
- Existing remedy owner and world-check behavior are unchanged.

Tests:

- Structured operator remedy pins ask and reason.
- Structured world remedy pins ask, reason, and check.
- Legacy remedy pins the standard ask.
- Legacy remedy pins untouched whitespace in raw reason.
- Passing, invalid, missing, and open-ticket exclusions remain covered.

Focused result:

```text
cargo test -p lisa-core parking
6 passed; 0 failed
```

The filter also matched three provenance test names; all passed.

## Step 2 — CLI Waiting on you rendering

Completed in `crates/lisa-cli/src/status.rs`.

Changes:

- `waiting_on_you_lines` now returns two lines per visible remedy.
- The ticket ID and plain ask remain the first ticket line.
- The raw reason follows as `Reviewer's note: ...`.
- World-owned asks retain `Lisa checks on its own.`.
- Agent-owned remedies remain hidden from Waiting on you.
- The section still appears before the DAG summary.

Tests:

- Structured operator lead/detail strings are exact.
- Structured world lead/detail strings are exact.
- Agent filtering remains exact.
- The complete T-046-06-03 field reason is a regression fixture.
- The field reason is explicitly absent from the entry's first line.
- The field reason is explicitly present on the following line.

Focused result:

```text
cargo test -p lisa-cli status::tests
14 passed; 0 failed
```

## Step 3 — dashboard Waiting on you rendering

Completed in `crates/lisa-plugin/src/ui.rs`.

Changes:

- Added required `reason` data to `WaitingItem`.
- The renderer emits ask before raw reviewer note.
- World automatic-check copy is unchanged.
- Empty-state behavior is unchanged.
- Waiting on you remains ahead of attention and thread sections.

Tests:

- Structured operator and world strings remain pinned.
- Both structured reasons are now pinned beneath their asks.
- The full T-046-06-03 field text is pinned.
- Exact output indices prove the fallback is first and reason is second.
- The raw reason is explicitly absent from the lead line.

Focused results:

```text
cargo test -p lisa-plugin waiting_section
3 passed; 0 failed

cargo test -p lisa-plugin legacy_field_block_never_puts_the_raw_reason_first
1 passed; 0 failed
```

## Step 4 — plugin adapter

Completed in `crates/lisa-plugin/src/lib.rs`.

Changes:

- Operator-owned parked remedies copy `reason` into `WaitingItem`.
- World-owned parked remedies copy `reason` into `WaitingItem`.
- Agent-owned filtering remains unchanged.
- Structured adapter fixture expects authored ask plus raw reason.
- Orphaned legacy field fixture expects standard ask plus field reason.

Focused results:

```text
cargo test -p lisa-plugin dashboard_projection
1 passed; 0 failed

cargo test -p lisa-plugin orphaned_legacy_block
1 passed; 0 failed
```

## Step 5 — rendering commit

Committed through Lisa's isolated transaction:

```text
fa8533df98001b152f04564b731e6f766f95dd41
Lead parked blocks with a plain ask
```

Exact includes:

- `crates/lisa-core/src/parking.rs`
- `crates/lisa-cli/src/status.rs`
- `crates/lisa-plugin/src/ui.rs`
- `crates/lisa-plugin/src/lib.rs`

No ordinary Git staging or commit command was used.

## Step 6 — Review authoring contract

Completed in both workflow copies:

- `docs/knowledge/rdspi-workflow.md`
- `crates/lisa-cli/data/rdspi-workflow.md`

Changes:

- Added an explicit bystander rule.
- Requires the action to be said plainly.
- Assigns subsystem names, measurements, and jargon to `reason` or `steps`.
- Prohibits that technical detail from becoming the `ask`.
- Quotes the complete T-046-06-03 disposition as the counter-example.
- Preserves the existing release bad/good example.

Brand-voice check:

- Production fallback sentence is plain and tells the reader what to do.
- `Reviewer's note` labels technical detail as context.
- Workflow rule uses direct verbs: write, say, keep, never use.
- Technical field prose appears only as a clearly labeled bad example.

## Step 7 — embedded contract assertions

Completed in `crates/lisa-cli/src/templates.rs`.

Changes:

- Pinned the bystander/plain-action/jargon sentence.
- Pinned the complete field counter-example.
- Retained the byte-for-byte bundled workflow equality test.

Focused results:

```text
cargo test -p lisa-cli templates::tests::test_rdspi_workflow_embedded
1 passed; 0 failed

cargo test -p lisa-cli templates::tests::test_review_disposition_contract_is_injected
1 passed; 0 failed
```

## Step 8 — authoring commit

Committed through Lisa's isolated transaction:

```text
2eb443295bba929e536e1d914269e72b82719c95
Teach reviewers to write bystander-ready asks
```

Exact includes:

- `docs/knowledge/rdspi-workflow.md`
- `crates/lisa-cli/data/rdspi-workflow.md`
- `crates/lisa-cli/src/templates.rs`

## Plan deviation — black-box status snapshots

The initial plan identified unit rendering tests but did not list the existing
black-box file `crates/lisa-cli/tests/parked_ux.rs` as an owned change.

The first full workspace run correctly exposed two stale assertions there.
They expected a blank line immediately after the ask and expected the structured
reason to be hidden. The acceptance contract requires the reason to remain
visible below the ask, so those assertions represented prior behavior.

Correction:

- Updated structured operator snapshot to require ask then reason.
- Updated structured world snapshot to require ask then reason.
- Preserved checks that `steps`, schema fields, and ownership internals stay hidden.
- Added a real `lisa status` field regression with the complete legacy reason.
- Pinned the shared fallback sentence ahead of that field reason.

Focused result:

```text
cargo test -p lisa-cli --test parked_ux
13 passed; 0 failed
```

Correction commit:

```text
ea3dd1821e091525ea1a69ca99cee7526d6916cd
Pin parked status lead and detail order
```

Exact include:

- `crates/lisa-cli/tests/parked_ux.rs`

## Whole-ticket verification

Formatting:

```text
cargo fmt --all -- --check
passed
```

Workspace:

```text
cargo test --workspace
passed
```

The suite includes:

- 21 `lisa-cli` library tests;
- 346 `lisa` CLI unit tests;
- all CLI integration suites, including 13 parked UX tests;
- 232 `lisa-core` tests;
- 429 `lisa-plugin` tests;
- core state-machine and recorded-regression tests;
- documentation tests;
- one intentionally ignored real-Zellij boundary test.

Repository check:

```text
just check
passed
```

This passed:

- `cargo check -p lisa-plugin --target wasm32-wasip1`;
- the repeated full workspace test suite.

One first `just check` attempt saw the unrelated runtime checksum test fail.
That test had passed in the preceding full workspace run, passed immediately
when rerun alone, and passed in the successful full `just check` rerun. No
runtime source was changed; the event is recorded as a transient test result.

## Source cleanliness

`git status --short -- <all eight ticket-owned paths>` produced no output.
Therefore no ticket-owned source file is staged, modified, or untracked.

The remaining worktree entries are Lisa-managed state, ticket transitions, and
published work artifacts. They were not included in any ticket source commit.

## Remaining Implement work

None.

Next phase: Review.
