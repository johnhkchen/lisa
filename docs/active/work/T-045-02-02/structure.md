# Structure — T-045-02-02 zellij-injects-launcher-only

## Change inventory

Two ticket-owned source files will be modified:

1. `crates/lisa-plugin/src/adapter.rs`
2. `crates/lisa-plugin/src/lib.rs`

No source file will be created or deleted.
No CLI source change is required because `lisa launch-codex` already exists.
No core type, config, manifest, template, hook, or documentation file is changed.
Phase artifacts remain private under this attempt work directory.

## `adapter.rs` responsibility

This module remains the provider-specific command-construction boundary.
It will own how a published assignment path changes a provider's fresh launch.
It will not perform filesystem I/O or invoke processes.
It will continue to return shell descriptions to the scheduler.

### Trait interface

Change:

```rust
fn launch_command(&self, ctx: &SpawnContext) -> String;
```

to:

```rust
fn launch_command(&self, ctx: &SpawnContext, assignment_path: &Path) -> String;
```

The new parameter means an atomically published, pane-addressable assignment path.
The trait documentation will state that launch construction occurs after publication.
It will distinguish this path from `assignment_text`, which produces the bytes.

The path will be required rather than optional.
There is no default implementation because both current adapters must acknowledge
the interface explicitly.

### `ClaudeCodeAdapter`

Its implementation will name the new parameter `_assignment_path`.
It will continue returning `build_claude_command(...)` exactly as before.
No Claude command component will be added, removed, or reordered.
No Claude reset, readiness, signals, or reuse method will change.

This explicit unused parameter demonstrates that the scheduler's publication ordering
is shared while transport remains provider-specific.

### `CodexAdapter::model_flag`

The method will retain its current returned fragment:

```text
 --model '<model>'
```

Its comment will describe the Lisa launcher line rather than direct interactive Codex.
No parsing or model validation is added.

### `CodexAdapter::interactive_line`

Change its signature to accept `assignment_path: &Path`.
Keep it private to `CodexAdapter`.
Continue returning one shell command `String`.

The environment prefix remains:

- `LISA_BIN`;
- `LISA_AGENT_CLIENT=codex`;
- `LISA_PANE_ID`;
- `LISA_TICKET_ID`;
- `LISA_ATTEMPT_ID`.

Replace the direct child segment:

```text
codex <safety flags> <model>
```

with:

```text
<lisa_bin> launch-codex <model> -- <assignment_path>
```

The exact resolved `lisa_bin` appears twice:

- once as the `LISA_BIN` environment value inherited by hooks;
- once as the executable starting the native launcher.

Both appearances use `shell_quote` independently.
The assignment path uses `assignment_path.to_string_lossy()` only at this shell edge,
consistent with existing path command construction in the plugin.
The result is wrapped with `shell_quote`.
The downstream native launcher retains `PathBuf`/`OsString` identity after Clap parsing.

The optional model fragment is emitted before ` -- `.
The assignment path follows the separator as the only positional argument.
The existing `|| { mkdir ...; date ... > pane-<id>.error; }` tail remains.

### `CodexAdapter` trait implementation

`launch_command` forwards both inputs to `interactive_line`.
`assignment_text`, `reuse_prompt`, `reset_strategy`, signals, and readiness are unchanged.

### Adapter test helper

Add a local stable path helper or constant such as:

```text
.lisa/attempts/T-042-01/1/work/assignment-1-17.md
```

Every adapter test invoking `launch_command` supplies this path.
`SpawnContext` remains unchanged.

### Adapter assertion updates

Claude tests continue exact equality with `build_claude_command`.
Codex tests change their direct-command expectations to the hidden launcher.
They will cover:

- resolved absolute Lisa binary is invoked;
- bare `lisa` fallback is invoked when configuration is absent;
- lifecycle environment is unchanged;
- `launch-codex` is present;
- `--model` is present only when routed;
- `--` precedes the quoted assignment path;
- direct `codex --dangerously...` text is absent;
- full workflow body text is absent;
- error marker is retained.

Routing tests will look for `launch-codex` rather than ` codex `.
The pending-delivery test will continue proving no `LISA_ASSIGNMENT` marker is placed
in the launcher line.

## `lib.rs` responsibility

This module remains the scheduler, publication coordinator, and Zellij input boundary.
It will guarantee publication precedes launch construction.
It will not construct Codex-specific CLI fragments.

### Launch ordering pattern

Every fresh launch site will use this structural order:

1. build provider assignment bytes;
2. call `prepare_assignment`;
3. retain the returned `AssignmentRef` or run existing error handling;
4. translate `assignment.path` with `strip_host_prefix`;
5. call `adapter.launch_command(&ctx, &translated_path)`;
6. call `prepare_fresh_launch` with that payload;
7. send only the returned script invocation to the pane.

The retained `State::assignment_refs` map remains populated by step 2.
No caller directly inserts or replaces map entries.

### `acknowledge_shell_ready`

This is the same-pane startup recovery relaunch path.
Replace its current `if let Err(...)` publication check with a `match`.
On success, retain the successor attempt's `AssignmentRef`.
On error, keep the current `fail_startup_recovery` behavior and return `false`.

Translate the returned path and pass it to `launch_command`.
The successor lease marker publication remains after launch-script preparation and
before pane submission, preserving current ordering.

