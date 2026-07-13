# Design: deterministic field-incident replay

## Decision summary

Add one deterministic plugin regression that drives the real CLI capture
outcome processor, then drives the real plugin attribution consumer over seven
sequential ownership intervals on one recycled pane. Seven tickets represent
the first pane assignment plus six later assignments that the old stale
environment key would have overwritten. The same fixture includes one
unattributable successful capture and one empty-transcript Stop.

Make the CLI outcome processor injectable at the process boundary: explicit
input reader, provider selection, pane ID, timestamp, and diagnostic writer.
Keep `run_capture_usage` as the unchanged command-facing adapter that supplies
stdin, environment, current time, and stderr. Export the deterministic wrapper
only through a `test-support` Cargo feature used by the plugin's existing CLI
dev-dependency.

## Goals

- Encode all six field overwrite opportunities in a durable test.
- Exercise the append-only capture writer rather than manufacturing every
  successful row directly.
- Exercise the pane-time consumer used by terminal provenance emission.
- Prove every ticket retains distinct, correctly attributed usage.
- Prove no later pane recycle rewrites an earlier provenance row.
- Prove an unowned successful observation is quarantined by session ID.
- Prove quarantine raises an operator-visible activity warning.
- Prove an identified no-capture Stop writes a durable marker.
- Prove the no-capture Stop emits a visible diagnostic rather than silence.
- Keep the test independent of wall clock, global environment mutation, binary
  discovery, and Cargo build order.
- Preserve all production behavior implemented by prerequisite tickets.

## Non-goals

- Do not change the pane-time ownership algorithm.
- Do not change capture or provenance schemas.
- Do not teach the plugin to consume `no-captures.jsonl`.
- Do not add cost calculation or a usage dashboard.
- Do not address Claude/Codex cache-token parity.
- Do not reconstruct the missing external field-report document.
- Do not reproduce a paid live provider run.
- Do not add a broadly supported public library API for capture processing.

## Option 1: enlarge only the existing plugin consumer test

Replace the two-ticket fixture with seven tickets and append deterministic
`CaptureRecord` values directly.

Advantages:

- Smallest source diff.
- No CLI refactor.
- Fully deterministic.
- Directly tests pane-time attribution and quarantine.

Disadvantages:

- Does not drive the writer that formerly trusted `LISA_TICKET_ID`.
- Cannot prove no-capture stderr behavior in the same scenario.
- A future writer regression could restore overwrite behavior while this test
  stayed green.
- The test would reproduce the consumer shape, not the complete incident path.

Rejected because the ticket explicitly ties failure against old env-keyed
overwrite logic to attribution and visible no-capture behavior together.

## Option 2: add only a larger CLI binary integration test

Invoke the compiled `lisa capture-usage` command seven times with a stale
`LISA_TICKET_ID`, then once with an empty transcript.

Advantages:

- Directly proves the old writer contract is gone.
- Exercises stdin, environment, parsing, append persistence, and stderr.
- Fits the existing `capture_usage_cli.rs` harness.

Disadvantages:

- The CLI intentionally has no authoritative ticket ownership.
- It cannot prove the seven observations reach seven provenance records.
- It cannot exercise `owner_at`, `read_usage`, or quarantine.
- Reimplementing attribution assertions in the CLI test would not protect the
  plugin consumer.

Rejected because it covers only the producer half of the acceptance criterion.

## Option 3: run the CLI binary from a plugin unit test

Have the plugin fixture spawn `lisa capture-usage` for every Stop, then consume
the emitted ledger.

Advantages:

- One test reaches both packages.
- Exercises the full command-facing boundary.
- Requires no new callable API.

Disadvantages:

- `CARGO_BIN_EXE_lisa` is not guaranteed in another package's unit tests.
- Locating `target/debug/lisa` depends on build order and target directory.
- Spawning nested Cargo can deadlock on build locks and makes focused plugin
  tests expensive.
- Wall-clock timestamps can collide at one-second resolution.
- Process-global environment would still need careful setup for every child.

Rejected as an unreliable regression harness.

## Option 4: expose a normal public capture library API

Move `capture_usage` into the `lisa-cli` library and publish a parameterized
processing function unconditionally.

Advantages:

- Clean dependency access from the plugin test.
- Easy deterministic input injection.
- Could support future alternate frontends.

Disadvantages:

- Creates a production API solely for a regression test.
- Expands the supported CLI library surface without a product requirement.
- Makes internal payload and diagnostic details harder to evolve.

Rejected in its unconditional form because the test seam should remain narrow.

## Option 5: feature-gated deterministic test support

Refactor the writer internally around a parameterized outcome processor. Keep
the command wrapper. Expose a small test wrapper from the CLI library only when
the `test-support` feature is enabled. Enable that feature only on the plugin's
dev-dependency.

Advantages:

