# Design — T-043-02-01 append capture, not overwrite

## Goal

Replace the ticket-guessed overwrite artifact emitted by `lisa capture-usage` with an append-only stream of observed capture facts, while preserving the current transcript parsers and the current best-effort boundary for unsuccessful captures.

## Decision drivers

- A native client process can honestly know its pane ID.
- A Stop payload can honestly provide its provider session ID.
- The writer can honestly record its current capture time.
- The writer can calculate aggregate input and output tokens from the transcript.
- The writer cannot infer current ticket ownership from inherited `LISA_TICKET_ID`.
- Multiple captures from one recycled pane must all survive.
- The later plugin consumer needs pane and time for scheduler ownership lookup.
- The later quarantine path needs session identity.
- The shared core crate already owns the record and append semantics.
- The acceptance test must exercise the compiled CLI boundary twice.
- Empty or unreadable capture visibility remains assigned to T-043-02-02.

## Option 1 — One append-only capture ledger per provider

Write successful rows to:

```text
.lisa/claude/captures.jsonl
.lisa/codex/captures.jsonl
```

Each row is a `lisa_core::capture::CaptureRecord` and therefore contains its pane and session identifiers explicitly.

Advantages:

- Directly replaces mutable per-ticket artifacts with an event stream.
- Uses the core append helper exactly as designed by the prerequisite ticket.
- Keeps provider separation already established by the current directory layout.
- Gives the later consumer one predictable input per provider.
- Avoids any filename derived from stale ticket state.
- Avoids creating one ever-growing collection of tiny files.
- Makes cross-pane capture order available from file order.
- Provides a stable location for the dependent no-capture marker ticket.

Costs:

- The later consumer must scan or incrementally track a shared provider ledger.
- Concurrent hook processes share one append destination.
- The plugin reader must change before usage reaches terminal provenance again.

Disposition: chosen. It is the smallest application of the existing shared append contract and most directly expresses “raw capture observations awaiting attribution.”

## Option 2 — One append-only ledger per pane

Write rows to a path such as:

```text
.lisa/claude/pane-7.captures.jsonl
```

Advantages:

- A consumer looking up one pane reads less unrelated data.
- Independent panes usually write independent files.
- The filename gives an additional visible partition.

Costs:

- Pane identity is duplicated between filename and record.
- Filename construction becomes another key-resolution mechanism.
- A malformed filename/row mismatch creates two sources of truth.
- The acceptance language asks to remove the key path, not introduce a renamed variant.
- The next ticket's no-capture path would need a pane even when required inputs are incomplete.

Disposition: rejected. Pane is data inside the record and should remain the single factual representation.

## Option 3 — One append-only ledger per provider session

Write rows below a session-derived filename.

Advantages:

- Quarantine by session becomes physically partitioned.
- Captures from independent sessions do not share a file.

Costs:

- External opaque identifiers require safe filename encoding.
- Session-derived paths create portability and traversal concerns.
- Later ticket attribution still needs to discover all session files.
- Session-keyed quarantine is a consumer decision for unattributable rows, not the writer's success-store contract.

Disposition: rejected. It turns an opaque value into filesystem structure without a ticket requirement.

## Option 4 — Append old-shaped values to a ticketless JSONL file

Keep `usage_artifact`, remove only its key, and append the nested `usage` object.

Advantages:

- Small local diff.
- Preserves the existing extraction helper's nested shape.

Costs:

- Omits pane, session, and time required for later attribution.
- Duplicates a schema already settled by T-043-01-01.
- Would not satisfy the required `CaptureRecord` acceptance language.

Disposition: rejected because it loses the facts the new flow depends on.

## Payload decision

Extend `StopPayload` with:

```rust
#[serde(default)]
session_id: Option<String>
```

Retain `transcript_path` as optional. A successful capture proceeds only for a non-empty session ID.

Rationale:

- The Stop payload is the authoritative provider-session boundary.
- The identifier remains opaque and is copied without interpretation.
- Optional deserialization preserves defensive compatibility with drift or older payloads.
- Empty values do not describe an honest session fact.
- No fallback to ticket, pane, transcript filename, or shared string is introduced.

## Pane decision

Read `LISA_PANE_ID`, require it to be non-empty, and parse it as `u32`.

Rationale:

- Pane ID is process context intentionally stable across pane recycling.
- The shared schema and scheduler use `u32`.
- Parsing at the boundary prevents invalid textual pane values from entering persisted records.
- There is no honest fallback pane.
- Missing/invalid pane continues the existing best-effort no-write behavior until the next ticket defines marker behavior.

