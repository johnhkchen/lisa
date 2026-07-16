# Research — T-046-01-01 version parse and supported range

## Ticket boundary

T-046-01-01 is the first ticket in story S-046-01.

It establishes a shared version contract in `lisa-core`.

The following ticket, T-046-01-02, consumes that contract from `lisa doctor`
and `lisa loop` preflight.

This ticket does not alter doctor output, loop startup, runtime resolution, the
plugin scheduler, or any user-facing remedy text.

The acceptance criteria require three verdict classes:

- a parsed version inside the supported range;
- a parsed version below the supported floor;
- an output that cannot be parsed.

An unparseable output must never share the successful path.

The declared tested floor is Zellij 0.43.0.

The ticket separately records a theoretical API floor of Zellij 0.41.0.

The enforced floor intentionally follows what Lisa tests and pins rather than
the earliest host whose protocol can decode the pane-writing calls.

## Workspace organization

The workspace contains three Rust crates.

`crates/lisa-core` contains shared types and logic used across process and WASM
boundaries.

`crates/lisa-cli` owns commands such as `doctor` and `loop`.

`crates/lisa-plugin` owns the Zellij WASM plugin and depends on `lisa-core`.

The root `Cargo.toml` declares workspace package metadata.

The root `Cargo.lock` records a single dependency graph for the workspace.

## lisa-core module conventions

`crates/lisa-core/src/lib.rs` exposes one public module declaration per source
file.

The crate currently has modules for capture, claims, client selection,
completion, DAG logic, diagnostics, disposition, provenance, routing, tickets,
and general types.

There is no existing version-specific module or public version type.

Small shared contracts commonly live in focused top-level modules rather than
being added to the large `types.rs` file.

For example, `client.rs` contains `AgentClient`, parsing, display behavior, and
unit tests together.

`route.rs` similarly keeps its shared route value, operations, documentation,
and tests in one module.

Modules use Rustdoc comments to state ownership boundaries and downstream
usage.

Unit tests are normally inline under `#[cfg(test)] mod tests`.

Public data types generally derive `Debug`, cloning/equality traits, and other
traits justified by their value semantics.

Display implementations produce canonical user-facing forms when a value has
a natural textual representation.

## Existing dependency graph

`lisa-core` currently depends directly on serde, YAML, JSON, and thiserror.

It does not directly declare the `semver` crate.

The workspace lockfile already includes `semver` 1.0.27 as a transitive
dependency through other packages.

Because Rust requires direct dependencies for direct use, `lisa-core` would
still need its own Cargo manifest entry to expose semantic-version behavior.

The installed `semver` 1.0.27 API supplies a const `Version::new` constructor.

Its `Version` value implements semantic ordering, parsing, and display,
including prerelease and build metadata behavior.

## Zellij pin location

`crates/lisa-plugin/Cargo.toml` declares `zellij-tile = "0.43"`.

That dependency is the plugin-facing SDK pin named by the ticket.

Cargo's `0.43` requirement accepts compatible patch releases in the 0.43
minor line.

The ticket declares 0.43.0 as the runtime floor aligned with that pin.

No code currently relates the Cargo requirement to a runtime host version.

No supported-Zellij constant currently exists elsewhere in the tree.

Searches for version, supported range, and semver found only opaque CLI version
reporting and unrelated package/document version references.

## Current doctor behavior

`crates/lisa-cli/src/doctor.rs` defines a generic `CheckResult`.

Its successful form stores a `version: String`.

`get_command_version` runs a binary, checks a successful exit status, takes the
first stdout line, trims it, and returns it without interpretation.

`check_zellij` runs `zellij --version` through that helper.

Any successful process output, including empty or malformed text, currently
becomes `CheckResult::Found`.

`has_failures` regards only `CheckResult::NotFound` as a required-dependency
failure.

Consequently the current doctor path has no below-floor or unparseable state.

T-046-01-02 owns changing that behavior and consuming the shared API produced
here.

## Expected command-output shape

The real Zellij CLI identifies itself with a product token followed by a
semantic version, for example `zellij 0.43.1`.

The input crosses a process boundary and arrives as arbitrary text.

Whitespace and trailing newlines are normal command-output concerns.

Prerelease identifiers are legal semantic-version syntax, such as
`0.43.0-rc.1`.

Build metadata is also legal semantic-version syntax and does not affect
semantic precedence.

Garbage can include a missing product token, a missing version, an invalid
numeric component, or extra unrelated tokens.

## Ordering constraints

The version type must be comparable, so equality and total ordering matter.

Numeric comparison cannot safely be implemented as string comparison because
`0.43.10` must sort after `0.43.9`.

Prerelease ordering also differs from ordinary lexical ordering.

A prerelease has lower precedence than the release with the same
major/minor/patch components.

Thus `0.43.0-rc.1` is below the declared stable 0.43.0 floor.

A later minor prerelease such as `0.44.0-rc.1` remains above 0.43.0.

Build metadata does not change whether a version is supported.

## Constant constraints

The supported range currently has a lower bound and no declared upper bound.

The floor must live once in production code so consumers do not repeat the
numeric tuple.

The rationale beside it must name the `zellij-tile` 0.43 pin.

The same comment must distinguish the 0.41.0 hard protocol floor from the
0.43.0 tested and enforced floor.

Downstream callers need both a membership operation and access to the floor so
they can report detected-versus-required details.

## Test boundaries

The acceptance tests can remain native `lisa-core` unit tests.

They do not need a real Zellij process because this ticket owns pure parsing
and classification.

Stable release fixtures need to cover at least the exact floor and a newer
release.

A below-floor stable release such as 0.40.1 demonstrates numeric ordering.

A prerelease at the floor's release tuple demonstrates semantic prerelease
ordering rather than accidental numeric-only acceptance.

Garbage needs an explicit `Unparseable` assertion.

Additional focused tests can protect canonical display and direct comparison.

## Repository state and concurrency

The ordinary worktree already contains many unrelated untracked planning
files and Lisa bookkeeping changes.

Those paths belong to the user or other concurrent work and are outside this
ticket.

The ticket source paths are expected to be limited to `lisa-core`'s manifest,
module declaration, new version module, and the workspace lockfile if Cargo
updates direct dependency ownership.

The assignment requires exact-path `lisa commit-ticket` commits.

Ordinary `git add`, `git commit`, and broad staging commands are prohibited.

Attempt artifacts stay under
`.lisa/attempts/T-046-01-01/1/work/` until Lisa publishes them.
