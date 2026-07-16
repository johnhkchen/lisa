# Design — T-046-01-01 version parse and supported range

## Objective

Add a small, reusable Zellij runtime-version contract to `lisa-core`.

The contract must parse actual `zellij --version` output, preserve semantic
version ordering, and classify the result against one declared supported
range.

It must fail closed when the process output is not recognizable.

The contract must be sufficient for T-046-01-02 to report and enforce the
floor without repeating parsing rules or version literals.

## Option 1 — compare raw strings

The smallest representation could retain the version token as `String` and
compare it lexically with `"0.43.0"`.

Advantages:

- no new dependency;
- very little code;
- preserves the original token for display.

Disadvantages:

- lexical ordering puts `0.43.10` before `0.43.9`;
- it does not model prerelease precedence;
- malformed numeric values can look ordered even though they are invalid;
- callers could accidentally compare noncanonical strings.

Rejected because the ticket explicitly asks for a comparable version type,
not a display string with unreliable ordering.

## Option 2 — parse only a numeric triple

Lisa could define a structure with `major`, `minor`, and `patch` integers and
derive total ordering.

Advantages:

- const construction is straightforward;
- no external parser is required;
- release versions compare correctly;
- public fields could make diagnostics easy to format.

Disadvantages:

- prerelease input must either be rejected or stripped;
- stripping would incorrectly treat `0.43.0-rc.1` as equal to stable 0.43.0;
- rejecting all prereleases does not provide a parsed below-floor verdict for
  a real semantic-version form;
- extending the parser toward full semantic versions duplicates established
  rules and increases risk.

Rejected because prerelease behavior is an explicit acceptance boundary.

## Option 3 — expose `semver::Version` directly

The module could use a public type alias and a free-standing floor constant.

Advantages:

- complete semantic parsing and ordering;
- familiar API for Rust callers;
- very little wrapper code.

Disadvantages:

- a type alias does not identify a version as specifically a Zellij runtime;
- downstream callers could parse arbitrary version strings without enforcing
  the command-output shape;
- the external crate becomes Lisa's public vocabulary;
- the supported range still needs a Lisa-owned representation;
- future Zellij-specific parsing changes would leak through call sites.

Rejected as the public contract, though the semantic implementation is useful
internally.

## Option 4 — Lisa newtype backed by `semver::Version`

Define `ZellijVersion` as a public newtype whose inner semantic version remains
private.

Delegate parsing of the version token, comparison, hashing, and display to
`semver::Version`.

Add a const release constructor for the declared floor.

Advantages:

- gives the domain value a distinct type;
- inherits complete and tested semantic precedence;
- supports const construction because semver 1.0.27 exposes const
  `Version::new`;
- prevents consumers from depending on the external representation;
- keeps command-output parsing at a single entry point;
- preserves a canonical version string for diagnostics.

Disadvantages:

- adds `semver` as a direct `lisa-core` dependency;
- requires a thin set of trait implementations;
- the wrapper intentionally exposes less of semver's API.

Chosen because it supplies correct ordering with a narrow Lisa-owned public
surface.

## Supported-range representation

Define `ZellijVersionRange` with a public `minimum: ZellijVersion` field.

Do not add a maximum field because the story declares a floor and tests newer
minor releases as supported.

Expose `contains(&self, version: &ZellijVersion) -> bool`.

Implement `Display` as `>= <minimum>` so doctor can render the declared range
without reconstructing wording or numeric literals.

Declare one public `SUPPORTED_ZELLIJ_RANGE` constant containing release
0.43.0.

Place the pin rationale directly on that constant.

The comment will state that the tested floor matches
`crates/lisa-plugin/Cargo.toml`'s `zellij-tile = "0.43"` pin.

The same comment will state that 0.41.0 is only the theoretical protocol floor
for decoding `write_chars_to_pane_id`/`write_to_pane_id`.

This makes the range constant the single runtime-support source of truth and
the obvious edit site paired with any SDK pin bump.

## Parsing boundary

Expose `ZellijVersion::parse_command_output(&str) -> Result<Self,
ParseZellijVersionError>`.

The parser trims surrounding whitespace, then requires exactly two
whitespace-separated fields:

1. the literal product name `zellij`;
2. one semantic-version token.

The product name check prevents an unrelated tool's version output from being
accepted merely because it contains a semantic version.

Exactly two fields reject empty output, missing versions, and unrelated suffix
text.

Ordinary trailing newlines and spacing remain accepted through
`split_whitespace`.

The version token is passed to `semver::Version::parse`.

The public parse error is a zero-sized domain error rather than the semver
parser's detailed error.

Its display text names the expected `zellij <semver>` shape.

The caller still owns the original output and can include it in its own
diagnostic in T-046-01-02.

## Verdict representation

Define `ZellijVersionVerdict` with three variants:

- `InRange(ZellijVersion)`;
- `BelowFloor(ZellijVersion)`;
- `Unparseable`.

Expose `classify_zellij_version_output(&str) -> ZellijVersionVerdict` as the
one-step consumer API.

On successful parsing, the function asks `SUPPORTED_ZELLIJ_RANGE.contains`.

On parse failure, it returns `Unparseable` without manufacturing a version or
defaulting to support.

The parsed version is retained in both parsed verdicts so doctor and loop can
name the detected value.

The floor remains accessible through `SUPPORTED_ZELLIJ_RANGE.minimum` rather
than being copied into the verdict.

This avoids duplicating invariant data while keeping pattern matching simple.

## Trait surface

`ZellijVersion` derives `Debug`, `Clone`, `PartialEq`, `Eq`, `PartialOrd`,
`Ord`, and `Hash`.

Those traits make it an ordinary immutable value for comparisons, maps, and
tests.

It implements `Display` for canonical semantic-version text.

It does not expose mutation of the wrapped semver value.

`ZellijVersionRange` derives copy/equality/order traits because its floor value
is immutable and const-constructible.

`ZellijVersionVerdict` derives debug/clone/equality traits for direct tests and
downstream matching.

## Prerelease semantics

Semantic precedence controls classification.

`zellij 0.43.0-rc.1` parses successfully but is below the stable 0.43.0 floor.

That produces `BelowFloor`, not `Unparseable` and not `InRange`.

`zellij 0.44.0-rc.1` is above 0.43.0 and therefore in range.

Build metadata is accepted and ignored for precedence as semantic versioning
requires.

This separates syntactic validity from support policy.

## Test design

Add inline unit tests in the new module.

Stable fixtures cover exact floor, patch release, newer minor, and old host.

Prerelease fixtures cover a prerelease at the stable floor and one above it.

Garbage fixtures cover arbitrary text, an invalid semantic version, missing
product token, and extra tokens.

Comparison tests directly demonstrate numeric patch ordering and prerelease
ordering.

Display tests protect canonical version and supported-range rendering for the
dependent CLI ticket.

No process stub or CLI test belongs here because no command is executed in
this ticket.

## Rejected scope

Do not modify `doctor.rs` or `loop_cmd.rs`.

Do not add installation advice.

Do not introduce a maximum supported Zellij version without evidence.

Do not change the `zellij-tile` dependency itself.

Do not alter scheduler behavior.

Do not serialize the new values until a consumer demonstrates that need.
