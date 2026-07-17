# Design — T-049-05-02 plain-ask-floor

## Desired behavior

Every operator-visible parked block gets a two-level presentation:

1. A plain, actionable lead sentence.
2. The reviewer's original reason as supporting detail.

For a structured block, the lead is its authored `ask`.
For a legacy block, the lead is the ticket-pinned standard sentence:

> This ticket needs a decision from you. The reviewer's note is below — you can paste it to your coding agent.

The raw reason follows on a distinct line labeled `Reviewer's note:`.
World-owned blocks keep the existing `Lisa checks on its own.` suffix.
Agent-owned blocks remain absent from Waiting on you.

## Decision 1: where to establish the fallback ask

### Option A — change disposition parsing

`unstructured_block` could replace its fallback `ask` with the standard line.

Advantages:

- All downstream consumers automatically receive the plain ask.
- The fallback has a single definition.
- `ReviewDisposition::Block.ask` would always be operator-ready.

Costs:

- It changes the established parser meaning.
- Existing parser tests intentionally assert that legacy bytes survive in `ask`.
- Non-rendering consumers may rely on the parser's lossless fallback.
- The parser would acquire presentation copy despite being a validation boundary.

Decision: reject.

### Option B — implement the fallback independently in both renderers

Status and dashboard could each inspect an `unstructured` flag and substitute copy.

Advantages:

- Presentation policy remains in presentation code.
- Each surface can choose its own layout.

Costs:

- The exact fallback string would be duplicated.
- Drift could recreate different behavior across the two required surfaces.
- Every future parked-ticket surface would need to remember the policy.
- T-049-07-02 already anticipates one shared string source.

Decision: reject.

### Option C — normalize the parked-remedy projection

Keep parser semantics lossless, but make `collect_parked_remedies` turn an
unstructured block's outward `ask` into a shared constant while carrying the
original reason separately.

Advantages:

- The shared human-facing domain projection enforces one floor.
- Both current surfaces receive the same exact fallback copy.
- The parser still preserves raw legacy input.
- Future parked-remedy consumers inherit the floor by default.
- The projection already describes itself as shared by status and dashboard.

Costs:

- `ParkedRemedy` gains another field.
- Adapter fixtures must be updated.
- Consumers that only wanted the old fallback ask now see standard copy.

Decision: choose Option C.

## Decision 2: raw reason representation

### Option A — synthesize one preformatted multiline string in core

Core could hand renderers a fully formatted display string.

Advantages:

- Exact parity is automatic.
- Renderers become trivial.

Costs:

- ANSI, indentation, and terminal-width choices leak into core.
- Status and dashboard have different state and formatting types.
- Tests lose clear semantic fields.

Decision: reject.

### Option B — add `reason` to `ParkedRemedy` and UI `WaitingItem`

Core carries the raw reason semantically. The plugin adapter copies it into the
self-contained UI type. Each renderer owns only line layout.

Advantages:

- Raw data is preserved until presentation.
- Type fields reveal the lead/detail relationship.
- Both surfaces can pin ordering without sharing terminal details.
- The UI layer remains self-contained as intended.

Costs:

- Struct literals across tests need a new field.
- Two small formatters remain, though their copy source is shared.

Decision: choose Option B.

## Decision 3: retain `unstructured` in `ParkedRemedy`

Once core chooses the fallback ask, the renderers do not need to distinguish
legacy from structured blocks. They need only an `ask` and `reason`.

Carrying `unstructured` farther would expose parsing history without changing
layout. It would also invite surface-specific policy divergence.

Decision: do not expose `unstructured` on `ParkedRemedy`. Use it locally during
collection to select the shared fallback ask.

## Decision 4: line layout

### Lead line

Keep the current ticket prefix and spacing:

`T-001  <ask>`

For world-owned blocks, keep the existing suffix:

`T-001  <ask> — Lisa checks on its own.`

This preserves recognizable status/dashboard behavior while changing the
human-first content.

### Detail line

Render a second line immediately afterward:

