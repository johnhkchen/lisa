# Design: CLI pre-ownership reconstruction

## Decision summary

Extend `lisa status` with a ticket-focused pre-ownership evidence mode.
The operator will run:

```text
lisa status --ticket <ticket-id>
```

By default it reads `<path>/.lisa/provenance.jsonl`.
For offline reports and deterministic tests it also accepts:

```text
lisa status --ticket <ticket-id> --ledger <jsonl-path>
```

When `--ticket` is present, dispatch enters ledger mode before loading project
configuration or scanning ticket files.
Without `--ticket`, the existing DAG status behavior is unchanged.

## Option 1: add a new top-level command

A command such as `lisa failures` or `lisa provenance` could own the report.

Advantages:

- clean separation from DAG status;
- room for future provenance query flags;
- a direct noun or action in command help.

Disadvantages:

- changes the deliberately pinned operator command taxonomy;
- requires expanding help-surface tests from twelve commands;
- introduces naming decisions broader than this narrow ticket;
- duplicates the existing operator promise that status explains why tickets
  are waiting or did not proceed.

This option is viable but rejected for this ticket.
The established status command is already the place an operator looks for
ticket state and reasons.

## Option 2: always append ledger evidence to DAG status

The existing `lisa status` output could inspect the default ledger and attach
failure rows to every ticket.

Advantages:

- no new arguments;
- evidence appears in the normal overview;
- potentially useful at-a-glance aggregate view.

Disadvantages:

- cannot satisfy “fixture alone” because current status first requires ticket
  Markdown and a valid DAG;
- can make an overview noisy when ledgers accumulate many attempts;
- couples historical evidence parsing to every normal status invocation;
- makes exact ticket investigation less direct;
- unclear behavior when the ledger is absent in a healthy project.

This option is rejected.
It would broaden default output and retain the wrong dependency on ticket
files.

## Option 3: optional ticket evidence mode on status

Add `--ticket <id>` and `--ledger <path>` to the existing command.

Advantages:

- preserves the existing top-level command set;
- gives an explicit one-ticket investigation path;
- can branch before all DAG and ticket filesystem work;
- naturally defaults to the production ledger location;
- permits fixture-only and copied-ledger analysis;
- leaves normal status output byte-for-byte unchanged.

Disadvantages:

- `status` now has two modes;
- `--ledger` is meaningful only with `--ticket`;
- future generalized provenance queries may eventually deserve a separate
  command.

This option is selected.
The mode split is small, explicit, and grounded in the existing operator
surface.

## Argument contract

`--ticket` is an optional string.
It is not validated against ticket Markdown because ledger-only use is a hard
requirement.
The exact string filters top-level `ticket_id` on assignment rows.

`--ledger` is an optional path.
Clap will declare that it requires `--ticket`, preventing a meaningless ledger
override in normal DAG mode.
When omitted in evidence mode, the resolved path is
`<resolved --path>/.lisa/provenance.jsonl`.
An absolute override stays absolute; a relative override is resolved under the
project path so behavior is stable when `--path` is used.

The explicit fixture path used by the integration test is absolute through
`CARGO_MANIFEST_DIR`, avoiding dependence on the test process working
directory.

## Report read boundary

Add a public CLI-module function that accepts:

- ledger path;
- ticket ID;
- an output writer.

The writer parameter makes rendering unit-testable without global stdout
capture.
The main dispatch can pass a locked stdout handle.
The black-box test still proves the real binary boundary.

The reader uses `std::fs::File` and `BufRead::lines`.
Each non-empty line is decoded as `lisa_core::provenance::ProvenanceLedgerRecord`.
This reuses schema-v2/v3 compatibility already established in core.

For each decoded row:

- ignore `Execution` rows;
- ignore assignment rows for other tickets;
- retain matching assignment rows in ledger order;
- render every match.

The implementation can render as it reads, but collecting matching typed rows
first separates file validity from output.
That prevents partial output followed by a parse error.
Collection is acceptable because provenance ledgers are operational history,
not unbounded streaming input in this CLI context.

## Error behavior

An unreadable or absent ledger returns an error naming the path.
This is preferable to “no failures,” which would conflate absent evidence
storage with a valid empty query.

A malformed nonblank row returns an error naming the path and one-based line
number.
The CLI exits nonzero through the existing `Error: ...` dispatch convention.
Valid execution rows remain accepted and ignored.

Blank lines are ignored.
Although writers emit one row per line without blanks, ignoring blanks is
friendly to copied or hand-inspected fixtures and does not hide malformed JSON.

## Empty-result behavior

A valid ledger with no matching pre-ownership rows succeeds and prints:

```text
No pre-ownership failures found for <ticket-id>.
```

This distinguishes a successful query with no matching evidence from a missing
or unreadable ledger.
It is also script-friendly because absence is a report result, not a parser or
filesystem failure.

## Rendered shape

For matching rows, print a heading with ticket ID and count, followed by one
block per append-order row.
Each block includes:

- attempt number;
- pane number;
- stable named state;
- stored reason;
- stored provider;
- `started_at` epoch seconds;
- `ended_at` epoch seconds;
- `wall_clock_secs` duration.

Example:

```text
Pre-ownership failures for T-040-02-01 (1):
Attempt 7 (pane 12)
  state: delivery-failed
  reason: provider did not acknowledge the bounded chat assignment
  provider: openai
  started_at: 1752000000
  ended_at: 1752000030
  wall_clock_secs: 30
```

The raw field labels align with the ledger schema.
This makes comparison with JSON straightforward and avoids ambiguous locale or
timezone formatting.

## Named-state rendering

Use an exhaustive local match over `AssignmentState`.
This produces exactly the same kebab-case vocabulary as serde:

- `delivery-failed`;
- `recovery-failed`;
- `startup-failed`.

An exhaustive match causes a compile error when core adds a state, forcing the
CLI to decide its visible spelling rather than silently emitting debug text.

## Fixture design

Create a committed JSONL fixture under CLI integration-test fixtures.
It will contain the literal T-040-02-01 row shape from the schema dependency:

- schema version 3;
- `assignment-transition` discriminator;
- ticket `T-040-02-01` in both attribution locations;
- attempt 7;
- pane 12;
- provider `openai`;
- state `delivery-failed`;
- the bounded-chat acknowledgement reason;
- deterministic epoch timestamps and duration.

The fixture is hand-written rather than serialized during the test.
That ensures the CLI test detects schema/render drift against persisted bytes.

## Test design

The required CLI-level test invokes the built `lisa` binary with:

- `status`;
- `--ticket T-040-02-01`;
- `--ledger <fixture>`.

It asserts success, empty stderr, and exact stdout.
Because it supplies no project ticket directory and starts no pane, success
proves the report is ledger-only.

Module unit tests will cover supporting edge cases cheaply:

- mixed ledger ignores execution and unrelated assignment rows;
- no match renders the explicit empty message;
- malformed rows report a one-based line number.

Existing status tests continue to cover default DAG mode.
Existing help tests verify the top-level command grouping remains unchanged.

## Compatibility and scope

No persisted schema changes.
No plugin changes.
No new dependency such as `chrono` is necessary.
No changes to normal status output.
No attempt to infer ownership or outcome from scheduling gaps.
The CLI reports only durable rows that the ledger actually contains.

## Future extension boundary

If provenance queries later include execution outcomes, date ranges, JSON
output, or aggregation, a dedicated command may become worthwhile.
The reader and renderer introduced here remain small enough to move to a
sibling module then.
This ticket should not pre-design that larger surface.
