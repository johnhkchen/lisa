# Design: quarantine unattributable usage

## Decision summary

Extend the plugin capture consumer at its existing `owner_at` branch.

When a valid capture on the current pane has no unique owner and is not later
than the current closed interval, append it to a provider-local, session-keyed
quarantine file.

Use this layout:

```text
.lisa/<client>/quarantine/<encoded-session-id>.jsonl
```

Store the original `CaptureRecord` together with its stable 1-based source
line in `captures.jsonl`.

Use that source line as the idempotence key when the append-only source ledger
is scanned again.

After a new quarantine row is persisted, log an `ActivityEvent::Warning` that
names the session, pane, timestamp, and destination.

If quarantine inspection or append fails, log `ActivityEvent::Error` with the
same capture identity and leave provenance attribution fail-closed.

Do not add quarantined tokens to any provenance record or fallback aggregate.

## Goals

- Preserve a valid capture that lacks confident pane-time ownership.
- Partition held captures by their observed provider session.
- Keep provider namespaces distinct.
- Make successful quarantine immediately visible in the activity feed.
- Make quarantine persistence failures visible as errors.
- Avoid repeated rows and warnings when the capture ledger is rescanned.
- Avoid quarantining captures whose ownership interval may not have closed.
- Preserve all existing owned-capture attribution and summation behavior.
- Treat ambiguous overlapping ownership the same as missing ownership.
- Keep the change inside the current consumer/storage boundary.

## Non-goals

- Do not infer a ticket from session identity.
- Do not repair malformed capture JSON.
- Do not create a new cost or usage dashboard.
- Do not change the `CaptureRecord` producer contract.
- Do not rewrite or remove rows from `captures.jsonl`.
- Do not reconcile a quarantine row back into provenance later.
- Do not implement the six-overwrite field regression assigned to 03-03.
- Do not change cache-token dimension handling.
- Do not add shared or `last` fallback files.

## Option 1: one global quarantine ledger

Append all unmatched captures to
`.lisa/<client>/quarantine.jsonl` and retain `session_id` in every row.

### Advantages

- One append path.
- Simple sequential inspection.
- No filename encoding requirement.

### Disadvantages

- The durable store is physically a shared bucket.
- It does not satisfy the acceptance language's session-ID-keyed file.
- Per-session recovery requires filtering the whole ledger.
- It visually resembles the removed `last` fallback failure mode.

### Disposition

Rejected because storage partitioning is an explicit ticket requirement.

## Option 2: raw capture rows in per-session files

Append the original `CaptureRecord` directly to
`quarantine/<session-id>.jsonl` whenever `owner_at` returns `None`.

### Advantages

- The file payload reuses the public capture schema exactly.
- Every file is naturally grouped by provider session.
- Implementation is small and can reuse `append_capture_record`.

### Disadvantages

- `read_usage` scans the source ledger on every teardown.
- Blind appends duplicate both quarantine rows and activity warnings.
- Deduplicating only by full `CaptureRecord` equality collapses two distinct
  identical source rows.
- There is no durable source-row identity in `CaptureRecord` itself.

### Disposition

Rejected in its raw form because repeat processing is normal, not exceptional.

## Option 3: per-session files with source-line identity

Enumerate every physical line in the append-only source ledger.

For each parsed, unattributable row, write a private quarantine envelope:

```text
QuarantinedCaptureRecord {
    source_line: u64,
    capture: CaptureRecord,
}
```

Before append, parse the selected session file and check whether that
`source_line` already exists.

### Advantages

- Directly satisfies session-keyed storage.
- Preserves the complete original capture.
- Repeat scans do not duplicate rows or warnings.
- Two byte-identical capture rows remain distinct because their line numbers
  differ.
- Malformed source rows do not destabilize later identities because physical
  line enumeration occurs before JSON parsing.
- The source ledger is contractually append-only, making line identity stable.
- The envelope remains local to the only consumer that needs it.

### Disadvantages

- The quarantine payload is not a bare `CaptureRecord`.
- The store relies on the source ledger's existing no-rewrite contract.
- Every new quarantine checks its session file before append.

### Disposition

Selected. It is the smallest durable design that meets both partitioning and
idempotence without changing the capture producer schema.

## Option 4: remember quarantined rows only in plugin memory

Track `(client, source_line)` in a `HashSet` and append raw capture rows once per
plugin process.

### Advantages

- Fast repeat checks.
- Bare `CaptureRecord` output remains possible.

