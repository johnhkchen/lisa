# Research — T-043-01-01 append-only capture record schema

## Ticket boundary

- The ticket begins in the Research phase.
- Its acceptance criterion is confined to `lisa-core`.
- The required domain object is named `CaptureRecord`.
- The record represents one observed capture, not a ticket run.
- Required facts are pane ID, session ID, capture time, input tokens, and output tokens.
- The acceptance test must serialize and deserialize the record.
- It must also append two records for the same pane to JSONL.
- The first row must remain byte-for-byte unchanged after the second append.
- The ticket does not ask the CLI writer to use the new record yet.
- The ticket does not ask the plugin consumer to read the new record yet.
- The parent story explicitly reserves those integrations for later stories.

## Workspace and crate layout

- The workspace contains `lisa-core`, `lisa-cli`, and `lisa-plugin` crates.
- `crates/lisa-core/src/lib.rs` declares the core crate's public modules.
- Core domain types currently live in focused modules rather than one facade export.
- Consumers import public items through paths such as `lisa_core::provenance::...`.
- `lisa-core` already depends on `serde` with derive support.
- `lisa-core` already depends on `serde_json`.
- Its dev dependencies already include `tempfile`.
- No new dependency is needed for a serializable record or filesystem test.

## Existing capture path

- `crates/lisa-cli/src/capture_usage.rs` implements the current Stop-hook capturer.
- It reads a JSON Stop payload from standard input.
- The payload type currently extracts only `transcript_path`.
- Claude transcript events are summed across assistant messages.
- Claude cache-creation and cache-read input counts are folded into input tokens.
- Codex rollout token events are cumulative, so the latest total wins.
- Both parsers reduce their observations to `UsageTotals`.
- `UsageTotals` has `input_tokens: u64` and `output_tokens: u64`.
- The old `usage_artifact` nests those totals under a `usage` field.
- The old artifact also carries a guessed `key`.
- `resolve_key` prefers inherited `LISA_TICKET_ID`.
- It falls back to `pane-<LISA_PANE_ID>` and then `last`.
- `run_capture_usage` writes `<key>.usage.json` with `std::fs::write`.
- That write replaces a prior file at the same path.
- The current file-level comments describe last-write-wins behavior.
- The current CLI tests verify parsing and old artifact compatibility.
- They do not provide an append-only record contract.

## Existing plugin read path

- The plugin currently reads provider-specific `<ticket>.usage.json` files.
- It uses provenance token extraction on the nested `usage` value.
- The plugin's terminal provenance rows contain optional token totals.
- Later S-043 tickets replace ticket-keyed reading with pane-and-time attribution.
- This ticket does not alter those readers or provenance records.

## Existing append-only precedent

- `crates/lisa-core/src/provenance.rs` owns an append-only JSONL schema.
- It defines serializable record structs beside their append functions.
- Its timestamps are UTC epoch seconds stored as `u64`.
- `system_time_to_epoch` converts `SystemTime` to that convention.
- `append_record` accepts a `&Path` and a borrowed record.
- It creates a missing parent directory.
- It serializes a compact JSON object with `serde_json::to_string`.
- It adds exactly one newline after the object.
- It opens the destination with `create(true).append(true)`.
- It writes the complete line with `write_all`.
- Serialization errors are mapped to `io::ErrorKind::InvalidData`.
- Its tests use `tempfile::tempdir` for isolated filesystem behavior.
- Its append tests parse each JSONL line back into the record type.
- They assert two appends create two lines.
- They assert the first parsed record remains the original record.
- A separate test verifies an append produces exactly one newline-terminated row.
- Another test verifies a failed append does not disturb existing contents.

## Type and naming conventions

- Persisted core records derive `Debug`, `Clone`, `PartialEq`, and serde traits.
- Records whose fields support total equality also derive `Eq`.
- Pane IDs elsewhere in core and plugin data are `u32`.
- Token counts in provenance are `Option<u64>` because terminal records may lack usage.
- Successful transcript captures already calculate concrete `u64` totals.
- A no-capture marker is assigned to later ticket T-043-02-02.
- The success record therefore does not need to represent missing token facts.
- Session IDs are external identifiers and are represented naturally as `String`.
- Core timestamps use integer epoch seconds instead of adding a datetime library.
- The project does not have a chrono dependency in `lisa-core`.

## Story and dependency context

- S-043-01 has two parallel tickets touching disjoint crates.
- T-043-01-01 owns the core capture schema.
- T-043-01-02 owns plugin pane-time ownership lookup.
- The story says both settle contracts before writers and consumers are changed.
- S-043-02 later rewrites `run_capture_usage` to append this record.
- That later writer needs a public append function as well as the public type.
- S-043-03 later consumes capture records and attributes them with `owner_at`.
- That later consumer needs a stable public module path and serde shape.
- The story calls this a correctness-only slice.
- Cache-split parity is explicitly deferred.
- Cost fields are not part of the capture record.
- Ticket IDs are deliberately absent because the capturing process cannot know ownership honestly.
- Provider identity is not listed among the required facts.
- Attribution and quarantine behavior are not part of this contract ticket.

## Repository state and workflow constraints

- The ordinary worktree already shows modifications to this ticket and its sibling ticket.
- Those changes are Lisa-managed phase/lease state and are not ticket-owned source edits.
- They must not be staged or included in a ticket source commit.
- Phase artifacts belong under the attempt-private work directory.
- Lisa will publish admitted artifacts after lease verification.
- Source edits must be committed only with `lisa commit-ticket`.
- That command requires exact repository-relative include paths.
- Ordinary `git add` and `git commit` are prohibited for this work.
- All ticket-owned source changes must be clean before Review finishes.

## Verification surface

- A focused `cargo test -p lisa-core` run covers the new unit tests.
- `cargo fmt --all -- --check` verifies workspace formatting without mutation.
- `cargo test --workspace` exercises downstream compilation and all existing tests.
- No live provider, Zellij instance, or model tokens are required.
- The acceptance condition can be proven deterministically with a temporary file.

## Observed constraints

- The JSONL writer must use append mode; a read-modify-rewrite implementation would weaken the contract.
- The first-row check must compare raw bytes, not only equivalent parsed JSON.
- Both records should use the same pane ID to reproduce the overwrite-risk key collision.
- Session IDs or timestamps should differ so the two captures are distinguishable.
- Serialization should remain compact so one record is exactly one JSONL row.
- The module must be declared publicly from `lib.rs` for later crates to consume it.
- No current source file defines `CaptureRecord` or a capture JSONL path.
- The exact eventual on-disk location belongs to the later writer ticket, not this schema.
