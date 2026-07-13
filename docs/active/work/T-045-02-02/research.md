# Research — T-045-02-02 zellij-injects-launcher-only

## Ticket boundary

The ticket belongs to story `S-045-02`, the per-ticket launcher/argv slice.
Its direct dependency, `T-045-02-01`, is complete on the current branch.
That dependency added a native `lisa launch-codex` command.
This ticket connects that command to the plugin's Zellij pane transport.
The acceptance boundary is fixture/stub based.
Real Codex plus real Zellij validation belongs to a later story.
Claude launch and assignment delivery are explicitly outside this ticket.
Claim admission and ownership state changes are also outside this ticket.

## Repository state

The repository is a Rust workspace.
`crates/lisa-plugin` is the Zellij WASM plugin and scheduler.
`crates/lisa-cli` is the native host binary.
`crates/lisa-core` owns shared ticket, route, lease, and claim types.
The plugin cannot directly spawn native processes from WASM.
It describes a shell command and sends that description to a Zellij pane.
The native CLI can use `std::process::Command` once the pane invokes it.

The worktree contains unrelated Lisa runtime ledgers and materialized planning files.
Those files are not owned by this ticket.
The ordinary Git index is empty.
Ticket source changes must use `lisa commit-ticket` with exact includes.

## Assignment production

`crates/lisa-plugin/src/assignment.rs` owns immutable assignment publication.
`write_assignment` receives an attempt artifact directory, lease, nonce, and bytes.
It writes through `RustPublication` and a same-directory temporary.
The final filename is produced by `lisa_core::claim::assignment_file_name`.
Its shape is `assignment-<attempt>-<nonce>.md`.
The returned `AssignmentRef` retains the exact lease, nonce, and `PathBuf`.
Publication returns only after the complete final file is visible.

`State::prepare_assignment` in `crates/lisa-plugin/src/lib.rs` calls this writer.
It stores the returned reference in `State::assignment_refs`, keyed by ticket ID.
The map is the scheduler's retained identity for later delivery and claim checks.
Several launch paths currently discard the returned value after checking success.
The exact path is therefore available but not threaded into adapter launch construction.

## Assignment content

`ticket_prompt` in `crates/lisa-plugin/src/lib.rs` builds the full workflow instructions.
It names the ticket, provider-specific context file, workflow file, and attempt directory.
It requires all remaining RDSPI phases and exact Review disposition output.
For Review recovery it adds immediate recovery instructions.
The body can be materially larger than a shell command.
It is provider-neutral except for `CLAUDE.md` versus `AGENTS.md`.

`AgentAdapter::assignment_text` supplies this content per provider.
`ClaudeCodeAdapter` selects `CLAUDE.md`.
`CodexAdapter` selects `AGENTS.md`.
The content is written before any provider lifecycle input.

## Adapter interface

`crates/lisa-plugin/src/adapter.rs` defines `AgentAdapter`.
`launch_command(&SpawnContext)` returns a complete provider launch description.
`assignment_text(&SpawnContext)` returns the full durable instructions.
`reuse_prompt(&SpawnContext)` supplies same-process ticket delivery.
`assignment_reference` supplies a bounded Codex chat reference with an ack marker.

`SpawnContext` carries:

- ticket directory;
- ticket ID;
- pane ID;
- attempt ID;
- attempt artifact directory;
- optional assignment acknowledgment generation.

It does not carry the nonce-bearing assignment path.
The adapter therefore cannot currently name the exact assignment file at launch.

## Current Codex launch description

`CodexAdapter::interactive_line` builds a shell command string.
The line exports `LISA_BIN`, `LISA_AGENT_CLIENT`, `LISA_PANE_ID`,
`LISA_TICKET_ID`, and `LISA_ATTEMPT_ID` for lifecycle hooks.
It then directly invokes `codex` with full-access and hook-trust flags.
An optional routed model is appended as `--model <value>`.
Failure writes `.lisa/signals/pane-<id>.error`.

The line deliberately contains no assignment body today.
It also contains no assignment path.
Codex consequently starts at an empty interactive composer.
The scheduler later delivers assignment input as a separate pane write.

`CodexAdapter::reset_strategy` is `ExitThenFresh`.
A completed Codex TUI is not cleared and reused for another ticket.
The scheduler sends `/exit`, waits for a bounded shell-return grace, and launches again.
This already supplies the process-lifetime policy needed by the ticket.

## Prerequisite native launcher

`crates/lisa-cli/src/codex_launcher.rs` implements the native launcher.
`CodexLauncherArgs` carries an assignment `PathBuf`, Codex executable, and model.
`build_codex_argv` builds `Vec<OsString>`.
It supplies the two Codex safety flags as separate elements.
It supplies optional model flag and value as separate elements.
It adds `--` and then the exact assignment path as one final element.
It never converts the path into a shell-composed Codex command.

`run_codex_launcher` requires the assignment to be a regular file.
It starts the child with `Command::new(...).args(...).status()`.
Environment and terminal streams remain inherited.
The child exit status is returned to the CLI arm.

`crates/lisa-cli/src/main.rs` registers hidden `launch-codex` plumbing.
The command accepts positional assignment, `--codex-bin`, and optional `--model`.
The hidden command does not alter operator help.
`crates/lisa-cli/tests/codex_launcher.rs` captures the actual child argv.
Its hostile path fixture proves the final assignment path stays one element.

## Existing pane transport

