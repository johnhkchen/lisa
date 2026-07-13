# Research — T-043-02-01 append capture, not overwrite

## Ticket boundary

- The ticket starts in Research and requires all remaining RDSPI phases.
- Its implementation target is the `capture-usage` CLI path.
- Its stated defect is false ticket attribution caused by inherited environment.
- Its stated data-loss defect is replacement of an existing usage artifact.
- The acceptance case invokes the CLI twice for one pane.
- Each invocation uses a different transcript.
- The result must contain two `CaptureRecord` values.
- Each record must contain pane, session, capture time, and token totals.
- The old `<ticket>.usage.json` artifact must not be created.
- The old `resolve_key` ticket-guess path must be absent.
- Failed or empty capture signaling belongs to the dependent T-043-02-02.
- Plugin-side attribution belongs to S-043-03.

## Repository and workflow state

- The workspace contains `lisa-core`, `lisa-cli`, and `lisa-plugin`.
- The current branch already contains completed T-043-01-01 and T-043-01-02 work.
- T-043-01-01 introduced the shared capture schema and append function.
- T-043-01-02 introduced pane-at-time scheduler ownership lookup.
- The ordinary worktree contains Lisa-managed ticket and provenance changes.
- Those existing changes are not source owned by this ticket.
- Attempt artifacts belong only in `.lisa/attempts/T-043-02-01/1/work/`.
- Lisa publishes admitted phase artifacts later.
- Ticket frontmatter phase and status are Lisa-managed.
- Source work must be committed with `lisa commit-ticket` and exact include paths.
- Ordinary `git add` and `git commit` are prohibited.

## Current CLI command surface

- `crates/lisa-cli/src/main.rs` declares the `CaptureUsage` subcommand.
- The command accepts a `--cwd` project-root argument.
- Its default project root is `.`.
- `main` resolves that path before invoking the capture module.
- `main` intentionally ignores the returned result.
- The surrounding comment describes the Stop hook as best-effort.
- The command help still describes a ticket-keyed Claude usage file.
- The CLI binary is named `lisa` in `crates/lisa-cli/Cargo.toml`.
- `lisa-cli` depends directly on `lisa-core`.
- It already depends on serde and serde_json.
- Its dev dependencies already include tempfile.
- No integration-test process helper crate is currently listed.

## Current capture module

- `crates/lisa-cli/src/capture_usage.rs` owns the native-TUI capture logic.
- Its module comments cover both Claude and Codex native transcript formats.
- Claude is the default client when `LISA_AGENT_CLIENT` is absent.
- Codex is selected when `LISA_AGENT_CLIENT=codex`.
- Provider selection also selects `.lisa/claude` or `.lisa/codex`.
- The function reads the Stop-hook payload from process standard input.
- Standard-input read failure currently returns `Ok(())`.
- Malformed Stop JSON currently returns `Ok(())`.
- A missing transcript path currently returns `Ok(())`.
- An unreadable transcript currently returns `Ok(())`.
- A transcript yielding zero totals currently returns `Ok(())`.
- Those no-write branches are retained for the next ticket to surface.

## Stop payload facts

- `StopPayload` currently derives `Deserialize`.
- It currently declares only `transcript_path: Option<String>`.
- The type-level comment says other payload fields are ignored.
- Native Stop payloads also carry a provider session identifier.
- The ticket requires that session identifier in every successful record.
- Serde ignores undeclared JSON fields by default.
- Therefore existing payload parsing currently discards `session_id`.
- The payload fixture can supply `session_id` without changing outer CLI wiring.
- Session IDs are opaque strings in the shared core contract.

## Pane facts

- The native agent launch command supplies `LISA_PANE_ID`.
- The current writer already reads that variable indirectly in `resolve_key`.
- `resolve_key` treats it as an arbitrary non-empty string.
- `CaptureRecord` requires a numeric `u32` pane ID.
- Scheduler and plugin pane IDs are also `u32`.
- The inherited pane ID remains stable when a pane is recycled.
- Unlike inherited ticket ID, pane identity remains an honest capture fact.
- A missing or malformed pane cannot populate a valid success record.
- The current command contract is best-effort for unavailable required inputs.

## Existing token parsers

- `UsageTotals` is a private two-field value type.
- Both fields are concrete `u64` values.
- Claude parsing visits each transcript JSONL line.
- It ignores blank, malformed, and non-assistant lines.
- It reads `message.usage` from assistant events.
- It sums fresh input, cache-creation input, and cache-read input.
- It sums assistant output tokens.
- Missing individual token fields count as zero.
- Codex parsing visits cumulative `event_msg/token_count` records.
- The latest `total_token_usage` object wins.
- Codex currently retains only aggregate input and output totals.
- Provider cache-split parity is explicitly outside the story.
- Parser behavior is already covered by focused unit tests.

## Existing artifact construction

- `usage_artifact` creates a nested serde JSON value.
- Its top-level `key` field stores the guessed attribution key.
- Its nested `usage` field stores input and output totals.
- The old shape exists to match `provenance::extract_usage`.
- A unit test asserts that cross-module nested shape.
- The new `CaptureRecord` is a flat serialized object.
- Its schema does not contain `key`, `usage`, or `ticket_id`.
- The old artifact helper is only used by `run_capture_usage` and its unit test.
- Replacing the writer removes its production use.

