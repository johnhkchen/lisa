# Review — T-049-05-02 plain-ask-floor

## Disposition

Pass.

The ticket's rendering and authoring requirements are implemented, committed,
and verified. No ticket-owned source path remains outside Lisa's isolated
commit history.

## Outcome

Parked tickets now have a reliable human-first reading order on both operator
surfaces:

1. A plain sentence the operator can act on.
2. The reviewer's original technical note as labeled context.

Structured blocks lead with their authored `ask`.
Legacy blocks lead with the exact standard sentence:

> This ticket needs a decision from you. The reviewer's note is below — you can paste it to your coding agent.

The raw Review `reason` follows under `Reviewer's note:`.
The T-046-06-03 field disposition is never the first ticket line.

Review authors now receive an explicit bystander rule and the complete field
text as a counter-example in both the canonical and bundled workflow copies.

## Files changed

### `crates/lisa-core/src/parking.rs`

- Added the shared `LEGACY_BLOCK_ASK` constant.
- Added raw `reason` to the `ParkedRemedy` projection.
- Preserved structured asks unchanged.
- Replaced unstructured fallback asks at the human-facing projection boundary.
- Preserved raw reasons exactly for second-line rendering.
- Added core regression coverage for structured and legacy projections.

Assessment:

- Correct boundary: parser validation semantics remain lossless and unchanged.
- Correct sharing: status and plugin consume one fallback string source.
- Correct scope: scheduling, ownership, checks, and sorting did not change.

### `crates/lisa-cli/src/status.rs`

- Changed Waiting on you entries from one line to lead/detail pairs.
- Kept ticket ID and ask on the lead line.
- Kept the world-owned automatic-check suffix.
- Added the labeled raw reason immediately below.
- Added the full field disposition regression fixture.
- Explicitly asserts that fixture is absent from the lead line.

Assessment:

- The Waiting on you section remains ahead of DAG output.
- Agent-owned remedies remain excluded.
- The renderer does not rewrite or summarize reviewer evidence.

### `crates/lisa-plugin/src/ui.rs`

- Added `reason` to the self-contained `WaitingItem` type.
- Rendered ask first and reviewer note second.
- Preserved heading, suffix, empty state, and operations ordering.
- Added exact index-based legacy field regression coverage.
- Updated structured operator/world rendering fixtures.

Assessment:

- The exact output ordering is tested rather than inferred from substring presence.
- ANSI styling does not obscure the entry string assertions.
- No schema jargon was introduced into the visible lead.

### `crates/lisa-plugin/src/lib.rs`

- Carries raw reason from core projection into dashboard state.
- Updated structured adapter expectation.
- Updated the existing orphaned T-046-06-03 fixture.
- That fixture now expects standard ask plus untouched raw reason.

Assessment:

- This proves production state can supply the pure UI renderer correctly.
- World check eligibility still uses the unchanged core `check` field.
- Park/reconcile behavior remains unchanged.

### `crates/lisa-cli/tests/parked_ux.rs`

- Updated black-box structured operator output.
- Updated black-box structured world output.
- Preserved hiding of steps and schema internals.
- Added a real `lisa status` legacy field fixture.
- Pinned standard ask before the complete raw field reason.

Assessment:

- This closes the gap between formatter unit tests and executable CLI output.
- The integration fixture uses the actual legacy JSON shape.
- The output prefix includes the next `DAG:` section, pinning section boundaries.

### `docs/knowledge/rdspi-workflow.md`

- Added the bystander ask rule.
- Requires a plain statement of what to do.
- Directs subsystem names, measurements, and jargon to reason/steps.
- Quotes the complete field disposition as a counter-example.

Assessment:

- The new production instruction is plain and verb-forward.
- The jargon wall appears only as explicitly bad source material.
- Existing remedy-owner and check guidance remains intact.

### `crates/lisa-cli/data/rdspi-workflow.md`

- Mirrors the canonical Review-body edit exactly.

Assessment:

- Byte-for-byte rendered workflow equality passes.
- New projects receive the same rule as this repository.

### `crates/lisa-cli/src/templates.rs`

- Pinned the new bystander instruction.
- Pinned the complete field counter-example.
- Retained the existing byte-sync guard.

Assessment:

- A future removal or weakening of the rule fails close to the embedded contract.

## Acceptance criterion 1

> String-pinned rendering tests: a legacy/unstructured block renders the
> standard plain lead line first with the raw reason after it, in both
> `lisa status` and the dashboard; a structured block leads with its ask; the
> T-046-06-03 field disposition text is the regression fixture and never
> appears as a first line.

Result: met.

Evidence:

