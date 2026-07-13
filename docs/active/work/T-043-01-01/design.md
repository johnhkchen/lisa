# Design — T-043-01-01 append-only capture record schema

## Goal

Define the shared success-record contract for one token capture and the operation that appends it to JSONL without selecting or overwriting a ticket-keyed artifact.

The design must be usable by the later CLI writer and plugin consumer while changing neither one in this ticket.

## Decision drivers

- The capture process must record only facts available at capture time.
- Pane and capture time will become the attribution inputs.
- Session ID will distinguish and quarantine capture streams later.
- The same pane may produce many records over its lifetime.
- Existing bytes must survive every later append.
- The representation must work in both native CLI and WASM plugin consumers.
- The implementation should follow established `lisa-core` conventions.
- No new dependency is justified for this five-field record.
- Cache-class token detail, provider, cost, and ticket attribution are out of scope.

## Option 1 — Add a dedicated `capture` module with record and append API

Shape:

```rust
pub struct CaptureRecord {
    pub pane_id: u32,
    pub session_id: String,
    pub captured_at: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

pub fn append_capture_record(path: &Path, record: &CaptureRecord) -> io::Result<()>;
```

The module serializes one compact JSON object, adds one newline, creates the parent directory when necessary, and opens the file with create-plus-append.

Advantages:

- Gives the new concept a stable, unsurprising `lisa_core::capture` namespace.
- Keeps capture facts separate from attributed terminal provenance.
- Makes append semantics part of the shared contract rather than test scaffolding.
- Mirrors the proven append API in `provenance.rs`.
- Lets the later CLI ticket call one public helper.
- Lets the later plugin deserialize the same public record.
- Requires only one new implementation file and one module declaration.

Costs:

- Repeats a small amount of JSONL append code currently private to provenance.
- Introduces another focused module in `lisa-core`.

## Option 2 — Put `CaptureRecord` in `provenance.rs`

The capture type and append function could be added beside `ProvenanceRecord` and reuse the private serialization helper directly.

Advantages:

- Minimal append implementation duplication.
- Both record families ultimately contribute to usage provenance.

Costs:

- Capture rows are pre-attribution observations, not terminal execution provenance.
- The provenance ledger has its own versioned, mixed-row contract and destination.
- Co-location suggests capture rows belong in `.lisa/provenance.jsonl`, which is not the later story's stated flow.
- It widens an already substantial module with a separate lifecycle.
- The parent story describes the capture record as a shared attribution contract, not a new provenance ledger variant.

Disposition: rejected because semantic separation is more important than avoiding a small private helper duplication.

## Option 3 — Define only the struct; test append behavior locally

This option would expose `CaptureRecord` but make the acceptance test open a file itself with append mode.

Advantages:

- Smallest production-code addition.
- Leaves destination and write mechanics entirely to the later CLI ticket.

Costs:

- The test would prove its own local setup, not a reusable product behavior.
- Every writer could accidentally choose overwrite semantics despite a passing core test.
- The story explicitly calls the contract append-only.
- The later CLI ticket would need to invent a second append implementation.

Disposition: rejected because append behavior is part of the shared contract.

## Option 4 — Generalize a public JSONL utility and refactor provenance

The existing private provenance serializer could move into a shared utility used by both record families.

Advantages:

- One implementation of parent creation, compact serialization, newline framing, and append opening.
- Future JSONL ledgers could reuse it.

Costs:

- Requires editing and retesting the established provenance module.
- Expands ticket ownership beyond the new capture contract.
- Creates a generic public or crate-private abstraction based on only two call sites.
- Raises collision risk with unrelated work on provenance.
- A utility refactor does not improve this ticket's acceptance evidence.

Disposition: rejected for this ticket; duplication is small and locally understandable.

## Field decisions

### `pane_id: u32`

Use the same pane identifier type used by scheduler threads and provenance records. A numeric field avoids encoding conventions such as `pane-7` into the data contract.

### `session_id: String`

Provider session identifiers are opaque external values. A required owned string round-trips without assumptions about UUID format or provider syntax. Empty-value validation belongs at the writer boundary, not serde.

### `captured_at: u64`

Store UTC epoch seconds, matching core provenance timestamps and the ownership lookup story. This avoids a datetime dependency and gives deterministic ordering/comparison input. Converting `SystemTime` belongs to the caller or existing core helper.

### `input_tokens: u64` and `output_tokens: u64`

Use concrete counts for a successful capture. The current transcript parsers already produce these values. Missing or unreadable observations are not successful `CaptureRecord`s; the later no-capture ticket owns their marker shape. `Option<u64>` would blur “captured zero” with “not captured” and is unnecessary here.

### Deliberately absent fields

- No `ticket_id`: the capture process cannot honestly infer ownership.
- No guessed key: pane/session/time are sufficient raw attribution evidence.
- No provider: not required by the ticket or first attribution flow.
- No cost: it cannot be derived honestly at this boundary.
- No cache split: explicitly deferred by the epic/story.
- No schema version: the requested record carries only capture facts; versioning can be added when an actual compatibility change requires it.

## Serialization decisions

- Derive `Serialize` and `Deserialize` directly on the record.
- Derive `Debug`, `Clone`, `PartialEq`, and `Eq` for tests and consumers.
- Keep serde's default snake_case field names.
- Serialize compactly with `serde_json::to_string`.
- Terminate each record with exactly one newline.
- Treat serialization failure as invalid input data in the I/O API.

## Append decisions

- Accept an explicit path; the core contract does not decide where the application stores captures.
- Create a non-empty parent directory if absent.
- Use `OpenOptions::create(true).append(true)`.
- Never open with truncate and never read/rewrite existing rows.
- Serialize before opening so a serialization error cannot touch the file.
- Write the line bytes through the append handle.
- Return `std::io::Result<()>` consistently with the provenance append API.

## Acceptance-test design

1. Construct a first record with all required fields.
2. Serialize it and deserialize it to prove field-level round-trip equality.
3. Append it to a temporary nested JSONL path.
4. Snapshot the complete first-write bytes.
5. Construct a second record using the same `pane_id` but different session/time/token facts.
6. Append the second record through the same public function.
7. Assert the final bytes begin with the exact first-write snapshot.
8. Assert the original snapshot itself is byte-for-byte the final prefix.
9. Split on newlines and assert there are exactly two non-empty rows.
10. Deserialize both rows and compare them with the two original records.

This test distinguishes append from overwrite and from a read/re-serialize rewrite that might preserve semantic JSON but alter the original bytes.

## Chosen design

Adopt Option 1: a dedicated public `capture` module containing the five-field `CaptureRecord` and `append_capture_record`.

This is the smallest design that makes both the schema and append-only behavior reusable. It remains inside the contract-only boundary, matches existing core practices, and gives downstream tickets a stable API without prematurely changing their behavior or choosing their storage path.
