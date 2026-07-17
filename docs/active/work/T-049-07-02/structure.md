# Structure — T-049-07-02 disposition-check-at-the-source

## Checked-in change inventory

Modify:

1. `crates/lisa-core/src/disposition.rs`
2. `crates/lisa-core/src/parking.rs`
3. `crates/lisa-cli/src/main.rs`
4. `crates/lisa-cli/src/templates.rs`
5. `docs/knowledge/rdspi-workflow.md`
6. `crates/lisa-cli/data/rdspi-workflow.md`

Create:

1. `crates/lisa-cli/src/check_disposition.rs`
2. `crates/lisa-cli/tests/check_disposition_cli.rs`

No checked-in file is deleted or renamed.
No plugin source file needs modification.
No ticket frontmatter is changed by this attempt.

## Private attempt artifacts

The attempt work directory contains:

- `research.md`
- `design.md`
- `structure.md`
- `plan.md`
- `progress.md`
- `review.md`
- `review-disposition.json`

These remain private until Lisa admits and publishes them.

## `lisa-core::disposition`

### New public API

Add:

```rust
pub fn check_review_disposition(
    path: impl AsRef<Path>,
) -> Result<ReviewDisposition, String>
```

It reads a file, parses JSON, and applies strict authoring validation.
The error string identifies the concrete schema correction without a CLI prefix.
The returned domain value is `Pass`, `Note`, or a structured `Block`.
It never returns `Invalid` or an unstructured block on success.

### Shared read/JSON helpers

Factor file reading and JSON parsing only as far as useful to keep diagnostics consistent.
`parse_review_disposition` must preserve its current public return type and every legacy behavior.
Read failures in the strict path return `Err`.
Read failures in the fallback path remain `ReviewDisposition::Invalid`.

### Strict document dispatcher

Add a private strict dispatcher over `serde_json::Value`.
It verifies the top-level object and string disposition field.
It dispatches to pass, note, or block validators.
Unknown disposition names produce a supported-class correction.
Missing or non-string disposition produces a field-specific correction.

### Strict pass validator

Consume `reason` and require null.
Require the remaining object to be empty.
On extras, direct the author to the exact two-field pass JSON.

### Strict note validator

Consume null reason and the three required strings.
Reuse `DispositionNote::new` for nonblank validation and value construction.
Before the generic extra-field check, recognize complaint-shaped keys.
Return the specialized “use block when work itself needs changes” message.
List the exact allowed note fields for other extras.

### Strict block validator

Consume nonblank reason, owner, ask, optional steps, and optional check.
Return errors rather than calling `unstructured_block`.
Reuse `RemedyOwner`, `non_empty_string`, and the existing domain variant.
Require optional steps, when present, to be a nonempty array of nonblank strings.
Require no remaining fields.
Leave ask-floor validation to `parking` so schema and rendering policy stay distinct.

### Tests

Add a strict-check helper using a temporary file.
Test valid pass, block, and note.
Test malformed JSON and non-object input.
Test pass extra fields.
Test note missing evidence citation.
Test complaint-shaped note.
Test incomplete/legacy block rejection.
Test invalid optional block structure and unknown block fields.
Keep all existing fallback-parser tests intact.

## `lisa-core::parking`

### Existing shared source

Keep `LEGACY_BLOCK_ASK` byte-for-byte unchanged.

### New constants

Add one exact pane-facing correction constant for no leading action.
Add one exact correction constant for a leading line that is too long or multiline.
Constants live beside `LEGACY_BLOCK_ASK` so rendering fallback and authoring floor have one source module.

### New validator

Add:

```rust
pub fn validate_block_ask(ask: &str) -> Result<(), &'static str>
```

It trims only for inspection and never returns normalized content.
It rejects empty or multiline text.
It extracts the leading sentence through terminal punctuation.
It rejects a leading sentence over the documented character ceiling.
It tokenizes ASCII word-like runs and searches an explicit action-cue list.
It recognizes `run:` as well as ordinary action verbs.
It returns a stable correction constant for each failure family.

### Tests

Pin `LEGACY_BLOCK_ASK` exactly.
Pin both correction constants exactly.
Accept representative asks for agent, operator, and world remedies.
Accept the workflow's release example.
Reject a subsystem observation without an action.
Reject multiline content.
Reject the full T-046 field counter-example.

## `lisa-cli::check_disposition`

### Public module entry point

Define:

```rust
pub fn run_check_disposition(
    project_root: &Path,
    ticket_id: &str,
) -> Result<String, String>
```

The function is read-only.
On success it returns a line naming the ticket and validated path.
Main prints the line.
On failure it returns a string beginning `Fix review-disposition.json:`.

### Path resolver

Private helper validates ticket id as one normal path component.
Read `LISA_TICKET_ID` and `LISA_ATTEMPT_ID` together.
When an attempt is active, require the requested ticket to match.
Require a positive numeric attempt.
Build the private attempt path.
When neither environment variable is present, build the canonical path.
Treat half-present attempt environment as an actionable configuration failure.

### Validation flow

Call `lisa_core::disposition::check_review_disposition`.
If it returns `Block`, call `lisa_core::parking::validate_block_ask` on its ask.
Prefix core/schema and ask errors uniformly.
Pass and Note need no further CLI policy step.

### Unit tests

Keep pure ticket-id/path helper tests local if they do not need environment mutation.
Prefer black-box tests for environment resolution to avoid process-global environment races in unit tests.

## `lisa-cli::main`

Declare `mod check_disposition;`.
Add hidden `Commands::CheckDisposition` with:

- positional `ticket_id: String`;
- `--path <PATH>` defaulting to `.`.

Dispatch after path resolution.
Print success to stdout.
Print failure as `Error: <message>` to stderr and exit 1, following existing CLI style.
Do not add it to the curated top-level plumbing footer.

## CLI integration test

Create a temporary root and helper that writes exact disposition bytes.
The main fixture writes private attempt paths and sets matching environment.
Each invocation removes unrelated inherited Lisa variables before setting fixture values.
Test these successes independently:

- exact pass;
- fully structured block with plain ask;
- exact note with criterion and evidence citation.

Test failures and exact fix substrings:

- malformed JSON;
- missing note citation;
- complaint-shaped note;
- legacy/incomplete block;
- overlong technical ask floor violation.

Add canonical fallback success with no Lisa attempt environment.
Add mismatched pane ticket failure.
Assert nonzero exit, empty success output on failure, and pane-visible stderr.

## Workflow documents

Insert the command ritual in the Review section after authoring/class guidance and before waiting.
Use identical bytes in canonical and data copies.
Do not change legacy embedded workflow generations.

## Template test

Extend `test_review_disposition_contract_is_injected`.
Assert `lisa check-disposition <ticket-id>` is present.
Assert the instruction says to correct every reported issue before finishing Review.
The existing byte-for-byte sync assertion remains the primary copy guard.

## Dependency direction

`lisa-cli` depends on `lisa-core`; no new crate dependency is needed.
Core disposition constructs validated domain values.
Core parking owns ask presentation/authoring floor vocabulary.
CLI resolves pane context and composes the two validations.
Workflow docs teach the ritual.
Plugin completion and rendering continue consuming existing core APIs unchanged.

## Commit boundaries

Core unit includes only disposition and parking.
CLI unit includes the new module, main dispatch, and integration test.
Workflow unit includes both copies and template assertions.
Private phase artifacts are not passed to `commit-ticket`.
