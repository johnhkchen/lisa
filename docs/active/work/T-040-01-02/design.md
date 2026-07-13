# Design: Disposition parse model

## Goals

The design must expose one typed decision boundary for Review completion.
An exact valid pass becomes `Pass`; a valid block becomes `Block` and retains
its reason; every other observation becomes `Invalid`.
The API must make fail-closed behavior natural for the scheduler consumer.

## Option 1: return `Result<Disposition, Error>`

A conventional parser could return successful `Pass` or `Block` values and put
missing files, malformed JSON, and contradictions in an error enum.

This preserves detailed error categories and follows `ticket.rs` conventions.
It is less aligned with the ticket's requested three-way typed outcome.
It also leaves downstream code with two axes (`Result` and disposition), making
it easier to flatten an error using an unsafe default.

This option is rejected because invalidity is scheduler-relevant domain state,
not merely an exceptional parser failure.

## Option 2: serde directly into a public tagged enum

Serde can parse a tagged enum from `{ "disposition": ... }` and bind the block
reason to a field.
This is compact for valid documents.

The pass document contains an explicit `reason: null`, which does not naturally
fit a unit enum variant without extra serde configuration.
More importantly, serde errors alone cannot express the public `Invalid`
variant or retain a stable diagnostic for missing and contradictory input.

This option is rejected as the complete boundary, though serde_json remains the
syntax parser.

## Option 3: private JSON inspection plus public outcome

The module reads the path, parses a `serde_json::Value`, validates the required
object keys and their relationship, and returns a public domain enum:

```rust
pub enum ReviewDisposition {
    Pass,
    Block { reason: String },
    Invalid { reason: String },
}
```

The invalid reason describes the failed read, JSON syntax, schema, or
contradiction.
Consumers still use a single exhaustive match and only `Pass` authorizes
completion.

This option makes the small validity matrix explicit and preserves useful
operator diagnostics.
It is selected.

## Public API decision

Expose:

```rust
pub fn parse_review_disposition(path: impl AsRef<Path>) -> ReviewDisposition
```

The function owns both file reading and document validation because “missing
file” is explicitly one of its required outcomes.
Taking `impl AsRef<Path>` matches other core filesystem APIs and is convenient
for plugin callers and tests.

The enum derives `Debug`, `Clone`, `PartialEq`, and `Eq`.
It does not derive serde traits because it is the validated domain model, not
the wire schema.

`Invalid` carries a reason even though the ticket spells the variant without a
field.
This matches the story's need for visible/actionable refusal and allows the next
ticket to explain why completion was denied.
Callers can still match `Invalid { .. }` as one non-passing class.

## Validation decision table

`disposition == "pass"` is valid only when:

- the `reason` key exists;
- its value is JSON null.

`disposition == "block"` is valid only when:

- the `reason` key exists;
- its value is a JSON string;
- trimming the string leaves at least one character.

The original block reason is retained rather than trimmed so the parser does
not rewrite agent evidence.
Trimming is only a validity check.

All other cases are `Invalid`, including:

- unreadable or missing path;
- invalid UTF-8;
- malformed JSON;
- non-object root;
- absent or non-string `disposition`;
- unknown disposition string;
- absent `reason`;
- pass with any string reason, including empty;
- block with null, non-string, empty, or whitespace-only reason.

## Unknown fields

Unknown object fields will be accepted when both required fields form a valid
contract document.
The predecessor fixed the required keys and relationship but did not explicitly
declare a closed schema or require rejection of extensions.
Forward-compatible extra metadata does not make pass and reason contradictory.

This is intentionally distinct from accepting missing or mistyped required
fields, which are invalid.

## Error text

Invalid reasons are descriptive strings rather than a second public error enum.
The scheduler's authority depends on the variant, while the text is diagnostic.
Tests should assert the variant and only selectively inspect text, avoiding a
brittle public string protocol.

Read errors include the path and underlying I/O error.
JSON errors identify malformed JSON.
Schema errors identify the violated disposition/reason relationship.

## Test design

Tests live beside the implementation and create files in `tempfile::tempdir`.
A helper writes a named disposition file and calls the public parser.

Required positive tests assert exact equality:

- pass equals `ReviewDisposition::Pass`;
- block equals `ReviewDisposition::Block { reason: ... }`.

Negative tests use pattern matching that fails if the result is either positive
variant.
The contradictory block cases are grouped to cover missing, null, empty, and
whitespace reason values.
The contradictory pass test supplies a non-empty block reason.

Additional cheap schema tests cover a missing reason on pass, unknown
disposition, and non-object JSON if implementation remains focused.

## Verification

First run `cargo fmt --check` after formatting.
Run `cargo test -p lisa-core disposition` for focused feedback.
Then run `cargo test --workspace` to ensure public module addition does not
disturb other crates.

Commit the two ticket-owned source paths together because the module cannot be
consumed without its registry declaration and `lib.rs` cannot compile without
the module file.

