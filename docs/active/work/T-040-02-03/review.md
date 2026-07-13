# Review: CLI pre-ownership reconstruction

## Disposition

PASS.

T-040-02-03 satisfies its acceptance criterion.
An operator can reconstruct why a ticket ended before provider ownership with
`lisa status --ticket <ticket-id>`.
The report reads retained ledger rows only and requires no live pane.

No blocking correctness, test, commit-scope, or ownership issue was found.

## Commit reviewed

```text
2f647152c327c4d7d70dce2f7121027e9cc60cdd
feat(cli): report pre-ownership failures
```

The isolated commit contains exactly:

```text
crates/lisa-cli/src/main.rs
crates/lisa-cli/src/status.rs
crates/lisa-cli/tests/fixtures/preownership-ledger.jsonl
crates/lisa-cli/tests/preownership_status.rs
```

`git show --check` reports no whitespace errors.
All four ticket-owned paths are clean after the commit.

## Operator surface

The existing `status` command now has a focused evidence mode:

```text
lisa status --ticket T-040-02-01
```

It defaults to the production ledger at:

```text
<project-path>/.lisa/provenance.jsonl
```

An operator can inspect copied, archived, or fixture evidence explicitly:

```text
lisa status \
  --ticket T-040-02-01 \
  --ledger path/to/provenance.jsonl
```

Relative ledger overrides resolve under `--path`.
Absolute overrides remain absolute.
Clap rejects `--ledger` without `--ticket`, so the override cannot silently do
nothing in normal DAG mode.

## Compatibility with existing status

No new top-level command was added.
The operator command grouping and twelve-command help contract remain intact.

When `--ticket` is absent, dispatch calls the original `run_status` function.
Its config resolution, ticket scan, DAG validation, wave rendering, and ready
summary are unchanged.
All seven pre-existing status unit tests still pass.

Ticket evidence mode branches before config loading and ticket scanning.
That ordering is essential to the fixture-only acceptance requirement.
The command does not require `CLAUDE.md`, `.lisa.toml`, a ticket directory, or
ticket Markdown.

## Ledger decoding

The reader opens JSONL with a buffered line iterator.
It decodes every nonblank line through the shared core
`ProvenanceLedgerRecord` type.

This keeps the CLI aligned with the persisted schema rather than introducing a
second partial JSON structure.
It also preserves the compatibility contract established by T-040-02-01:

- schema-v2 terminal execution rows remain valid;
- schema-v3 assignment-transition rows remain valid;
- both shapes can coexist in one append-only ledger.

Execution rows are accepted and ignored for this report.
Assignment rows for other tickets are accepted and ignored.
All assignment rows whose top-level `ticket_id` exactly matches the query are
rendered in ledger append order.

## Evidence rendered

Each matching row reports:

- numeric attempt ID;
- historical pane ID;
- stable named assignment state;
- exact retained reason;
- exact retained provider vendor;
- `started_at` UTC epoch seconds;
- `ended_at` UTC epoch seconds;
- `wall_clock_secs` duration.

The acceptance-required state, reason, provider, and timestamps are therefore
visible without reverse-engineering a gap in scheduling activity.
Attempt and pane correlation are included because they are already durable and
help distinguish multiple failures for the same ticket.

The three state spellings are exhaustively mapped from the typed enum:

- `delivery-failed`;
- `recovery-failed`;
- `startup-failed`.

An added core state will produce a compile-time non-exhaustive-match failure,
forcing the CLI to choose its operator-visible name.

## Multiple and empty results

The reader retains every matching transition rather than selecting only the
latest row.
This preserves attempt history and avoids hiding repeated delivery/recovery
failures.
Multiple blocks are separated deterministically and retain append order.

A valid ledger with no matching assignment transition succeeds with:

```text
No pre-ownership failures found for <ticket-id>.
```

This is distinct from an absent or unreadable ledger.
An unreadable ledger is an error because treating missing evidence storage as
a valid empty result could mislead an operator.

## Error handling

Ledger open failures name the attempted path.
Physical line read failures name the path and one-based line number.
JSON/schema failures name the path and one-based line number.

The implementation validates the full ledger before writing any report.
A malformed later row therefore cannot leave a partial stdout report that
looks complete.
Blank lines are ignored, while malformed nonblank lines fail loudly.

