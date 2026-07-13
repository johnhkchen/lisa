# Review — T-043-01-01 append-only capture record schema

## Disposition

Pass. The ticket's source change is complete, isolated, committed, and directly covered by the required unit test. No critical issue or completion blocker remains.

## What changed

Commit reviewed:

- `2c2e314224a38444479322bcd7b92ea7f389bdf9`
- `feat(core): add append-only capture records`

Diff size:

- 2 source files changed.
- 110 insertions.
- No deletions.
- No dependency or lockfile change.

### New `crates/lisa-core/src/capture.rs`

The new module defines `CaptureRecord`, the shared pre-attribution success record for token captures.

The type carries exactly:

- Numeric terminal pane ID.
- Opaque session ID.
- UTC epoch-second capture timestamp.
- Aggregate input token count.
- Output token count.

The type intentionally does not carry:

- Ticket ID.
- A guessed or fallback artifact key.
- Provider or model route.
- Dollar cost.
- Scheduler ownership state.
- Cache-class token dimensions.
- No-capture state.

This matches the ticket's “facts the capturing process honestly knows” boundary. In particular, omitting ticket identity prevents this shared schema from preserving the inherited-environment attribution error it is meant to replace.

The module also defines `append_capture_record`.

The helper:

- Accepts an explicit destination path.
- Serializes one compact JSON object.
- Frames it with one trailing newline.
- Creates missing parent directories.
- Opens the destination in create-plus-append mode.
- Writes the framed row without reading existing contents.
- Returns ordinary I/O errors to the caller.

It does not select a storage directory or filename. That application decision remains with the later CLI writer ticket.

### Modified `crates/lisa-core/src/lib.rs`

The crate root now publicly declares the capture module.

Downstream tickets can use:

- `lisa_core::capture::CaptureRecord`
- `lisa_core::capture::append_capture_record`

The module-qualified interface is consistent with the crate's existing public organization.

## Acceptance-criterion assessment

Criterion:

> A lisa-core unit test round-trips a CaptureRecord (pane_id, session_id, captured_at, input/output tokens) and shows that appending two captures for the same pane yields two JSONL rows with the first row byte-intact — never an overwrite.

Assessment: satisfied.

The new unit test:

1. Constructs a record with every named field populated.
2. Serializes it to compact JSON.
3. Deserializes it back to `CaptureRecord`.
4. Checks equality with the original.
5. Appends it to a temporary nested JSONL path.
6. Saves the exact bytes after the first append.
7. Constructs a second record with the same pane ID.
8. Gives the second record different session, timestamp, and token values.
9. Appends the second record through the public API.
10. Checks that final file bytes begin with the complete saved first-write bytes.
11. Checks the first raw row equals the original compact JSON bytes.
12. Checks exactly two non-empty rows exist.
13. Deserializes both rows and checks their order and complete values.

The byte-prefix check is important: it proves more than equivalent parsed content. The first write is retained as-is after the second call, excluding overwrite and read/re-serialize behavior.

## Test coverage

### Focused coverage

`cargo test -p lisa-core capture`

- 1 passed.
- 0 failed.
- Exercises the complete ticket acceptance path.

`cargo test -p lisa-core`

- 197 unit tests passed.
- 2 integration tests passed.
- 0 failures.

### Broad coverage

`cargo fmt --all -- --check`

- Passed.

`cargo test --workspace`

- Passed with exit code 0.
- CLI library and binary suites passed.
- CLI integration suites passed.
- Core unit and integration suites passed.
- Plugin's 375-test suite passed.
- Doc tests passed.
- The environment-gated real-Zellij test remained ignored as designed.

The broad run confirms the new public module does not break downstream compilation or behavior.

## Code-quality assessment

### Correctness

- `OpenOptions::append(true)` encodes the required non-truncating behavior.
- Serialization occurs before opening the file, so serialization failure cannot disturb the target.
- Parent creation matches existing core ledger behavior.
- Compact serde output cannot introduce an embedded JSONL newline from string data; JSON escaping keeps the object on one physical row.
- The test uses raw bytes for preservation evidence.
- Concrete `u64` counts correctly model a successful capture rather than an absent observation.

### API scope

- The interface is minimal and sufficient for both named downstream consumers.
- There is no premature writer or reader integration.
- There is no generic JSONL refactor unrelated to acceptance.
- Existing provenance schemas and versioning remain untouched.
- Existing CLI overwrite behavior remains visibly deferred rather than partially changed.

### Documentation

- Module docs explain pre-attribution semantics.
- Public fields state their meanings.
- The append function documents create and preservation behavior.
- The absence of ticket identity is explained at the module boundary.

## Gaps and limitations

### Expected integration gaps

This contract is not wired into the running system yet, by explicit story design:

- The CLI still emits old `<key>.usage.json` artifacts until T-043-02-01.
- The plugin still reads old ticket-keyed usage artifacts until T-043-03-01.
- Empty or unreadable captures remain the responsibility of T-043-02-02.
- Unattributable capture quarantine remains the responsibility of T-043-03-02.

These are planned downstream tickets, not missing work in this schema ticket.

### Concurrency scope

The test proves ordered sequential appends and byte preservation, exactly as required. The helper uses the same append-handle pattern as the existing provenance ledger. It does not add explicit inter-process locking or claim transactional grouping across multiple records. Each call emits only one small framed record; if a future workload requires stronger cross-process concurrency guarantees, that concern should be specified and tested at the writer/storage boundary.

### Validation scope

The core record does not reject empty session IDs or validate that timestamps are current. The type is a serde contract for externally observed facts; deciding whether an input observation is valid belongs to the later capture writer. Adding validation here without writer requirements would be premature.

### Schema evolution

The record has no schema-version field because this ticket requests only the five capture facts and no compatibility transition exists yet. Future incompatible evolution will need an explicit compatibility decision. This is not an immediate concern for the first writer and reader tickets, which share the same core version.

## Workspace hygiene

- Both ticket-owned source paths are clean after the isolated commit.
- `git show --check` reports no whitespace errors.
- No ticket-owned source remains staged, modified, or untracked.
- Lisa-managed ticket and work-artifact changes remain outside the source commit.
- Concurrent sibling-ticket changes in `lisa-plugin` were observed but not touched or included.
- No ordinary-index staging or ordinary commit command was used.

## Open concerns requiring human attention

None critical.

Reviewers should understand the intentional staged rollout: landing this ticket alone does not stop the legacy overwrite in live capture. It establishes the shared contract that T-043-02-01 must adopt, after which T-043-03-01 can consume and attribute the rows. That ordering is the parent story's declared DAG, not an implementation omission.

## Final assessment

The implementation is small, cohesive, and aligned with the researched codebase precedent. It makes append semantics reusable, prevents ticket guesses from entering the shared raw-capture schema, and proves the exact byte-preservation regression requested by the ticket. The work is ready for Lisa's completion gate.
