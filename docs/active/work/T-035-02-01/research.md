# Research: deterministic real-Zellij delivery boundary

## Ticket boundary

T-035-02-01 starts in Research and owns a committed, automated reproduction of the
fresh-pane delivery failure found by T-034-03-02.

The required proof is deliberately below the provider/model layer. It must use a local
stub, spend no model tokens, and run through a real Zellij terminal plus the real Lisa
WASM plugin.

The test must cover the successful two-stage lifecycle and three fault classes:

- process start is suppressed;
- chat acknowledgement is suppressed;
- the initial shell is left at a real `dquote>` continuation prompt.

Every fault must terminate within configured bounds without publishing Owned. The
`dquote>` case must recover and relaunch in the same physical terminal pane.

The ticket frontmatter is Lisa-owned and must not be edited by this attempt.

## Prerequisite state

All named dependencies are complete at the current base revision.

T-035-01-02 replaced long inline fresh commands with an attempt-private launch script.
The PTY receives only a bounded `sh <path>` indirection.

T-035-01-04 added a finite fresh-start deadline.

T-035-04-01 split fresh ownership into:

```text
Starting -> ReadyForAssignment -> Delivering -> Owned
```

T-035-04-02 added successor-lease rotation, Ctrl-C, positive shell readiness, and a
single same-pane relaunch for an unobserved original startup.

The current source revision includes the prerequisite implementation commits and their
Lisa completion commits.

## Production launch boundary

`crates/lisa-plugin/src/lib.rs` owns pane transport and scheduler state.

`prepare_fresh_launch` atomically writes the complete provider command below the exact
attempt directory. Its caller injects a short `sh <attempt-script>` line into the terminal.

`send_line_to_pane` writes characters immediately and queues Enter separately after a
small delay. This is the actual Zellij host boundary that native unit tests cannot run.

Fresh dispatch records `Starting` before submission. The start deadline is armed only
when the deferred Enter is flushed.

The launcher contains lifecycle environment including pane ID, ticket ID, attempt ID,
and provider selection. Assignment prose is absent.

For the Claude route the executable name is `claude`; therefore a directory placed first
on `PATH` can provide a deterministic local implementation without changing production
adapter code.

## Process-start boundary

The authoritative marker is `.lisa/signals/pane-<id>.lease`.

A native provider normally runs `.lisa/hooks/on-start.sh`, which verifies its immutable
environment against that marker and atomically copies it to `pane-<id>.started`.

The plugin scans `.started` files on its five-second poll.

`acknowledge_process_start` requires exact agreement among:

- the pane's Starting generation;
- the slot ticket and lease;
- the signal payload;
- the current authoritative lease.

A valid signal moves only to `ReadyForAssignment`. It cannot produce Owned.

The poll deliberately calls `deliver_ready_assignments` before consuming new start
signals. Readiness therefore remains visible for at least one complete poll boundary.

## Chat-delivery boundary

On the next poll, `deliver_assignment_to_pane` resolves the complete assignment file
under `.lisa/attempts/<ticket>/<generation>/work/assignment.md`.

Only a bounded reference is typed into the live provider composer:

```text
Read and follow the complete assignment at <path>.
LISA_ASSIGNMENT {"ticket_id":"...","generation":N}
```

The scheduler then records `Delivering` with an acknowledgement deadline.

The stub can read this line from its controlling terminal, record the exact received
bytes, and withhold acknowledgement until the external harness opens a gate. This makes
the Delivering state observable instead of racing immediately to Owned.

## Acknowledgement boundary

`.lisa/hooks/on-ack.sh` normally atomically stores the provider's complete
`UserPromptSubmit` JSON as `pane-<id>.ack`.

`codex_ack::detect_codex_ack` is provider-neutral at the normalized scheduler boundary.
It requires:

- valid JSON;
- `hook_event_name` equal to `UserPromptSubmit`;
- a prompt line beginning with `LISA_ASSIGNMENT `;
- the current ticket ID and current attempt generation.

Only an exact matching ack moves Delivering to Owned.

A local stub can construct the same normalized payload from the chat bytes it received.

## Bounded failure behavior

`assignment_ack_timeout_secs` is configurable and validated as at least one second.
The effective deadline also includes the deferred Enter delay.

An original Starting timeout enters same-pane recovery rather than ownership or immediate
spare consumption.

A Delivering timeout retries the identical bounded chat reference once. A second timeout
enters `DeliveryFailed` and marks the thread failed without Owned.

A replacement Starting timeout enters terminal startup failure. There is no second
same-pane relaunch.

The five-second plugin poll cadence means wall-clock harness bounds must allow multiple
polls even when the configured acknowledgement timeout is one second.

