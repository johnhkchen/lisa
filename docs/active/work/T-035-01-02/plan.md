# Plan — T-035-01-02 atomic first-launch delivery

## Step 1 — establish shared shell quoting

Add `shell_quote` to `crates/lisa-plugin/src/lib.rs`. Use POSIX single-quote
encoding for all strings, including empty and simple strings.

Add focused table-driven tests for:

- empty and alphanumeric values;
- spaces and newlines;
- single and double quotes;
- `$()`, `${}`, backticks, globs, semicolons, and backslashes;
- tabs, carriage returns, and escape/control bytes;
- Unicode.

Verification: each result is one shell word and a harmless `sh` invocation
recovers the original bytes for every non-NUL case.

## Step 2 — make provider payloads shell-safe

Update `build_claude_command` to quote `LISA_BIN`, ticket ID, model, and prompt.
Keep literal environment names, executable, and flags unchanged.

Import the helper into `adapter.rs`. Update `CodexAdapter::model_flag` and
`interactive_line` to quote model, environment values, prompt, and relevant
dynamic paths. Remove the narrow double-quote replacement.

Adjust existing command-shape tests to the new quoting representation. Ensure
tests still prove:

- Claude uses `CLAUDE.md` and Codex uses `AGENTS.md`;
- routed models are present;
- `LISA_BIN`, pane, and ticket lifecycle values remain present;
- Codex assignment generation remains embedded;
- the Codex `.error` fallback remains present.

Verification: focused adapter and command-builder tests pass.

## Step 3 — implement atomic attempt-scoped preparation

Add `State::prepare_fresh_launch` in `lib.rs` near the existing delivery seam.

Implementation sequence:

1. Create the artifact directory.
2. Select `.lisa-launch-<pane>.sh` as final path.
3. Select a same-directory nonce temporary path.
4. Write shebang, complete payload, and newline to the temporary file.
5. Rename to the final path.
6. Remove the temporary path on rename failure.
7. Strip `/host/` for the pane shell.
8. Return a shell-quoted `sh <path>` launcher.

Errors must describe the failed operation/path and return before any pane API.

Add tests using `tempfile::TempDir`:

- small payload creates the exact script;
- a multi-hundred-kilobyte payload creates the exact script;
- returned launcher is identical for small and large payloads at one path;
- launcher contains neither payload;
- quote/control-heavy payload bytes are preserved;
- no `.tmp` entry remains after success;
- a file used where a directory is required produces `Err` and no final file.

Verification: preparation tests pass natively.

## Step 4 — route initial fresh dispatch through preparation

In `schedule_ready_tickets`, preserve the full adapter payload only until it is
prepared. For an empty pane:

- call `prepare_fresh_launch` with the current attempt directory;
- on success send the bounded launcher and mark the session as currently done by
  existing semantics (T-035-01-03 changes ownership later);
- on failure log, revoke/leave assignment consistently, and do not call the PTY
  seam or append `PendingEnter`.

For `ResetStrategy::FreshExec`, apply the same sequence.

For cross-provider recycling, prepare the incoming payload before sending
`/exit`. This avoids destroying the resident session when preparation is known
to have failed. Store/log the bounded launcher rather than full prompt payload.

Verification: scheduler tests show successful branches queue one Enter for the
launcher and failure branches queue none.

## Step 5 — route post-exit launch through preparation

In `check_transition_timeouts`, reconstruct the full adapter payload as today,
then atomically prepare it before calling `send_line_to_pane`.

On failure:

- log an error naming ticket and pane;
- do not queue Enter;
- do not set `has_session`;
- do not start assignment acknowledgement;
- leave transition state truthful for later recovery work.

On success, retain the existing state changes and log the bounded launcher in
`SessionLaunch`.

Update `test_recycle_exit_grace_launches_fresh_incoming_client` so its fixture
has a writable attempt directory and verifies the queued transport is bounded
indirectly through the generated script and pending Enter.

Add a failure fixture with an unwritable/invalid attempt path and assert no
pending Enter and no false session launch.

Verification: transition tests pass and preserve E-033 acknowledgement state.

## Step 6 — regression and compatibility checks

Run formatting:

```text
cargo fmt --all -- --check
```

If formatting changes are needed, run `cargo fmt --all`, inspect only the two
ticket-owned files, and ensure unrelated files were not altered.

Run focused tests:

```text
cargo test -p lisa-plugin shell_quote
cargo test -p lisa-plugin fresh_launch
cargo test -p lisa-plugin adapter::tests
cargo test -p lisa-plugin test_recycle_exit_grace
```

Run full verification:

```text
cargo test --workspace
cargo check -p lisa-plugin --target wasm32-wasip1
```

If the repository's `just check` adds material checks beyond those commands,
run it as well.

Inspect `git diff --check` and the exact source diff. Confirm no ticket/status
frontmatter edits were made by this work.

## Step 7 — commit the source unit

The quoting helper, both provider builders, atomic writer, and all fresh call
sites form one behaviorally meaningful transport unit across two files. Commit
them together through Lisa's isolated transaction:

```text
lisa commit-ticket --ticket-id T-035-01-02 \
  --message "fix(plugin): atomically prepare fresh launches" \
  --include crates/lisa-plugin/src/lib.rs \
  --include crates/lisa-plugin/src/adapter.rs
```

Do not stage or commit artifacts. Do not use ordinary Git index commands.

After the transaction, verify exact status. Ticket-owned source files must not
be staged, modified, or untracked. Existing Lisa/ticket mutations from the
scheduler are not this ticket's source ownership and must not be included.

## Step 8 — implementation and review artifacts

Maintain `progress.md` with completed steps, commands, results, deviations, and
the Lisa commit hash/result.

Write `review.md` after implementation. It must summarize source changes,
testing, acceptance coverage, the future real-Zellij proof boundary, and open
concerns such as shell portability or script lifecycle.

Remain on T-035-01-02 after `review.md`; Lisa owns phase publication, completion
commit, and seat release.

## Acceptance mapping

Bounded fresh command:
`prepare_fresh_launch` returns a path-only launcher, tested against widely
different payload lengths.

Full payload outside PTY:
the complete shell-safe provider command is written to an attempt-scoped file
and atomically renamed before delivery.

Safely addressed:
the final file path is shell-quoted and provider dynamic values use the same
single-argument encoder.

No Enter for partial transport:
no `send_line_to_pane` call occurs unless preparation returns success; failure
tests assert `pending_enters` remains empty.

Long/control/quote-heavy prompts:
unit tests preserve and execute representative payload/argument bytes.

Actual real-Zellij delivery:
the prepared boundary is consumable by T-035-02-01, which owns the integration
test per the story dependency graph.
