# Review — T-046-01-01 version parse and supported range

## Disposition

Pass.

The ticket supplies a comparable Zellij version type, one declared supported
range with floor 0.43.0, and fail-closed classification for stable,
prerelease, below-floor, and malformed command output.

Both acceptance criteria are covered by production code and native unit tests.

## Source summary

The ticket's net source change spans exactly four repository paths:

- `crates/lisa-core/src/version.rs`;
- `crates/lisa-core/src/lib.rs`;
- `crates/lisa-core/Cargo.toml`;
- `Cargo.lock`.

No CLI, plugin, scheduler, ticket-frontmatter, or shared work-artifact source
was changed by this ticket's net commits.

## New core module

`crates/lisa-core/src/version.rs` is a new public module.

It owns the Zellij-specific process-output grammar and runtime support policy.

`crates/lisa-core/src/lib.rs` exposes it as `lisa_core::version`.

Individual items are not re-exported at crate root, matching existing focused
module import conventions.

## Comparable version type

`ZellijVersion` is a domain newtype around `semver::Version`.

Its inner value is private, so downstream code does not depend directly on the
external crate's representation.

It derives total ordering, equality, cloning, debugging, and hashing.

The total ordering handles numeric components correctly, including 0.43.10
sorting after 0.43.9.

It also handles semantic prerelease precedence, including 0.43.0-rc.1 sorting
below stable 0.43.0.

Its `Display` implementation emits canonical semantic-version text for later
diagnostics.

The const `release` constructor supports a compile-time floor declaration.

## Command-output parsing

`ZellijVersion::parse_command_output` recognizes the real CLI shape:

`zellij <semver>`

It accepts surrounding whitespace, repeated field whitespace, and trailing
newlines.

It requires the literal `zellij` product token.

It requires exactly one version token and rejects extra unrelated fields.

It delegates semantic-version syntax to `semver` rather than maintaining a
partial local parser.

Malformed output returns the named `ParseZellijVersionError`.

The error states the expected output shape and implements the standard error
trait.

## Supported range

`ZellijVersionRange` represents the supported policy as an inclusive minimum
with no maximum.

The open upper bound matches the story requirement that 0.43.x and 0.44.x pass.

`contains` is the only membership operation.

Its display form is `>= 0.43.0`, suitable for the dependent doctor and loop
diagnostics.

`SUPPORTED_ZELLIJ_RANGE` is the single production declaration of the 0.43.0
floor.

The adjacent maintenance comment identifies
`crates/lisa-plugin/Cargo.toml` and its `zellij-tile = "0.43"` pin.

It directs a pin bump to the range declaration so the runtime floor and its
documentation move together.

The same comment records Zellij 0.41.0 as the theoretical hard protocol floor.

It names `write_chars_to_pane_id` and `write_to_pane_id` as the calls that
cannot be decoded by older hosts.

It explicitly explains that Lisa enforces the tested SDK-aligned 0.43.0 floor
instead of that theoretical floor.

## Verdict API

`ZellijVersionVerdict` has exactly the three required policy outcomes:

- `InRange(ZellijVersion)`;
- `BelowFloor(ZellijVersion)`;
- `Unparseable`.

The parsed variants retain the normalized detected version for downstream
messages.

`Unparseable` contains no default or synthetic version.

`classify_zellij_version_output` parses once and then checks only the declared
range constant.

Every parse error maps to `Unparseable`; there is no fallback path that can
turn malformed text into a pass.

## Dependency changes

`crates/lisa-core/Cargo.toml` now declares `semver = "1.0"` as a direct normal
dependency.

No optional features are enabled.

The package was already present transitively in the workspace lockfile at
1.0.27.

The ticket's net `Cargo.lock` change only adds `semver` to the `lisa-core`
package dependency list.

No registry package or version changed.

## Unit-test coverage

Seven new inline tests cover the contract.

Stable in-range fixtures include:

