# Structure — T-035-01-02 atomic first-launch delivery

## Source files

### `crates/lisa-plugin/src/lib.rs`

Modify this file. It remains the owner of shared prompt construction, Claude
launch construction, scheduler delivery, attempt paths, and the PTY Enter queue.

Add a private module-level function:

```rust
pub(crate) fn shell_quote(value: &str) -> String
```

It returns one POSIX-shell argument using single-quote encoding. Visibility is
`pub(crate)` so `adapter.rs` can share it without creating a new module for one
small transport primitive.

Update `build_claude_command` to quote every dynamic shell value. Preserve the
function signature so adapter and tests do not need a public contract change.

Add a private state method conceptually shaped as:

```rust
fn prepare_fresh_launch(
    &self,
    artifact_dir: &Path,
    pane_id: u32,
    payload: &str,
) -> Result<String, String>
```

`artifact_dir` is the filesystem path visible to the plugin, normally the
attempt work directory under `/host`. The method owns directory creation,
temporary-file construction, complete script write, same-directory rename,
cleanup on error, host-path conversion, and bounded launcher construction.

The final filename is `.lisa-launch-<pane-id>.sh`. The temporary filename adds
a nanosecond nonce. The file content is `#!/bin/sh`, the complete adapter
payload, and a final newline.

Integrate the method into fresh command call sites:

- initial empty-pane dispatch;
- `ResetStrategy::FreshExec` dispatch;
- precompute before cross-provider `/exit`;
- post-exit `WaitingForExit` launch/recovery.

At each site, only pass the returned launcher to `send_line_to_pane`. Errors are
logged and the scheduling branch exits without a fresh PTY write. Existing
same-provider reuse paths continue to pass bare prompts directly.

Activity events and local `launch_cmd` values should contain the bounded
launcher rather than the payload. This keeps dashboard/log behavior aligned
with what was actually typed into the pane.

Add unit tests in the existing `#[cfg(test)] mod tests` for:

- exact shell-quote handling of simple and hostile values;
- prompt-length independence of the returned launcher;
- complete on-disk payload for long/control/quote-heavy strings;
- shell execution preserving a quote/control-heavy argument;
- preparation failure returning `Err` without a final launch file;
- scheduler failure not appending a pending Enter where a convenient state
  fixture can exercise the branch without Zellij host calls.

Update existing Claude command assertions for the new single-quoted shape.

### `crates/lisa-plugin/src/adapter.rs`

Modify this file only for shared shell-safe command construction.

Import `shell_quote` from the crate root alongside existing prompt helpers.

Replace Codex's ad hoc double-quote replacement with `shell_quote` for the full
assignment prompt. Quote dynamic `LISA_BIN`, ticket ID, and model values. Preserve
literal provider flags and the `.error` fallback behavior.

Update adapter tests so they assert semantic contents in the single-quoted
command rather than the old double-quoted/escaped JSON spelling. Add explicit
quote-heavy prompt coverage if the lower-level helper tests do not reach the
Codex builder.

## Artifact files

All workflow artifacts remain private to this attempt:

- `.lisa/attempts/T-035-01-02/2/work/research.md`
- `.lisa/attempts/T-035-01-02/2/work/design.md`
- `.lisa/attempts/T-035-01-02/2/work/structure.md`
- `.lisa/attempts/T-035-01-02/2/work/plan.md`
- `.lisa/attempts/T-035-01-02/2/work/progress.md`
- `.lisa/attempts/T-035-01-02/2/work/review.md`

These are not source commit includes; Lisa publishes admitted artifacts.

## Unchanged files and boundaries

`crates/lisa-cli/src/agent_exec.rs` is unchanged. The sibling T-035-01-01 owns
start-signal production there.

`crates/lisa-core` is unchanged. No public types or serialization formats are
required for the transport primitive.

Generated hooks and templates are unchanged. Process start acknowledgement is
not part of payload preparation.

Ticket and story frontmatter are unchanged. Lisa observes artifacts and manages
phase transitions.

No real-Zellij harness is added. T-035-02-01 owns that integration artifact.

## Data flow after the change

```text
ticket + route + lease
        |
        v
adapter.launch_command(ctx)
  full shell-safe provider payload
        |
        v
State::prepare_fresh_launch(attempt_dir, pane, payload)
  temp write -> atomic rename
        |
        v
bounded `sh '<attempt launch file>'`
        |
        v
send_line_to_pane -> deferred Enter
        |
        v
pane shell opens complete script -> provider process
```

Preparation error terminates the flow before `send_line_to_pane`, so there is
no `PendingEnter` and no partial command at the shell.

## Ordering

1. Add and test `shell_quote`.
2. Convert Claude and Codex command builders to it.
3. Add atomic preparation helper and focused filesystem tests.
4. Route all fresh launch call sites through the helper.
5. Adjust scheduler tests and activity assertions.
6. Run formatting, focused tests, workspace tests, and WASM check.
7. Commit the meaningful source unit through one exact-path Lisa transaction if
   both source files form one inseparable transport unit; otherwise commit the
   quoting builder change and scheduler preparation change separately with exact
   includes and verify no ticket-owned file remains dirty.

## Interface invariants

- `AgentAdapter::launch_command` still returns a complete provider shell payload.
- `send_line_to_pane` still writes its argument then queues delayed Enter.
- Fresh launch callers must pass only a prepared launcher to that method.
- Reuse callers may still pass TUI commands/prompts directly.
- `prepare_fresh_launch` returns only after final-file rename succeeds.
- The returned command never includes payload bytes.
- Launcher size is a function of attempt identity/path, not prompt length.
- A launch preparation error is recoverable state, never a reason to submit.

## Commit ownership

The likely ticket-owned source unit is:

```text
crates/lisa-plugin/src/lib.rs
crates/lisa-plugin/src/adapter.rs
```

They must be committed with:

```text
lisa commit-ticket --ticket-id T-035-01-02 --message <message> \
  --include crates/lisa-plugin/src/lib.rs \
  --include crates/lisa-plugin/src/adapter.rs
```

No broad include, ordinary `git add`, or ordinary `git commit` is permitted.