`State::send_line_to_pane` is the common Zellij input boundary.
It calls `write_chars_to_pane_id` with the text.
It queues a raw Enter for two seconds later.
The delay avoids text and Enter coalescing in interactive TUIs.
It suppresses all injection while a pane is awaiting human input.

`State::prepare_fresh_launch` atomically writes a short shell script.
The script lives in the exact attempt work directory.
Its stable name is `.lisa-launch-<pane>.sh`.
The provided launch payload is the script body.
The pane receives only `sh '<path-to-script>'`.
The helper shell-quotes the script path and uses atomic publication.
Its tests prove pane input stays invariant even for a 500 KB payload.

Thus there are two command boundaries:

1. Zellij injects the bounded `sh <script>` launcher line.
2. The script invokes native provider plumbing.

The full assignment is already absent from the first boundary.
The current later chat-delivery path is where assignment-related text reaches the TUI.

## Primary scheduling path

`State::schedule_ready_tickets` resolves an adapter and route per ticket.
It mints and publishes an attempt lease.
It creates the attempt artifact directory.
It builds `SpawnContext`.
It persists `adapter.assignment_text` before pane lifecycle actions.
It then selects fresh, reused, or recycled seat behavior.

For a fresh pane it builds `adapter.launch_command`.
It passes that payload to `prepare_fresh_launch`.
It injects the returned bounded script invocation.
For a resident Codex pane, `ExitThenFresh` first injects `/exit`.
The future launcher payload is prepared before the old process exits.
After the exit timeout, the scheduler sends the prepared launch.

The resulting seat is `Starting` for every fresh provider launch.
Codex readiness mode is `Grace` because its SessionStart hook follows first prompt creation.
Claude readiness mode is `SessionStart` because it has pre-prompt evidence.

## Existing fresh-Codex assignment delivery

For grace-mode Codex, the startup deadline expires after `STARTUP_GRACE_SECS`.
The state advances toward `ReadyForAssignment`.
`deliver_ready_assignments` calls `deliver_assignment_to_pane`.
That function loads the retained `AssignmentRef` and validates its lease and file.
It builds `adapter.assignment_reference`.
The reference says to read the complete assignment at the exact path.
It appends `LISA_ASSIGNMENT` ticket/generation identity.
It calls `send_line_to_pane`, placing that text into the live composer.

This is the delivery shape the ticket replaces for a fresh Codex TUI.
If left active after launcher integration, Codex would receive an initial path prompt
and later a second assignment-reference prompt.
The acceptance criterion excludes that duplicate pane input.

## Recovery and relaunch paths

`acknowledge_shell_ready` relaunches after a bounded same-pane startup reset.
It mints or receives the successor attempt, republishes assignment bytes,
rebuilds a fresh launch payload, and injects the script invocation.
It must use the successor assignment reference rather than a predecessor path.

`check_transition_timeouts` handles an old interactive client returning to its shell.
It republishes the pane lease marker and assignment for the current attempt.
It builds and injects the fresh provider launch.
This is the normal second-ticket Codex path because Codex uses `ExitThenFresh`.
The exact newly returned assignment path is available at this point.

The same transition code can also operate on Claude routes.
Any interface change must preserve Claude's existing command bytes and reset handshake.

## Existing tests and fixtures

`adapter.rs` unit tests pin Codex launch environment, flags, optional model,
error marker, and lack of assignment text.
They currently expect direct `codex` text in the launch line.
Routing tests also inspect the Codex command string.
Claude adapter tests pin its existing launch output.

`lib.rs` has extensive native scheduler tests.
Native builds replace Zellij host functions with no-op stubs.
Tests therefore normally inspect queued Enter state, activity events,
seat states, retained assignment references, and written launch scripts.
`test_prepare_fresh_launch_is_bounded_and_preserves_complete_payload` proves
the script indirection itself does not leak its body into pane input.

Scheduler tests around `T-NAME` inspect fresh launch and assignment retention.
Startup recovery tests inspect successor assignment refs and launch scripts.
The repository does not currently expose a generic pane-write capture in `State`.
A ticket fixture can instead model the stub pane by reading the returned injected line
and the atomically written script, which are the two values passed to Zellij.

## Constraints surfaced

The exact assignment path can contain spaces, quotes, or shell metacharacters.
The plugin must shell-quote it while invoking `lisa launch-codex` from a script.
The native launcher then preserves it as one Codex argv element.
The plugin runs in a `/host`-mounted path environment under Zellij.
Paths passed to pane-side commands use `strip_host_prefix`.
The stored host-side `AssignmentRef.path` must therefore be translated before launch.

The model value is also dynamic and must remain shell-quoted in the script.
The resolved Lisa binary may be absolute or the fallback bare `lisa`.
The lifecycle variables must remain on the launcher invocation so Codex inherits them.
The `.error` fallback must remain so native launcher or child failure is visible.

The ticket cannot depend on the future claim consumer to make its fixture pass.
It also cannot broadly replace `SeatAssignmentState` because that is the next story.
The smallest observable contract is the command constructed for each fresh Codex launch
and the absence of the assignment body from injected pane text.

## Research conclusion

All required inputs already exist at the scheduler boundary:
the resolved Lisa binary, routed model, lifecycle identity, exact published assignment,
attempt-specific directory, and fresh-process reset strategy.
The missing connection is explicit assignment-path flow into `launch_command`.
Every fresh or recovery launch site already publishes the assignment immediately before
building the command, so no new storage or discovery mechanism is required.
The affected source boundary is concentrated in `adapter.rs` and the launch call sites
and fixture tests in `lib.rs`.
