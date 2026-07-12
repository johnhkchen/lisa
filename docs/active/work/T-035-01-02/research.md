# Research — T-035-01-02 atomic first-launch delivery

## Ticket boundary

The ticket addresses delivery of a fresh provider launch through a shell pane.
The PTY write must remain bounded as the ticket prompt grows, the complete
payload must be prepared outside that write, and Enter must never be queued for
an incomplete preparation.

The parent story assigns this ticket the `crates/lisa-plugin/src/lib.rs`
delivery seam. The sibling start-signal ticket owns `agent_exec.rs` and generated
hooks. Later tickets consume both changes to gate ownership and add bounded
recovery. The deterministic real-Zellij regression belongs to T-035-02-01.

## Prompt construction

`crates/lisa-plugin/src/lib.rs::ticket_prompt` constructs the common RDSPI
instruction text. It discovers the descriptive ticket filename, selects the
provider-specific context filename supplied by the adapter, and embeds the
attempt-private artifact directory.

The prompt is intentionally shared between providers. Its length varies with
ticket paths, attempt paths, and workflow wording. Nothing here bounds it.

`build_claude_command` creates a complete shell line. It adds `LISA_BIN` when
configured, pane and ticket environment variables, the optional model flag,
the Claude executable and permission flag, and the entire prompt in a
double-quoted shell argument.

`crates/lisa-plugin/src/adapter.rs::CodexAdapter::interactive_line` does the
equivalent for Codex. It includes lifecycle environment, bypass flags, optional
model routing, the complete assignment prompt, and an error-signal fallback.
It escapes JSON double quotes in the assignment marker, but does not generally
quote arbitrary prompt characters for the shell.

Both adapters expose the string through `AgentAdapter::launch_command`. The
scheduler receives one string whose size is proportional to the prompt.

## Scheduler delivery paths

`State::schedule_ready_tickets` has three assignment shapes:

1. Cross-provider recycling sends `/exit`, records `WaitingForExit`, and retains
   the incoming fresh launch command.
2. Same-provider reuse sends `/clear`, waits for `.cleared`, then sends the bare
   prompt into the already-running provider TUI.
3. A truly fresh pane immediately sends the adapter launch command to its shell.

`State::check_transition_timeouts` completes cross-provider recycling after the
exit grace period. It reconstructs the adapter and `SpawnContext`, builds the
fresh launch command, and sends it into the returned shell.

The `ResetStrategy::FreshExec` branch also sends a fresh shell command directly.
Native Claude and Codex currently use `ClearHandshake`, but this branch shares
fresh-command semantics.

The `.cleared` signal and clear-timeout paths send `reuse_prompt`, not a shell
command. Those writes can be prompt-sized, but they target a live TUI composer
rather than the shell parser involved in the observed `dquote>` failure. The
ticket and parent story limit this change to fresh launch.

## PTY write and Enter scheduling

`State::send_line_to_pane` is the text-plus-submit seam. It first refuses to
inject into a pane marked as awaiting human input. Otherwise it calls Zellij's
`write_chars_to_pane_id(text, pane_id)` once.

After that host call returns, it unconditionally appends a `PendingEnter` with
an absolute deadline two seconds in the future and arms a timer. There is no
acknowledgement that all characters crossed the PTY.

`State::flush_pending_enters` later emits byte 13 for every due entry. Absolute
deadlines prevent unrelated timers from submitting early, which is T-029-02's
timing guarantee, but do not prove the preceding large write was complete.

The T-034-03-02 real run captured both provider commands stopping inside the
double-quoted prompt at a zsh `dquote>` continuation prompt. No provider process
had launched. Evidence is in
`docs/active/work/T-034-03-02/evidence/live-run.md`.

## Available indirection boundary

The plugin already performs host-mounted filesystem I/O with `std::fs`.
Attempt artifacts normally live under
`/host/.lisa/attempts/<ticket>/<attempt>/work`, provided by
`State::attempt_work_dir`.

The module has an established atomic-publication pattern:

- create the destination directory;
- write a uniquely named temporary file in that directory;
- rename the temporary file to the final path;
- remove the temporary file if rename fails.

Lease markers and admitted artifacts use this pattern. Same-directory rename
prevents readers from seeing partial file contents.

`strip_host_prefix` converts a WASI `/host/...` path to the path used by a shell
running from the project root. The private attempt directory is scheduler state,
so a per-attempt launch file is lease-scoped and cannot collide across attempts.

## Shell parsing constraints

The current prompt wrappers use double quotes. Within double quotes, shells can
expand dollar signs, command substitutions, backticks, and backslash sequences.
An embedded quote can terminate the argument. Escaping only double quotes is
therefore insufficient for arbitrary prompt text.

POSIX single-argument quoting can represent ordinary UTF-8 and control
characters by surrounding the value with single quotes and replacing every
literal single quote with `'"'"'`. This prevents parameter, command, glob, and
whitespace expansion. NUL cannot be represented in a Unix argument and is not
present in Lisa's generated text.

Other dynamic values cross the shell boundary: `LISA_BIN`, ticket ID, model,
and the launch-file path. Applying the same quoting rule avoids a second
fragile boundary.

## Existing tests and observability

Adapter tests assert that launch commands contain the prompt, provider context
files are selected, model flags flow, assignment markers survive, and the Codex
error fallback is present.

`lib.rs` tests cover Claude command shape and scheduler state. The recycle test
observes one pending Enter after post-exit launch. Native tests cannot invoke
Zellij's host write, so effects are observed through `pending_enters` and state.

There are no tests for command-length independence, atomic preparation failure,
or long/control/quote-heavy payloads. No test asserts that failed preparation
leaves `pending_enters` empty.

The activity log stores `ActivityEvent::SessionLaunch { command }`. Today that
can retain the full prompt. A bounded launcher also makes this event smaller.

## Constraints

- Source changes must use `lisa commit-ticket` with exact include paths.
- Ticket phase/status frontmatter is scheduler-owned.
- Preparation must succeed before `send_line_to_pane` is called.
- Failure must be visible and must not queue Enter or report a launch.
- Both providers must retain flags, routing, hook environment, attempt prompt,
  and Codex error-signal behavior.
- Recycled Codex acknowledgement tagging must remain in the payload.
- Deferred-Enter timing remains intact for the short launcher and reuse prompts.
- T-035-02-01 remains responsible for the real-Zellij proof; this ticket supplies
  focused native regression coverage.