Output failures are converted to the CLI's normal string error boundary.
Main uses the established `Error: ...` stderr and nonzero-exit behavior.

## Fixture regression

The committed fixture is literal JSONL rather than runtime serialization.
It carries the T-040-02-01 row shape:

- schema version 3;
- record type `assignment-transition`;
- ticket `T-040-02-01` at top level and in the attempt lease;
- attempt ID 7;
- pane ID 12;
- provider `openai`;
- state `delivery-failed`;
- bounded chat acknowledgement failure reason;
- deterministic start, end, and duration values.

Literal persisted bytes protect the public read contract from drifting in lock
step with a generated current struct.

## CLI-level acceptance test

`preownership_status.rs` invokes `CARGO_BIN_EXE_lisa`, so it covers:

- Clap parsing;
- the `--ledger`/`--ticket` relationship;
- main dispatch;
- path handling;
- physical fixture reading;
- mixed ledger deserialization;
- ticket filtering;
- state-name mapping;
- exact text rendering;
- process exit behavior.

The test creates no temporary project and supplies no ticket files.
It launches no Zellij process or provider client.
It sets no pane or scheduler environment variables.
It asserts successful exit, empty stderr, and exact complete stdout.

This directly proves “from a ledger fixture alone” and “with no live pane
required.”

## Unit coverage

Three focused native tests were added in `status.rs`.

`preownership_status_filters_mixed_ledger` supplies:

- a literal schema-v2 execution row;
- an unrelated schema-v3 assignment row;
- a matching schema-v3 assignment row.

It asserts the execution and unrelated rows do not leak into output and checks
the exact matching report.

`preownership_status_reports_no_matches` checks a valid execution-only ledger
and exact empty-result wording.

`preownership_status_reports_malformed_line_before_writing` checks a malformed
second line, path/line diagnostics, and the no-partial-output guarantee.

## Verification results

Focused status tests:

```text
cargo test -p lisa-cli status::tests
10 passed; 0 failed
```

Required black-box target:

```text
cargo test -p lisa-cli --test preownership_status
1 passed; 0 failed
```

Full CLI crate:

```text
cargo test -p lisa-cli
279 unit tests passed
5 non-Zellij integration tests passed
1 real-Zellij test ignored by its established environment gate
```

Full workspace and formatting:

```text
cargo test --workspace
cargo fmt --all -- --check
```

Both commands succeeded.
Observed unit totals were 279 CLI, 169 core, and 336 plugin tests, with all
enabled integration and doc-test targets successful.

## Acceptance-criterion assessment

“A lisa CLI command ... prints a ticket's pre-ownership failure evidence” — met
by `lisa status --ticket`.

“named state, reason, provider, timestamps” — met by the stable state label,
exact stored reason/vendor, and all three retained time values.

“from a ledger fixture alone” — met by the literal committed JSONL fixture and
binary-level test without project setup.

“fixture containing the T-040-02-01 row shape” — met with the exact schema-v3
assignment-transition fields introduced by that dependency.

“with no live pane required” — met; the CLI reads pane correlation from the row
and performs no pane lookup or Zellij invocation.

## Known limitations

Timestamps are displayed as canonical UTC epoch seconds rather than formatted
calendar dates.
This exactly preserves stored evidence and avoids a new time dependency, but a
future general reporting surface may add human/date or JSON output modes.

The query matches the row's top-level `ticket_id`.
It does not separately reject a malformed logical invariant where nested
`attempt_lease.ticket_id` differs.
The writers and schema tests already preserve same-ticket attribution; a
future ledger validator could enforce cross-field invariants globally.

The reader collects matching rows in memory after validating the ledger.
This ensures no partial report on malformed input and is appropriate for the
current operational ledger size.
A future high-volume query system could stream into an intermediate sink or
add indexing while retaining validation semantics.

No functional issue is open for this ticket.

## Scope preserved

No core schema changed.
No plugin emission or scheduler state changed.
No execution outcome or ownership was inferred.
No active ticket phase/status was manually edited.
No shared work artifact was directly written by this agent.
No ordinary Git index or ordinary commit command was used.

The remaining dirty/untracked worktree paths belong to Lisa lifecycle state and
concurrent tickets.
They were excluded from commit `2f64715` and remain preserved.

The ticket is ready for Lisa's completion transaction.