- Core projection test pins legacy fallback and raw reason preservation.
- CLI unit test pins the full T-046-06-03 reason on the second line.
- CLI black-box test pins actual `lisa status` output and section boundary.
- CLI structured unit and integration tests pin authored ask first.
- Dashboard UI test pins exact vector indices for the field fixture.
- Dashboard structured test pins authored ask before each reason.
- Plugin adapter test pins production legacy ask/reason projection.
- In both surface regressions, the raw field string is asserted absent from the
  first ticket-specific line.

No raw reason is discarded. It is demoted, not hidden.

## Acceptance criterion 2

> The workflow doc's Review section gains the bystander-ask rule with the
> field counter-example; the bundled-copy sync test still passes; brand-voice
> check on all new strings.

Result: met.

Evidence:

- Canonical Review instructions say `Write for a bystander`.
- They say plainly what to do and place jargon outside `ask`.
- The complete field disposition appears as the counter-example.
- Embedded data contains the identical workflow body.
- `test_rdspi_workflow_embedded` passes byte-for-byte.
- `test_review_disposition_contract_is_injected` pins both new additions.

Brand-voice assessment:

- `This ticket needs a decision from you` uses everyday language.
- `you can paste it to your coding agent` gives a concrete next move.
- `Reviewer's note` identifies context without internal schema vocabulary.
- `Write for a bystander` is direct.
- `say plainly what they should do` is action-first.
- The deliberately technical quote is labeled as a counter-example.

## Test coverage

### Focused coverage

- Core parking projection: pass.
- CLI status unit suite: pass.
- Dashboard waiting tests: pass.
- Plugin adapter structured projection: pass.
- Plugin orphaned legacy projection: pass.
- CLI parked UX integration suite: 13 passed.
- Canonical/bundled byte sync: pass.
- Injected Review contract assertions: pass.

### Broad coverage

```text
cargo fmt --all -- --check
passed

cargo test --workspace
passed

just check
passed
```

`just check` verified the plugin for `wasm32-wasip1` and repeated the full
workspace suite successfully.

One real-Zellij boundary test remains intentionally ignored by the repository
because it requires external tools and a live runtime. This ticket changes pure
rendering and has direct unit/integration coverage, so that omission does not
leave an acceptance gap.

## Commit review

### Commit `fa8533df98001b152f04564b731e6f766f95dd41`

Message: `Lead parked blocks with a plain ask`

Contains exactly:

- `crates/lisa-core/src/parking.rs`
- `crates/lisa-cli/src/status.rs`
- `crates/lisa-plugin/src/ui.rs`
- `crates/lisa-plugin/src/lib.rs`

### Commit `2eb443295bba929e536e1d914269e72b82719c95`

Message: `Teach reviewers to write bystander-ready asks`

Contains exactly:

- `docs/knowledge/rdspi-workflow.md`
- `crates/lisa-cli/data/rdspi-workflow.md`
- `crates/lisa-cli/src/templates.rs`

### Commit `ea3dd1821e091525ea1a69ca99cee7526d6916cd`

Message: `Pin parked status lead and detail order`

Contains exactly:

- `crates/lisa-cli/tests/parked_ux.rs`

All three commits were created through `lisa commit-ticket`.
No ordinary-index staging or commit command was used.

## Plan deviation review

The black-box parked UX test file was omitted from the initial planned include
set. The first full workspace run exposed the stale snapshots. The correction
was narrow, acceptance-driven, separately verified, and separately committed.
It also improved coverage by adding the actual legacy field fixture to the CLI
integration layer.

This deviation does not expand product scope.

## Open concerns

No blocking concern.

Minor operational observations:

- Long raw reasons remain long terminal lines; this ticket changes ordering,
  not wrapping or summarization.
- Reviewers can still author a poor structured ask; the workflow now instructs
  against it, while the fallback guarantees safety only for legacy shape.
- A first `just check` run saw an unrelated checksum test fail transiently.
  The preceding full suite, isolated rerun, and final full `just check` all
  passed, and no runtime code was touched.

These are documented limitations rather than completion blockers.

## Worktree review

All eight ticket-owned checked-in paths are clean.
No ticket-owned source file is staged, modified, or untracked.

Remaining worktree entries are Lisa-managed journals, ticket transitions, and
published work artifacts. They were present or produced by Lisa outside the
ticket source transactions and were not claimed by this implementation.

## Final assessment

The implementation meets both acceptance criteria with shared policy, explicit
data preservation, unit tests, black-box CLI coverage, dashboard coverage,
bundled documentation synchronization, and successful WASM/workspace checks.

Ready for Lisa to publish and complete.