## Existing key resolution

- `resolve_key` first reads `LISA_TICKET_ID`.
- It accepts any non-empty ticket value.
- It next reads `LISA_PANE_ID`.
- It formats the pane fallback as `pane-<value>`.
- It finally falls back to the shared string `last`.
- `run_capture_usage` embeds the resolved key in JSON.
- It also embeds the resolved key in the filename.
- The inherited ticket value is fixed at native client process birth.
- Pane recycling does not update that process environment.
- Later ticket work in the same pane therefore retains the first ticket value.
- The old filename causes later captures to replace the first ticket's file.
- The fallback `last` also blends unrelated unkeyed sessions.
- No other function in the CLI references `resolve_key`.

## Existing write semantics

- `run_capture_usage` creates the provider-specific directory.
- It constructs `<key>.usage.json` inside that directory.
- It serializes the old artifact with pretty JSON.
- It writes with `std::fs::write`.
- `std::fs::write` truncates an existing destination.
- A second capture with the same resolved key destroys the first artifact.
- The file comments explicitly describe last-write-wins behavior.
- The comments assume terminal teardown reads a final cumulative total.
- That assumption is invalid once a long-lived pane changes ticket ownership.

## Shared capture contract already available

- `crates/lisa-core/src/capture.rs` is public through the crate root.
- It defines `CaptureRecord`.
- `pane_id` is `u32`.
- `session_id` is `String`.
- `captured_at` is UTC epoch seconds stored as `u64`.
- `input_tokens` and `output_tokens` are `u64`.
- The record derives serde serialization and deserialization.
- It intentionally carries no ticket ID.
- It intentionally carries no guessed artifact key.
- `append_capture_record` accepts an explicit path and borrowed record.
- It creates missing parent directories.
- It serializes compact one-line JSON.
- It terminates each record with a newline.
- It opens the destination with create-plus-append.
- It never reads or truncates previous rows.
- Its unit test proves two same-pane records preserve the first row byte-for-byte.

## Time conventions

- `lisa-core::provenance::system_time_to_epoch` is public.
- It converts `SystemTime` to epoch seconds.
- It saturates pre-epoch values to zero.
- `CaptureRecord::captured_at` uses the same unit.
- Plugin ownership intervals use epoch seconds for lookup.
- The CLI writer currently has no clock abstraction.
- A process-level acceptance test can validate a nonzero current timestamp.
- Two fast invocations may legitimately share the same epoch second.
- Ordering is represented by JSONL row order even when seconds are equal.

## Existing tests and test boundaries

- Capture parser tests live inside `capture_usage.rs`.
- They can call private helpers directly.
- They do not invoke the compiled CLI process.
- `crates/lisa-cli/tests/` contains binary-facing integration tests.
- Cargo exposes `CARGO_BIN_EXE_lisa` to integration-test binaries.
- Standard `std::process::Command` can invoke the binary without a new crate.
- `Command::env_remove` can isolate provider selection.
- `Command::env` can supply pane and stale ticket values.
- `Stdio::piped` can feed distinct Stop payloads to standard input.
- A temporary project root can hold both transcripts and `.lisa` output.
- The integration test can deserialize rows through `lisa_core::capture::CaptureRecord`.

## Downstream consumer boundary

- The plugin still reads `.lisa/<client>/<ticket>.usage.json` today.
- That reader will become stale when this ticket lands.
- T-043-03-01 is explicitly assigned to replace that reader.
- This ticket therefore changes the writer before the consumer.
- During that staged interval, new captures are durable but not yet attributed.
- Keeping old ticket-keyed writes in parallel would preserve the known false state.
- The acceptance criterion explicitly requires no old ticket artifact.
- No plugin source file belongs to this ticket.

## Documentation boundary

- `main.rs` has command help describing the old path.
- `capture_usage.rs` has module and function docs describing overwrite behavior.
- Those source comments become incorrect when the writer changes.
- The embedded Stop-hook behavior is owned by T-043-02-02, not this ticket.
- The live hook and hooks guide are also owned by T-043-02-02.
- Public operator documentation beyond truthful command help is not required here.

## Acceptance evidence required

- The test must run `capture-usage` twice, not merely call the core append helper.
- Both calls must use the same pane ID.
- They must use different transcript contents.
- Payload session IDs must be distinguishable.
- Token totals must prove both parsers' results were retained in order.
- Each row must contain a meaningful epoch capture time.
- A stale `LISA_TICKET_ID` should be present to reproduce the old defect.
- The corresponding `<ticket>.usage.json` must be absent after both calls.
- The append destination must contain exactly two parseable rows.
- Source inspection and compilation must show `resolve_key` is gone.

## Observed constraints and assumptions

- Successful records require both pane and session identity.
- The provider-specific directory remains useful because transcript semantics differ.
- One append-only file per provider is sufficient to retain all capture observations.
- Pane ID inside each row supports later filtering and attribution.
- Session ID inside each row supports later quarantine behavior.
- The next ticket owns visible markers for unsuccessful captures.
- The next story owns attribution and consumption.
- No live provider, Zellij session, or billable model run is needed.
- The full acceptance path is deterministic with temporary files and the CLI binary.
