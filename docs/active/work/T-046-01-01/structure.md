# Structure — T-046-01-01 version parse and supported range

## Change summary

This ticket adds one focused public module to `lisa-core`.

It changes one crate manifest to declare semantic-version parsing directly.

It updates the workspace lockfile only as required to associate that direct
dependency with `lisa-core`.

No files are deleted.

No CLI or plugin source file changes.

## File: `crates/lisa-core/src/version.rs`

This is a new production module and the primary source unit.

It owns all Zellij runtime-version vocabulary introduced by the ticket.

The module-level documentation explains that it parses the external CLI output
and provides the support policy used by later CLI checks.

The module is organized in the following order:

1. standard-library and semver imports;
2. `ZellijVersion` value type;
3. parsing and construction methods;
4. display implementation;
5. parse error type and error/display implementations;
6. `ZellijVersionRange` value type and methods;
7. supported-range constant with pin and hard-floor rationale;
8. range display implementation;
9. `ZellijVersionVerdict` enum;
10. top-level classification function;
11. inline unit tests.

### `ZellijVersion`

Public tuple newtype with a private `semver::Version` field.

Public traits:

- `Debug`;
- `Clone`;
- `PartialEq` and `Eq`;
- `PartialOrd` and `Ord`;
- `Hash`.

Public methods:

- `pub const fn release(major: u64, minor: u64, patch: u64) -> Self`;
- `pub fn parse_command_output(output: &str) -> Result<Self,
  ParseZellijVersionError>`.

The release constructor is the const path used by the supported-range
constant.

The parse method owns the external process-output grammar.

`Display` delegates to the wrapped semantic version.

The private field prevents consumers from bypassing the Zellij-specific
parsing boundary accidentally.

### `ParseZellijVersionError`

Public zero-sized error type.

It derives debug, clone, copy, and equality traits.

It implements `Display` with a stable statement of the expected output shape.

It implements `std::error::Error` so callers can use ordinary error plumbing.

Internal semantic-parser details are intentionally not part of the public
error contract.

### `ZellijVersionRange`

Public immutable range value containing one field:

- `pub minimum: ZellijVersion`.

Public method:

- `pub fn contains(&self, version: &ZellijVersion) -> bool`.

`contains` performs an inclusive lower-bound comparison.

There is no maximum field because the declared support policy is open-ended.

`Display` produces `>= 0.43.0` for concise diagnostic consumption.

### `SUPPORTED_ZELLIJ_RANGE`

Public const of type `ZellijVersionRange`.

Its only numeric definition is `ZellijVersion::release(0, 43, 0)`.

The adjacent Rustdoc is part of the maintenance contract.

It identifies the plugin Cargo manifest and its `zellij-tile = "0.43"` pin.

It tells maintainers to review this constant when bumping that pin.

It documents 0.41.0 as the theoretical host-protocol floor for
`write_chars_to_pane_id` and `write_to_pane_id`.

It explicitly states that Lisa enforces the tested 0.43.0 floor instead.

### `ZellijVersionVerdict`

Public enum with exactly three policy outcomes:

- `InRange(ZellijVersion)`;
- `BelowFloor(ZellijVersion)`;
- `Unparseable`.

The parsed variants retain the normalized detected version.

The unparseable variant carries no synthetic version and cannot be mistaken
for successful detection.

### `classify_zellij_version_output`

Public free function taking `&str` and returning `ZellijVersionVerdict`.

It is the preferred downstream entry point.

It calls `ZellijVersion::parse_command_output` exactly once.

Successful parses are classified solely through
`SUPPORTED_ZELLIJ_RANGE.contains`.

Errors map only to `Unparseable`.

### Unit-test organization

Tests use small helpers only when they improve fixture readability.

One test groups stable in-range examples.

One test protects below-floor stable classification.

One test protects prerelease-at-floor classification.

One test protects a newer prerelease as supported.

One test groups garbage forms and asserts all are unparseable.

One comparison test proves direct total ordering is semantic.

One display test protects canonical output and range text.

## File: `crates/lisa-core/src/lib.rs`

Add `pub mod version;` alongside the existing public modules.

Do not re-export individual version types at crate root.

Existing consumers conventionally import focused module values, such as
`lisa_core::client::AgentClient`.

The downstream path will therefore be `lisa_core::version::...`.

No other declarations change.

## File: `crates/lisa-core/Cargo.toml`

Add `semver = "1.0"` under normal dependencies.

It is a production dependency because runtime CLI parsing uses it.

No serde feature is requested because these values are not serialized by this
ticket.

No dev-only dependency is sufficient because the production parser and
ordering delegate to semver.

Keep the dependency list in the existing simple key/value style.

## File: `Cargo.lock`

Cargo will add `semver` to the dependency list of the existing `lisa-core`
package record.

The semver package entry already exists transitively at version 1.0.27.

No new registry package is expected.

The lockfile change remains ticket-owned because it records the new direct
workspace dependency relationship.

## Artifact: `progress.md`

Track implementation steps, verification commands, and any plan deviations in
the private attempt directory.

This file is not committed with source through `lisa commit-ticket`.

Lisa publishes admitted workflow artifacts separately.

## Review artifacts

Write `review.md` in the private attempt directory after source verification
and ticket commit.

Write `review-disposition.json` beside it with the exact pass/block schema.

These are not source paths in the ticket commit.

## Dependency direction

`lisa-core::version` depends only on `std` and the direct `semver` crate.

The module does not depend on CLI types.

The CLI can consume the module in T-046-01-02 because `lisa-cli` already
depends on `lisa-core`.

The plugin also sees the type through its existing `lisa-core` dependency but
does not need to use it in this ticket.

There is no reverse dependency from core to CLI or plugin.

## Source ownership and commit boundary

The meaningful implementation unit is the shared contract plus its manifest
and lockfile wiring.

Its exact commit include paths are:

- `crates/lisa-core/src/version.rs`;
- `crates/lisa-core/src/lib.rs`;
- `crates/lisa-core/Cargo.toml`;
- `Cargo.lock`.

These four paths form one atomic unit because none of the individual manifest,
module, or implementation changes should land alone.

No ticket, story, provenance, completion journal, or unrelated untracked path
belongs in that transaction.

## Verification boundary

Run formatting checks for the new Rust source.

Run the focused `lisa-core` test target first.

Run the full workspace tests because a public core dependency can affect all
three crates.

Run the project's quick `just check` command if available after focused tests.

Inspect the exact diff and status before committing.

After `lisa commit-ticket`, confirm none of the four ticket-owned paths remains
staged, modified, or untracked.
