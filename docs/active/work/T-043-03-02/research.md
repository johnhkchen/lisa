# Research: quarantine unattributable usage

## Ticket boundary

Ticket `T-043-03-02` starts in Research.

Its acceptance criterion concerns a successful, syntactically valid usage
capture that cannot be mapped to a ticket at its pane and timestamp.

The required observable results are:

- the capture is held aside;
- the holding location is keyed by provider session ID;
- an operator-visible activity event is raised;
- no capture is combined into a shared fallback bucket.

The ticket follows `T-043-03-01` and extends the consumer introduced there.

The full field-incident reproduction remains assigned to `T-043-03-03`.

Cache-dimension parity and new reporting surfaces are outside story scope.

## Capture production

`crates/lisa-core/src/capture.rs` defines `CaptureRecord`.

A capture contains only observed facts:

- `pane_id: u32`;
- `session_id: String`;
- `captured_at: u64` epoch seconds;
- `input_tokens: u64`;
- `output_tokens: u64`.

The schema intentionally contains no ticket ID.

The module documents that ticket attribution belongs to a later consumer.

`append_capture_record` serializes one compact JSON object plus a newline.

It creates the parent directory and opens the destination with append mode.

It never rewrites or truncates existing rows.

The core test proves round-trip serialization and same-pane append behavior.

`crates/lisa-cli/src/capture_usage.rs` is the native capture producer.

It derives the physical pane and provider session from hook context.

It reads the provider transcript and derives aggregate token totals.

Successful observations append to `.lisa/<client>/captures.jsonl`.

Stops without observable usage append to `no-captures.jsonl` instead.

Capture persistence failures are returned to the CLI caller.

The producer does not choose a ticket or fallback attribution key.

## Provider storage boundary

`State` in `crates/lisa-plugin/src/lib.rs` owns two provider directories.

`codex_dir` points at `.lisa/codex` below the mounted project root.

`claude_dir` points at `.lisa/claude` below the mounted project root.

Both are initialized in plugin `load()`.

Native tests replace them with temporary directories.

The generated `.lisa/.gitignore` ignores both provider directories.

Capture, no-capture, and related provider artifacts are machine-owned state.

They are not ticket DAG inputs and are not intended for Git commits.

No quarantine path or quarantine record type currently exists.

No production code currently writes a `last` or shared usage bucket.

The old shared/fallback behavior was removed from the active capture writer by
the preceding story tickets.

## Provenance emission

`State::emit_provenance` runs while a finishing thread is still in memory.

It derives the thread's provider client, pane, start, and end time.

It builds a provisional `ProvenanceRecord` with null usage.

It calls `read_usage(client, &record)` before appending that record.

It fills returned token values into the final execution record.

It then appends the record to `.lisa/provenance.jsonl`.

Write failures become `ActivityEvent::Error` and do not crash the loop.

The current execution record is not yet durable during usage attribution.

Consequently `read_usage` combines prior durable execution rows with the
current in-memory record.

Assignment-transition provenance rows do not establish provider ownership.

## Existing usage consumer

`State::read_usage` is in `crates/lisa-plugin/src/lib.rs`.

It currently borrows `State` immutably.

It selects the provider directory from the supplied `AgentClient`.

It reads the provider's `captures.jsonl` as text.

A missing or unreadable capture ledger returns entirely null usage.

It reads the provenance ledger and parses `ProvenanceLedgerRecord` rows.

Only execution rows enter its ownership history.

Malformed provenance and capture rows are skipped.

The consumer scans the complete capture ledger on every terminal emission.

It first filters captures to the current physical pane.

It calls `ownership::owner_at` for the capture pane and capture timestamp.

Only rows whose unique owner equals the current ticket are summed.

Rows owned by another ticket are ignored for the current record.

Rows for another pane are ignored for the current record.

Rows for which `owner_at` returns `None` are also silently ignored today.

Token addition uses `checked_add`.

An overflow fails closed to null usage.