### Disadvantages

- Plugin reload loses the set.
- A reload would append duplicate rows and repeat warnings.
- Durable behavior would depend on process lifetime.

### Disposition

Rejected because the quarantine itself is durable machine state.

## Option 5: attribute or quarantine in `capture-usage`

Move ownership lookup into the native CLI writer and choose the final store at
capture time.

### Advantages

- Each source observation is handled once.
- No plugin rescan idempotence is required.

### Disadvantages

- The CLI capture process does not own scheduler provenance state.
- It would duplicate or weaken the established `owner_at` boundary.
- It expands scope across crates and reverses the preceding ticket's contract.
- Capture collection would become coupled to ticket attribution availability.

### Disposition

Rejected because the plugin consumer is the established ownership authority.

## Eligibility for quarantine

The consumer will continue parsing the full append-only ledger.

Each source line receives a 1-based line number before parse.

Malformed rows remain skipped because they lack reliable session identity.

Captures on a pane other than `current.pane_id` remain untouched by this call.

This matters because another pane may have a live interval not represented in
the prior provenance rows supplied to `read_usage`.

A same-pane capture with `captured_at > current.ended_at` also remains untouched.

That capture is beyond the latest closed interval available to this call.

It may belong to a subsequent execution whose current record is not yet
available, as the recycled-pane regression fixture demonstrates.

For a same-pane capture at or before `current.ended_at`, resolve `owner_at` over
prior execution rows plus `current`.

The result has three branches:

1. Current ticket: include the tokens in the current totals.
2. Another ticket: skip; the capture is attributable, but not to this record.
3. `None`: quarantine; the evidence is missing or conflicting.

No branch assigns an unmatched capture to a default ticket or aggregate.

## Session filename encoding

`session_id` is opaque and cannot be trusted as a path component.

Encode its UTF-8 bytes with a reversible percent-style scheme.

Preserve ASCII letters, digits, hyphen, and underscore.

Encode every other byte as `%HH`, including slash, backslash, percent, and dot.

Represent an empty session ID with a dedicated encoded sentinel.

Encoding dot prevents `.` and `..` path traversal components.

Encoding percent keeps the mapping injective.

The activity warning will show the original session ID, so operators do not
need to decode the filename to understand the event.

## Durable append behavior

The quarantine helper derives the provider directory from the same `client`
used to select `captures.jsonl`.

Codex and Claude sessions therefore cannot collide across providers.

The helper reads an existing session file when present.

If a parsed envelope has the same `source_line`, the operation is already
complete and produces no new event.

Malformed existing quarantine rows do not prove completion and are ignored for
the idempotence check.

An unreadable existing file is an error rather than permission to append.

Appending creates the quarantine directory and uses append-only file mode.

The existing file is never truncated or rewritten.

Serialization failure is translated to an I/O invalid-data error consistently
with the capture append helper.

## Activity behavior

A successful first append logs `ActivityEvent::Warning`.

Warning is chosen because usage was preserved but cannot enter authoritative
ticket provenance without ownership evidence.

The existing UI conversion maps it to a visible warning entry.

The message includes:

- provider client;
- original session ID;
- pane ID;
- capture timestamp;
- quarantine path.

An idempotent repeat produces no warning because no new quarantine occurred.

A read, directory, serialization, or append failure logs `ActivityEvent::Error`.

Quarantine failure does not fabricate ownership and does not abort summing
other uniquely owned captures.

## API impact

Change `read_usage` from `&self` to `&mut self` so it can emit activity.

Its arguments and token/cost return tuple remain unchanged.

`emit_provenance` already has mutable state, so its call shape remains valid.

The quarantine envelope and filename helper remain private to the plugin.

No public core or CLI API changes are required.

## Test strategy

Add a focused plugin regression beside the provenance usage tests.

Create a current execution interval on a pane.

Append one valid capture for that pane and session at a timestamp outside every
known ownership interval but before the current close.

Run `read_usage` or terminal provenance emission.

Assert null tokens so the capture did not blend into current usage.

Assert the expected encoded-session quarantine file exists.

Parse its envelope and compare the original capture and source line.

Assert no shared `quarantine.jsonl`, `last`, or alternate session file exists.

Assert the activity log contains the quarantine warning.

Convert that event through `activity_event_to_ui_entry` and assert it is a
visible warning.

Invoke the consumer again and assert one durable row and one warning remain.

Retain the existing recycled-pane and provider-flow tests as regression gates.
