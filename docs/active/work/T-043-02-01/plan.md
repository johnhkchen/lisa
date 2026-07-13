# Plan — T-043-02-01 append capture, not overwrite

## Objective

Make `lisa capture-usage` append honest `CaptureRecord` rows instead of overwriting guessed ticket files, prove the exact two-invocation behavior at the compiled CLI boundary, commit only ticket-owned paths through Lisa's isolated transaction, and leave the workspace verified and clean.

## Preconditions

- Work under attempt generation 1 for T-043-02-01.
- Do not edit ticket phase or status.
- Preserve Lisa-managed provenance, journal, and ticket worktree changes.
- Keep phase artifacts in the attempt-private work directory.
- Do not edit plugin attribution behavior.
- Do not edit Stop-hook silence behavior assigned to T-043-02-02.
- Do not use the ordinary Git index.
- Use exact repository-relative paths for the source commit.

## Step 1 — Add the failing CLI regression test

Create:

- `crates/lisa-cli/tests/capture_usage_cli.rs`

Fixture actions:

1. Create a temporary project root.
2. Write a first Claude transcript with known assistant token usage.
3. Write a second Claude transcript with different known assistant usage.
4. Use one stable pane ID for both command executions.
5. Use a deliberately stale ticket ID for both executions.
6. Use two distinct provider session IDs.

Process helper actions:

1. Start the compiled `lisa` binary.
2. Pass `capture-usage --cwd <temp-root>`.
3. Set `LISA_PANE_ID`.
4. Set `LISA_TICKET_ID` to reproduce the old defect.
5. Remove `LISA_AGENT_CLIENT` so the fixture uses Claude parsing.
6. Pipe a JSON payload with `session_id` and `transcript_path` to stdin.
7. Wait for completion and assert success.

Assertion actions:

1. Invoke the helper for the first transcript/session.
2. Invoke it again for the second transcript/session.
3. Read `.lisa/claude/captures.jsonl`.
4. Deserialize every non-empty row as `CaptureRecord`.
5. Assert exactly two rows in invocation order.
6. Assert both pane IDs equal the supplied pane.
7. Assert session IDs match each payload.
8. Assert each row's token totals match its transcript.
9. Assert capture timestamps fall between test wall-clock bounds.
10. Assert `.lisa/claude/<stale-ticket>.usage.json` does not exist.

Regression verification:

```bash
cargo test -p lisa-cli --test capture_usage_cli
```

Expected before implementation:

- The test fails because `captures.jsonl` is absent.
- The old ticket usage file is produced and overwritten.

## Step 2 — Extend the Stop payload fact set

Modify:

- `crates/lisa-cli/src/capture_usage.rs`

Actions:

1. Add optional `session_id` to `StopPayload`.
2. Keep serde default behavior for compatibility.
3. Update the payload documentation to name both consumed fields.
4. In `run_capture_usage`, require a non-empty session ID.
5. Keep failure as `Ok(())` for this ticket.

Independent check:

- No session ID is derived from ticket, pane, transcript path, or fallback text.
- Existing provider transcript tests still compile.

## Step 3 — Read the honest pane fact

Modify the same capture module.

Actions:

1. Read `LISA_PANE_ID` directly in `run_capture_usage`.
2. Reject a missing or empty value.
3. Parse the value as `u32`.
4. Reject a malformed/out-of-range value.
5. Do not read `LISA_TICKET_ID` anywhere in the module.

Independent check:

```bash
rg -n "LISA_TICKET_ID|resolve_key" crates/lisa-cli/src/capture_usage.rs
```

Expected:

- No matches.

## Step 4 — Replace old persistence with `CaptureRecord`

Modify the same capture module.

Actions:

1. Import `CaptureRecord` and `append_capture_record`.
2. Import `SystemTime` and the shared epoch conversion.
3. Preserve provider selection and token parser selection.
4. Preserve existing zero-total no-observation behavior.
5. Construct a record with pane, session, current epoch seconds, and totals.
6. Select `.lisa/claude` or `.lisa/codex` as today.
7. Append to `captures.jsonl` inside that directory.
8. Return the append helper's result.
9. Remove direct directory creation now handled by the helper.
10. Remove `std::fs::write` and pretty artifact serialization.

Independent check:

- Source contains `append_capture_record`.
- Source contains no `<key>.usage.json` construction.
- Two CLI calls retain two rows.

## Step 5 — Delete false-attribution helpers

Modify the capture module.

Actions:

1. Delete `resolve_key`.
2. Delete its comments describing ticket-to-pane-to-last fallback.
3. Delete `usage_artifact`.
4. Delete the old nested artifact compatibility unit test.
5. Retain all parser unit tests.

Independent check:

```bash
rg -n "resolve_key|usage_artifact|last\.usage|key.*usage" crates/lisa-cli/src/capture_usage.rs
```

