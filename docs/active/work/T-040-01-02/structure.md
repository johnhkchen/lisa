# Structure: Disposition parse model

## Change inventory

Create:

- `crates/lisa-core/src/disposition.rs`

Modify:

- `crates/lisa-core/src/lib.rs`

Delete:

- nothing

No Cargo manifest change is required because `serde_json` and `tempfile` are
already dependencies of `lisa-core` in the needed scopes.

## Module boundary

`disposition.rs` owns the Review disposition file boundary.
It translates filesystem contents and the JSON wire contract into a validated
domain outcome.
It does not know about tickets, attempts, scheduler threads, artifact polling,
or completion commands.

That isolation lets both plugin completion sites share exactly the same
fail-closed interpretation in T-040-01-03.

`lib.rs` adds:

```rust
pub mod disposition;
```

No root-level re-export is needed; existing core modules are accessed through
their module namespace.

## Public model

The public enum is:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewDisposition {
    Pass,
    Block { reason: String },
    Invalid { reason: String },
}
```

`Pass` contains no payload because the wire contract requires its reason to be
null.
`Block` owns its reason so the outcome outlives the parsed JSON buffer and can
be displayed by consumers.
`Invalid` owns a diagnostic reason for the same reason.

The enum deliberately has no “default” implementation.
There must be no implicit outcome, especially no implicit pass.

## Public parser

The public function is:

```rust
pub fn parse_review_disposition(path: impl AsRef<Path>) -> ReviewDisposition
```

It performs three ordered operations:

1. normalize the generic argument to `&Path`;
2. read the complete file as UTF-8 text;
3. parse and validate the JSON document.

Any read failure immediately returns `Invalid`.
Any JSON parse failure immediately returns `Invalid`.
Only validation can construct `Pass` or `Block`.

## Internal organization

The module starts with module-level documentation describing the fail-closed
authority boundary.
Imports are limited to `std::fs`, `std::path::Path`, and `serde_json::Value`.

The parser may delegate the value relationship to a private helper:

```rust
fn validate_document(value: Value) -> ReviewDisposition
```

This keeps filesystem failure formatting separate from the finite JSON
decision matrix and permits direct reasoning about ownership of block strings.

The helper first requires a JSON object.
It then separately obtains the `disposition` and `reason` keys.
It matches their values as a tuple so contradictory combinations are visible in
one place.

The canonical match arms are:

- string `pass` plus null reason -> `Pass`;
- string `block` plus non-blank string reason -> `Block`;
- every other relationship -> `Invalid`.

Dedicated earlier checks provide clearer diagnostics for missing required keys
and non-string disposition values.

## Unit-test organization

An internal `#[cfg(test)] mod tests` uses `super::*` and `tempfile::tempdir`.
A small helper accepts JSON text, writes `review-disposition.json`, and returns
the parsed outcome.

Tests are named by observable contract:

- `parses_pass`;
- `parses_block_with_reason`;
- `missing_file_is_invalid`;
- `malformed_json_is_invalid`;
- `block_without_reason_is_invalid`;
- `pass_with_block_reason_is_invalid`.

The block-without-reason test iterates representative documents for absent,
null, empty, and whitespace-only reasons.
Every negative assertion destructures `Invalid`; it must not merely assert the
value differs from one expected success because both successes are unsafe.

Optional focused tests cover missing pass reason, unknown disposition, and an
invalid root shape.

## Source ownership and commit shape

The meaningful source unit consists of both:

- `crates/lisa-core/src/disposition.rs`;
- `crates/lisa-core/src/lib.rs`.

They are committed in one isolated transaction with exact `--include` flags.
No documentation source, active ticket, unrelated worktree entry, or private
attempt artifact belongs in that transaction.

## Dependency direction

The new module depends only on the standard library and serde_json.
No existing core module depends on it in this ticket.
The later plugin ticket will depend inward on `lisa_core::disposition`, keeping
the scheduler dependency pointed toward the reusable core interpretation.

## Compatibility

Adding a public module and enum is additive.
There is no persisted Rust representation and no schema migration.
The wire behavior precisely follows the already-published JSON contract.

## Verification surfaces

Formatting covers both changed Rust files.
Focused core tests exercise every acceptance criterion.
Workspace tests validate that the new public module compiles in all current
dependency graphs.
Git status is checked before and after the isolated commit to verify only the
pre-existing unrelated changes remain.

