# Research — T-045-02-01 launcher argv construction

## Ticket boundary

T-045-02-01 asks for a Lisa-owned launcher that starts interactive Codex with an
attempt-private assignment path.
The required observable is process argv, not pane rendering or scheduler state.
An assignment path containing shell metacharacters and quotes must reach Codex as one
unchanged argument.
The ticket is the first half of story S-045-02.
T-045-02-02 owns changing Zellij pane injection to invoke this launcher.
Later stories own claim evidence, ownership transitions, timeout behavior, provider
exit, nonce revocation, authoritative completion, and live Codex/Zellij validation.
Claude launch behavior is outside this ticket.

## Dependency contract

T-045-01-01 introduced `crates/lisa-plugin/src/assignment.rs`.
`write_assignment` publishes complete bytes under the exact attempt work directory.
The durable leaf has the form:

`assignment-{attempt_id}-{nonce}.md`

The writer returns `AssignmentRef` containing the full `AttemptLease`, nonce, and
durable `PathBuf`.
Publication writes a hidden sibling temporary and atomically renames it.
The path becomes visible to scheduler state only after publication succeeds.
The assignment writer does not launch a process or invoke a shell.

T-045-01-02 has introduced shared assignment claim vocabulary in
`crates/lisa-core/src/claim.rs`.
The shared `assignment_file_name` helper preserves full numeric attempt and nonce
identity.
That work is adjacent but does not define a Codex launch API.

## Existing Codex launch construction

`crates/lisa-plugin/src/adapter.rs` contains `CodexAdapter`.
The adapter currently owns the interactive Codex shell line.
`CodexAdapter::interactive_line` returns one formatted `String`.
The line exports lifecycle identity through shell environment assignments:

- `LISA_BIN`;
- `LISA_AGENT_CLIENT=codex`;
- `LISA_PANE_ID`;
- `LISA_TICKET_ID`;
- `LISA_ATTEMPT_ID`.

The line then invokes `codex` with:

- `--dangerously-bypass-approvals-and-sandbox`;
- `--dangerously-bypass-hook-trust`;
- optional `--model <model>`.

It appends a shell `||` fallback that writes a pane error signal.
Every dynamic string is passed through the repository's `shell_quote` helper.
The existing line contains no initial positional prompt.
The assignment is currently delivered later through the live TUI composer.

The adapter boundary is WASM-resident.
Its module documentation states that adapters describe commands because the plugin
cannot directly pipe to or spawn host subprocesses.
Therefore a no-shell `std::process::Command` boundary cannot live in the plugin.

## Existing fresh-launch transport

`crates/lisa-plugin/src/lib.rs` owns `State::prepare_fresh_launch`.
It writes the adapter's full shell payload to an attempt-private script:

`.lisa-launch-{pane_id}.sh`

The script is atomically published through `RustPublication`.
The pane receives only `sh <quoted-script-path>`.
This keeps pane input bounded independently of assignment size.
It does not remove shell interpretation inside the published script.

Fresh launch preparation is used by normal dispatch, same-pane startup recovery,
provider recycling, and relaunch paths.
Those scheduler call sites currently consume `AgentAdapter::launch_command` as a
string.
Changing those call sites is explicitly assigned to T-045-02-02.

## Existing assignment delivery

`AgentAdapter::assignment_text` creates the complete workflow instructions.
For Codex, `ticket_prompt` names `AGENTS.md`, the ticket, workflow definition, and the
attempt-private artifact directory.
The scheduler publishes that complete text with `write_assignment`.
It retains the exact resulting `AssignmentRef` in `State::assignment_refs`.

`AgentAdapter::assignment_reference` can build a bounded chat instruction referring
to an arbitrary `Path`.
Current Codex delivery uses that bounded reference only after provider startup pacing.
The new story changes the initial process transport, but this ticket does not edit
delivery state or acknowledgement behavior.

## CLI host boundary

`crates/lisa-cli/src/main.rs` is the native `lisa` binary.
It uses Clap derive with one `Commands` enum and a direct match in `main`.
Machine-oriented commands such as `agent-exec`, `capture-usage`, `commit-ticket`, and
`complete-ticket` are hidden from Clap's ordinary command list.
The top-level help contains a curated plumbing footer.

`crates/lisa-cli/src/lib.rs` exposes reusable native boundaries.
At present it exports `commit_transaction` and conditionally exposes
`capture_usage` for test support.
Most CLI implementation modules are private to the binary.

