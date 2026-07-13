# Research: Disposition parse model

## Ticket boundary

T-040-01-02 adds a parser and typed outcome to `lisa-core`.
It does not connect the outcome to scheduler completion; T-040-01-03 owns that
integration.
The parser must consume the contract established by T-040-01-01 and must make
every absent or unusable input non-passing.

The accepted file name is `review-disposition.json`.
The two valid JSON documents are conceptually:

```json
{"disposition":"pass","reason":null}
```

and:

```json
{"disposition":"block","reason":"non-empty actionable text"}
```

Both keys are part of the predecessor contract.
A pass reason is contradictory.
A block with a null, missing, empty, or whitespace-only reason cannot provide
the operator action required by the contract.

## Core crate layout

`crates/lisa-core/src/lib.rs` is the module registry.
It currently exposes `client`, `dag`, `diagnostics`, `provenance`, `route`,
`ticket`, and `types`.
The requested module is a sibling of `provenance.rs`, so it belongs directly in
`crates/lisa-core/src/` and needs a `pub mod` declaration in `lib.rs`.

`lisa-core` already depends on `serde` with derive support and `serde_json`.
No dependency addition is needed.
Its dev dependencies include `tempfile`, which is already used for filesystem
tests in `provenance.rs` and is suitable for missing/present disposition cases.

## Existing parsing conventions

`ticket.rs` accepts a path, reads it with `std::fs`, and separates I/O errors
from format errors through `TicketError`.
That API returns `Result` because callers need a fully valid `Ticket` or an
operational error.

The disposition ticket specifies a different semantic boundary: parsing turns
all file states into one typed `Pass | Block { reason } | Invalid` outcome.
Missing and malformed input are expected domain states, not success values.
Consequently, callers should be able to match one outcome without accidentally
using `Result::unwrap_or` or another fallback that could manufacture success.

`provenance.rs` demonstrates the local style for small public data models:
derive `Debug`, `Clone`, and equality traits; keep filesystem logic in the
owning core module; and place focused unit tests in an internal `tests` module.
It uses compact serde models internally while exposing domain vocabulary at
the public boundary.

## Downstream boundary

T-040-01-03 names two plugin completion sites that will consume this module.
Those callers need to distinguish an explicit pass from both an agent-declared
block and unreadable/contradictory state.
They also need the block reason for operator visibility.
An invalid reason is useful for diagnostics even though the acceptance text
only names `Invalid` rather than prescribing its fields.

The parser therefore needs to retain actionable explanations without turning
transport or schema details into public error control flow.
The critical downstream invariant is simple: only the exact `Pass` variant is
completion eligibility.

## JSON shape observations

Serde can represent the wire document with a private struct containing a
disposition value and an optional reason.
However, `Option<String>` alone does not distinguish a missing `reason` key
from an explicit JSON `null`.
The predecessor contract says every document contains both keys, so schema
presence must be validated rather than silently filled by serde defaults.

Using `serde_json::Value` would preserve all presence and type distinctions,
but would require manual object and string extraction.
A private wire struct can preserve presence with a nested option or a custom
field representation, though that is more machinery than the two-field schema
requires.
Strict object inspection is bounded and makes the validity table explicit.

Unknown fields are not identified as contradictory by the ticket or predecessor
contract.
The established examples describe the required shape, while the acceptance
criteria focus on missing, malformed, and contradictory values.
The parser must at least reject missing required keys, wrong types, unknown
disposition values, and the named reason contradictions.

## Filesystem observations

`std::fs::read_to_string` distinguishes missing files, directories, permission
errors, and invalid UTF-8 through `io::Error`.
All of those mean there is no trustworthy explicit pass and therefore map to
`Invalid`.
JSON whitespace around the document is accepted by `serde_json`, which is
normal JSON behavior.

An empty file parses as malformed JSON and must be invalid.
Multiple concatenated JSON values are rejected by `serde_json::from_str`.
A JSON scalar or array is structurally invalid because the contract requires an
object.

## Testing conventions and required cases

Module-local unit tests can use a temporary directory and write exact payloads.
The required cases are:

1. canonical pass produces `Pass`;
2. canonical block preserves its reason;
3. a path with no file produces `Invalid`;
4. malformed JSON produces `Invalid`;
5. block without a usable reason produces `Invalid`;
6. pass with a block reason produces `Invalid`.

The final three named categories must be asserted against the exact enum, not
only checked with a boolean helper, so a future default-to-pass regression is
visible.
Separate cases for null, absent, empty, and whitespace block reasons strengthen
the contract at low cost.

## Repository and workflow constraints

The ordinary worktree already contains Lisa-managed and other-ticket changes.
This ticket owns only its new module and the module declaration in `lib.rs`.
Those source files must be committed through `lisa commit-ticket` with exact
repository-relative include paths.
The private phase artifacts remain under this attempt directory and are not
included in the source commit.

No ticket phase or status frontmatter is to be edited.
After implementation, the core crate tests and workspace tests are the relevant
verification surfaces.

## Research conclusion

The repository already supplies the JSON, serde, filesystem, tempfile, module,
and unit-test foundations needed for a self-contained core parser.
The key semantic boundary is that filesystem and schema failures are values in
the disposition domain and can never be confused with explicit agent approval.