## Real `dquote>` construction

The regression should leave zsh itself, rather than a mock state machine, at its
continuation prompt.

The stub provider runs as the foreground process from the complete launch script. On the
first attempt in the `dquote` scenario it can schedule a short delayed Zellij input,
return successfully, and let both the launch script and outer `sh <path>` finish.

Once the parent zsh regains the foreground terminal, the delayed helper writes an
unterminated double quote plus Enter to the same pane. zsh then visibly enters `dquote>`.

The provider emits no `.started` file for that predecessor attempt.

At timeout Lisa sends Ctrl-C, then its successor-scoped shell probe. The probe can execute
only after zsh has escaped the continuation parser. Exact `pane-<id>.shell-ready` evidence
causes a single relaunch in the same pane under generation N+1.

The stub can key behavior by `LISA_ATTEMPT_ID`: fail attempt 1 into `dquote>`, then behave
normally for attempt 2.

## Zellij automation surfaces

The installed Zellij 0.44.3 CLI supports all required noninteractive controls:

- named sessions;
- `action list-panes --json` for stable terminal/plugin IDs;
- `action dump-screen --pane-id` for dashboard and terminal evidence;
- `action write-chars` and `action write` for deterministic input;
- `kill-session` for cleanup.

A client still needs a PTY while creating the session. The harness can run `lisa loop`
under the platform `script` utility in the background, then control the named session
through separate Zellij CLI calls.

The fixture and session name must be unique per scenario so parallel developer runs do
not collide.

## CLI and WASM path

`lisa loop` is preferable to a hand-written plugin layout because it exercises the normal
production bootstrap:

- verifies the fixture;
- uses the current Lisa executable;
- writes the embedded WASM to a content-hashed path;
- clears stale plugin cache;
- pregrants plugin permissions;
- generates the real two-agent-pane plus dashboard layout;
- exports the exact Lisa executable through plugin configuration.

The harness can use Cargo's `CARGO_BIN_EXE_lisa` when invoked by an ignored integration
test, ensuring the binary and embedded WASM come from the checkout being tested.

For direct shell execution, a `LISA_BIN` override can point to a freshly built binary.

## Isolated fixture

Each scenario needs a fresh canonicalized temporary Git repository.

The fixture requires `CLAUDE.md`, the workflow document, one minimal story, one Research
ticket routed to Claude, `.lisa.toml`, and runtime directories.

Using `lisa init` is the safest way to create hooks and configuration. The baseline must
be committed because Lisa's scheduler and completion commands assume a repository.

The local `claude` stub must implement `--version` so loop preflight accepts it.

The ticket never writes workflow artifacts, so no completion transaction or model-like
behavior is necessary. The harness terminates the session once the boundary assertion is
complete.

## Observation strategy

The stub should append machine-readable events under a scenario-owned evidence directory:

- launch with pane and generation;
- started signal publication;
- exact chat receipt;
- ack publication;
- forced `dquote>` injection.

Dashboard screen dumps supply the scheduler's rendered state labels. They prove that the
production WASM, not the stub's opinion, reports ReadyForAssignment, Delivering, Owned, or
the named terminal failure.

Terminal dumps prove the real `dquote>` prompt and later relaunch in that same pane.

The lease and event logs prove generation rotation and the absence of extra relaunches.

## Existing test organization

The workspace has one Cargo integration wrapper in
`crates/lisa-cli/tests/atomic_provider_contract.rs`. It invokes a retained shell harness
and asserts its completion receipt.

There is no existing real-Zellij automated test.

A new ignored Cargo integration test is appropriate because real Zellij, a WASM target,
and PTY support are environment requirements unsuited to every `cargo test --workspace`
run. The test remains automated and has a canonical explicit invocation.

The shell implementation should live under `crates/lisa-cli/tests/fixtures/` so it remains
stable source rather than depending on phase-artifact publication timing.

## Repository constraints

The shared worktree contains unrelated modified and untracked files, including ticket and
Lisa runtime metadata. They must remain untouched.

Ticket-owned source files must be committed with `lisa commit-ticket` and exact paths.

The attempt-private phase artifacts remain under
`.lisa/attempts/T-035-02-01/1/work/`; Lisa alone publishes them later.

## Research conclusion

No additional scheduler implementation is required by the current evidence.

The missing coverage is one external harness that replaces Claude through PATH, boots the
real embedded plugin in isolated named Zellij sessions, holds positive boundaries open for
observation, and drives the three finite faults.

The strongest implementation will combine a shell harness with an ignored Cargo wrapper,
assert dashboard state plus independent stub evidence, and clean every session and fixture
on success or failure.
