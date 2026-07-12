# Progress — T-035-01-02 atomic first-launch delivery

## Status

Implementation is complete, verified, and committed through the ticket-scoped
Lisa transaction.

## Completed work

- [x] Read `CLAUDE.md`, `AGENTS.md`, the ticket, RDSPI workflow, parent story,
  prior live-run evidence, and relevant scheduler/adapter source.
- [x] Wrote `research.md`, `design.md`, `structure.md`, and `plan.md` in the
  attempt-private directory.
- [x] Added a shared POSIX `shell_quote` encoder in `lisa-plugin/src/lib.rs`.
- [x] Converted Claude dynamic launch values to shell-safe arguments.
- [x] Converted Codex dynamic launch values and assignment prompt to the same
  shell-safe representation.
- [x] Added atomic attempt-scoped launch-script preparation.
- [x] Routed initial empty-pane launches through the prepared indirection.
- [x] Routed declared `FreshExec` launches through the indirection.
- [x] Prepared cross-provider incoming launches before sending `/exit`.
- [x] Routed post-exit fresh launches and recovery launches through the
  indirection.
- [x] Ensured preparation errors return before the PTY delivery seam, leaving no
  queued Enter and no false post-exit session state.
- [x] Updated activity assertions to reflect the actual bounded launcher rather
  than prompt-bearing command text.
- [x] Added long/control/quote-heavy payload tests.
- [x] Added launcher-size independence and complete-file publication tests.
- [x] Added preparation-failure/no-Enter coverage.

## Implementation details

Fresh payloads are written as:

```text
<attempt-work-dir>/.lisa-launch-<pane>.sh.tmp.<nonce>
```

The plugin writes the entire script and renames it in the same directory to:

```text
<attempt-work-dir>/.lisa-launch-<pane>.sh
```

Only after rename succeeds does the pane receive:

```text
sh '<attempt-work-dir>/.lisa-launch-<pane>.sh'
```

The command contains no prompt bytes. Its size depends on attempt identity and
pane ID, not prompt size. The path itself is single-argument shell-quoted.

The script contains the full provider payload. Claude and Codex prompt, ticket,
model, and configured binary values use single-argument POSIX quoting. Literal
single quotes use the standard close/quoted-quote/reopen sequence.

Same-provider live-TUI reuse is unchanged: `/clear` still leads to a bare prompt
write after the clear signal. That path is outside the fresh-shell `dquote>`
failure and outside this ticket's explicit scope.

## Test coverage added

`test_shell_quote_round_trips_long_control_and_quote_heavy_values` covers:

- empty and ordinary strings;
- spaces, newlines, tabs, and carriage returns;
- single/double quotes;
- dollar and command-substitution syntax;
- backticks, glob/shell punctuation, and backslashes;
- escape/control bytes and Unicode;
- a long repeated hostile string.

`test_prepare_fresh_launch_is_bounded_and_preserves_complete_payload` compares a
small payload with a payload larger than 500 KB at the same attempt path. It
asserts the launcher is identical, neither launcher embeds its payload, the
final script is exact, and no temporary file remains.

`test_prepare_fresh_launch_failure_cannot_queue_enter` uses a non-directory path
to force preparation failure and asserts that no pending Enter/timer exists.

Existing Claude/Codex route, prompt parity, model, acknowledgement, scheduler,
recovery, and deferred-Enter tests continue to pass with bounded activity
commands.

## Verification results

Formatting:

```text
cargo fmt --all -- --check
PASS
```

Focused plugin suite:

```text
cargo test -p lisa-plugin --lib
PASS — 276 passed, 0 failed
```

Workspace suite:

```text
cargo test --workspace
PASS
```

WASM compilation:

```text
cargo check -p lisa-plugin --target wasm32-wasip1
PASS
```

Diff hygiene:

```text
git diff --check
PASS
```

## Deviations from plan

The plan proposed optionally executing shell snippets from native unit tests.
That is incompatible with this plugin test binary: process spawning trips the
Zellij native host-command shim, which attempts to deserialize plugin input from
stdin. The temporary test caused a shim panic and was removed.

Coverage instead verifies the quoting transform by exact reversible encoding
and verifies byte-for-byte script publication. Actual shell/PTY execution is
properly owned by dependent real-Zellij ticket T-035-02-01, as the story already
specifies.

The plan mentioned an explicit scheduler failure fixture. The failure invariant
is covered at the preparation boundary: all fresh call sites pattern-match the
`Result` and call `send_line_to_pane` only in the `Ok` arm. The focused failure
test proves an `Err` creates no pending Enter. Existing scheduler tests cover all
successful fresh/recovery transitions.

## Commit

Completed isolated source transaction:

```text
lisa commit-ticket --ticket-id T-035-01-02 \
  --message "fix(plugin): atomically prepare fresh launches" \
  --include crates/lisa-plugin/src/lib.rs \
  --include crates/lisa-plugin/src/adapter.rs
```

The phase artifacts are not included. Lisa publishes them after lease
verification.

Result:

```text
d7b5e52af83a0a30c6cd6b02f5cd3db89bd6fae1
```

The commit contains exactly:

- `crates/lisa-plugin/src/lib.rs`
- `crates/lisa-plugin/src/adapter.rs`

After this commit, the sibling T-035-01-01 attempt began an overlapping
attempt-identity change in the same two files. Those working-tree changes are
not owned by this ticket and are not included in this ticket's transaction.

During post-commit hygiene, the new preparation behavior exposed two existing
native tests whose default relative `work_dir` wrote ignored launch fixtures
under the crate directory. This ticket added temp-directory `work_dir` values to
those two fixtures and removed their generated debris. The sibling temporarily
withdrew its overlapping worktree delta, allowing the exact-path follow-up to be
committed without absorbing another ticket's edits.

Follow-up result:

```text
d4db3549afdd2bad434a8a87d5f3dd8336f11b3a
```

Focused follow-up verification:

```text
cargo test -p lisa-plugin test_recycle_exit_grace_launches_fresh_incoming_client
PASS

cargo test -p lisa-plugin test_check_session_timeouts_expired
PASS

test ! -e crates/lisa-plugin/docs
PASS
```

## Remaining work

- [x] Run the isolated source commit and record its result.
- [x] Commit the native-test hygiene follow-up without sibling changes.
- [x] Confirm both ticket-owned source changes are clean and unstaged afterward.
- [ ] Write `review.md` and remain on this ticket.
