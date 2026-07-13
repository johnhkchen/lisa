# Research — T-043-02-02 loud no-write signal

## Ticket boundary

- The ticket starts in Research.
- It follows T-043-02-01, which replaced guessed ticket files with append-only capture facts.
- The requested behavior concerns unsuccessful native Stop-hook captures.
- The acceptance case names empty, missing, and unreadable transcripts.
- An unsuccessful capture must leave an operator-visible marker.
- The marker must carry pane identity.
- The marker must carry provider session identity.
- The marker must carry a reason.
- Embedded and live Stop hooks must stop redirecting Lisa's standard error to `/dev/null`.
- The assignment requires all remaining RDSPI phases in one continuous pass.
- Phase artifacts belong under the private attempt work directory.
- Ticket phase and status fields are managed by Lisa and are not agent-owned edits.
- Ticket source must be committed through `lisa commit-ticket` with exact paths.

## Story context

- Story S-043-02 covers the capture writer end to end.
- Its first ticket made successful observations honest and append-only.
- This ticket makes unsuccessful observations durable and visible.
- Plugin-side attribution remains assigned to S-043-03.
- Session-keyed quarantine remains assigned to T-043-03-02.
- Cache-dimension parity remains explicitly out of scope.
- The writer records capture-time facts rather than guessing ticket ownership.

## Capture command entry point

- `crates/lisa-cli/src/main.rs` declares the `CaptureUsage` subcommand.
- The command accepts a `--cwd` project root.
- The command resolves that path before calling the capture module.
- Dispatch currently ignores the result of `run_capture_usage`.
- Its comment explicitly says hook errors are swallowed.
- Therefore an I/O failure can currently vanish even if shell stderr is no longer redirected.
- Other CLI commands generally print an error and exit nonzero when their implementation fails.
- The capture command is hidden plumbing but is also directly executable in integration tests.

## Capture module

- `crates/lisa-cli/src/capture_usage.rs` owns native transcript capture.
- `run_capture_usage` reads the Stop payload from standard input.
- `StopPayload` currently has optional `transcript_path` and `session_id` fields.
- The pane comes from `LISA_PANE_ID` rather than the payload.
- Pane parsing requires a nonempty unsigned 32-bit integer.
- Provider selection comes from `LISA_AGENT_CLIENT=codex`.
- Any other value, including absence, selects Claude.
- Provider selection also determines `.lisa/claude` versus `.lisa/codex`.

## Current silent exits

- A standard-input read error returns `Ok(())`.
- Malformed Stop JSON returns `Ok(())`.
- A missing transcript path returns `Ok(())`.
- A missing or empty session ID returns `Ok(())`.
- A missing, empty, or invalid pane environment value returns `Ok(())`.
- An unreadable transcript returns `Ok(())`.
- A transcript that produces default zero totals returns `Ok(())`.
- Every one of those paths writes no successful capture row.
- The acceptance criterion specifically requires a pane, session, and reason on the marker.
- Some early failures occur before pane or session can be established.
- Those failures cannot truthfully produce the required fully identified marker.
- They can still be surfaced as command errors once dispatch stops swallowing errors.

## Transcript parsing

- Claude transcripts are newline-delimited JSON.
- Only events with `type = assistant` contribute usage.
- The parser sums fresh input, cache-creation input, and cache-read input.
- The parser separately sums output tokens.
- Blank, malformed, and non-assistant lines are skipped.
- Missing individual token fields count as zero.
- An empty Claude transcript produces `UsageTotals::default()`.
- A transcript containing no observed assistant usage also produces default totals.
- Codex transcripts use cumulative `event_msg/token_count` events.
- The last available `total_token_usage` record wins.
- Missing or malformed Codex usage events likewise produce default totals.
- The parsers intentionally do not distinguish a measured all-zero usage record from no observation.
- Existing comments describe this as never fabricating a measured zero.

## Successful capture contract

- `lisa_core::capture::CaptureRecord` is the shared successful record type.
- It contains `pane_id`, `session_id`, `captured_at`, `input_tokens`, and `output_tokens`.
- `captured_at` is a UTC epoch-second value.
- `append_capture_record` creates parent directories as needed.
- It appends compact newline-terminated JSON.
- It never truncates existing rows.
- The provider-specific destination is `captures.jsonl`.
- The successful record has no status, reason, or optional token fields.
- Existing integration tests deserialize every `captures.jsonl` line as `CaptureRecord`.
- Future plugin consumer work is also scoped around loading `CaptureRecord` rows.
- Mixing a differently shaped failure marker into this file would invalidate that homogeneous stream.

