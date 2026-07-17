# Plan — T-049-07-02 disposition-check-at-the-source

## 1. Add strict disposition authoring validation

Implement a new strict file-check API in `lisa-core::disposition`.
Keep `parse_review_disposition` and `validate_document` behavior unchanged.
Validate exact pass fields.
Validate complete structured block fields without fallback.
Validate exact note fields and specialized work-complaint rejection.
Reuse domain constructors and field helpers.

Verification:

- valid pass returns Pass;
- valid structured block returns `unstructured: false`;
- valid criteria/evidence note returns Note;
- malformed JSON names JSON correction;
- strict pass rejects extras;
- missing note citation names `evidence_citation`;
- complaint-shaped note says to use block;
- legacy block is rejected by strict check;
- existing fallback parser still returns an unstructured Block for legacy input.

## 2. Establish the shared executable ask floor

Add stable correction strings and `validate_block_ask` to `lisa-core::parking`.
Keep the existing standard legacy lead unchanged.
Inspect only the leading sentence.
Reject multiline or excessively long leads.
Require a word-boundary action cue.
Allow a short follow-up sentence.

Verification:

- workflow release example passes;
- ordinary Run, Choose, Publish, Wait, and Contact asks pass;
- subsystem-only observation fails with action correction;
- multiline ask fails with short-leading-sentence correction;
- full T-046 field paragraph fails;
- exact shared strings are pinned.

## 3. Run core regression tests

Format the two core files.
Run disposition module tests.
Run parking module tests.
Run the complete lisa-core package tests.
Inspect the core diff for any fallback behavior change.

Commit the core unit through:

```text
lisa commit-ticket --ticket-id T-049-07-02 \
  --message "Add strict disposition authoring validation" \
  --include crates/lisa-core/src/disposition.rs \
  --include crates/lisa-core/src/parking.rs
```

## 4. Add active-attempt path resolution

Create `crates/lisa-cli/src/check_disposition.rs`.
Validate the ticket id as one safe path component.
Resolve a matching `LISA_TICKET_ID` plus positive `LISA_ATTEMPT_ID` to private work.
Fall back to canonical work only when no attempt environment is present.
Reject partial or mismatched attempt context.

Verification:

- matching active pane selects `.lisa/attempts/.../work`;
- no pane selects `docs/active/work/...`;
- mismatch names the requested and active tickets;
- unsafe ticket id fails before file access.

## 5. Compose schema and ask validation in the CLI module

Call the strict core disposition checker.
For structured Block, apply the shared ask-floor validator.
Prefix all failures with `Fix review-disposition.json:`.
Return a success message with ticket id and selected path.
Do not write any file or trigger completion.

Verification:

- pass and note skip ask validation and succeed;
- block with a good ask succeeds;
- block with a field-paragraph ask fails with the shared fix;
- file errors name the expected artifact path.

## 6. Wire the Clap command

Declare the new module in `main.rs`.
Add hidden positional `check-disposition <ticket-id>` with optional `--path`.
Resolve the project root using the existing helper.
Print success on stdout.
Follow standard `Error:` stderr and exit-1 behavior on failure.

Verification:

- `lisa check-disposition --help` resolves;
- top-level help snapshot is unchanged because the command is hidden;
- existing own-command enumeration remains valid because it is a deliberately pinned historical set, unless the test treats all hidden commands as exhaustive and needs an intentional update.

## 7. Add black-box command coverage

Create `crates/lisa-cli/tests/check_disposition_cli.rs`.
Use the built binary and temporary repository roots.
Set matching Lisa pane environment for private-attempt fixtures.
Explicitly remove inherited Lisa environment for canonical fixtures.

Success matrix:

1. pass;
2. structured block;
3. note;
4. canonical fallback outside a pane.

Failure matrix:

1. malformed JSON;
2. note missing citation;
3. work-complaint note shape;
4. legacy block missing structured remedy;
5. ask-floor counter-example;
6. active pane ticket mismatch.

For each failure assert nonzero status and stderr naming the exact fix.
For success assert stderr empty and stdout names validation.

## 8. Run CLI tests and commit the command unit

Format the new and modified CLI Rust files.
Run the new integration test.
Run the help-surface integration test.
Run all lisa-cli tests if targeted checks pass.

Commit through:

```text
lisa commit-ticket --ticket-id T-049-07-02 \
  --message "Add check-disposition reviewer command" \
  --include crates/lisa-cli/src/check_disposition.rs \
  --include crates/lisa-cli/src/main.rs \
  --include crates/lisa-cli/tests/check_disposition_cli.rs
```

## 9. Update the Review ritual

Edit `docs/knowledge/rdspi-workflow.md`.
Tell the reviewer to run `lisa check-disposition <ticket-id>` after writing the disposition.
Tell them to correct every reported issue before finishing Review.
Apply the identical edit to `crates/lisa-cli/data/rdspi-workflow.md`.
Extend the template contract test with exact command and correction assertions.

Verification:

- canonical workflow contains the instruction in Review;
- bundled data copy contains identical instruction;
- `test_rdspi_workflow_embedded` byte comparison passes;
- disposition contract injection test pins the new ritual.

Commit through:

```text
lisa commit-ticket --ticket-id T-049-07-02 \
  --message "Check dispositions before Review finishes" \
  --include docs/knowledge/rdspi-workflow.md \
  --include crates/lisa-cli/data/rdspi-workflow.md \
  --include crates/lisa-cli/src/templates.rs
```

## 10. Full verification

Run `cargo fmt --all -- --check`.
Run `cargo test -p lisa-core`.
Run `cargo test -p lisa-cli`.
Run relevant existing S-049-05 tests without altering their fixtures.
Run `cargo test --workspace`.
Run `just check` if the configured WASM target is available.
Run `git diff --check` and inspect exact committed diffs.

## 11. Check transaction cleanliness

Use read-only Git status and diff commands.
Confirm every ticket-owned checked-in path is clean after Lisa commits.
Confirm the ordinary Git index was not consumed or changed by this ticket.
Ignore unrelated pre-existing modifications.
Record commit ids and test results in `progress.md`.

## 12. Review

Read the committed diff and all failures/success output.
Assess each acceptance criterion explicitly.
Document source changes, tests, compatibility, and open concerns in `review.md`.
Write the exact pass disposition only if all required work is committed and verified.
Run the newly built `check-disposition` against this attempt's own Review disposition.
If that succeeds, leave `review-disposition.json` as exact pass JSON.
Remain on this ticket for Lisa's completion confirmation.

## Expected deviations policy

If shared ask-floor heuristics reject an existing positive workflow example, revise the rule before tests rather than weakening the fixture.
If a currently running Lisa binary lacks the new command during final self-check, use the freshly built target binary; the installed runtime predates this ticket by definition.
If unrelated dirty files prevent workspace-wide checks, run package-targeted checks and document the exact external conflict.
Do not absorb or reset unrelated changes.
