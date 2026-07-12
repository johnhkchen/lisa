# Design — T-035-01-02 atomic first-launch delivery

## Decision

Prepare each fresh provider launch as an atomically published shell script in
the current attempt's private work directory. Only after publication succeeds,
type a bounded `sh <quoted-script-path>` launcher into the pane and use the
existing deferred-Enter mechanism.

Quote every dynamic shell argument in the prepared payload with one shared
POSIX single-quote encoder. The PTY command then has no prompt content, and the
payload script preserves long, quote-heavy, and control-character prompt text.

## Goals

- The fresh-pane PTY text length is independent of prompt length.
- A shell never receives a prefix of the full provider command as its input.
- Enter is queued only after a complete payload is durably addressable.
- Claude and Codex retain their existing interactive launch behavior.
- The solution remains inside the plugin/adapter boundary and adds no daemon.
- Native unit tests can exercise the preparation and quoting contract.

## Non-goals

- Replacing prompt injection into an already-running TUI after `/clear`.
- Adding process-start acknowledgement or changing Owned transitions.
- Adding startup retry/recovery states.
- Building the real-Zellij regression harness assigned to T-035-02-01.
- Changing agent lifecycle hooks, CLI execution, or completion publication.

## Option A — chunk the PTY write

The scheduler could split the long command into small writes and queue Enter
after the final chunk.

This bounds each individual host call but does not acknowledge transport. Zellij
can still lose or truncate a chunk, and Lisa would still queue Enter because it
only knows it invoked the last write. A missing quote would still leave the
shell at `dquote>`. Chunking also introduces ordering and timer complexity.

Rejected: it reduces write size without making completion atomic or observable.

## Option B — encode the prompt into the PTY command

The prompt could be base64-encoded and decoded by a short shell pipeline.

Encoding avoids quote parsing, but the encoded command still grows with prompt
length and crosses the same PTY boundary. It directly violates the bounded-size
acceptance condition.

Rejected: shell safety improves, transport atomicity does not.

## Option C — host command prepares a prompt file, CLI reads it

The plugin could invoke a new CLI subcommand via Zellij `run_command`, wait for
`RunCommandResult`, then type a launcher that asks the CLI to read the prompt
file and `exec` the provider.

This provides explicit preparation acknowledgement and avoids a shell script.
It also expands the change into the CLI command surface, requires asynchronous
pending-launch state and callback attribution, and duplicates provider launch
configuration between plugin and CLI. The plugin already has filesystem access,
so a host subprocess is not required to establish atomic publication.

Viable but rejected as larger than the ticket boundary.

## Option D — atomically publish an attempt-scoped launch script

The plugin builds the provider command as it does today, but with robust shell
argument quoting. It writes `#!/bin/sh\n<command>\n` to a temporary file under
the current attempt directory, then renames that file to a fixed final name.
After rename succeeds it types `sh <quoted-relative-path>` into the pane.

The launcher's size depends on the attempt path, not the prompt. The attempt
path is already bounded scheduler identity and does not vary with prompt length.
The shell opens either the complete final file or no final file; it cannot see a
partially written payload because publication is a same-directory rename.

Chosen: it uses existing filesystem and atomic-write patterns, keeps provider
command ownership in adapters, and directly makes the PTY transport bounded.

## Payload file location and lifecycle

Use `.lisa-launch-<pane>.sh` inside the attempt work directory. Including the
pane ID distinguishes unusual cases where an attempt moves during recovery,
while the attempt directory distinguishes leases.

Production writes through the WASI `/host/...` path. The launcher uses the same
path after `strip_host_prefix`, interpreted from the project-root shell cwd.
Native tests can use a temporary absolute directory; the quoting helper handles
absolute paths as ordinary shell arguments.

The file is private ignored state, not a phase artifact. It is overwritten
atomically if a post-exit recovery reconstructs the same launch. No cleanup is
needed at dispatch because attempt directories are already Lisa-managed state.

## Atomic preparation algorithm

