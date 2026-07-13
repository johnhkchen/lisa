# Plan — T-043-02-02 loud no-write signal

## Goal

Implement append-only no-capture markers for identified empty, missing, and unreadable transcript outcomes; surface unrepresentable or persistence failures through the CLI and hook; update both embedded and live hook generations; verify the full behavior; and commit only exact ticket-owned paths through Lisa's isolated transaction.

## Preconditions

1. Preserve existing runtime changes in `.lisa/provenance.jsonl`.
2. Preserve Lisa-managed ticket phase changes.
3. Preserve `.lisa/completion-journal.jsonl` runtime state.
4. Do not use `git add`, `git commit`, or the ordinary index.
5. Keep successful capture semantics unchanged.
6. Keep plugin consumer work out of scope.

## Step 1 — Add a failing compiled CLI regression

Modify `crates/lisa-cli/tests/capture_usage_cli.rs`.

1. Make the command helper return `Output`.
2. Retain all current environment and stdin behavior.
3. Keep the successful-capture test asserting zero exit status.
4. Define a test-local deserializable no-capture row.
5. Create a new temporary project root.
6. Create one empty transcript.
7. Reserve one nonexistent transcript path.
8. Record an epoch timestamp before invoking the command.
9. Invoke `capture-usage` for one pane and an empty-transcript session.
10. Invoke it again for the same pane and an unreadable-transcript session.
11. Record an epoch timestamp after both commands.
12. Expect each invocation to succeed after persisting a marker.
13. Expect stderr to contain a visible no-capture notice.
14. Expect each notice to name its stable reason.
15. Read `.lisa/claude/no-captures.jsonl`.
16. Expect exactly two rows.
17. Expect both rows to carry the pane.
18. Expect distinct supplied sessions in append order.
19. Expect `empty-transcript` then `unreadable-transcript`.
20. Expect all timestamps within the invocation bounds.
21. Expect no successful `captures.jsonl`.

Verification:

```sh
cargo test -p lisa-cli --test capture_usage_cli empty_and_unreadable_transcripts_append_visible_no_capture_markers
```

Expected before implementation: failure because `no-captures.jsonl` does not exist and stderr is empty.

Record the red result in `progress.md`.

## Step 2 — Add the marker schema and append helper

Modify `crates/lisa-cli/src/capture_usage.rs`.

1. Import `Write` and `Serialize`.
2. Add the private `NoCaptureMarker` struct.
3. Add stable reason constants.
4. Add `append_no_capture_marker`.
5. Serialize compact JSON.
6. Map serialization errors to `InvalidData`.
7. Add a newline.
8. Create the provider directory.
9. Open `no-captures.jsonl` with create and append.
10. Write the row fully.
11. Emit the stderr notice only after the append succeeds.
12. Return the filesystem result without swallowing it.

Verification:

```sh
cargo fmt --all -- --check
cargo test -p lisa-cli --test capture_usage_cli empty_and_unreadable_transcripts_append_visible_no_capture_markers
```

The test may still fail until control flow is wired; compilation verifies the helper shape.

## Step 3 — Classify all identified transcript failures

Continue modifying `crates/lisa-cli/src/capture_usage.rs`.

1. Read stdin and convert failure to `InvalidData`.
2. Parse payload and convert malformed JSON to `InvalidData`.
3. Select provider before transcript classification.
4. Require a nonempty session ID.
5. Return `InvalidInput` when session is absent.
6. Parse and require `LISA_PANE_ID`.
7. Return `InvalidInput` when pane is absent or invalid.
8. Construct the provider client directory.
9. Filter empty transcript path strings.
10. Append `missing-transcript-path` when no usable path exists.
11. Read the supplied transcript.
12. Append `unreadable-transcript` on read failure.
13. Preserve provider-specific parser selection.
14. Append `empty-transcript` on default totals.
15. Preserve successful `CaptureRecord` construction and append.
16. Ensure every identified Stop writes exactly one outcome row.

Update module and function comments to describe both ledgers and strict error behavior.

Verification:

```sh
cargo test -p lisa-cli --test capture_usage_cli
cargo test -p lisa-cli capture_usage
```

Expected: current successful append regression and new no-capture regression both pass; parser unit tests pass.

## Step 4 — Make CLI dispatch preserve failures

Modify `crates/lisa-cli/src/main.rs`.

1. Keep `cwd` resolution.
2. Stop assigning the capture result to `_`.
3. Print an `Error:` line when capture returns an error.
4. Exit status 1 on error.
5. Remove the stale swallowed-error comment.
6. Leave clap command shape and help unchanged.

Optional targeted verification:

```sh
printf '{}' | cargo run -q -p lisa-cli -- capture-usage --cwd /tmp
test $? -ne 0
```

Use a safe temporary directory if running this diagnostic.

Primary verification remains the compiled integration suite.

## Step 5 — Validate capture source unit

Run:

```sh
cargo fmt --all -- --check
cargo test -p lisa-cli --test capture_usage_cli
cargo test -p lisa-cli capture_usage
```

Inspect:

```sh
git diff -- crates/lisa-cli/src/capture_usage.rs crates/lisa-cli/src/main.rs crates/lisa-cli/tests/capture_usage_cli.rs
```

Confirm:

- no ticket attribution was added;
- successful rows are unchanged;
- no-capture rows are append-only;
- failures are not silently ignored;
- test assertions cover pane, session, reason, time, and visibility.

## Step 6 — Commit capture source unit

Run exactly:

```sh
lisa commit-ticket \
  --ticket-id T-043-02-02 \
  --message "fix(cli): record visible no-capture outcomes" \
  --include crates/lisa-cli/src/capture_usage.rs \
  --include crates/lisa-cli/src/main.rs \
  --include crates/lisa-cli/tests/capture_usage_cli.rs
```

Record the returned commit ID in `progress.md`.

Verify those three paths are clean afterward.

## Step 7 — Update embedded Stop hook

Modify `crates/lisa-cli/src/templates.rs`.

1. Copy the exact pre-change `ON_STOP_HOOK` into the legacy slice.
2. Retain the older no-capture hook generation.
3. Remove `2>/dev/null` from current `ON_STOP_HOOK`.
4. Remove `|| true` from current `ON_STOP_HOOK`.
5. Update shell comments to name durable markers and visible errors.
6. Update Rust docs to describe append-only successful/no-capture ledgers.
7. Extend the template test with negative suppression assertions.
8. Preserve stdin forwarding and `.stopped` signal behavior.

Verification:

```sh
cargo test -p lisa-cli templates::tests::stop_hook
```

If the test filter does not match due to naming, run the full templates test module or package tests.

## Step 8 — Update live Stop hook

Modify `.lisa/hooks/on-stop.sh`.

1. Apply the same comments and command as `ON_STOP_HOOK`.
2. Preserve executable mode.
3. Compare file bytes to extracted current template content conceptually through diff/tests.
4. Confirm no stderr redirection remains.
5. Confirm no forced success remains.

Search verification:

```sh
rg -n 'capture-usage.*(2>/dev/null|\|\| true)' crates/lisa-cli/src/templates.rs .lisa/hooks/on-stop.sh
```

Expected: no match in current hook bodies. A match inside the explicitly retained legacy template is allowed and must be inspected rather than blindly removed.

## Step 9 — Document operator-visible outcomes

Modify `crates/lisa-cli/data/hooks-guide.md`.

1. Add successful capture ledger locations.
2. Add no-capture ledger locations.
3. Explain pane/session/time/reason marker fields.
4. Explain visible stderr for genuine failures.
5. Avoid claiming plugin consumption before S-043-03.
6. Keep manual hook setup consistent with generated behavior.

Review the rendered Markdown source for clarity and line length conventions.

## Step 10 — Validate hook upgrades and package behavior

Run:

```sh
cargo fmt --all -- --check
cargo test -p lisa-cli
```

This verifies:

- template shape;
- current-template no-op behavior;
- known legacy-template upgrade behavior;
- unknown-hook preservation;
- compiled capture CLI tests;
- help and other CLI regressions.

Inspect exact diffs for the three hook/document paths.

## Step 11 — Commit hook source unit

Run exactly:

```sh
lisa commit-ticket \
  --ticket-id T-043-02-02 \
  --message "fix(cli): surface Stop hook capture failures" \
  --include crates/lisa-cli/src/templates.rs \
  --include .lisa/hooks/on-stop.sh \
  --include crates/lisa-cli/data/hooks-guide.md
```

Record the returned commit ID in `progress.md`.

Verify all three paths are clean afterward.

## Step 12 — Full verification

Run:

```sh
cargo test --workspace
```

Then run the project quick check if time and environment permit:

```sh
just check
```

If `just check` duplicates the workspace tests plus WASM check, record each result accurately.

Run focused search:

```sh
rg -n 'capture-usage 2>/dev/null|capture-usage.*\|\| true' \
  crates/lisa-cli/src/templates.rs .lisa/hooks/on-stop.sh
```

Only the deliberately retained legacy hook may contain the old suffix.

## Step 13 — Repository hygiene

Run:

```sh
git status --short
git diff --cached --name-only
```

Confirm:

- all six ticket-owned source paths are clean;
- no ticket-owned source is staged;
- no ticket-owned source is untracked;
- unrelated Lisa runtime/ticket changes remain untouched;
- phase artifacts exist only in the attempt work directory.

## Step 14 — Progress artifact

Write `progress.md` with:

- red test evidence;
- implementation details;
- marker schema and reasons;
- CLI error behavior;
- hook and upgrade changes;
- documentation change;
- commit IDs;
- targeted and full test results;
- deviations, if any;
- final hygiene state.

## Step 15 — Review artifacts

Write `review.md` summarizing:

- exact modified files;
- behavior and design rationale;
- acceptance coverage;
- tests run and results;
- open concerns or limitations;
- confirmation that plugin attribution remains deferred;
- repository cleanliness.

Write `review-disposition.json` as exactly one valid shape.

Use pass only if all required behavior is implemented, committed, verified, and clean.

After both Review artifacts exist, remain on this ticket and stop. Do not publish, edit ticket state, or start another ticket.