No capture means null token values, while a measured zero remains concrete.

Capture records do not carry cost, so cost remains null.

## Ownership confidence boundary

`crates/lisa-plugin/src/ownership.rs` defines `owner_at`.

It accepts an iterator of `ProvenanceRecord` references.

It selects records for the requested pane whose closed interval contains the
capture timestamp.

Both interval endpoints are inclusive because timestamps have second-level
resolution.

One matching ticket returns that ticket ID.

Repeated matching records for the same ticket retain that answer.

Overlapping matching records for different tickets return `None`.

No matching interval returns `None`.

Record iteration order does not resolve conflicting evidence.

The function therefore already expresses the ticket's “cannot confidently
map” condition.

The `None` result covers both missing ownership and ambiguous ownership.

## Rescan and timing behavior

The input ledger is append-only and is rescanned from its first row each time.

An unattributable row can therefore be encountered during multiple later
ticket teardowns on the same pane.

Any durable side effect added at this seam must account for repeat observation.

The existing recycled-pane regression preloads captures for both A and B.

It attributes A before B's execution record has been appended.

B's capture timestamps are later than A's closed interval.

At that intermediate point, `owner_at` cannot yet see B's current interval.

Production normally appends captures over time, but the consumer contract is
also exercised with preloaded append-only data.

The current record's `ended_at` is the latest closed ownership boundary known
for its pane during the call.

A same-pane capture after that boundary is not yet proven permanently
unattributable by the available history.

Other panes may also have live in-memory threads whose terminal provenance has
not yet been appended.

The current consumer avoids that cross-pane uncertainty by filtering to the
current pane before calling `owner_at`.

## Activity event path

`lisa_core::types::ActivityEvent` is the shared activity vocabulary.

It includes generic `Error`, `Warning`, and `Info` variants.

`State::log_activity` appends events to the bounded in-memory activity log.

The log is capped by `MAX_ACTIVITY_LOG` and drops its oldest row when full.

`activity_event_to_ui_entry` maps `ActivityEvent::Warning` to
`ui::ActivityType::Warning`.

The resulting entry is included in the dashboard activity feed.

`format_activity_event` also includes warning text in textual snapshots.

Generic warning events are therefore already operator-visible without adding
a new UI component.

`read_usage` cannot currently log because it takes `&self`.

Its caller, `emit_provenance`, already takes `&mut self`.

## Existing tests

Plugin provenance tests live in the large test module in `lib.rs`.

`provenance_codex_usage_flows_into_record` covers a current-pane Codex capture.

`provenance_claude_usage_flows_into_record` covers provider directory routing.

`provenance_recycled_pane_attributes_capture_sums_to_each_ticket` covers
multiple captures, recycled pane ownership, summation, and append semantics.

`provenance_claude_record_has_null_tokens` covers absent capture behavior.

Ownership unit tests cover gaps, wrong panes, inclusive endpoints, duplicate
same-ticket intervals, and conflicting overlaps.

Activity conversion tests establish that warning events produce visible UI
entries.

Test helpers construct temporary provider directories and provenance ledgers.

No existing test asserts quarantine persistence or a quarantine event.

## Constraints surfaced by the repository

The quarantine input can only be a successfully parsed `CaptureRecord` because
an invalid row has no reliable session identity.

Session ID is opaque provider data and is not documented as a filesystem-safe
path component.

The provider directory supplies namespace separation between Codex and Claude.

The quarantine store must not change the original capture ledger.

The provenance usage sum must continue to include only the current owner.

A capture uniquely owned by another ticket is not unattributable.

An unreadable quarantine destination is an operational persistence failure.

The ticket requires a visible success signal when quarantine occurs.

The workflow requires ticket-owned source changes to use Lisa's isolated
`commit-ticket` command with exact repository-relative paths.

Research, Design, Structure, Plan, Progress, and Review artifacts belong only
under this attempt's private work directory until Lisa admits them.
