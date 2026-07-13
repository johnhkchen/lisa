# Structure: CLI pre-ownership reconstruction

## File-level summary

Modify:

- `crates/lisa-cli/src/main.rs`;
- `crates/lisa-cli/src/status.rs`.

Create:

- `crates/lisa-cli/tests/preownership_status.rs`;
- `crates/lisa-cli/tests/fixtures/preownership-ledger.jsonl`.

Do not modify:

- `crates/lisa-core/src/provenance.rs`;
- plugin scheduler code;
- knowledge documentation outside the attempt artifacts;
- active ticket frontmatter;
- shared `docs/active/work/T-040-02-03/` paths.

## `main.rs` command shape

Extend the existing `Commands::Status` variant.
Keep `path: PathBuf` unchanged.
Add:

```text
ticket: Option<String>
ledger: Option<PathBuf>
```

The user-facing flag names are `--ticket` and `--ledger`.
The ledger flag carries Clap's `requires = "ticket"` relationship.
Help text states that ticket mode shows retained failures before ownership and
that the default ledger is `.lisa/provenance.jsonl` under the project path.

The command remains at operator display order 2.
No enum variant is added, removed, or reordered.

## `main.rs` dispatch

Destructure all three status fields:

```text
Commands::Status { path, ticket, ledger }
```

Resolve the project path with the existing `resolve_path` helper.
If `ticket` is absent, call the unchanged DAG entry point:

```text
status::run_status(&path)
```

If `ticket` is present:

1. resolve the ledger override;
2. default to `path.join(".lisa/provenance.jsonl")`;
3. call the status evidence entry point;
4. let the shared dispatch error handling print and exit.

A relative ledger override is joined to the resolved project path.
An absolute override is used unchanged.

The branch exists at dispatch time, but the evidence function itself also has
no dependency on project config or ticket scanning.

## `status.rs` imports

Retain `std::path::Path`.
Add standard-library imports for:

- `std::fs::File`;
- `std::io::{BufRead, BufReader, Write}`.

Import from core provenance:

- `AssignmentState`;
- `AssignmentTransitionRecord`;
- `ProvenanceLedgerRecord`.

No serialization implementation is duplicated in the CLI.

## Public evidence entry point

Define:

```text
pub fn run_preownership_status(
    ledger_path: &Path,
    ticket_id: &str,
) -> Result<(), String>
```

This is the stdout adapter used by `main.rs`.
It locks stdout and delegates to the writer-based function.

The existing `run_status(root)` signature stays intact so all current callers
and unit tests remain source-compatible.

## Internal writer-based function

Define a private or module-visible helper:

```text
fn write_preownership_status<W: Write>(
    ledger_path: &Path,
    ticket_id: &str,
    output: &mut W,
) -> Result<(), String>
```

Responsibilities:

1. open the ledger;
2. read it line by line;
3. skip blank lines;
4. deserialize every other line through `ProvenanceLedgerRecord`;
5. retain matching `AssignmentTransitionRecord` values;
6. render the empty or populated report;
7. convert write errors to actionable strings.

Opening and parsing errors name `ledger_path.display()`.
Parsing errors additionally name the one-based line number.
Read errors also name the line being read.

The helper validates the whole ledger before writing output.
It therefore returns no partial success report for a malformed later line.

## State-name helper

Define:

```text
fn assignment_state_name(state: AssignmentState) -> &'static str
```

Use an exhaustive match for all current variants.
The returned values are the serde-compatible kebab-case names.
This helper contains the only CLI mapping between typed state and display text.

## Output organization

For zero matches, write one line and return.

For matches, write the heading once:

```text
Pre-ownership failures for {ticket_id} ({count}):
```

For each record, write a fixed seven-line block.
Separate multiple blocks with one blank line.
Use the nested attempt lease's numeric ID and the row's pane ID for the block
heading.
Use the stored provider, reason, and timestamp values without transformation.

No color or terminal-width logic is introduced.
The output is stable under redirected and captured stdout.

## Unit-test additions in `status.rs`

Keep current DAG tests unchanged.
Extend the test module with an in-memory output vector and temporary ledger.

Test `preownership_status_filters_mixed_ledger`:

- write a valid schema-v2 execution row;
- write an assignment row for another ticket;
- write one matching assignment row;
- call the writer helper;
- assert only the matching row is rendered;
- assert stable state and evidence values.

Test `preownership_status_reports_no_matches`:

- use a valid ledger with no matching transition;
- assert exact empty-result output.

Test `preownership_status_reports_malformed_line`:

- include a valid first line and malformed second line;
- assert an error with the path and `line 2`;
- assert no bytes were written before validation completed.

Fixture duplication in unit tests should be minimized.
Small literal rows are acceptable because they specifically exercise mixed
schema decoding.

## Integration-test file

`crates/lisa-cli/tests/preownership_status.rs` is a black-box test target.
It imports only standard-library process and path utilities.

Build the fixture path from:

```text
PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("tests/fixtures/preownership-ledger.jsonl")
```

Invoke `env!("CARGO_BIN_EXE_lisa")`.
Pass the ticket and absolute fixture path.
Do not create ticket files, launch Zellij, or set pane environment variables.

Assert:

- exit status is successful;
- stderr is empty;
- stdout exactly equals the complete expected report including final newline.

The test name should state that reconstruction works from a ledger fixture
without a pane.

## Fixture file

The fixture contains compact JSONL with a trailing newline.
Its assignment row exactly follows `AssignmentTransitionRecord`:

```text
schema_version = 3
record_type = assignment-transition
ticket_id = T-040-02-01
attempt_lease.ticket_id = T-040-02-01
attempt_lease.attempt_id = 7
pane_id = 12
provider = openai
state = delivery-failed
reason = provider did not acknowledge the bounded chat assignment
started_at = 1752000000
ended_at = 1752000030
wall_clock_secs = 30
```

This is the row shape introduced by T-040-02-01.
The fixture is committed as ticket-owned test evidence.

## Interface boundaries

Core remains responsible for persisted schemas and compatibility decoding.
Status remains responsible for CLI-specific selection and human rendering.
Main remains responsible for argument relationships and path resolution.
The integration test owns the public command contract.

The plugin remains solely responsible for emission timing and scheduler
authority.
The report does not read or infer plugin state.

## Change ordering

1. add status reader/renderer and native unit tests;
2. add main argument and dispatch wiring;
3. add the literal fixture and black-box test;
4. format and run focused tests;
5. run CLI and workspace regression suites;
6. commit all four ticket-owned paths in one isolated transaction.

The main and status changes form one functional source unit because neither
provides the accepted CLI behavior alone.
The fixture and integration test are part of that same regression unit.

## Ownership boundary

The isolated commit includes exactly the four listed CLI paths.
It excludes private phase artifacts, active tickets, `.lisa/provenance.jsonl`,
and all plugin changes.
After commit, those four source/test paths must be clean.
