# Progress — T-043-02-01 append capture, not overwrite

## Status

- Research complete.
- Design complete.
- Structure complete.
- Plan complete.
- Implementation complete.
- Source commit complete.
- Focused verification complete.
- Workspace verification complete.
- Remaining phase at the time of this artifact: Review.

## Implemented behavior

`lisa capture-usage` now appends successful native transcript observations to:

```text
.lisa/claude/captures.jsonl
.lisa/codex/captures.jsonl
```

Each row is the shared `lisa_core::capture::CaptureRecord` shape:

- `pane_id`
- `session_id`
- `captured_at`
- `input_tokens`
- `output_tokens`

The writer no longer chooses a ticket-derived filename. It does not read `LISA_TICKET_ID`, does not fall back through pane/`last` keys, and does not call `std::fs::write` for successful capture persistence.

## Step 1 — Test-first CLI regression

Created:

- `crates/lisa-cli/tests/capture_usage_cli.rs`

The test launches the compiled `lisa` binary twice with:

- the same pane ID (`42`);
- the same deliberately stale ticket ID;
- two distinct Stop payload session IDs;
- two distinct transcript paths;
- two distinct Claude token totals.

It supplies each Stop payload over actual child-process stdin and passes a temporary project root through `--cwd`.

Initial command:

```bash
cargo test -p lisa-cli --test capture_usage_cli
```

Initial result:

- Failed as expected against the old writer.
- Failure occurred while reading `.lisa/claude/captures.jsonl` because the old code did not create it.
- Exit code was 101 from the failing Rust test.
- This established that the new test is sensitive to the old behavior.
- The old process calls themselves exited successfully, matching the best-effort command contract.

The test also asserts that `.lisa/claude/T-STALE-FIRST-TICKET.usage.json` is absent. That path reproduces the inherited-ticket defect directly rather than relying only on source inspection.

## Step 2 — Stop payload session fact

Modified:

- `crates/lisa-cli/src/capture_usage.rs`

Changes:

- Added `session_id: Option<String>` to `StopPayload`.
- Retained `#[serde(default)]` for defensive payload parsing.
- Required a non-empty value before writing a successful record.
- Copied the opaque provider value directly into `CaptureRecord`.
- Added no session fallback or filename encoding.

Missing or empty session behavior remains a no-write `Ok(())`, intentionally awaiting T-043-02-02's visible no-capture marker.

## Step 3 — Honest pane fact

Changes in the same module:

- Read `LISA_PANE_ID` directly.
- Required a non-empty value.
- Parsed it as `u32` to match the shared record and scheduler types.
- Returned the current best-effort no-write result for invalid values.
- Removed every `LISA_TICKET_ID` read from the capture module.

The stable pane environment remains valid across recycled native sessions, while inherited ticket identity does not.

## Step 4 — Append-only shared record persistence

Changes in the same module:

- Imported `CaptureRecord`.
- Imported `append_capture_record`.
- Imported `SystemTime`.
- Reused `lisa_core::provenance::system_time_to_epoch`.
- Preserved provider selection from `LISA_AGENT_CLIENT`.
- Preserved both provider transcript parsers.
- Preserved the existing all-zero no-observation guard.
- Constructed the record only after a successful transcript observation.
- Appended it to the provider's `captures.jsonl`.

The shared helper now owns:

- parent-directory creation;
- compact serialization;
- newline framing;
- create-plus-append file opening;
- retaining every earlier row.

No local read-modify-write or overwrite operation remains.

## Step 5 — Legacy false-attribution path removal

Deleted from `capture_usage.rs`:

- `resolve_key`;
- the `LISA_TICKET_ID → pane → last` fallback logic;
- `usage_artifact`;
- nested legacy key/usage object construction;
- the old `artifact_shape_matches_extract_usage` test.

The old shape test no longer represented emitted product data. Cross-crate contract evidence is now the CLI integration test deserializing actual output as `CaptureRecord`.

Search verification found no `LISA_TICKET_ID`, `resolve_key`, or `usage_artifact` match in `capture_usage.rs` after the change.

## Step 6 — Truthful command documentation

Modified:

- `crates/lisa-cli/src/capture_usage.rs`
- `crates/lisa-cli/src/main.rs`

Updates:

- Replaced module claims about overwrite and last-write-wins behavior.
- Documented append-only provider capture JSONL.
- Documented the pre-attribution boundary.
- Changed command help from Claude `<ticket>.usage.json` to native `<client>/captures.jsonl`.
- Made the `--cwd` help provider-neutral.

The embedded Stop-hook and operator guide remain unchanged because their stderr/no-capture behavior is explicitly assigned to dependent T-043-02-02.

## Focused verification

Formatting:

```bash
cargo fmt --all
```

Result: completed successfully.

CLI acceptance regression:

```bash
cargo test -p lisa-cli --test capture_usage_cli
```

Result after implementation:

- 1 passed.
- 0 failed.
- Both compiled CLI invocations exited successfully.
- Two JSONL rows parsed as `CaptureRecord`.
- Both rows carried pane `42`.
- Sessions remained ordered as `session-first`, `session-second`.
- Totals remained ordered as `16/5`, `151/50`.
- Both timestamps fell within invocation epoch bounds.
- The stale ticket usage artifact was absent.

Capture parser tests:

```bash
cargo test -p lisa-cli capture_usage
```

Result:

- 5 capture module tests passed.
- 0 failed.
- Claude summation, malformed-line handling, missing fields, empty observation, and Codex cumulative selection remain covered.

Help surface:

```bash
cargo test -p lisa-cli --test help_surface
```

Result:

- 3 passed.
- 0 failed.
- Command resolution and operator/help classification remain intact.

Diff hygiene:

```bash
git diff --check -- \
  crates/lisa-cli/src/capture_usage.rs \
  crates/lisa-cli/src/main.rs \
  crates/lisa-cli/tests/capture_usage_cli.rs
```

Result: passed with no whitespace errors.

## Isolated source commit

Command:

```bash
lisa commit-ticket \
  --ticket-id T-043-02-01 \
  --message "fix(cli): append honest capture records" \
  --include crates/lisa-cli/src/capture_usage.rs \
  --include crates/lisa-cli/src/main.rs \
  --include crates/lisa-cli/tests/capture_usage_cli.rs
```

Result:

- Commit: `ea9b29507bb6dd583e3d5e856ed53b4b89e8e991`.
- Subject: `fix(cli): append honest capture records`.
- 3 files changed.
- 137 insertions.
- 61 deletions.
- No manifest or lockfile change.
- No ordinary Git staging or commit command was used.

## Broad verification

Commands:

```bash
cargo fmt --all -- --check
cargo test --workspace
```

Result:

- Exit code 0.
- Formatting check passed.
- All workspace crate unit tests passed.
- CLI integration tests passed, including the new regression.
- Core unit and integration tests passed.
- Plugin suite passed: 377 tests, 0 failures.
- Doc tests passed.
- No live provider, Zellij session, or model tokens were required.

## Post-commit hygiene

`git show --check ea9b295...` passed.

The three ticket-owned paths are clean:

- no unstaged changes;
- no staged changes;
- no untracked ticket test file.

Remaining visible worktree entries are Lisa-managed state and published work artifacts:

- `.lisa/provenance.jsonl` modified;
- `docs/active/tickets/T-043-02-01.md` modified;
- `.lisa/completion-journal.jsonl` untracked;
- `docs/active/work/T-043-02-01/` untracked/published by Lisa's workflow.

None were included in the source commit or altered through the ordinary index.

## Deviations from plan

No implementation-scope deviation occurred.

One verification search over both `capture_usage.rs` and all of `main.rs` still found `LISA_TICKET_ID` in `main.rs`'s unrelated `AgentExec` command documentation. That occurrence is outside the `CaptureUsage` variant and remains correct for the headless agent wrapper. The capture module itself has no match, and no change was made to unrelated AgentExec behavior.

## Deferred named work

- T-043-02-02 will make empty/unreadable/no-write outcomes operator-visible.
- T-043-02-02 will remove Stop-hook stderr suppression.
- T-043-03-01 will consume capture rows and attribute them through pane-time ownership.
- T-043-03-02 will quarantine unattributable session records.
- Provider cache-split parity remains outside this story.

## Implement conclusion

The ticket-owned source unit is implemented, committed through Lisa's isolated transaction, fully verified, and clean. No implementation blocker remains; proceed to Review.
