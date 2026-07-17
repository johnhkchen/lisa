# Progress — T-049-07-02 disposition-check-at-the-source

## Status

Implementation is complete.
Three meaningful source units are committed through Lisa's isolated transaction.
Targeted core, CLI, workflow, and fallback-rendering tests pass.
Full workspace and WASM verification remain before Review.

## Completed: strict authoring boundary

Added `check_review_disposition` in `lisa-core::disposition`.
The API reads the same artifact as the downstream parser but returns a strict `Result`.
Exact pass accepts only disposition plus null reason.
Structured block requires reason, owner, ask, and valid optional steps/check.
Note requires null reason, criterion quote, evidence citation, and summary.
Unknown fields fail for all strict classes.
Complaint-shaped note fields receive a class-specific instruction to use block.

The existing `parse_review_disposition` path was left intact.
Its legacy block coercion still assigns operator ownership and an unstructured fallback.
The regression test exercises the same legacy JSON through both APIs:
strict check rejects it; fallback parser safely preserves it.

Commit:

`ddfb4cb43e9a94fb81562045fad0f357f00cfb9c` — Add strict disposition authoring validation

Exact includes:

- `crates/lisa-core/src/disposition.rs`
- `crates/lisa-core/src/parking.rs`

## Completed: shared ask floor

Kept `LEGACY_BLOCK_ASK` unchanged in the parking module.
Added shared, string-pinned ask correction constants beside it.
Added `validate_block_ask` in the same module used as the rendering source.
The validator requires a short, single-line leading sentence with an action cue.
It allows the existing release example and a follow-up sentence.
It rejects a state-only observation, multiline prose, and the full T-046 field paragraph.

This policy is applied only while checking new structured blocks.
Unchecked historical blocks remain renderable through the standard plain fallback.

## Core verification

Passed:

- `cargo test -p lisa-core disposition::tests` — 19 tests
- `cargo test -p lisa-core parking::tests` — 7 tests
- `cargo test -p lisa-core` — 248 unit tests and 2 integration regressions

The generated completion state machine and recorded livelock regression both pass.

## Completed: CLI command

Added hidden `lisa check-disposition <ticket-id> --path <root>`.
In an active pane, it resolves the private attempt from `LISA_TICKET_ID` and `LISA_ATTEMPT_ID`.
It refuses a requested ticket that differs from the active pane ticket.
Outside an active pane, it checks the canonical work artifact.
Ticket identifiers must be one safe path component.

The command composes strict schema validation with the shared block ask floor.
Every failure starts with `Fix review-disposition.json:` and names the correction.
Success names both the ticket and selected artifact path.
The command is read-only.

Added black-box coverage for:

- well-formed pass;
- well-formed structured block;
- well-formed note;
- malformed JSON;
- missing note evidence citation;
- work-complaint note shape;
- legacy unstructured block;
- T-046 ask-floor violation;
- canonical fallback;
- active-pane ticket mismatch.

Updated the hidden-command inventory from 16 to 17 commands.
The top-level operator help remains byte-for-byte unchanged.

Commit:

`4928d4ffb63ed61f285b8c53413aacbb0a88899f` — Add check-disposition reviewer command

Exact includes:

- `crates/lisa-cli/src/check_disposition.rs`
- `crates/lisa-cli/src/main.rs`
- `crates/lisa-cli/tests/check_disposition_cli.rs`
- `crates/lisa-cli/tests/help_surface.rs`

## CLI verification

Passed:

- `cargo test -p lisa-cli --test check_disposition_cli` — 6 tests
- `cargo test -p lisa-cli --test help_surface` — 6 tests
- `cargo test -p lisa-cli` — all unit and integration tests; real-Zellij field test remains intentionally ignored by its existing gate
- all 13 `parked_ux` regressions inside the package run

## Completed: workflow ritual

Added the complete note shape to Review authoring guidance.
Explained that note is only for criteria-versus-evidence disputes and block is for work changes.
Directed every reviewer to run `lisa check-disposition <ticket-id>` after writing the disposition.
Directed the reviewer to correct every reported issue before finishing Review.
Applied identical bytes to canonical and embedded workflow copies.
Pinned note schema, command name, and correction ritual in template tests.

Commit:

`4a2a7ef17520e27a71c84b3d263114385b3e9a1a` — Check dispositions before Review finishes

Exact includes:

- `docs/knowledge/rdspi-workflow.md`
- `crates/lisa-cli/data/rdspi-workflow.md`
- `crates/lisa-cli/src/templates.rs`

## Workflow verification

Passed:

- `templates::tests::test_rdspi_workflow_embedded`
- `templates::tests::test_review_disposition_contract_is_injected`

The embedded/canonical byte-equality assertion passes.

## Deviations from plan

The CLI unit also modified `crates/lisa-cli/tests/help_surface.rs`.
The structure initially assumed the hidden command could leave the historical command inventory untouched.
Inspection showed the file claims to enumerate every Lisa subcommand.
Leaving it at 16 would make that safety contract inaccurate even though the test passed.
The command count and hidden-command list were therefore updated intentionally and included in the CLI commit.

The workflow update also documented the note JSON shape.
The prior workflow only taught pass and block despite the merged note domain.
The command validates all three classes, so reviewers need the corresponding authoring shape at the source.
This is directly within the ticket's full-contract requirement.

The first targeted Cargo command attempted two test filters in one invocation.
Cargo accepts only one positional filter and rejected the command before running tests.
The filters were immediately rerun as separate successful commands.
No source or repository state changed because of the rejected invocation.

## Remaining

1. Write Review artifacts.
2. Use the freshly built command to validate this attempt's own disposition.

## Final verification update

`cargo fmt --all -- --check` passed.

The first parallel `cargo test --workspace` run reached one timing-sensitive failure:
`triage_agent::tests::bounded_runner_returns_valid_proposal_and_surfaces_failure`
returned `TimedOut` while the rest of the heavily parallel CLI binary suite passed.
That test had passed in the earlier full `lisa-cli` package run.
It passed immediately when rerun alone.
The entire workspace then passed with `--test-threads=1`, including 356 CLI binary tests, 248 core tests, 437 plugin tests, integration suites, and doc tests.

`just check` passed in its standard configuration.
It successfully checked `lisa-plugin` for `wasm32-wasip1` and reran the full parallel workspace suite successfully.
The real-Zellij delivery test remained intentionally ignored under its existing environment gate.

All three ticket commits pass `git show --check`.
Every ticket-owned checked-in source path is clean.
The ordinary index has no staged paths.
Remaining status entries are Lisa's completion journal/provenance, Lisa-managed ticket/work publication, and are not ticket-owned source edits left by implementation.
