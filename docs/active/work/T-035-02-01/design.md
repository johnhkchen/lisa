# Design: real-Zellij stub-provider regression

## Objective

Add a deterministic, model-free regression that drives Lisa's production WASM through
the two actual terminal delivery stages and proves ownership is published only after an
exact matching chat acknowledgement.

The regression must also prove finite negative behavior for missing start, missing ack,
and a real zsh `dquote>` startup fault.

## Non-goals

This ticket will not change scheduler states, timeout policy, adapters, hook formats,
lease representation, dashboard rendering, or production configuration.

It will not invoke installed Claude or Codex and will not spend provider tokens.

It will not turn the normal workspace suite into a Zellij-dependent test.

It will not use a hand-written copy of the scheduler state machine as evidence.

## Option 1: native Rust state tests only

The existing prerequisite tests already exercise private State methods and exact leases.
Extending those tests would be fast, deterministic, and CI-friendly.

This option cannot execute Zellij's `write_chars_to_pane_id`, delayed Enter, terminal
foreground process behavior, shell parsing, or real pane identity.

It would reproduce intent rather than the T-034-03-02 failure boundary.

Rejected because the ticket explicitly requires an isolated real-Zellij test.

## Option 2: hand-written Zellij layout loading the WASM directly

The harness could build `lisa.wasm`, create its own KDL layout, pregrant permissions, and
start Zellij without the Lisa CLI.

This offers exact control and removes CLI initialization work.

It also duplicates production bootstrap details: plugin cache busting, permission cache
keys, host paths, pane count, configuration serialization, and the embedded-WASM path.

A test could pass while `lisa loop` is broken or could fail because its copied layout
drifted.

Rejected in favor of testing the normal CLI bootstrap.

## Option 3: run `lisa loop` with a PATH-injected provider stub

The harness creates a real fixture, puts a local executable named `claude` first on PATH,
and runs the checkout's Lisa binary under a named Zellij session.

Production code still owns:

- launch script creation;
- bounded `sh <path>` terminal injection;
- pane input and deferred Enter;
- lease marker publication;
- start-signal scanning;
- bounded assignment reference generation;
- chat delivery;
- ack detection;
- timeout/recovery transitions;
- dashboard status rendering.

The stub owns only external provider behavior: when to publish normalized start/ack
signals and how to hold or fault its composer.

Chosen because it tests the narrow real boundary with minimal production duplication.

## Option 4: pseudo-terminal test without Zellij

A test could spawn zsh under a PTY library and emulate Lisa's writes.

That would test shell quoting and foreground process behavior, but not Zellij plugin host
calls, pane discovery, plugin timers, or dashboard state.

Rejected because Zellij delivery was the original field failure boundary.

## Test packaging decision

Add a shell harness under `crates/lisa-cli/tests/fixtures/` and a Rust integration wrapper
under `crates/lisa-cli/tests/`.

The Rust test is marked ignored with a precise reason. It is run explicitly with:

```text
cargo test -p lisa-cli --test real_zellij_delivery_boundary -- --ignored --nocapture
```

Ignoring is a dependency boundary, not a manual test design. Once invoked, the complete
test is automated and fails on any unmet assertion.

This keeps `cargo test --workspace` portable on machines without Zellij, zsh, `script`, or
the WASM target while giving maintainers one stable command.

## Harness process model

The Rust wrapper resolves `CARGO_BIN_EXE_lisa` and invokes the shell harness with it in
`LISA_BIN`.

The harness checks required commands and that `LISA_BIN` is executable.

For each scenario it creates an independent temporary root, initializes an isolated Git
repository, and writes one ticket.

It starts `lisa loop` under `script` so the Zellij client has a PTY. `ZELLIJ_SESSION_NAME`
and an explicit unique session argument make out-of-band actions deterministic.

Cleanup traps kill every named session and remove all temporary roots.

## Fixture initialization

Use the tested `lisa init --path <root>` command rather than copying hook templates.

After initialization, write a minimal `.lisa.toml` with:

- `max_threads = 1`;
- `assignment_ack_timeout_secs = 1`;
- `auto_advance = false`;
- Claude as the client.

Create one story and one Research-phase ticket with `agent: claude`.

Commit the fixture baseline using ordinary Git only inside the disposable repository.
This does not touch the parent ticket transaction.

## Stub protocol

The provider stub is one generated executable shared by scenarios through environment
variables.

For `--version`, it prints a stable fake version and exits zero so loop preflight passes.

For a normal launch it records:

```text
launch pane=<P> generation=<N>
```

It copies the exact pane lease to `.started` atomically unless the scenario suppresses
start.

It then reads submitted terminal lines. Each bounded assignment is appended to an event
log with its marker intact.