1. Create the attempt work directory.
2. Build the final `.lisa-launch-<pane>.sh` path.
3. Build a same-directory temporary filename with a time nonce.
4. Write the complete script bytes to the temporary path.
5. Rename the temporary path to the final path.
6. On rename failure, attempt to remove the temporary path and return an error.
7. Convert the final path to its host-shell form.
8. Return `sh <shell-quoted-final-path>`.

Only callers that receive `Ok(launcher)` may call `send_line_to_pane`. Therefore
the existing method's unconditional Enter queue is safe: it is queueing Enter
for the complete, short launcher, never for payload preparation.

## Shell quoting

Add one module-level `shell_quote` helper. It always returns a single-quoted
argument and replaces `'` with `'"'"'`. Always quoting, including simple values,
keeps the contract uniform and makes tests exact.

Use it for:

- Claude `LISA_BIN`, ticket ID, model, and prompt;
- Codex `LISA_BIN`, ticket ID, model, prompt, and error-signal path where useful;
- the launch script path in the bounded PTY command.

Environment assignments use `NAME=<quoted-value>`, which POSIX shells accept.
Provider flags remain literal trusted tokens.

The generated prompt is UTF-8. Newlines, tabs, carriage returns, escape bytes,
dollar expressions, backticks, double quotes, backslashes, and single quotes
remain literal inside the one shell argument. NUL is outside Unix argv semantics
and is not accepted as a supported prompt character.

## Scheduler integration

Introduce `State::prepare_fresh_launch(artifact_dir, pane_id, payload)` returning
the bounded launcher or a descriptive error.

At initial dispatch, prepare before any fresh shell write. For cross-provider
recycling, prepare before sending `/exit`; a preparation failure therefore
leaves the resident client untouched. For `FreshExec`, prepare before send.

At `WaitingForExit` completion, prepare before sending into the shell. On
failure, log an error and leave the state available for the later recovery work;
do not set `has_session`, start acknowledgement, or queue Enter.

Same-provider `/clear` and reuse prompt delivery are unchanged.

## Failure semantics

Directory creation, temporary write, or rename errors include the affected path
and are logged as `ActivityEvent::Error`. The caller must not call the PTY seam.

The atomic rename means a previous complete file could remain if replacement
fails. Since the launcher is not sent on error, that stale file is not executed.

Preparation does not prove provider start. T-035-01-03 consumes the separate
start signal to change ownership semantics. This design proves only that the
shell was given a complete reference to a complete payload.

## Test design

Unit-test shell quoting by executing or parsing representative values containing
spaces, both quote types, dollar/command substitutions, backticks, backslashes,
newlines, tabs, carriage returns, and escape/control bytes.

Unit-test preparation with a small and a very large payload. Assert:

- both returned launchers have identical length and content for the same path;
- neither launcher contains payload text;
- the final script contains the complete payload byte-for-byte;
- no temporary files remain after success.

Unit-test a quote/control-heavy provider command in the prepared script and run
it with `sh` against a harmless capture command where practical, proving the
argument arrives literally.

Unit-test failure with an invalid/uncreatable artifact directory and verify no
launcher is returned. Scheduler tests should retain the invariant that only a
successful preparation path appends a pending Enter.

Run focused plugin tests, then the whole workspace suite and WASM check because
filesystem behavior and Zellij compilation can differ from native tests.

## Compatibility

Provider commands still launch under `sh`, matching their existing shell-line
semantics. The prepared file adds one shell process that immediately runs the
provider command; without explicit `exec`, hook/process ancestry changes. The
payload should therefore use `exec` for the provider invocation where compatible,
while Codex's `||` error fallback requires a normal command followed by fallback.

The safer minimal choice is to preserve the existing command bodies exactly
apart from argument quoting and let the script shell wait for the provider. The
pane still presents the provider TUI, and after exit returns to the script shell,
which then exits to the original pane shell. Cross-provider grace already waits
for the TUI to return; one immediate shell exit is negligible but should be
covered by the future real-Zellij regression.
