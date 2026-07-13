# Review: field repro regression guard

## Disposition

Pass. The ticket's regression guard is implemented, all ticket-owned source is
committed through Lisa's isolated transaction, focused and workspace tests are
green, the WASM check is green, and no ticket-owned source remains staged,
modified, or untracked.

## Outcome

The repository now has one deterministic cross-crate regression for the usage
attribution incident.

It models a process-birth ticket followed by six later tickets on the same
physical Claude pane. Under the predecessor contract, all six later Stops would
reuse the first ticket's inherited environment key and replace one mutable usage
file. Under the current contract, every Stop appends an honest capture fact and
every ticket receives only the capture inside its pane-time ownership interval.

The same scenario includes:

- one valid successful capture with no pane-time owner;
- one identified empty-transcript Stop with no measured usage.

The former is held in a session-specific quarantine and raises visible plugin
activity. The latter is appended to the no-capture ledger and emits a visible
diagnostic. Neither can blend into ticket usage.

## Source changes

Five tracked source/configuration paths changed.

### `crates/lisa-cli/Cargo.toml`

Added a non-default empty Cargo feature:

```text
test-support
```

It exposes deterministic capture processing only when explicitly requested by
a test dependency. Normal CLI builds do not enable it by default.

### `crates/lisa-cli/src/lib.rs`

Conditionally exports the existing `capture_usage` module under the
`test-support` feature.

The existing `commit_transaction` library boundary remains unconditional.

No normal library consumer receives a new capture API unless it opts into the
test feature.

### `crates/lisa-cli/src/capture_usage.rs`

Separated process-specific input acquisition from Stop outcome processing.

The shared processor now receives explicit:

- input reader;
- provider selection;
- pane input;
- capture timestamp;
- diagnostic writer.

The public command adapter still gets those facts from the same production
sources:

- stdin;
- `LISA_AGENT_CLIENT`;
- `LISA_PANE_ID`;
- current system time;
- stderr.

Successful captures still append the same `CaptureRecord` schema to the same
path.

No-capture outcomes still append the same marker schema and reason strings to
the same path. Their diagnostic is now written through the supplied writer,
which is stderr in production and an in-memory buffer in the regression.

The diagnostic remains write-after: marker persistence completes before the
operator message is emitted.

A feature-gated doc-hidden wrapper exposes the deterministic processor to the
plugin test. A narrow dead-code allowance is attached because Cargo feature
unification also compiles the binary's copy of this source during plugin test
builds.

### `crates/lisa-plugin/Cargo.toml`

Enabled `lisa-cli/test-support` on the existing dev-dependency.

This does not affect plugin runtime or WASM dependencies.

### `crates/lisa-plugin/src/lib.rs`

Added:

```text
provenance_field_repro_keeps_six_recycles_distinct_and_surfaces_failures
```

The test is placed beside the existing provenance usage regressions and uses
the real CLI outcome processor, real capture ledger, real plugin usage consumer,
real pane-time ownership lookup, real provenance append, real quarantine store,
and real activity-to-UI projection.

## Field topology

One physical pane runs these seven ordered ticket intervals:

```text
T-FIELD-01
T-FIELD-02
T-FIELD-03
T-FIELD-04
T-FIELD-05
T-FIELD-06
T-FIELD-07
```

Ticket 1 represents the environment created with the native process. Tickets 2
through 7 are exactly six later recycle events.

Every ticket Stop has:

- a distinct provider session;
- an exact timestamp inside only its own interval;
- a distinct input-token total;
- a distinct output-token total.

Intervals and capture times are fixed integers. The fixture does not depend on
wall clock or scheduling speed.

## Successful capture evidence

The CLI processor writes eight successful observations:

- one unowned observation;
- seven ticket-owned observations.

The test deserializes and compares the complete ordered `CaptureRecord` vector.
This verifies pane, session, capture time, and both token totals for every row.

It also verifies:

- no no-capture session appears in the successful ledger;
- no legacy `T-FIELD-01.usage.json` exists;
- no legacy `last.usage.json` exists;
- successful Stops emit no failure diagnostic.

## Per-ticket attribution evidence

The test closes all seven execution intervals in chronological order.

For each interval, it invokes `State::read_usage` before appending the current
record, mirroring `emit_provenance` ordering. Prior records supply durable pane
history and the current record closes the in-memory interval.

The final provenance ledger is compared as a complete ordered vector of:

```text
(ticket_id, tokens_in, tokens_out)
```

All seven unique tuples match expected values after all six later recycles.
This catches both cross-ticket attribution and rewrite of an earlier row.

All cost values remain null because capture facts carry no dollar-cost
observation.

## Unattributable capture evidence