## Existing test coverage

- Unit tests in `capture_usage.rs` cover both transcript parsers.
- They cover empty Claude input returning default totals.
- They cover malformed and non-assistant lines.
- They cover missing token fields.
- They cover Codex's latest cumulative record selection.
- `crates/lisa-cli/tests/capture_usage_cli.rs` exercises the compiled CLI.
- Its helper supplies a pane, session, and transcript path.
- It captures child stdout and stderr.
- It currently asserts only successful exit.
- Its main test invokes the CLI twice for one pane.
- It verifies two preserved successful `CaptureRecord` rows.
- It verifies stale inherited ticket identity is ignored.
- There is no unsuccessful-capture integration case yet.

## Embedded Stop hook

- `crates/lisa-cli/src/templates.rs` defines `ON_STOP_HOOK`.
- The hook first writes the pane's `.stopped` signal when a pane is available.
- It then reads the complete Stop JSON from stdin once.
- It pipes that JSON to `${LISA_BIN:-lisa} capture-usage`.
- The current invocation ends with `2>/dev/null || true`.
- Standard error is therefore hidden.
- The shell status is also forced to success.
- The hook comments describe this as best effort.
- The Rust doc comment still describes the older overwrite artifact and swallowed failures.
- Template tests verify that the hook writes `.stopped`, invokes capture, uses `LISA_BIN`, and reads stdin.
- Template tests do not currently assert stderr visibility.

## Live Stop hook

- `.lisa/hooks/on-stop.sh` is tracked in this repository.
- Its content currently matches `ON_STOP_HOOK`.
- It contains the same `2>/dev/null || true` suffix.
- The ticket acceptance criterion explicitly names both embedded and live hooks.
- Updating only the embedded constant would leave this running repository on the silent generation.
- `lisa init` treats current and known legacy template bytes specially.
- The pre-capture stop hook is currently the only entry in `LEGACY_ON_STOP_HOOKS`.
- When the current template changes, the immediately preceding template must remain recognized for safe upgrades.

## Hooks guide

- `crates/lisa-cli/data/hooks-guide.md` is embedded as the `hooks-guide` output.
- It explains the lifecycle scripts and manual setup.
- It currently focuses on signal files rather than capture failure artifacts.
- The story scope explicitly includes this guide.
- The guide is an operator-facing place to name the no-capture ledger and stderr behavior.

## Init and upgrade behavior

- `crates/lisa-cli/src/init.rs` pairs `ON_STOP_HOOK` with `LEGACY_ON_STOP_HOOKS`.
- Exact known old templates are upgraded.
- Exact current templates are skipped as already up to date.
- Unknown project-owned hook content is preserved with a safety skip.
- Tests cover known legacy upgrades, current-template no-op behavior, and unknown-hook preservation.
- Changing `ON_STOP_HOOK` without adding the previous bytes to the legacy slice would cause existing installations to be preserved as unknown.
- The repository's live hook can be updated directly because the ticket explicitly owns it.

## Visibility and persistence constraints

- A durable marker must not overwrite an earlier unsuccessful observation.
- A provider-specific location keeps Claude and Codex capture semantics separated.
- A failure row needs a timestamp to distinguish repeated Stops and align with successful observations.
- The acceptance criterion does not explicitly name a timestamp, but the surrounding append-only design uses capture time.
- A human-readable reason is required.
- A stable machine-readable reason also enables deterministic assertions and future surfacing.
- Standard error is the immediate operator channel for actual command failures.
- A ledger is the durable operator trace for expected no-observation outcomes.
- If marker persistence itself fails, returning that I/O error is the only remaining honest behavior.

## Workspace state

- `.lisa/provenance.jsonl` was already modified by runtime activity.
- The active ticket files were already modified by Lisa phase management.
- `.lisa/completion-journal.jsonl` was already untracked runtime state.
- Those paths are not ticket-owned source changes for this implementation.
- They must not be included in ticket commits.
- The private attempt artifact directory is intentionally separate from published work.

## Observed boundaries

- The core successful schema should remain unchanged for this ticket.
- The transcript parsers should retain their provider semantics.
- Ticket attribution must remain absent from capture-time rows.
- Hook signal emission must remain independent of capture success.
- The live and embedded hooks must stay byte-aligned after the edit.
- Existing user-modified hooks must continue to be preserved by init.
- Known Lisa hook generations must remain upgradeable.
- Tests can exercise failure behavior through the compiled CLI with a temporary project root.
- Tests can inspect both the durable marker and process stderr/status.
- No live provider session is required to reproduce empty or unreadable transcript behavior.