- One fixture exercises the actual successful/no-capture processing code and
  the actual plugin attribution code.
- Timestamps and diagnostics are deterministic.
- No process environment is mutated by the regression.
- No subprocess or build-order dependency is introduced.
- The normal CLI library API remains unchanged.
- Production command behavior remains at the same boundary.

Disadvantages:

- Adds a Cargo feature and library module registration.
- `capture_usage.rs` is compiled into both the binary module and feature-gated
  library module during test builds.
- The internal function signature becomes more parameterized.
- The regression still bypasses the Clap command dispatcher and OS stdin.

Chosen because it protects the complete correctness chain with bounded,
test-only surface area.

## Writer seam

Retain:

```text
run_capture_usage(cwd)
```

It will:

1. lock/read stdin through the shared processor;
2. derive Claude versus Codex from `LISA_AGENT_CLIENT`;
3. parse `LISA_PANE_ID` with the existing validation;
4. obtain the capture timestamp from `SystemTime::now()`;
5. lock stderr as the diagnostic destination;
6. return the same `io::Result<()>` to command dispatch.

The internal processor will accept:

```text
cwd, reader, is_codex, pane_id, captured_at, diagnostics
```

It will retain all existing payload parsing, transcript parsing, append, marker,
and error behavior. The no-capture helper will receive `captured_at` and a
generic `Write` destination instead of calling time and `eprintln!` itself.

The feature-gated wrapper will expose those deterministic inputs to another
crate's tests. It will not expose private marker or parser types.

## Field fixture topology

Use one physical pane and seven ticket IDs:

```text
T-FIELD-01 -> T-FIELD-02 -> ... -> T-FIELD-07
```

Each interval is non-overlapping and separated by a gap. Each successful Stop
falls strictly inside its ticket's interval. Ticket `n` receives deliberately
unique totals, for example input `n * 100 + 7` and output `n * 10 + 3` after
Claude transcript parsing.

The six tickets after `T-FIELD-01` are the six writes that old logic would have
keyed to the pane's first inherited ticket and overwritten in place.

Before the first owned interval, drive another successful Stop on the same pane
with session `session-unattributable`. It becomes a valid capture row but has no
pane-time owner once the first terminal record closes. The first consumer scan
must quarantine it, exclude its tokens, and raise exactly one warning.

Drive one empty-transcript Stop with session `session-no-capture`. It must append
one no-capture row and write a diagnostic containing the session and
`empty-transcript`. It must not append a measured capture row.

## Provenance replay

For each of the seven intervals in chronological order:

1. Build the current null-usage `ProvenanceRecord`.
2. Call the real `State::read_usage` with prior ledger rows plus current.
3. Fill the returned totals into the current record.
4. Append through `provenance::append_record`.
5. Re-read the ledger and assert all preceding bytes/values remain represented.

Using `read_usage` directly permits exact deterministic interval endpoints while
still exercising the consumer used by `emit_provenance`. Existing focused tests
continue to cover the thread-derived terminal adapter.

## Assertions

- `captures.jsonl` has exactly eight rows: seven owned plus one unowned.
- Every row has the recycled pane ID and its expected session/tokens/time.
- No stale first-ticket usage artifact exists.
- No `last.usage.json` exists.
- The provenance ledger has exactly seven ordered execution rows.
- Each ticket ID and token pair matches only its own Stop.
- No record is overwritten when later tickets are appended.
- The unowned session has exactly one quarantine row with its source line and
  unchanged capture.
- No provider-wide `quarantine.jsonl` exists.
- The activity log has one quarantine warning naming the session.
- The warning maps to a visible UI warning.
- `no-captures.jsonl` has exactly one marker for the no-capture session.
- The marker reason is `empty-transcript` and uses the injected timestamp.
- Diagnostic output visibly names no capture, session, and reason.
- No-capture tokens do not appear in any capture or provenance row.

## Failure against the old implementation

The old writer would fail this guard before attribution:

- it would create `<stale-first-ticket>.usage.json`;
- each of the six later Stops would overwrite that file;
- `captures.jsonl` would not contain the eight append-only facts;
- the empty transcript would return success without a marker or diagnostic.

The old plugin consumer would also look up a ticket-named artifact instead of
resolving every capture through pane-time intervals, so it could not produce the
seven distinct provenance totals or session quarantine.

## Compatibility and risk

- Command input, environment names, output files, schemas, reasons, and exit
  behavior remain unchanged.
- Generic `Read`/`Write` parameterization follows standard Rust testability
  patterns.
- A feature-gated module can accidentally drift from the binary's module if the
  two compile paths use different configuration; both reference the same source
  file, so ordinary workspace tests compile both.
- Test-support must not be enabled in normal dependencies or release behavior.
- Focused CLI tests guard the process-facing wrapper after refactoring.
- Workspace tests and `just check` guard broader compatibility.