`LISA_TICKET_ID` is never read. It may be present in the process environment, but it has no influence on capture content or destination.

## Timestamp decision

Set `captured_at` immediately before appending, using:

```rust
lisa_core::provenance::system_time_to_epoch(SystemTime::now())
```

Rationale:

- It shares the exact UTC epoch-second convention used by scheduler ownership lookup.
- It reuses a tested conversion instead of duplicating epoch handling.
- Capture time represents when the writer completed transcript observation.
- Second precision is the contract already selected by T-043-01-01.
- JSONL order distinguishes rapid same-second captures.

## Record decision

Construct exactly:

```rust
CaptureRecord {
    pane_id,
    session_id,
    captured_at,
    input_tokens: usage.input_tokens,
    output_tokens: usage.output_tokens,
}
```

Do not add provider, ticket, key, transcript path, cache splits, cost, or status fields. Provider remains an enclosing storage partition rather than record attribution.

## Append decision

Call `append_capture_record(&client_dir.join("captures.jsonl"), &record)`.

The CLI module does not open or serialize the file itself. This ensures the actual product path inherits the prerequisite's tested create, compact JSONL framing, and append-only behavior.

The obsolete `usage_artifact` and `resolve_key` helpers are deleted. Keeping unused compatibility helpers would leave the false attribution path available for future accidental reuse and would contradict the acceptance requirement that the guess path is gone.

## Parser decision

Leave `sum_claude_transcript_usage` and `codex_transcript_usage` behavior unchanged.

The ticket changes persistence and attribution facts, not provider token calculation. Existing focused parser tests remain valuable regression coverage. The old artifact-shape test is deleted because the new writer no longer emits that shape; `CaptureRecord` parsing in the CLI integration test replaces it as the cross-crate contract evidence.

## Unsuccessful capture boundary

Retain `Ok(())` without a success row for:

- unreadable stdin;
- malformed payload JSON;
- absent transcript path;
- absent or empty session ID;
- absent, empty, or nonnumeric pane ID;
- unreadable transcript;
- transcript with no observed nonzero totals.

This is intentionally temporary. T-043-02-02 explicitly owns operator-visible no-capture markers and hook stderr behavior. Inventing that marker here would overlap its source unit and acceptance test.

Filesystem append errors still propagate from `run_capture_usage`, although `main` currently swallows them for hook safety. That command-level behavior also remains outside this ticket.

## CLI integration-test design

Add a new integration test under `crates/lisa-cli/tests/` using only the standard library and `tempfile`.

Fixture setup:

1. Create a temporary project root.
2. Write two distinct Claude JSONL transcripts.
3. Give the first transcript one assistant usage total.
4. Give the second transcript a distinguishable total.
5. Capture wall-clock epoch bounds around the two process invocations.

Invocation setup:

1. Launch `env!("CARGO_BIN_EXE_lisa") capture-usage --cwd <root>`.
2. Set the same `LISA_PANE_ID` for both calls.
3. Set the same deliberately stale `LISA_TICKET_ID` for both calls.
4. Remove `LISA_AGENT_CLIENT` so Claude parsing is deterministic.
5. Pipe one payload containing transcript path plus unique session ID to each call.
6. Assert both child processes exit successfully.

Assertions:

1. Read `.lisa/claude/captures.jsonl`.
2. Assert exactly two non-empty rows.
3. Deserialize both as `CaptureRecord`.
4. Assert row order matches invocation order.
5. Assert both pane IDs equal the supplied pane.
6. Assert session IDs match their payloads.
7. Assert token totals match their distinct transcripts.
8. Assert timestamps fall within the test's before/after epoch bounds.
9. Assert `.lisa/claude/<stale-ticket>.usage.json` does not exist.

This test fails against the old writer because no capture ledger exists and the stale ticket file is created and overwritten.

## Documentation decision

Update comments in `capture_usage.rs` and `main.rs` that specifically claim overwrite or ticket-keyed output. The new wording describes provider capture JSONL and pre-attribution facts. Do not edit the embedded hook or operator hook guide; those belong to the next dependent ticket.

## Chosen design

Use a provider-specific `captures.jsonl`, extend the Stop payload with session identity, parse stable pane identity from `LISA_PANE_ID`, timestamp through the shared epoch helper, construct the shared `CaptureRecord`, and append through the shared core API. Delete both the old artifact builder and `resolve_key`. Prove the full behavior through two compiled CLI invocations with a stale ticket environment present.