- exact floor `zellij 0.43.0`;
- patch release `zellij 0.43.1` with a trailing newline;
- newer minor `zellij 0.44.0` with surrounding/repeated whitespace.

The below-floor stable fixture is `zellij 0.40.1`.

The test asserts the exact `BelowFloor` verdict and detected version.

The prerelease-at-floor fixture is `zellij 0.43.0-rc.1`.

It parses successfully and is classified below the stable floor.

The newer prerelease fixture is `zellij 0.44.0-rc.1`.

It parses successfully and remains above the 0.43.0 floor.

Garbage fixtures cover empty output, arbitrary text, missing product name,
missing version, invalid semantic version, wrong product name, and extra
tokens.

Every garbage fixture asserts the exact `Unparseable` verdict.

Direct comparison tests cover multi-digit patch ordering and prerelease
ordering.

Display tests cover build metadata preservation and supported-range text.

## Verification evidence

`cargo fmt --all -- --check` passed.

`git diff --check` passed for the ticket source paths.

`cargo test -p lisa-core version` passed 7 tests with 0 failures.

`cargo test -p lisa-core` passed 207 unit tests and both core integration tests
with 0 failures.

`cargo test --workspace` passed all runnable workspace tests.

The existing `real_zellij_delivery_boundary` test remained ignored under its
declared live-environment requirement; this ticket is pure parsing and does
not expand that boundary.

`just check` passed its `wasm32-wasip1` plugin check and repeated full
workspace test suite.

## Commit evidence

The implementation was committed through the required isolated transaction:

`5479aa75dda4533a836df73b3d57152242faf218`

The first commit's lockfile snapshot picked up one concurrent foreign
`directories` line while another ticket was modifying `lisa-cli`.

That line was removed through a second exact-path isolated transaction:

`2edf4e367460fde25429b04ad807f15cf264f8a0`

The net diff from the implementation commit's parent through the corrective
commit contains exactly the four planned paths.

The net lockfile diff contains only the intended `lisa-core` semver edge.

The ordinary Git index was empty after both transactions.

## Concurrency note

Concurrent T-046-02-03 work currently modifies `crates/lisa-cli/Cargo.toml`,
`crates/lisa-cli/src/doctor.rs`, and the shared lockfile's `directories` edge.

Those modifications match that ticket's runtime-cache work and are not part of
this ticket's net commits.

They were preserved and not staged, reverted, or committed here.

This exposed the repository-wide lockfile as a shared path even when tickets
otherwise own disjoint crate sources.

The corrective commit makes this ticket's durable net ownership exact; the
foreign worktree hunk remains for its owning transaction.

## Acceptance assessment

Acceptance criterion 1 passes.

Realistic stable output, prerelease output, below-floor output, and garbage
produce distinct in-range, below-floor, and unparseable verdicts.

The garbage tests prove unparseable output never passes.

Acceptance criterion 2 passes.

The supported range lives in one core constant with the zellij-tile pin
rationale and the distinct 0.41.0 hard-floor note beside it.

## Open concerns and limitations

Doctor and loop do not consume the new API in this ticket.

That is intentional and owned by dependent ticket T-046-01-02.

The parser deliberately requires the product token and exactly one semantic
version token. If a future Zellij release changes its `--version` output shape,
Lisa will fail closed as unparseable until the shared parser is updated.

The range intentionally has no upper bound because no incompatible upper
release is currently declared.

The type is not serialized because no current consumer needs persistence.

No blocking concern remains for this ticket.

## Handoff for T-046-01-02

Import from `lisa_core::version`.

Use `classify_zellij_version_output` for the command's first stdout line.

Pattern-match all three verdict variants.

Use the parsed version stored in `InRange` or `BelowFloor` for detected-version
text.

Use `SUPPORTED_ZELLIJ_RANGE.minimum` or the range's `Display` implementation
for required-floor/range text rather than repeating `0.43.0`.

Treat `Unparseable` as a named unsupported state and retain the raw command
output separately if the diagnostic needs to quote it.