`       Reviewer's note: <reason>`

The label tells a bystander why the detail is present and makes it clear the
technical prose is quoted context, not a command they must decode themselves.
The line is deliberately below the ask and is not folded into the lead.

The renderer should preserve the raw reason string exactly after the label.
It should not trim, rewrite, summarize, or infer a remedy.

Decision: use this two-line form in both surfaces.

## Decision 5: tests

### Core projection tests

Update structured fixtures to assert `reason` remains present.
Replace the legacy projection expectation with the standard lead sentence and
the exact raw reason in the detail field.

This test pins the shared floor before either renderer.

### CLI rendering tests

Expand `waiting_on_you_lines` fixtures with reasons.
Pin exact lead/detail line ordering for structured operator and world blocks.
Add the complete T-046-06-03 legacy reason.
Assert the fallback sentence is the entry's first line.
Assert the field reason is the next line and never the first line.

### Dashboard rendering tests

Extend `WaitingItem` with `reason`.
Pin the same structured ordering in `render_waiting_on_you`.
Add the full field legacy reason and standard fallback.
Assert exact vector positions so substring presence cannot hide bad ordering.

### Plugin adapter tests

Update projection fixtures to include `reason`.
For the field legacy block, assert `ask` equals the standard fallback while
`reason` equals the untouched field disposition.

### Workflow template tests

Keep the existing byte-for-byte bundled copy test.
Add assertions for the bystander rule and field counter-example so an edit that
removes either part fails close to the contract source.

## Decision 6: workflow wording

Extend the existing ask paragraph rather than introducing another section.
The rule will say:

- Write for a bystander who did not perform the work.
- State what the person should do in plain language.
- Put technical jargon in `reason` or `steps`, not the `ask`.
- Do not use the field disposition as an ask.

Quote the complete T-046-06-03 text as the counter-example, not an ellipsis.
The exact quote gives future reviewers a concrete failure mode and fulfills the
ticket's regression-documentation requirement.

Keep the existing release bad/good example because it demonstrates a different
aspect: naming an action rather than a subsystem condition.

## Decision 7: copy ownership

Define `LEGACY_BLOCK_ASK` as a public constant in `lisa_core::parking`.

Rationale:

- `parking` owns the human-facing remedy projection.
- Both CLI and plugin already depend on `lisa-core`.
- Tests can import one exact source rather than retyping production copy.
- Future disposition checking can reuse the same floor if appropriate.

The renderer label `Reviewer's note:` can remain local, because it is layout
copy rather than the fallback action contract. Its exact text is still pinned
independently in both rendering tests.

## Decision 8: implementation units

Unit 1 is the shared projection plus both renderers and their regression tests:

- `crates/lisa-core/src/parking.rs`
- `crates/lisa-cli/src/status.rs`
- `crates/lisa-plugin/src/ui.rs`
- `crates/lisa-plugin/src/lib.rs`

These files form one behavior that cannot satisfy acceptance independently.

Unit 2 is the authoring contract and embedded-copy assertion:

- `docs/knowledge/rdspi-workflow.md`
- `crates/lisa-cli/data/rdspi-workflow.md`
- `crates/lisa-cli/src/templates.rs`

Both units will use exact-path `lisa commit-ticket` transactions.

## Compatibility and non-goals

- Do not modify disposition JSON schema.
- Do not alter block ownership.
- Do not alter park/unpark transitions.
- Do not alter world recheck execution.
- Do not hide the raw reason.
- Do not summarize or reinterpret reviewer prose.
- Do not change how agent-owned remedies are handled.
- Do not update ticket phase/status manually.
- Do not publish attempt artifacts directly to shared work.

## Resulting invariant

For every operator/world `ParkedRemedy`, `ask` is suitable as a first line and
`reason` is available as second-line context. A structured block gets its own
ask unchanged. A legacy block gets the shared standard ask. Both visible
surfaces render ask before reason, and workflow authors are explicitly taught
to keep future structured asks plain enough for a bystander.