When acknowledgement is enabled, the stub waits for a harness-created gate file, encodes
the received prompt as JSON, and atomically publishes `.ack`.

The stub remains alive after ack so Lisa observes a resident provider and does not fall
back into shell behavior during the assertion window.

## Success scenario

The harness waits for the start event and exact `.started` consumption.

Before the next delivery poll, it dumps the dashboard until
`ready-for-assignment` appears and asserts `owned` is absent for the ticket.

It then waits for the stub's chat receipt, dumps the dashboard until `delivering` appears,
and again asserts Owned is absent.

The harness opens the ack gate.

It waits for the stub to publish ack and the dashboard to show `owned`.

It validates the launch script is bounded and contains no `Read the ticket` or complete
assignment prose, while the chat event contains both the assignment-file reference and
the exact `LISA_ASSIGNMENT` marker.

## Suppressed-start scenario

The stub launches but never writes `.started`.

It remains interruptible at the terminal. On the first timeout Lisa rotates the lease and
performs same-pane reset/relaunch.

The replacement stub also suppresses start.

The harness waits for a named startup failure and verifies:

- no `ready-for-assignment`, `delivering`, or `owned` observation;
- exactly two launch attempts total;
- one physical pane ID across attempts;
- strictly increasing generations;
- completion within a fixed wall-clock limit.

This exercises finite relaunch behavior in addition to missing-start non-ownership.

## Suppressed-chat-ack scenario

The stub publishes exact process start and accepts every chat line but never writes ack.

The harness observes ReadyForAssignment followed by Delivering.

It waits for the bounded retry and terminal `delivery-failed` state.

Assertions require exactly one initial chat plus one retry, a single launch, no Owned, and
completion within the wall-clock bound.

## `dquote>` scenario

On generation 1 the stub schedules delayed input to its own pane and exits zero.

After the launch script returns to the parent zsh, the helper submits an unterminated
double quote. The harness requires the terminal dump to contain `dquote>`.

No generation-1 started signal or ownership is emitted.

Lisa's deadline sends Ctrl-C and the exact shell probe. A valid probe rotates to
generation 2 and relaunches the normal stub in the same pane.

The replacement publishes start, receives chat, and waits for the ack gate. The harness
asserts ReadyForAssignment and Delivering are still non-Owned, then opens the gate and
requires Owned.

Final assertions require:

- two launches and no third;
- generation 2 greater than generation 1;
- identical pane ID for both launches;
- real terminal evidence of `dquote>` before recovery;
- no spare pane receives the ticket;
- matching generation-2 ack is the only ownership trigger.

## Dashboard observation

Use `zellij action list-panes --json` to discover terminal and plugin IDs instead of
assuming numeric allocation.

Use `dump-screen` against the plugin pane and strip ANSI control bytes before matching.

Polling helpers retain the latest dump on timeout and print all fixture logs for diagnosis.

State matching is ticket-scoped where the rendered layout permits it. At minimum, each
scenario has only one ticket, so the single scheduler status is unambiguous.

## Wall-clock bounds

Every wait helper takes an explicit timeout.

The scenario process itself is additionally wrapped in a larger deadline when a portable
`timeout` implementation is available; internal wait loops remain authoritative on macOS.

Expected durations account for the five-second scheduler poll and one-second configured
ack deadline:

- success: roughly 10–20 seconds;
- missing ack: roughly 20–30 seconds;
- missing start: roughly 25–40 seconds;
- `dquote>` recovery: roughly 20–35 seconds.

No loop is unbounded.

## Failure diagnostics

On failure the harness prints:

- scenario name;
- Zellij pane JSON;
- terminal dump;
- dashboard dump;
- stub event log;
- signal directory listing and contents;
- attempt leases and launch scripts;
- captured loop client output.

This makes the regression useful in unattended environments.

## Pre-fix sensitivity

The T-034-03-02 behavior published Owned immediately after fresh dispatch despite no
provider process.

The success test's pre-start and pre-ack non-Owned assertions would fail there.

The `dquote>` scenario would also fail because the old implementation had no bounded
start-only same-pane reset and would retain phantom ownership.

Thus the harness is not merely a post-fix smoke test.

## Verification decision

Run the ignored integration test against the freshly compiled Lisa binary and embedded
release WASM.

Also run the ordinary workspace tests, format check, WASM check, shell syntax check, and
diff whitespace check.

## Final decision

Implement Option 3 as a source-retained shell harness plus ignored Cargo wrapper. Keep all
provider control deterministic through files and exact leases, use the dashboard only for
scheduler-owned state observations, and require real zsh continuation evidence for the
recovery scenario.