The valid unowned observation uses the same pane but a timestamp before every
owned interval.

The first closed-interval scan cannot resolve an owner and sends it through the
real quarantine branch.

The test verifies:

- the quarantine path is keyed by `session-unattributable`;
- exactly one row exists;
- source line 1 is retained;
- the complete capture is byte-semantically unchanged after deserialize;
- no provider-wide `quarantine.jsonl` exists;
- its conspicuous totals appear in no provenance row;
- exactly one activity warning names the session;
- the warning becomes a dashboard Warning activity;
- all later scans remain idempotent.

## No-capture evidence

The empty-transcript Stop is driven through the same processor with session
`session-no-capture`.

The test verifies one exact no-capture row with:

- pane 43;
- the no-capture session;
- the injected timestamp;
- reason `empty-transcript`.

The captured diagnostic contains:

- `lisa capture-usage: no capture`;
- the session ID;
- the reason.

No successful row or measured zero is fabricated for this Stop.

## Old-implementation failure mode

The predecessor at `ea9b295^` cannot satisfy the fixture:

- it resolves `LISA_TICKET_ID` as the artifact key;
- it writes `<key>.usage.json` with `std::fs::write`;
- the six later writes replace the same first-ticket file;
- it creates no append-only `captures.jsonl` facts;
- its plugin consumer reads a ticket-named file rather than pane-time rows;
- it has no session quarantine branch;
- an empty transcript returns success without marker or diagnostic.

The new guard requires the exact opposite observable state at every one of those
boundaries.

## Test coverage

### Focused new regression

```text
cargo test -p lisa-plugin provenance_field_repro -- --nocapture
1 passed; 0 failed; 381 filtered out
```

### Existing CLI boundary

```text
cargo test -p lisa-cli --test capture_usage_cli
2 passed; 0 failed
```

This retains compiled-command coverage for stale `LISA_TICKET_ID`, stdin,
environment, filesystem paths, stderr, and command exit behavior around the
refactored processor.

### Neighboring attribution coverage

```text
provenance_recycled_pane: 1 passed
provenance_unattributable: 1 passed
provenance_claude_usage: 1 passed
```

### Workspace and project gates

```text
cargo fmt --all -- --check: PASS
cargo test --workspace: 873 passed; 0 failed; 1 ignored
just check: PASS
```

The one ignored test is the pre-existing opt-in real-Zellij delivery harness.

`just check` passed both the `wasm32-wasip1` plugin check and a repeated full
workspace suite.

## Commits

```text
1347c3557455a9d64b33570907e9a5380c74ef5d
test(cli): expose deterministic capture support

4fee31cf4574962f426dde9d9f1c338d2837377a
test(plugin): replay six usage overwrites
```

The first commit contains exactly four capture-support paths. The second
contains exactly the plugin regression path.

No ordinary `git add` or `git commit` was used.

## Cleanliness

All five ticket-owned tracked paths are clean relative to HEAD.

No ticket-owned untracked file exists in either changed crate.

The ordinary index is empty.

Remaining worktree changes are Lisa-managed provenance, ticket phase, completion
journal, and admitted work artifacts. Neither isolated source commit included
them.

## Open concerns and limitations

- The test is synthetic and free; it is not a paid live metered provider rerun.
  Story `S-043-03` explicitly places live multi-pane verification outside this
  slice.
- The scenario models the incident mechanism as one pane with one initial
  ticket plus six recycles. It preserves the documented count and overwrite
  mechanism without depending on field-specific ticket names or token values.
- The combined regression enters the CLI below OS process acquisition through a
  test-only seam. Existing compiled CLI integration tests bracket stdin,
  environment, stderr, and exit behavior.
- The external report path named by the epic is not present in this checkout;
  the durable epic/story/ticket contract supplies the six-overwrite requirement.
- No plugin ingestion of `no-captures.jsonl` was added. Immediate no-capture
  visibility remains the CLI/hook stderr contract delivered by the prerequisite
  ticket; quarantine visibility remains plugin activity.
- Capture timestamps and provenance intervals share inclusive epoch-second
  resolution. The fixture deliberately separates all intervals, so it does not
  add new boundary semantics.

None of these limitations blocks the ticket acceptance criterion.

## Human review focus

A reviewer should focus on:

1. whether feature-gated test support is an acceptable cross-crate seam;
2. whether seven sequential tickets clearly encode six overwrite opportunities;
3. whether the no-capture diagnostic remains strictly after marker persistence;
4. whether the complete ordered ledger comparison is sufficient to prevent
   later silent rewrites;
5. whether the explicit live-run boundary remains correctly deferred.

No critical issue or follow-up TODO is known.