### `schedule_ready_tickets`

This is the primary dispatch path.
Replace its success-only assignment publication with a retained result.
On error, preserve current lease revocation, activity logging, unscheduled count,
and loop continuation.

Compute the translated assignment path once per ticket before launch branching.
All three command-producing branches use the same value:

- recycle / cross-provider or Codex `ExitThenFresh`;
- `FreshExec` reuse;
- empty-pane fresh launch.

The `ClearHandshake` reuse branch does not call `launch_command`.
It keeps its existing Claude `reuse_prompt` behavior.

No changes are made to:

- `reused_seat` calculation;
- `recycle` calculation;
- `/exit` delivery;
- `/clear` delivery;
- transition states;
- `fresh_launch` calculation;
- seat readiness classification;
- thread creation;
- activity/provenance recording.

### `check_transition_timeouts` exit-ready path

This is the normal launch-after-`/exit` path for a resident Codex seat.
Retain the `AssignmentRef` returned for `launch_lease`.
Preserve separate error handling for recovery and non-recovery cases.
Translate the returned path.
Pass it into `launch_command` before atomic script publication.

The resulting command is still sent through `send_line_to_pane`.
The slot still becomes a live session with `last_client = route.agent`.
Recovery seat and readiness state handling remain unchanged.

### Other `SpawnContext` sites

Contexts used only for:

- assignment reference delivery;
- `/clear` reuse prompt delivery;
- clear-timeout reuse delivery;

do not call `launch_command` and need no assignment path.
They remain structurally unchanged.

### Existing scheduler tests

Call sites inside production helpers compile through the new trait signature.
Assertions that read Codex launch scripts will be updated to expect the filename path.
Assertions that reject assignment body content remain.
Claude script assertions remain unchanged.

## New two-ticket stub-pane fixture

Add one focused `#[test]` near the existing scheduler fixture helpers and launch tests.
Its name will describe the acceptance contract, for example:

```text
codex_stub_panes_receive_only_fresh_per_ticket_launcher_lines
```

### Fixture setup

Use `consecutive_reuse_state(AgentClient::Codex, "T-LAUNCH", &[10, 11])`
or a closely scoped equivalent.
Convert the two fixture slots to empty shell panes:

- `has_session = false`;
- `last_client = None`.

Set `config.lisa_bin` to a recognizable path containing a space if supported by
the existing config type.
Limit the fixture DAG to the first schedulable wave through `max_threads = 2`.
Call `schedule_ready_tickets()` once.

Native test builds provide no-op Zellij host functions.
This makes the call an in-process stub-pane run while preserving the production
`send_line_to_pane` code path and pending Enter queue.

### Fixture observations

Collect the two active `(ticket_id, pane_id)` pairs.
For each pair:

1. load the current attempt lease;
2. load `State::assignment_refs[ticket_id]`;
3. read the assignment body;
4. locate `.lisa-launch-<pane>.sh` in the exact attempt work directory;
5. read the script;
6. locate the matching `ActivityEvent::SessionLaunch` command;
7. compare its script path with the expected atomic destination.

### Fixture assertions

The queued Enter collection contains one entry per pane.
Each recorded pane command begins with `sh ` and names only its script.
No pane command contains `Read the ticket`, `AGENTS.md`, or assignment bytes.
Each script contains `launch-codex` and its own exact quoted assignment path.
Each script excludes `Read the ticket` and the rest of the body.
Each script excludes direct ` codex --dangerously` invocation.
Each slot is assigned to Codex and enters a fresh `Starting` state.
The two assignment paths differ.
The two launch-script paths differ.
No `WaitingForClear` state appears.

This fixture owns no external executable and starts no real process.
It verifies the producer values at the Zellij stub boundary.

## Unchanged modules

`crates/lisa-cli/src/codex_launcher.rs` remains unchanged.
Its existing black-box test continues to prove native argv identity.

`crates/lisa-plugin/src/assignment.rs` remains unchanged.
Its atomic writer remains the source of exact references.

`crates/lisa-plugin/src/codex_ack.rs` remains unchanged.
Hook acknowledgment is not the ownership mechanism redesigned here.

`crates/lisa-core` remains unchanged.
There is no new serialized contract.

Claude templates, Codex hooks, init scaffolding, layout generation, dashboard labels,
claim command, completion path, and lease revocation remain unchanged.

## Commit boundary

The implementation is one meaningful scheduler/adapter transport unit.
Both files participate in one compiling interface change and should be committed together:

```text
crates/lisa-plugin/src/adapter.rs
crates/lisa-plugin/src/lib.rs
```

The commit message will describe Codex assignment launch transport.
The isolated transaction will include exactly those two repository-relative paths.
No phase artifact will be included; Lisa publishes those after Review.

## Verification boundary

Focused checks:

- adapter tests;
- new stub-pane fixture by exact test name;
- prerequisite CLI launcher integration test.

Package checks:

- `cargo test -p lisa-plugin`;
- `cargo test -p lisa-cli --test codex_launcher`.

Repository checks:

- `cargo fmt --all -- --check`;
- `cargo test --workspace`;
- `just check`;
- `git diff --check`;
- ticket-owned cleanliness audit after isolated commit.

No real Codex or Zellij run is claimed by this structure.
