# Research: attribute captures by pane time

## Ticket boundary

Ticket `T-043-03-01` replaces the plugin's legacy ticket-keyed usage read with
capture attribution based on pane ownership at capture time.

The acceptance case is one physical pane used first by ticket A and then by
ticket B.

The result must be two append-only execution provenance rows.

Ticket A's row must contain the sum of A's captures.

Ticket B's row must contain the sum of B's captures.

Completing B must not rewrite A's row.

The ticket starts after three foundation tickets:

- `T-043-01-01` added the shared `CaptureRecord` contract.
- `T-043-01-02` added pane-time `owner_at` lookup.
- `T-043-02-01` changed native capture to append `CaptureRecord` rows.

The parent story is `S-043-03`.

Its later `T-043-03-02` ticket owns quarantine and operator-visible reporting
for captures that cannot be attributed.

Its later `T-043-03-03` ticket owns the full six-overwrite field reproduction.

This ticket is therefore the attribution spine, not the quarantine or field
reproduction layer.

## Shared capture contract

`crates/lisa-core/src/capture.rs` defines `CaptureRecord`.

Each row contains:

- `pane_id: u32`;
- `session_id: String`;
- `captured_at: u64`;
- `input_tokens: u64`;
- `output_tokens: u64`.

`captured_at` is UTC epoch seconds.

The record deliberately has no ticket ID.

The capture process records observable pane, session, time, and token facts.

It does not infer scheduler ownership.

`append_capture_record` serializes one compact JSON object followed by a
newline.

It creates parent directories when absent.

It opens the destination in append mode.

Two captures for the same pane therefore remain two physical rows.

The core test proves round-trip deserialization and byte-preserving append.

## Capture producer

`crates/lisa-cli/src/capture_usage.rs` implements native Stop-hook capture.

Both Claude and Codex native clients write to a provider-specific directory.

The paths are:

- `.lisa/claude/captures.jsonl`;
- `.lisa/codex/captures.jsonl`.

Claude transcript usage is summed over assistant messages.

Its input total includes fresh, cache-creation, and cache-read tokens.

Codex rollout usage uses the latest cumulative token-count event.

The producer reads `LISA_PANE_ID` and the provider session ID.

It uses `provenance::system_time_to_epoch(SystemTime::now())` for capture time.

Missing or invalid inputs cause no successful capture row to be written.

An all-zero observation is also not written.

The producer no longer creates `<ticket>.usage.json`.

## Provenance schema

`crates/lisa-core/src/provenance.rs` owns the durable provenance ledger.

The plugin stores it at `.lisa/provenance.jsonl`.

`ProvenanceRecord` is the terminal execution row.

Its ownership-relevant fields are:

- `ticket_id`;
- `attempt_lease`;
- `started_at`;
- `ended_at`;
- `pane_id`.

Its usage fields are:

- `tokens_in: Option<u64>`;
- `tokens_out: Option<u64>`;
- `cost_usd: Option<f64>`.

Token options represent measured totals or absence of measurement.

The capture schema contains no dollar-cost fact.

The ledger is mixed-shape.

`ProvenanceLedgerRecord` has execution and assignment-transition variants.

Assignment-transition rows describe failures before provider ownership.

They are not execution ownership intervals.

`append_record` appends terminal execution rows without rewriting prior rows.

The current schema version is 3.

The untagged reader also supports the legacy execution JSON shape.

## Ownership lookup

`crates/lisa-plugin/src/ownership.rs` defines crate-visible `owner_at`.

Its inputs are an iterator of `&ProvenanceRecord`, a pane ID, and an epoch
second.

It matches records whose pane equals the capture pane and whose inclusive
`started_at..=ended_at` interval contains the capture time.

Inclusive endpoints reflect the ledger's one-second timestamp resolution.

Repeated matching rows for the same ticket retain that unique answer.

Overlapping matches for different ticket IDs return `None`.

No match also returns `None`.

The result is independent of input record order for conflicting ownership.

The ownership module's tests already cover a recycled pane with disjoint A and
B intervals.

They also cover boundaries, gaps, other panes, duplicate same-ticket evidence,
and conflicting overlap.

