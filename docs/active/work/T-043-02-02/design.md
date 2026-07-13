# Design — T-043-02-02 loud no-write signal

## Objective

Turn every identified Stop that cannot produce token totals into a durable, append-only, operator-visible no-capture observation, and let genuine command failures reach the operator instead of being erased by Rust dispatch or shell redirection.

## Design principles

- Record only facts known at capture time.
- Never claim a measured zero when no usage was observed.
- Never guess pane or provider session identity.
- Preserve every failure observation rather than overwriting a shared marker.
- Keep successful capture rows homogeneous for downstream attribution.
- Treat inability to write the marker as a real error.
- Keep `.stopped` signaling independent from usage capture.
- Make generated and checked-in live hooks express the same behavior.

## Option 1 — Put failure rows in `captures.jsonl`

Shape successful and failed observations as a tagged enum and append both to the existing ledger.

Advantages:

- One chronological file per provider.
- A future consumer sees all outcomes in one scan.
- No second path to document.

Costs:

- `captures.jsonl` is already a homogeneous stream of `CaptureRecord`.
- Existing tests deserialize each row directly as `CaptureRecord`.
- T-043-03-01 is scoped to consume `CaptureRecord` rows for attribution.
- Adding a tag to successful rows would change the just-landed core contract.
- Leaving successful rows untagged while adding tagged failures creates an awkward untagged enum.
- A plugin reader written against `CaptureRecord` would fail on the first marker.

Disposition: rejected. It expands the successful contract and couples this ticket to downstream consumer redesign.

## Option 2 — Write one mutable marker file per provider

Write `.lisa/<client>/no-capture.json` with the most recent unsuccessful Stop.

Advantages:

- Very simple for an operator to find.
- No mixed-row schema.
- Easy to implement with one atomic write.

Costs:

- Repeated failures overwrite evidence.
- The story exists because silent loss and overwrite make state dishonest.
- A single current marker cannot support chronological diagnosis.
- Concurrent panes would replace one another's failures.

Disposition: rejected. It recreates the destructive state pattern removed by the preceding ticket.

## Option 3 — Append a separate no-capture JSONL ledger

Append identified failures to:

```text
.lisa/claude/no-captures.jsonl
.lisa/codex/no-captures.jsonl
```

Each compact row contains:

```json
{"pane_id":42,"session_id":"session-a","captured_at":1752345600,"reason":"empty-transcript"}
```

Advantages:

- Successful `CaptureRecord` parsing remains unchanged.
- Failure observations are never overwritten.
- Provider separation matches successful capture storage.
- Operators can inspect one obvious durable file.
- Stable reason strings support tests and future activity surfacing.
- Pane/session/time facts preserve later diagnostic context.

Costs:

- Operators and future consumers have two ledgers.
- Chronology across successes and failures requires timestamp comparison.
- A small JSONL append helper is needed outside the successful typed helper.

Disposition: chosen. It meets the ticket without destabilizing the core successful record boundary.

## Marker schema

Define a private serializable `NoCaptureMarker` in `capture_usage.rs`.

Fields:

- `pane_id: u32`
- `session_id: String`
- `captured_at: u64`
- `reason: &'static str`

The type remains private because no runtime consumer is introduced by this ticket.

`captured_at` uses `system_time_to_epoch(SystemTime::now())`, matching successful observations and pane-time ownership units.

The marker omits:

- ticket ID, because capture time cannot establish ownership;
- token fields, because no usage was observed;
- transcript path, because it may disclose machine-specific or sensitive paths and is unnecessary for acceptance;
- provider, because provider is represented by the enclosing directory;
- free-form OS error text, because the stable reason is the durable contract and stderr can carry detailed errors.

## Reason taxonomy

Use three stable kebab-case reasons:

- `missing-transcript-path`: the valid Stop payload has no usable transcript path.
- `unreadable-transcript`: the path was supplied but the transcript could not be read.
- `empty-transcript`: the transcript parser observed no nonzero usage totals.

An empty string path is treated as missing rather than attempting to read the project directory.

`empty-transcript` intentionally covers:

- a zero-byte file;
- only blank lines;
- only malformed lines;
- only non-usage events;
- assistant/token-count records whose recognized totals are all zero.

This retains the established “do not fabricate measured zero” boundary.

## Identity validation

Reorder `run_capture_usage` so it establishes provider, session, and pane before transcript outcomes.

The required marker identity cannot be honestly constructed when:

- stdin cannot be read;
- the Stop payload is malformed;
- `session_id` is absent or empty;
- `LISA_PANE_ID` is absent, empty, nonnumeric, or out of range.

These cases return `std::io::Error` with `InvalidData` or `InvalidInput` rather than silently returning `Ok(())`.

They do not write partially identified marker rows.

## Transcript outcome flow

After identity is validated:

1. Select the provider directory.
2. Validate a nonempty transcript path.
3. On absence, append `missing-transcript-path`.
4. Attempt to read the transcript.
5. On read failure, append `unreadable-transcript`.
6. Parse with the existing provider-specific parser.
7. On default totals, append `empty-transcript`.
8. Otherwise append the existing successful `CaptureRecord`.

Every identified path writes exactly one row to exactly one ledger.

## Marker append behavior

Add a private `append_no_capture_marker` helper.

The helper:

- serializes one marker as compact JSON;
- maps serialization failure to `InvalidData`;
- appends a newline;
- creates the provider directory when absent;
- opens `no-captures.jsonl` with create plus append;
- writes the full row bytes;
- returns all filesystem errors.

The implementation parallels `append_capture_record` without generalizing the core API prematurely.

## Immediate operator visibility

After a marker is successfully persisted, emit one concise stderr message:

```text
lisa capture-usage: no capture for pane 42 session session-a: empty-transcript
```

The durable JSONL row is the marker; the stderr line makes the event immediately visible in an attached native session.

The message is emitted only after persistence succeeds, so it never claims a marker exists when the write failed.

## CLI error boundary

Change `Commands::CaptureUsage` dispatch to follow normal CLI error handling:

- resolve `cwd`;
- call `run_capture_usage`;
- print `Error: <message>` on failure;
- exit with status 1.

This makes malformed identity and marker-write failures visible.

Successful no-capture marker creation returns exit status 0.

## Hook behavior

Change the capture pipeline from:

```sh
printf '%s' "$in" | "${LISA_BIN:-lisa}" capture-usage 2>/dev/null || true
```

to:

```sh
printf '%s' "$in" | "${LISA_BIN:-lisa}" capture-usage
```

Effects:

- stderr reaches the native client's hook surface;
- a real capture failure produces a failing hook status;
- an expected empty/unreadable transcript still succeeds after writing its marker;
- the `.stopped` signal is already written before capture runs.

Apply the exact change to both `ON_STOP_HOOK` and `.lisa/hooks/on-stop.sh`.

## Template upgrade compatibility

The current pre-ticket Stop hook becomes a known legacy template.

Add its exact bytes to `LEGACY_ON_STOP_HOOKS` alongside the older pre-capture hook.

This permits `lisa init` to upgrade installations that have not customized the currently shipped silent hook.

Unknown project-owned hook content remains protected.

## Documentation

Update `hooks-guide.md` near the lifecycle explanation to state:

- Stop capture successes append to `captures.jsonl`;
- identified no-capture outcomes append to `no-captures.jsonl`;
- marker rows contain pane, session, capture time, and reason;
- capture command failures are intentionally visible rather than stderr-suppressed.

Update stale Rust comments that still describe overwrite and swallowed behavior.

## Test design

Extend the compiled CLI integration test with two identified failure invocations:

- an empty transcript file;
- an unreadable/nonexistent transcript path.

Assert:

- both commands exit successfully because their markers append;
- stderr visibly names the no-capture reason;
- no successful `captures.jsonl` is created for the isolated test root;
- `no-captures.jsonl` contains two rows;
- both rows carry the supplied pane and session;
- reasons are `empty-transcript` and `unreadable-transcript` in invocation order;
- capture timestamps fall within the invocation bounds.

Extend the template unit test to assert:

- no `2>/dev/null` occurs in `ON_STOP_HOOK`;
- no `|| true` masks capture status.

Use existing init tests to validate the newly expanded legacy generation list.

## Rejected expansions

- Do not add plugin activity events; S-043-03 owns surfacing during attribution.
- Do not add ticket attribution to markers.
- Do not add quarantine files keyed by session.
- Do not change successful `CaptureRecord`.
- Do not change transcript token semantics.
- Do not introduce cache token dimensions.
- Do not execute a live metered provider run.

## Decision summary

Append a separate provider-specific `no-captures.jsonl` stream of identified failure facts, use stable reasons for missing, unreadable, and empty transcripts, return real errors for missing identity or persistence failure, surface markers and errors through stderr, remove both shell suppression operators, preserve init upgrades by registering the previous hook generation, and prove the behavior at the compiled CLI and template boundaries.