Expected:

- No production legacy helper or fallback path remains.

## Step 6 — Make source documentation truthful

Modify:

- `crates/lisa-cli/src/capture_usage.rs`
- `crates/lisa-cli/src/main.rs`

Actions:

1. Replace overwrite/last-write-wins module docs.
2. Describe append-only pre-attribution records.
3. Update `run_capture_usage` docs to name `captures.jsonl`.
4. Update Clap help to remove `<ticket>.usage.json` claims.
5. Update the `--cwd` argument help to be provider-neutral.
6. Leave hook error suppression text/files for T-043-02-02.

Independent check:

```bash
rg -n "<ticket>\.usage\.json|overwritten|last-write-wins" \
  crates/lisa-cli/src/capture_usage.rs crates/lisa-cli/src/main.rs
```

Expected:

- No stale claim remains in the modified command path.

## Step 7 — Format and run focused verification

Commands:

```bash
cargo fmt --all
cargo test -p lisa-cli --test capture_usage_cli
cargo test -p lisa-cli capture_usage
cargo test -p lisa-cli --test help_surface
```

Criteria:

- New CLI regression test passes.
- Existing Claude and Codex parser tests pass.
- Existing help-surface command set remains intact.
- Formatting changes are limited to ticket-owned paths or inspected carefully.
- No manifest or lockfile update occurs.

If the test reveals command-level stderr or no-write behavior:

- Do not expand into T-043-02-02.
- Record the boundary in progress.
- Fix only behavior necessary for successful capture rows.

## Step 8 — Inspect the source diff

Commands:

```bash
git diff -- crates/lisa-cli/src/capture_usage.rs \
  crates/lisa-cli/src/main.rs \
  crates/lisa-cli/tests/capture_usage_cli.rs
git diff --check -- crates/lisa-cli/src/capture_usage.rs \
  crates/lisa-cli/src/main.rs \
  crates/lisa-cli/tests/capture_usage_cli.rs
```

Review criteria:

- Record contains only honest facts.
- `LISA_TICKET_ID` does not influence writer behavior.
- Existing provider parsers are unchanged.
- Append helper owns file creation and serialization.
- The test invokes the real binary twice.
- No unrelated files are included.

## Step 9 — Commit the meaningful source unit

The implementation and its acceptance test form one atomic writer behavior change.

Command:

```bash
lisa commit-ticket \
  --ticket-id T-043-02-01 \
  --message "fix(cli): append honest capture records" \
  --include crates/lisa-cli/src/capture_usage.rs \
  --include crates/lisa-cli/src/main.rs \
  --include crates/lisa-cli/tests/capture_usage_cli.rs
```

If `lisa` is not available on PATH, use the already-built project CLI binary with the identical `commit-ticket` arguments. Do not substitute ordinary Git staging or commit commands.

Post-commit criteria:

- Capture the returned commit ID in `progress.md`.
- All three ticket-owned paths are clean.
- Ordinary-index state belonging to others is untouched.
- Attempt artifacts remain private and uncommitted by this command.

## Step 10 — Run broad verification

Commands:

```bash
cargo fmt --all -- --check
cargo test --workspace
```

Criteria:

- Workspace formatting passes.
- All workspace tests pass.
- Core capture contract compiles through the CLI.
- Plugin tests remain green despite its intentionally staged old reader.
- No live provider or Zellij test is required.

If broad verification exposes a ticket-owned defect:

1. Document the deviation in `progress.md`.
2. Patch only the three owned paths.
3. Re-run focused verification.
4. Commit the correction through another exact-path `lisa commit-ticket` call.
5. Re-run workspace verification.

## Step 11 — Complete implementation evidence

Write:

- `.lisa/attempts/T-043-02-01/1/work/progress.md`

Record:

- Test-first regression result.
- Exact implemented behavior.
- Files changed.
- Focused test results.
- Isolated commit result.
- Workspace test result.
- Any deviations and their rationale.
- Remaining work, expected to be Review only.

## Step 12 — Review and disposition

Write:

- `.lisa/attempts/T-043-02-01/1/work/review.md`
- `.lisa/attempts/T-043-02-01/1/work/review-disposition.json`

Review checks:

1. Inspect the committed diff.
2. Confirm no false ticket key remains.
3. Confirm the stale ticket artifact assertion passes.
4. Confirm two rows preserve two sessions and totals.
5. Confirm timestamps use scheduler-compatible epoch seconds.
6. Confirm all ticket-owned paths are clean.
7. Name the intentionally deferred no-capture and consumer work.
8. Use pass only if implementation, commit, and verification are complete.

## Planned commit count

One meaningful source commit is expected. A second isolated commit is permitted only for a defect found after the first commit. Lisa, not this agent, will later publish phase artifacts and create the completion commit.