## Plugin state and filesystem paths

`crates/lisa-plugin/src/lib.rs` defines `State`.

Relevant fields are:

- `threads`, keyed by active ticket ID;
- `ledger_path` for `.lisa/provenance.jsonl`;
- `codex_dir` for `.lisa/codex`;
- `claude_dir` for `.lisa/claude`.

`load()` initializes those paths below the captured host project root.

Native tests use `with_ledger` to redirect all three paths into a temporary
directory.

An empty `ledger_path` intentionally makes provenance emission a no-op in
unrelated tests.

## Terminal emission order

`State::emit_provenance` is called at terminal teardown sites while the active
thread is still present.

The thread supplies client, start time, concurrency, pane, and attempt lease.

The method validates that a Done attempt is still the current lease.

It computes the current record's start and end epoch seconds.

It constructs the route and a `ProvenanceRecord` with null usage.

The existing implementation then calls `read_usage`.

It fills the returned token and cost values into the current record.

Only after usage filling does it call `provenance::append_record`.

Consequently, prior ownership rows are already durable, but the ownership
interval currently ending is not yet in the ledger.

The in-memory current record is the only complete representation of that final
interval before append.

Write failures are logged and swallowed rather than failing scheduler teardown.

## Legacy usage reader

`State::read_usage` currently accepts an `AgentClient` and ticket ID.

It chooses `codex_dir` or `claude_dir` from the client.

It joins `<ticket>.usage.json` beneath that directory.

It reads one JSON object and looks for nested `usage`.

It delegates token and cost aliases to `provenance::extract_usage`.

Missing files, invalid JSON, null usage, and unknown fields return three
`None` values.

The reader encodes ticket identity into the filename.

That file shape predates the append-only capture producer.

It cannot distinguish multiple ownership windows in a recycled pane.

## Existing provenance tests

The large native test module in `crates/lisa-plugin/src/lib.rs` has helpers for
temporary ledgers and deserialization.

`read_ledger` parses execution-only fixtures.

`read_mixed_ledger` parses both provenance row variants.

Existing usage tests create legacy ticket-keyed files for Codex and Claude.

They assert tokens flow into one terminal record.

Another test asserts missing Claude usage leaves tokens and cost null.

Retry coverage asserts a second terminal record is appended and the first
remains intact.

Lease and fencing tests exercise emission rejection and terminal outcomes.

No current plugin test writes `captures.jsonl` and drives two different tickets
through one pane's successive time windows.

## Timestamp and summation constraints

Capture and provenance timestamps share UTC epoch seconds.

`owner_at` expects the same unit and numeric type as `CaptureRecord::captured_at`.

The required aggregation is per attributed ticket across matching capture
rows.

`CaptureRecord` counts are concrete `u64` values.

`ProvenanceRecord` counts are optional because no attributed capture is a valid
absence case.

Rust integer addition can overflow in debug builds unless aggregation uses a
defined overflow policy.

The existing producer sums transcript values with ordinary `u64` addition.

No repository contract currently defines cross-capture overflow behavior.

## Read and failure boundaries

Both capture and provenance stores are JSONL append logs.

The existing core append functions do not lock across processes.

The plugin's old reader treats a whole-file read or JSON error as missing usage.

The CLI pre-ownership status reader reports malformed provenance lines as
errors, but the plugin has no shared production ledger-load helper.

The next story ticket requires unmatched captures to remain available for
quarantine rather than being assigned to a fallback owner.

No shared or `last` ownership bucket exists in the current code.

The attribution result must preserve `None` when no capture maps to the current
ticket.

## Source ownership and worktree state

The ticket's production and regression seam is
`crates/lisa-plugin/src/lib.rs`.

The predecessor-owned `capture.rs`, `capture_usage.rs`, and `ownership.rs`
already expose the required contracts.

The ordinary worktree also contains Lisa-managed changes to active ticket,
provenance, and completion-journal files.

Those are not ticket-owned source changes for this implementation.

Ticket work must be committed with `lisa commit-ticket` and exact include
paths.

Workflow artifacts belong only in this attempt-private work directory until
Lisa admits and publishes them.
