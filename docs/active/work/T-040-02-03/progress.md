# Progress: CLI pre-ownership reconstruction

## Current state

Implementation and isolated source commit are complete.
Focused, CLI-crate, workspace, and formatting verification pass.
All ticket-owned source paths are clean.

## Completed phase work

- [x] Read `CLAUDE.md`, `AGENTS.md`, the RDSPI workflow, and the ticket.
- [x] Mapped the existing status command and Clap dispatch.
- [x] Mapped help-surface command-count constraints.
- [x] Inspected the schema-v3 assignment transition row.
- [x] Inspected mixed schema-v2/schema-v3 ledger decoding.
- [x] Inspected the production `.lisa/provenance.jsonl` location.
- [x] Inspected CLI integration-test and fixture conventions.
- [x] Wrote `research.md` in the private attempt directory.
- [x] Evaluated top-level, aggregate, and status-mode command options.
- [x] Selected ticket-focused status mode.
- [x] Wrote `design.md` in the private attempt directory.
- [x] Defined the four-file CLI/test ownership boundary.
- [x] Wrote `structure.md` in the private attempt directory.
- [x] Sequenced implementation, verification, and isolated commit steps.
- [x] Wrote `plan.md` in the private attempt directory.

## CLI argument implementation

- [x] Extended `Commands::Status` without adding a top-level command.
- [x] Preserved existing `--path` behavior and display order.
- [x] Added optional `--ticket <ticket-id>`.
- [x] Added optional `--ledger <path>`.
- [x] Declared that `--ledger` requires `--ticket`.
- [x] Defaulted ticket mode to `<path>/.lisa/provenance.jsonl`.
- [x] Resolve absolute ledger overrides unchanged.
- [x] Resolve relative ledger overrides under the project path.
- [x] Branch to ledger mode before config loading or ticket scanning.
- [x] Preserve the existing `run_status` path when no ticket is supplied.

## Ledger reader implementation

- [x] Open the ledger through an actionable path-bearing error.
- [x] Read JSONL with `BufReader` and one-based physical line numbers.
- [x] Ignore blank physical lines.
- [x] Decode every nonblank row through `ProvenanceLedgerRecord`.
- [x] Retain backward-compatible schema-v2 execution rows as valid input.
- [x] Ignore execution rows for this pre-ownership report.
- [x] Ignore assignment rows belonging to other tickets.
- [x] Retain all matching assignment rows in append order.
- [x] Validate the complete ledger before writing report output.
- [x] Return the ledger path and line number for malformed content.
- [x] Return a distinct path-bearing error when the ledger cannot be opened.

## Renderer implementation

- [x] Added a stdout adapter for main dispatch.
- [x] Added an internal writer-based renderer for unit tests.
- [x] Added an exhaustive typed state-name mapping.
- [x] Render `delivery-failed`.
- [x] Render `recovery-failed`.
- [x] Render `startup-failed`.
- [x] Render a ticket heading and matching row count.
- [x] Render attempt and pane correlation.
- [x] Render the exact stored reason.
- [x] Render the exact stored provider.
- [x] Render `started_at` as stored UTC epoch seconds.
- [x] Render `ended_at` as stored UTC epoch seconds.
- [x] Render `wall_clock_secs` as stored duration.
- [x] Render multiple records with deterministic blank-line separation.
- [x] Render an explicit successful no-match message.
- [x] Convert stdout write failures into CLI errors.

## Unit-test implementation

- [x] Added a literal schema-v2 execution row to the status test module.
- [x] Added a compact schema-v3 assignment-row helper.
- [x] Tested a mixed ledger with execution, unrelated, and matching rows.
- [x] Asserted exact rendered evidence for the matching row.
- [x] Tested a valid ledger with no matching evidence.
- [x] Asserted the exact no-match report.
- [x] Tested a malformed second physical line.
- [x] Asserted the error names the ledger and line 2.
- [x] Asserted malformed input writes no partial report.

## CLI fixture and regression

