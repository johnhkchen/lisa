# Design — T-049-07-02 disposition-check-at-the-source

## Decision summary

Add a strict, read-only authoring validator alongside the existing coercing disposition parser.
Expose it from `lisa-core` so schema knowledge remains centralized.
Add a shared ask-floor validator beside the existing legacy ask source in `parking.rs`.
Add a hidden `lisa check-disposition <ticket-id> [--path <root>]` command in `lisa-cli`.
Resolve the private active-attempt artifact from pane environment and fall back to canonical work outside a pane.
Update both workflow copies to require the command after writing the disposition and before Review finishes.

## Boundary: authoring validation versus consumption parsing

### Option A — make `parse_review_disposition` strict for blocks

This would turn legacy or malformed block structures into `Invalid`.
It would reuse one entry point and make the new command simple.
It would also remove the exact fallback behavior the ticket says must remain.
Existing S-049-05 tests deliberately require incomplete structured blocks to coerce into safe operator-owned parks.

Decision: reject.

### Option B — duplicate the full schema in `lisa-cli`

The command could parse `serde_json::Value` and validate every field locally.
This would avoid touching core behavior.
It would create two independent definitions of pass, block, and note.
The next schema change could make the reviewer check disagree with the completion parser.

Decision: reject.

### Option C — add a strict core API beside the fallback API

Core retains `parse_review_disposition` for trusted downstream consumption.
A new `check_review_disposition` API reads the same file and applies exact authoring rules.
Both APIs reuse internal field decoding and domain types.
The strict path rejects unstructured blocks and extra pass/block fields.
The fallback path keeps every existing observable result.

Decision: choose Option C.

## Strict schema behavior

### Pass

The only accepted pass object is:

```json
{"disposition":"pass","reason":null}
```

Extra fields fail because “pass strict” is an acceptance requirement.
A non-null or missing reason fails with a correction that names `reason: null`.

### Block

A block requires:

- nonblank string `reason`;
- `remedy_owner` equal to `agent`, `operator`, or `world`;
- nonblank string `ask`;
- optional nonempty array of nonblank string `steps`;
- optional nonblank string `check`;
- no unknown fields.

The completion parser may still coerce documents that violate this shape.
The strict check refuses them and tells the reviewer which structure to add or correct.

### Note

A note requires null reason plus nonblank `criterion_quote`, `evidence_citation`, and `summary`.
No other fields are allowed.
The evidence citation must remain an inert string, matching T-049-06-01.
The command does not dereference it or judge the evidence.
An explicit work-complaint field gets a class-specific message: use block when the work itself needs changes.
Other extra fields receive an exact allowed-field correction.

## Work-complaint rule

The durable schema is the reliable enforcement boundary.
A note has no generic complaint field and exact-field validation prevents adding one.
Known complaint-shaped keys such as `work_complaint`, `complaint`, and `quality_concern` receive the specialized fix.
The validator should not attempt general natural-language classification of `summary`.
That would create false positives for legitimate criteria-versus-evidence summaries mentioning tests or implementation measurements.
The ticket asks for a check, not an embedded semantic model.

## Ask-floor design

### Shared home

Put `validate_block_ask` and its user-visible fix constants in `lisa-core::parking` beside `LEGACY_BLOCK_ASK`.
The parking module is already the shared source used by both rendering paths.
This realizes the earlier ticket's explicit anticipation of later shared validators.
No renderer needs to call the validator at display time; unchecked legacy content must still render safely.

### Executable floor

The first sentence/line of an ask must:

1. be a single physical line;
2. be bounded in length so the field paragraph cannot become the lead;
3. contain a recognizable action cue directed at a bystander.

Action cues cover the workflow's expected verbs and forms, including `run:`, publish, choose, decide, provide, amend, update, retry, install, open, confirm, approve, wait, and contact.
Matching is ASCII case-insensitive at word boundaries.
The first sentence is the text through the first `.`, `!`, or `?`; a second follow-up sentence is allowed because the existing positive release example contains one.
The exact counter-example fails the bounded leading-sentence rule even though an action appears late in its technical paragraph.

### Rejected: punctuation count only

The private triage helper only checks newline and punctuation count.
It would accept `The subsystem is broken.` even though it gives no action.
It would reject the existing positive workflow example because that example has two periods.

### Rejected: jargon dictionary

A fixed technical-word blacklist would age poorly and reject legitimate actions such as `Run just release` or `Update docs/active/...`.
Length plus an action cue enforces the mechanical floor while workflow prose continues teaching tone.

## Diagnostics

Every error begins with `Fix review-disposition.json:`.
The remainder names the exact field or class correction.
Examples include:

- keep pass to the exact two-field object;
- add `criterion_quote`, `evidence_citation`, and `summary` to note;
- remove work-quality complaints and use block;
- add structured block remedy fields;
- lead `ask` with a short plain action such as `Run ...`, `Choose ...`, or `Publish ...`.

The command prints a single explicit success line with ticket id and path.
It returns nonzero on file lookup, JSON, schema, class, or ask-floor failures.

## Artifact path resolution

The command accepts ticket id positionally and project root through `--path`.
If both environment values exist, match the requested ticket, and the attempt id is a positive integer, use:

`.lisa/attempts/<ticket>/<attempt>/work/review-disposition.json`

If the environment names a different ticket, fail rather than silently validate another location.
If no active attempt is supplied, use:

`docs/active/work/<ticket>/review-disposition.json`

Ticket ids are treated as one normal path component; separators and traversal are rejected.
This avoids allowing a diagnostic command to read an arbitrary path.

## CLI visibility

Make the command hidden from the everyday operator command list.
It remains directly invokable and has its own help.
The command is agent ritual/plumbing, not a new operator workflow.
Hiding it preserves the carefully pinned top-level help and avoids increasing unrelated snapshot churn.
Its Clap declaration still provides clear positional and `--path` help.

## Workflow wording

After describing disposition authoring, add a direct instruction:

`Run lisa check-disposition <ticket-id> after writing review-disposition.json. Correct every reported issue before finishing Review.`

Use the actual ticket id rather than the literal placeholder when executing.
The instruction belongs before the final wait paragraph.
The bundled data copy must be byte-identical after purpose-prefix assembly.
Template tests should pin both command name and correction-before-finish wording.

## Tests

Core strict-validator tests cover all three accepted classes.
They cover malformed JSON, strict pass extras, missing note citation, complaint-shaped note, incomplete block structure, and unknown fields.
Ask tests pin accepted action forms, newline, no-action, and the complete T-046 counter-example.
String tests pin both the fallback lead and the exact ask fix text from the same module.

Black-box CLI tests create private attempt paths and set pane environment.
They prove pass/block/note success and pane-visible named fixes for each required failure class.
They prove canonical fallback outside a pane and wrong-ticket environment refusal.
Existing disposition and parking tests remain unchanged in behavior.
Existing parked UX tests are the regression proof that unchecked legacy blocks still render safely.
Template sync and workspace tests protect integration.

## Commit units

First commit core strict validation and shared ask-floor rules.
Second commit the CLI command and black-box tests.
Third commit the workflow copies and template assertions.
Each uses exact repository-relative include paths through `lisa commit-ticket`.

## Risks and mitigations

The main risk is changing legacy parsing while refactoring.
Run the complete disposition and parking suites before committing core.
The next risk is overclaiming semantic plainness from a heuristic.
Keep the executable policy deliberately small, documented, and string-pinned.
The final risk is validating the canonical artifact instead of the active attempt.
Make private-attempt resolution the primary black-box fixture and print the selected path on success.
