# Plan — T-043-01-01 append-only capture record schema

## Objective

Add a public `lisa-core` capture record and append helper, prove the exact round-trip and byte-preserving two-row behavior, commit the two source paths through Lisa's isolated transaction, and verify the workspace remains green.

## Preconditions

- Work only from the current attempt lease.
- Do not edit ticket phase or status frontmatter.
- Preserve Lisa-managed changes already visible in ticket files.
- Do not change the legacy CLI writer or plugin reader.
- Do not use the ordinary Git index.
- Use `apply_patch` for source and artifact edits.

## Step 1 — Create the capture module and record schema

File:

- `crates/lisa-core/src/capture.rs`

Actions:

1. Add module documentation explaining raw observation versus later attribution.
2. Import serde derive traits.
3. Define public `CaptureRecord`.
4. Use `u32` for `pane_id`.
5. Use `String` for `session_id`.
6. Use epoch-seconds `u64` for `captured_at`.
7. Use concrete `u64` token counts.
8. Derive `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`, and `Deserialize`.
9. Document every public field.

Independent verification:

- Inspect the serialized field names in the unit test.
- Ensure the type contains no ticket ID or guessed key.
- Ensure no new dependency is required.

## Step 2 — Implement append-only JSONL emission

File:

- `crates/lisa-core/src/capture.rs`

Actions:

1. Add `append_capture_record(&Path, &CaptureRecord) -> io::Result<()>`.
2. Serialize with `serde_json::to_string` before touching the file.
3. Map serialization errors to `io::ErrorKind::InvalidData`.
4. Add exactly one trailing newline.
5. Create a missing parent directory.
6. Open with `OpenOptions::new().create(true).append(true)`.
7. Write the full framed row through the append handle.
8. Do not call `fs::write`, `truncate`, rename, or read existing contents.

Independent verification:

- Code inspection confirms append flags and absence of overwrite APIs.
- The test path's missing parent confirms directory creation.
- Newline assertions confirm JSONL framing.

## Step 3 — Add the acceptance-focused unit test

File:

- `crates/lisa-core/src/capture.rs`

Fixture:

- First pane ID: `7`.
- First session ID: a stable opaque string.
- First timestamp: a fixed epoch second.
- First token totals: non-zero, distinct values.
- Second pane ID: also `7`.
- Second session ID, timestamp, and totals: different from the first.

Actions:

1. Serialize the first record directly.
2. Deserialize it as `CaptureRecord`.
3. Assert equality to prove a complete round trip.
4. Assert the compact serialized value contains all five expected fields.
5. Append the first record to a temporary nested JSONL path.
6. Read and preserve the exact first-write bytes.
7. Confirm the first write ends in one newline.
8. Append the second same-pane record.
9. Read the final bytes.
10. Assert the final bytes begin with the saved first-write byte slice.
11. Assert the first-row portion before the newline equals the originally serialized first record bytes.
12. Split the final file on newline bytes and discard the trailing empty segment.
13. Assert exactly two row slices remain.
14. Deserialize both rows.
15. Assert their ordered equality to the original record pair.

Acceptance mapping:

- “round-trips a CaptureRecord” is covered by steps 1–3.
- `pane_id`, `session_id`, `captured_at`, and token fields are covered by fixture construction and equality.
- “same pane” is covered by both records using pane `7`.
- “two JSONL rows” is covered by row count and ordered parsing.
- “first row byte-intact” is covered by the raw prefix and raw row comparisons.
- “never an overwrite” is covered by retaining both ordered records after the second call.

## Step 4 — Expose the module

File:

- `crates/lisa-core/src/lib.rs`

Actions:

1. Add `pub mod capture;` before `pub mod client;`.
2. Do not add root-level item re-exports.
3. Leave every existing module declaration unchanged.

Independent verification:

- `cargo check` through the test run verifies the public module builds.
- Source inspection verifies the expected downstream path is `lisa_core::capture`.

## Step 5 — Format and run focused verification

Commands:

```bash
cargo fmt --all
cargo test -p lisa-core capture
cargo test -p lisa-core
```

Criteria:

- Formatting completes without unrelated changes.
- The new test passes.
- All existing `lisa-core` unit and integration tests pass.
- No generated dependency or lockfile changes appear.

If formatting changes unrelated paths:

- Do not include them in ticket ownership.
- Inspect whether they were pre-existing user changes.
- Keep the commit include list restricted to the two planned files.

## Step 6 — Commit the meaningful source unit

The schema, append API, test, and module exposure form one atomic public feature. Commit them together only after focused tests pass.

Command:

```bash
lisa commit-ticket \
  --ticket-id T-043-01-01 \
  --message "feat(core): add append-only capture records" \
  --include crates/lisa-core/src/capture.rs \
  --include crates/lisa-core/src/lib.rs
```

Post-commit checks:

- Confirm the command succeeds.
- Run `git status --short`.
- Confirm neither source path is staged, modified, or untracked.
- Ignore but preserve Lisa-managed ticket/work artifact changes.

## Step 7 — Run broad verification

Commands:

```bash
cargo fmt --all -- --check
cargo test --workspace
```

Criteria:

- The workspace is formatted.
- Every workspace test passes.
- Downstream crates compile against the new core module declaration.
- There is no need for a live provider or Zellij test.

If broad tests reveal a ticket-owned defect:

1. Record the deviation and failure in `progress.md`.
2. Patch only the owned source paths.
3. Re-run focused verification.
4. Commit the fix through a second exact-path `lisa commit-ticket` call.
5. Re-run broad verification.

## Step 8 — Complete Implement evidence

Artifact:

- `.lisa/attempts/T-043-01-01/1/work/progress.md`

Record:

- Each completed implementation step.
- The precise source paths changed.
- The isolated commit identifier or command result.
- Focused and workspace test results.
- Any deviations from the design or plan.
- Remaining work, expected to be Review only.

## Step 9 — Review and disposition

Artifacts:

- `.lisa/attempts/T-043-01-01/1/work/review.md`
- `.lisa/attempts/T-043-01-01/1/work/review-disposition.json`

Review checks:

1. Inspect the committed diff for accidental scope expansion.
2. Confirm the schema has only honest capture facts.
3. Confirm the append helper cannot truncate existing rows.
4. Confirm the test compares raw bytes, not only parsed values.
5. Confirm both source paths are clean.
6. Summarize coverage and explicitly name deferred writer/consumer integration.
7. Use pass disposition only if implementation and all verification are green.
8. Otherwise use block with a non-empty actionable reason.

## Planned commit count

One meaningful source commit is expected. A second commit is permitted only for a defect discovered after the first isolated commit. Phase artifacts are not included in either source commit because Lisa owns their admission and completion publication.