- [x] Added `tests/fixtures/preownership-ledger.jsonl`.
- [x] Fixture is a literal schema-version 3 row.
- [x] Fixture uses record type `assignment-transition`.
- [x] Fixture carries T-040-02-01 at both attribution locations.
- [x] Fixture carries attempt 7 and pane 12.
- [x] Fixture carries provider `openai`.
- [x] Fixture carries state `delivery-failed`.
- [x] Fixture carries the bounded acknowledgement failure reason.
- [x] Fixture carries deterministic start/end/duration timestamps.
- [x] Added `tests/preownership_status.rs`.
- [x] Test invokes the real built `lisa` binary.
- [x] Test supplies only command arguments and the ledger fixture.
- [x] Test creates no ticket directory or project configuration.
- [x] Test starts no Zellij pane and supplies no pane environment.
- [x] Test asserts successful exit.
- [x] Test asserts empty stderr.
- [x] Test asserts exact stdout including every required field.

## Deviation log

The first focused run used `cargo fmt --check` before applying formatting.
It correctly reported rustfmt-only layout differences in the two owned Rust
files.
The files were then formatted with `cargo fmt -p lisa-cli`.
No unrelated tracked path was changed by formatting.

The first unit assertion used Rust line-continuation string syntax for expected
output.
That syntax strips indentation whitespace, so the assertion expected labels
without their two-space indent and failed.
The renderer output itself was correct.
The expected strings in the unit and black-box tests were changed to `concat!`
fragments that preserve the intended spaces.
The focused tests then passed.

No behavioral or scope deviation from Design was required.

## Focused verification

Command:

```text
cargo test -p lisa-cli status::tests
```

Result after the expectation correction:

```text
10 passed; 0 failed
```

This includes seven existing status tests and three new evidence tests.

Command:

```text
cargo test -p lisa-cli --test preownership_status
```

Result:

```text
1 passed; 0 failed
```

The real binary reconstructed the fixture without a pane.

## CLI crate verification

Command:

```text
cargo test -p lisa-cli
```

Result:

```text
279 CLI unit tests passed
1 atomic provider integration test passed
3 help-surface integration tests passed
1 pre-ownership status integration test passed
1 real-Zellij integration test remained ignored by its environment gate
```

The pinned twelve-command help contract still passes because no new top-level
command was added.

## Workspace verification

Commands:

```text
cargo test --workspace
cargo fmt --all -- --check
```

Result: successful.

Observed workspace targets include:

- 279 CLI unit tests;
- 169 core unit tests;
- 336 plugin unit tests;
- CLI integration tests;
- zero-failure doc tests.

The full suite confirms mixed ledger compatibility and no plugin regression.

## Diff verification

- [x] `git diff --check` passes for all four owned paths.
- [x] Scoped diff contains only main dispatch and status reader changes.
- [x] New files are confined to the CLI integration test and fixture.
- [x] No core source was changed.
- [x] No plugin source was changed.
- [x] No ticket frontmatter was manually edited.
- [x] No shared published phase artifact was manually written.
- [x] Unrelated Lisa ledger, active ticket, and concurrent work artifacts remain
  outside the source scope.

## Owned source paths

```text
crates/lisa-cli/src/main.rs
crates/lisa-cli/src/status.rs
crates/lisa-cli/tests/preownership_status.rs
crates/lisa-cli/tests/fixtures/preownership-ledger.jsonl
```

## Remaining implementation actions

- [x] Commit the four paths through `lisa commit-ticket`.
- [x] Verify the returned commit contains exactly those paths.
- [x] Verify `git show --check` succeeds.
- [x] Verify all four owned paths are clean.
- [ ] Write `review.md`.
- [ ] Write `review-disposition.json`.
- [ ] Remain on T-040-02-03 for Lisa completion.

## Isolated commit result

Command:

```text
target/debug/lisa commit-ticket \
  --ticket-id T-040-02-03 \
  --message "feat(cli): report pre-ownership failures" \
  --include crates/lisa-cli/src/main.rs \
  --include crates/lisa-cli/src/status.rs \
  --include crates/lisa-cli/tests/preownership_status.rs \
  --include crates/lisa-cli/tests/fixtures/preownership-ledger.jsonl
```

Returned commit:

```text
2f647152c327c4d7d70dce2f7121027e9cc60cdd
```

`git show --name-only` lists exactly the four intended paths.
`git show --check` reports no whitespace errors.
Scoped `git status --short` produces no output for the four owned paths.

The remaining worktree entries are Lisa-owned ledger/ticket/work transitions
and concurrent plugin/test work.
They were neither included nor modified by this ticket's isolated commit.