`crates/lisa-cli/src/agent_exec.rs` is the closest process-launch precedent.
It builds a `Vec<String>` argv separately from execution.
It calls `Command::new(&args.codex_bin).args(&argv)`.
The command inherits environment, explicitly configures stdio, and reports spawn or
wait failures with contextual messages.
Its mode is headless `codex exec`, so its argv and outcome semantics do not directly
serve the interactive launcher.

`crates/lisa-cli/src/doctor.rs`, `init.rs`, and `templates.rs` also use
`std::process::Command` directly.
No general subprocess abstraction exists in the CLI crate.

## Current interactive Codex surface

The installed command is `codex-cli 0.144.3`.
`codex --help` reports the interactive form as:

`codex [OPTIONS] [PROMPT]`

The positional prompt is optional and starts the session when supplied.
Relevant current top-level options include:

- `--model <MODEL>`;
- `--dangerously-bypass-approvals-and-sandbox`;
- `--dangerously-bypass-hook-trust`;
- `--cd <DIR>`;
- `--ask-for-approval <POLICY>`.

The current adapter already uses the first three applicable options.
The assignment file path can occupy the one positional prompt slot.
A conventional `--` separator can prevent a path beginning with a hyphen from being
interpreted as an option.

The OpenAI Codex manual helper was attempted as required for current product
behavior, but its response lacked the required `x-content-sha256` integrity header.
An official-doc search did not surface a more specific interactive argv reference.
The installed CLI's own help is therefore the concrete version-matched evidence used
for this repository boundary.

## Path and argument types

Rust `PathBuf` and `OsString` preserve native operating-system argument boundaries.
`Command::arg(path.as_os_str())` passes one argument directly to the child process.
It does not run a shell, perform variable expansion, split whitespace, evaluate
quotes, execute command substitutions, or apply globbing.

Converting a path into a shell command string would reintroduce an interpretation
layer.
Converting through lossy UTF-8 can also alter a valid Unix path.
The ticket's hostile-path examples are representable as UTF-8, but the native command
API can preserve the broader `OsStr` contract without extra dependencies.

Environment inheritance is the existing mechanism for lifecycle identity.
A child created with `Command::new` inherits the launcher's environment unless it is
cleared or overridden.
Thus the existing `LISA_*` values can cross a native launcher without being rebuilt
as shell assignments.

## Exit and terminal behavior

An interactive launcher must preserve inherited stdin, stdout, and stderr so Codex
continues to own the pane terminal.
`Command::status` inherits all three streams by default and waits for the child.
The returned `ExitStatus` contains a platform exit code when the child exits normally.
Spawn failure and wait failure are distinct native I/O errors.

The existing plugin shell line writes a pane error signal on nonzero Codex exit.
That fallback belongs to the current injected script.
This ticket's acceptance criterion only requires the launcher to spawn exact argv;
the next ticket owns how the launcher invocation participates in pane recovery.

## Test conventions

CLI integration tests live under `crates/lisa-cli/tests`.
They use `env!("CARGO_BIN_EXE_lisa")` to exercise the built binary.
`tempfile` is already a CLI dev dependency.
Tests commonly create temporary project or fixture directories and use
`std::process::Command` for black-box invocation.

An argv-capture executable can be represented by a temporary Unix shell stub.
The stub receives arguments from the Rust launcher through the operating system,
quotes `"$@"`, and writes one record per argument.
The shell is only fixture implementation; the launcher under test does not use it to
compose or interpret Codex argv.
NUL-delimited capture avoids ambiguity from spaces, quotes, and newlines.

The hostile assignment path should include characters that a shell would otherwise
interpret, including spaces, single and double quotes, dollar syntax, semicolons,
parentheses, brackets, and backticks.
The test can create a real file at that path and assert the captured final argument is
byte-for-byte equal to the original path.
It can also assert the surrounding fixed flags and optional model occupy separate
arguments.

## Repository and concurrency constraints

The worktree contains Lisa-managed and concurrent ticket changes.
The dependency's core claim module is already committed at the current HEAD.
Other T-045-01-02 edits may appear while this ticket runs.
Ticket work must use exact `lisa commit-ticket --include` paths.
Ordinary `git add` and `git commit` are prohibited.
Unrelated modified or untracked files must remain untouched.

The smallest ticket-owned source surface is a CLI launcher module, its command wiring,
and a focused argv-capture integration test.
No crate dependency is needed.
No plugin scheduler, adapter, assignment writer, claim state, or documentation
publication path must change to establish the requested process-argv fact.
