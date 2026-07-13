# Research: CLI pre-ownership reconstruction

## Ticket boundary

T-040-02-03 asks for an operator-facing CLI/reporting surface.
The surface must reconstruct why one ticket never started from retained ledger
evidence.
It must expose the named state, reason, provider, and timestamps.
The acceptance test must use a ledger fixture alone.
It must not require a live Zellij pane or scheduler state.

The ticket begins in Research.
Its dependency T-040-02-01 is Done.
That dependency introduced the durable row schema and mixed-ledger reader type.
T-040-02-02 is also Done and writes the row at scheduler terminal sites, though
it is not an explicit dependency of this reader ticket.

## Repository state and concurrency

The worktree is not globally clean.
Lisa owns changes under `.lisa/` and active ticket frontmatter.
Another ticket has modified `crates/lisa-plugin/src/lib.rs` and added plugin
test artifacts.
This ticket does not need to modify the plugin.
The CLI files relevant to this ticket are clean at research time.
All source commits must use `lisa commit-ticket` with exact paths.

## CLI entry point

`crates/lisa-cli/src/main.rs` defines the Clap parser and dispatches every
subcommand.
The existing operator-facing commands are `init`, `validate`, `status`,
`doctor`, and `loop`.
`status` is described as showing which tickets are ready and waiting and why.
It currently accepts only a project `--path` argument.
Dispatch resolves that path and calls `status::run_status`.

The help-surface integration test deliberately pins exactly five foregrounded
operator commands and twelve total subcommands.
Adding a new top-level command would require changing those product-level
expectations.
Extending `status` can preserve the established command taxonomy.

## Current status implementation

`crates/lisa-cli/src/status.rs` owns the status command.
`run_status` currently loads project configuration, resolves the ticket
directory, scans ticket Markdown, builds the DAG, and prints scheduling waves.
It requires a ticket directory before it can produce output.
That behavior is appropriate for the existing default mode.

The module has native unit tests for empty, single-ticket, dependency-chain,
cycle, missing-dependency, missing-directory, and custom-config cases.
Those tests call `run_status` directly and do not capture stdout.
No reusable writer-based rendering boundary exists in this module today.

## Black-box CLI test conventions

`crates/lisa-cli/tests/help_surface.rs` invokes the built binary through
`env!("CARGO_BIN_EXE_lisa")` and captures `std::process::Output`.
Other CLI integration tests use the same binary boundary.
Fixture data already lives under `crates/lisa-cli/tests/fixtures/`.
A fixture-driven status test can therefore exercise argument parsing,
dispatch, file reading, mixed-row decoding, filtering, rendering, and process
exit status without a pane or plugin instance.

## Ledger location

The plugin sets its production ledger path to
`<project>/.lisa/provenance.jsonl`.
The knowledge documentation uses the same path.
The CLI does not currently centralize this path in config.
An operator report can derive the default from the status `--path` while also
allowing an explicit ledger path for fixture and offline inspection.

## Ledger schema

`crates/lisa-core/src/provenance.rs` owns all persisted provenance types.
The current `SCHEMA_VERSION` is 3.
The ledger is append-only JSONL.
It can contain both terminal execution rows and pre-ownership assignment
transition rows.

`ProvenanceLedgerRecord` is an untagged enum with two variants:

- `AssignmentTransition(AssignmentTransitionRecord)`;
- `Execution(ProvenanceRecord)`.

The untagged representation preserves unchanged schema-v2 execution rows,
which lack a record discriminator.
The assignment row is distinguishable through required `record_type`, `state`,
and `reason` fields.
Normal serde parsing can therefore decode a mixed ledger one line at a time.

## Assignment transition fields

`AssignmentTransitionRecord` contains all fields required by this ticket:

- `schema_version`;
- `record_type`;
- top-level `ticket_id`;
- `attempt_lease` with ticket ID and numeric attempt ID;
- numeric `pane_id`;
- vendor `provider`;
- stable `state`;
- human-readable `reason`;
- `started_at`;
- `ended_at`;
- `wall_clock_secs`.

The timestamp fields are UTC epoch seconds.
The duration is supplied by the writer and uses seconds as well.
The row has no execution outcome and no `authoritative` field.
That absence is intentional because ownership was never established.

## Named state vocabulary

`AssignmentState` currently has three variants:

- `DeliveryFailed`, serialized as `delivery-failed`;
- `RecoveryFailed`, serialized as `recovery-failed`;
- `StartupFailed`, serialized as `startup-failed`.

The type derives serde traits but does not implement `Display`.
A CLI renderer must map variants to their stable kebab-case names or serialize
them before display.
An explicit match keeps output independent of JSON quoting.

## Provider semantics

The provider field is a vendor string.
Current writers derive `anthropic` for Claude and `openai` for Codex.
It is not the integration method name.
The reader should retain and print the stored value rather than recompute it.

## Attempt and pane evidence

The acceptance criterion names state, reason, provider, and timestamps.
The schema additionally carries attempt and pane correlation.
Displaying those fields lets an operator distinguish multiple failed attempts
for the same ticket and connect evidence to historical pane activity.
No live pane lookup is needed because both values are retained in the row.

## Parsing boundary

JSONL needs line-oriented parsing so errors can identify the physical line.
Blank lines can be ignored safely.
Each nonblank line should decode as `ProvenanceLedgerRecord`.
Execution rows are valid ledger content but are irrelevant to this report.
Assignment rows for other tickets are also valid and should be filtered out.

Silently dropping malformed rows would create false reassurance for an
operator investigating missing evidence.
The existing codebase generally converts errors to actionable `String`
messages at CLI module boundaries.
The report should follow that convention and include the ledger path and line
number for malformed JSON.

## Output and ordering constraints

The ledger is append-only, so file order is evidence order.
The reader does not need to sort or deduplicate rows.
Multiple matching failures should all render in ledger order.
A deterministic text renderer is needed for exact CLI integration assertions.
Raw epoch seconds are already the schema's canonical timestamp representation
and require no new time dependency.

## Test isolation

The CLI-level fixture can be passed directly with an explicit `--ledger` path.
The report mode must branch before config loading and ticket-directory scans,
otherwise the fixture would not be sufficient on its own.
The integration test can run from an arbitrary temporary or repository working
directory because it supplies both the ticket ID and fixture path.

No Zellij binary, pane ID environment variable, signal directory, scheduler
state, ticket Markdown, or `.lisa.toml` is required for the report.

## Relevant files

Expected ticket-owned source scope is limited to:

- `crates/lisa-cli/src/main.rs` for Clap arguments and dispatch;
- `crates/lisa-cli/src/status.rs` for reading, filtering, and rendering;
- `crates/lisa-cli/tests/preownership_status.rs` for the black-box assertion;
- `crates/lisa-cli/tests/fixtures/preownership-ledger.jsonl` for retained input.

No core schema change is required.
No plugin scheduler change is required.
No ticket frontmatter or shared published work directory should be edited.

## Verification surfaces

Focused verification can run the new CLI integration test target and the
status module unit tests.
The full CLI crate suite checks Clap/help regressions and existing commands.
The workspace suite checks cross-crate compatibility.
`cargo fmt --all -- --check` checks Rust formatting.
`git diff --check` checks patch whitespace.

## Constraints carried into Design

The implementation must preserve default `lisa status` behavior.
The evidence mode must work from a ledger fixture alone.
It must use the shared mixed-ledger schema rather than a parallel JSON shape.
It must show all required evidence fields deterministically.
It must not consult live panes.
It must keep unrelated dirty plugin and Lisa-owned files outside the ticket
commit.
