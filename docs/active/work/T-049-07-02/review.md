# Review — T-049-07-02 disposition-check-at-the-source

## Outcome

The ticket is ready to pass.
Review dispositions now have a strict check at the authoring boundary.
Reviewers are instructed to run it before Review ends.
Malformed new output receives a precise fix in the same pane where it was written.
Unchecked legacy blocks still flow through the existing safe coercion and plain rendering fallback.

## What changed

### Strict core authoring validation

`crates/lisa-core/src/disposition.rs` now exposes `check_review_disposition`.
This API is deliberately separate from `parse_review_disposition`.
The new API enforces newly authored contract shape and returns `Result`.
The existing API remains the fail-closed/coercing downstream consumer.

Strict pass accepts only:

```json
{"disposition":"pass","reason":null}
```

Strict block requires a nonblank reason, valid owner, and nonblank ask.
It validates optional steps as a nonempty array of nonblank strings.
It validates optional check as a nonblank string.
It rejects unknown block fields.
It never returns an unstructured block on success.

Strict note requires null reason and three nonblank strings:

- `criterion_quote`
- `evidence_citation`
- `summary`

It rejects extra fields.
Complaint-shaped note fields receive a direct instruction to use block when work needs changes.
Evidence citations stay inert; validation does not execute or dereference them.

### Shared ask-floor policy

`crates/lisa-core/src/parking.rs` remains the shared source used by parked rendering.
The existing `LEGACY_BLOCK_ASK` is byte-for-byte unchanged.
The module now also owns two exact correction strings and `validate_block_ask`.

For new structured blocks, the first ask sentence must:

- stay on one line;
- be at most 160 characters;
- contain a recognizable action for a bystander.

Follow-up context is allowed after the leading sentence.
The workflow's positive release example passes.
The complete T-046 technical field paragraph fails.
Unchecked legacy blocks do not go through this authoring check and still get the standard render-time lead.

### CLI command

`crates/lisa-cli/src/check_disposition.rs` implements the read-only command behavior.
`crates/lisa-cli/src/main.rs` exposes:

```text
lisa check-disposition <ticket-id> [--path <project-root>]
```

The command is hidden from the everyday operator command list because it is an agent Review ritual.
Its direct help remains available.
The hidden-command contract now truthfully inventories 17 Lisa commands.
The operator-facing top-level help snapshot is unchanged.

In a Lisa pane, the command uses `LISA_TICKET_ID` and `LISA_ATTEMPT_ID` to select:

```text
.lisa/attempts/<ticket>/<attempt>/work/review-disposition.json
```

It refuses a requested ticket different from the active pane ticket.
Without pane attempt context, it selects canonical `docs/active/work/<ticket>`.
Ticket id traversal/path separators are rejected.

Every failure begins with:

```text
Fix review-disposition.json:
```

The rest names the exact field, class, or ask correction.
Success names the ticket and checked path.
The command does not write, publish, park, or complete anything.

### CLI integration coverage

`crates/lisa-cli/tests/check_disposition_cli.rs` drives the built binary.
It writes real private-attempt or canonical fixtures.
It controls inherited pane environment explicitly.
It asserts exit status, stdout, stderr, and exact fix substrings.

`crates/lisa-cli/tests/help_surface.rs` now includes the new command in the complete hidden-command inventory.
It also proves the command does not appear in the generated operator listing.

### Review ritual

`docs/knowledge/rdspi-workflow.md` now teaches all three disposition shapes.
It says note is only for a criterion backed by cited evidence.
It says block is for work that needs changes.
It requires `lisa check-disposition <ticket-id>` after the artifact is written.
It requires every reported issue to be corrected before Review finishes.

`crates/lisa-cli/data/rdspi-workflow.md` carries identical bundled content.
`crates/lisa-cli/src/templates.rs` pins the note shape, command, and correction instruction.

## Acceptance criteria assessment

### Well-formed classes

Pass, fully structured block, and criteria/evidence note each pass strict core tests and black-box CLI tests.

### Malformed schema

Malformed JSON fails nonzero and tells the reviewer to write valid JSON.
Non-object, unknown class, missing fields, invalid field types, and extra fields have strict core coverage.

### Missing note citation

A note without `evidence_citation` fails and names that exact field in the pane-visible message.

### Work-complaint note shape

A note containing `work_complaint` fails and says to remove work-quality complaints and use block.
The note schema still has no generic complaint body.

### Ask-floor violation

The complete T-046 field ask fails because its leading sentence exceeds the shared limit.
The fix says to use one short first sentence and move technical detail into reason or steps.
State-only asks also fail with plain action examples.

### Shared source and string pins

Fallback lead, action correction, and leading-sentence correction are co-located and exact-string tested in `lisa-core::parking`.
Both existing rendering paths already source their legacy lead from that module.

### Workflow sync

The canonical workflow contains the check-before-finish ritual.
The embedded copy is byte-identical under `test_rdspi_workflow_embedded`.
Individual contract strings are also pinned.

### Legacy fallback

A dedicated core regression passes the same legacy block through both boundaries:
strict authoring rejects it, while downstream parsing still produces `unstructured: true`.
All 13 existing black-box parked UX tests pass unchanged in behavior.
The plugin's existing unrecognized-completion fallback and legacy-field rendering tests pass in the full workspace suite.

## Test results

Passed targeted checks:

- 19 core disposition tests
- 7 core parking tests
- 6 black-box check-disposition tests
- 6 help-surface tests
- both workflow/template contract tests
- full `lisa-core` package
- full `lisa-cli` package

Passed quality gates:

- `cargo fmt --all -- --check`
- `cargo test --workspace -- --test-threads=1`
- `just check`

`just check` checked `lisa-plugin` for `wasm32-wasip1` and passed its normal parallel workspace run.
The existing real-Zellij delivery test remained ignored behind its explicit environment prerequisite.

One initial parallel workspace attempt saw the pre-existing triage-agent bounded-runner test time out under load.
The test passed immediately alone, passed in the serialized full workspace, and passed again in `just check`'s parallel suite.
This is not correlated with disposition code or CLI path resolution.

## Commit review

Three meaningful Lisa-isolated commits contain the complete checked-in change:

1. `ddfb4cb43e9a94fb81562045fad0f357f00cfb9c` — strict core validation and shared ask floor
2. `4928d4ffb63ed61f285b8c53413aacbb0a88899f` — CLI command and black-box/help tests
3. `4a2a7ef17520e27a71c84b3d263114385b3e9a1a` — workflow ritual and bundled sync

All three pass `git show --check`.
Every ticket-owned checked-in source path is clean.
The ordinary Git index is empty.
No ticket phase/status field was manually changed.
Phase artifacts were written only to the attempt-private work directory.

## Open concerns and limits

The ask floor is intentionally a deterministic mechanical minimum, not natural-language understanding.
It guarantees a bounded lead and recognizable action cue.
The workflow prose still carries the richer bystander and jargon guidance.

The note check enforces its restricted exact schema and complaint-shaped keys.
It does not classify arbitrary summary prose or compare the quoted criterion against ticket text.
That matches T-049-06-01's boundary: citations are recorded, not semantically adjudicated by parsing.

Canonical fallback outside a pane is useful for manual inspection, but Review agents should use the active pane environment so they validate the lease-private artifact that Lisa will admit.
The workflow's normal execution context supplies those variables.

No blocking concern remains.

## Disposition

Pass.
The source check now catches malformed disposition output before Review ends while preserving the downstream safety net.
